use crate::types::{FileAction, AccessCheckResult, SandboxConfig, FilesystemSandbox, CommandSandbox, NetworkSandbox};

/// Permission checker helper
pub struct PermissionChecker {
    workspace: String,
    filesystem: FilesystemSandbox,
    command: CommandSandbox,
    network: NetworkSandbox,
}

impl PermissionChecker {
    pub fn new(config: &SandboxConfig) -> Self {
        Self {
            workspace: config.workspace.clone(),
            filesystem: config.filesystem.clone(),
            command: config.command.clone(),
            network: config.network.clone(),
        }
    }

    /// Check file access permission
    pub fn check_file_access(&self, path: &str, action: FileAction) -> AccessCheckResult {
        // Normalize path to absolute
        let abs_path = if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            format!("{}/{}", self.workspace, path)
        };

        // Check denied patterns first
        for pattern in &self.filesystem.denied_patterns {
            if glob_match(pattern, &abs_path) {
                return AccessCheckResult::denied(format!("Path matches denied pattern: {}", pattern));
            }
        }

        // Check if path is in allowed paths
        let allowed = self.filesystem.allowed_paths.iter().any(|p| {
            glob_match(p, &abs_path) || abs_path.starts_with(p.trim_end_matches("**"))
        });

        if !allowed {
            return AccessCheckResult::denied("Path not in allowed paths");
        }

        // Check read-only for write/delete actions
        if matches!(action, FileAction::Write | FileAction::Delete) {
            for ro_path in &self.filesystem.read_only {
                if glob_match(ro_path, &abs_path) {
                    return AccessCheckResult::denied(format!("Path is read-only: {}", ro_path));
                }
            }
        }

        AccessCheckResult::allowed()
    }

    /// Check command execution permission
    pub fn check_command(&self, command: &str, _args: &[String]) -> AccessCheckResult {
        // Check denied commands first
        for denied in &self.command.denied_commands {
            if command.contains(denied) || denied == command {
                return AccessCheckResult::denied(format!("Command in denied list: {}", denied));
            }
        }

        // If whitelist is non-empty, command must be in it
        if !self.command.allowed_commands.is_empty() {
            let allowed = self.command.allowed_commands.iter().any(|c| command.starts_with(c));
            if !allowed {
                return AccessCheckResult::denied("Command not in allowed list");
            }
        }

        AccessCheckResult::allowed()
    }

    /// Check network access permission
    pub fn check_network(&self, host: &str, port: u16) -> AccessCheckResult {
        if !self.network.enabled {
            return AccessCheckResult::denied("Network access is disabled");
        }

        // Check denied hosts
        for denied in &self.network.denied_hosts {
            if host.contains(denied) || denied == host {
                return AccessCheckResult::denied(format!("Host in denied list: {}", denied));
            }
        }

        // If whitelist is non-empty, host must be in it
        if !self.network.allowed_hosts.is_empty() {
            let allowed = self.network.allowed_hosts.iter().any(|h| host.contains(h) || h == host);
            if !allowed {
                return AccessCheckResult::denied("Host not in allowed list");
            }
        }

        // Check port ranges
        if !self.network.allowed_ports.is_empty() {
            let port_allowed = self.network.allowed_ports.iter().any(|r| port >= r.start && port <= r.end);
            if !port_allowed {
                return AccessCheckResult::denied(format!("Port {} not in allowed ranges", port));
            }
        }

        AccessCheckResult::allowed()
    }
}

/// Simple glob pattern matching
pub fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim();
    let path = path.trim();

    // Match everything
    if pattern == "**/*" || pattern == "*" {
        return true;
    }

    // Match .env file anywhere
    if pattern == "**/.env" {
        return path == ".env" || path.ends_with("/.env") || path.contains("/.env/");
    }

    // Match .git directory anywhere
    if pattern == "**/.git/**" {
        return path.starts_with(".git/") ||
               path.contains("/.git/") ||
               path == ".git" ||
               path.starts_with(".git") ||
               path.ends_with("/.git") ||
               path.contains("/.git/");
    }

    // Match **/*.rs - any .rs file anywhere
    if pattern == "**/*.rs" {
        return path.ends_with(".rs");
    }

    // Match /foo/** pattern - /foo and anything under it
    if pattern.ends_with("/**") {
        let base = &pattern[..pattern.len() - 3];
        return path == base ||
               path.starts_with(&format!("{}/", base)) ||
               path.starts_with(base) ||
               path.contains(&format!("{}/", base));
    }

    // Handle * glob (matches any filename, not across /)
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for part in &parts {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = path[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
        return pos == path.len();
    }

    // Exact match or prefix match
    pattern == path || path.starts_with(&format!("{}/", pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("**/*.rs", "foo/bar/baz.rs"));
        assert!(glob_match("**/.env", ".env"));
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("**/*", "anything/here.txt"));
        assert!(!glob_match("**/.git/**", "src/main.rs"));
        assert!(glob_match("**/.git/**", ".git/config"));
    }

    #[test]
    fn test_permission_checker_file() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // Basic access should be allowed
        let result = checker.check_file_access("/tmp/test.txt", FileAction::Read);
        assert!(result.allowed);
    }

    #[test]
    fn test_permission_checker_denied_path() {
        let mut config = SandboxConfig::default();
        config.filesystem.denied_patterns.push("**/.env".to_string());

        let checker = PermissionChecker::new(&config);
        let result = checker.check_file_access("/project/.env", FileAction::Read);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_readonly() {
        let mut config = SandboxConfig::default();
        config.filesystem.read_only.push("/protected/**".to_string());

        let checker = PermissionChecker::new(&config);
        let result = checker.check_file_access("/protected/file.txt", FileAction::Write);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_command() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // rm -rf / should be denied
        let result = checker.check_command("rm -rf /", &[]);
        assert!(!result.allowed);
    }

    #[test]
    fn test_permission_checker_network() {
        let config = SandboxConfig::default();
        let checker = PermissionChecker::new(&config);

        // Network enabled by default, should allow
        let result = checker.check_network("api.example.com", 443);
        assert!(result.allowed);
    }
}
