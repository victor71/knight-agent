//! External Agent Manager
//!
//! Manages external agent processes and lifecycle.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock as AsyncRwLock;
use tokio::time::Duration;
use tracing::{debug, info};

use crate::types::*;

/// Managed process information
pub struct ManagedProcess {
    pub child: Child,
    pub id: String,
    pub state: ProcessState,
    pub started_at: std::time::Instant,
    pub config: ExternalAgentConfig,
    pub output_buffer: Vec<String>,
}

impl ManagedProcess {
    pub fn new(id: String, child: Child, config: ExternalAgentConfig) -> Self {
        Self {
            child,
            id,
            state: ProcessState::Starting,
            started_at: std::time::Instant::now(),
            config,
            output_buffer: Vec::new(),
        }
    }
}

/// External agent manager
pub struct ExternalAgentManager {
    /// Managed processes (agent_id -> ManagedProcess)
    processes: Arc<AsyncRwLock<HashMap<String, ManagedProcess>>>,
    /// Known agent definitions
    agent_definitions: HashMap<String, AgentDefinition>,
}

impl ExternalAgentManager {
    /// Create a new manager
    pub fn new() -> Self {
        let mut definitions = HashMap::new();

        // Claude Code
        definitions.insert(
            "claude-code".to_string(),
            AgentDefinition::new(
                "claude-code",
                "Claude Code",
                "claude",
                "https://docs.anthropic.com/en/docs/claude-code",
                r#"
Claude Code 安装指南:

macOS:
  brew install anthropic/claude-code/claude-code

Linux:
  npm install -g @anthropic-ai/claude-code

Windows:
  npm install -g @anthropic-ai/claude-code

安装后验证:
  claude --version
"#,
            ),
        );

        Self {
            processes: Arc::new(AsyncRwLock::new(HashMap::new())),
            agent_definitions: definitions,
        }
    }

    /// Discover available external agents
    pub async fn discover(&self) -> Vec<DiscoveredAgent> {
        let mut results = Vec::new();

        for def in self.agent_definitions.values() {
            let discovered = self.check_agent(def).await;
            results.push(discovered);
        }

        results
    }

    /// Check if a specific agent type is available
    pub async fn check_availability(&self, agent_type: &str) -> DiscoveredAgent {
        if let Some(def) = self.agent_definitions.get(agent_type) {
            self.check_agent(def).await
        } else {
            DiscoveredAgent::new(agent_type, agent_type)
                .with_unavailable("Unknown agent type", None)
        }
    }

    /// Check an agent definition
    async fn check_agent(&self, def: &AgentDefinition) -> DiscoveredAgent {
        let path = self.find_executable(&def.command).await;

        match path {
            Some(path) => {
                let version = self.get_version(&path, &def.version_flags).await;
                DiscoveredAgent::new(&def.agent_type, &def.name)
                    .with_installed(true, Some(path), version)
            }
            None => DiscoveredAgent::new(&def.agent_type, &def.name)
                .with_unavailable("Not found in PATH", Some(def.install_url.clone())),
        }
    }

    /// Find executable in PATH
    async fn find_executable(&self, command: &str) -> Option<String> {
        // Try to run the command with --version
        if let Ok(output) = Command::new(command).arg("--version").output().await {
            if output.status.success() {
                return Some(command.to_string());
            }
        }

        // Check common installation paths
        #[cfg(windows)]
        {
            let windows_paths = vec![
                format!("C:\\Program Files\\Claude\\bin\\{}.exe", command),
            ];
            for path in windows_paths {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }

        None
    }

    /// Get version of an executable
    async fn get_version(&self, path: &str, version_flags: &[String]) -> Option<String> {
        let mut cmd = Command::new(path);

        if version_flags.is_empty() {
            cmd.arg("--version");
        } else {
            for flag in version_flags {
                cmd.arg(flag);
            }
        }

        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Self::parse_version(&stdout);
            }
        }

        None
    }

    /// Parse version string
    fn parse_version(output: &str) -> Option<String> {
        // Simple regex-like parsing for version numbers
        let parts: Vec<&str> = output.split_whitespace().collect();
        for part in parts {
            if part.contains('.') && part.chars().all(|c| c.is_ascii_digit() || c == '.') {
                let components: Vec<&str> = part.split('.').collect();
                if components.len() >= 2 && components.iter().all(|p| p.parse::<u32>().is_ok()) {
                    return Some(part.to_string());
                }
            }
        }
        None
    }

    /// Get install instructions for an agent type
    pub fn get_install_instructions(&self, agent_type: &str) -> Option<String> {
        self.agent_definitions
            .get(agent_type)
            .map(|def| def.install_instructions.clone())
    }

    /// Spawn an external agent
    pub async fn spawn(
        &self,
        config: &ExternalAgentConfig,
        task: &str,
    ) -> ExternalAgentResult<String> {
        let agent_id = format!("{}-{}", config.agent_type, uuid::Uuid::new_v4());

        // Build command
        let mut cmd = Command::new(&config.command);
        for arg in &config.args {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Configure stdin/stdout
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        // Spawn process
        let mut child = cmd
            .spawn()
            .map_err(|e| ExternalAgentError::ProcessSpawnFailed(e.to_string()))?;

        // Get stdin
        let stdin = child
            .stdin
            .take()
            .ok_or(ExternalAgentError::StdinNotAvailable)?;

        // Send initial task
        let mut stdin_writer = tokio::io::BufWriter::new(stdin);
        stdin_writer
            .write_all(task.as_bytes())
            .await
            .map_err(|e| ExternalAgentError::CommunicationFailed(e.to_string()))?;
        stdin_writer
            .flush()
            .await
            .map_err(|e| ExternalAgentError::CommunicationFailed(e.to_string()))?;

        // Create managed process
        let mut managed = ManagedProcess::new(agent_id.clone(), child, config.clone());
        managed.state = ProcessState::Running;

        // Store process
        let mut processes = self.processes.write().await;
        processes.insert(agent_id.clone(), managed);

        info!("Spawned external agent: {}", agent_id);

        Ok(agent_id)
    }

    /// Send input to an external agent
    pub async fn send_input(
        &self,
        agent_id: &str,
        input: &str,
        is_final: bool,
    ) -> ExternalAgentResult<()> {
        let mut processes = self.processes.write().await;
        let process = processes
            .get_mut(agent_id)
            .ok_or_else(|| ExternalAgentError::ProcessNotFound(agent_id.to_string()))?;

        if let Some(ref mut stdin) = process.child.stdin {
            let mut writer = tokio::io::BufWriter::new(stdin);
            writer
                .write_all(input.as_bytes())
                .await
                .map_err(|e| ExternalAgentError::CommunicationFailed(e.to_string()))?;

            if is_final {
                // Close stdin for final input
                drop(writer);
            } else {
                writer
                    .flush()
                    .await
                    .map_err(|e| ExternalAgentError::CommunicationFailed(e.to_string()))?;
            }
        }

        if is_final {
            process.state = ProcessState::WaitingInput;
        }

        debug!("Sent input to agent: {}", agent_id);
        Ok(())
    }

    /// Get output from an external agent (non-blocking)
    pub async fn get_output(&self, agent_id: &str) -> ExternalAgentResult<(String, bool)> {
        let processes = self.processes.read().await;
        let process = processes
            .get(agent_id)
            .ok_or_else(|| ExternalAgentError::ProcessNotFound(agent_id.to_string()))?;

        let output = process.output_buffer.join("\n");
        let is_complete = matches!(
            process.state,
            ProcessState::Completed | ProcessState::Error | ProcessState::Killed
        );

        Ok((output, is_complete))
    }

    /// Get status of an external agent
    pub async fn get_status(&self, agent_id: &str) -> ExternalAgentResult<ExternalAgentStatus> {
        let mut processes = self.processes.write().await;
        let process = processes
            .get_mut(agent_id)
            .ok_or_else(|| ExternalAgentError::ProcessNotFound(agent_id.to_string()))?;

        let mut status = ExternalAgentStatus::new(agent_id.to_string(), process.state);

        // Try to get exit code if process has ended
        if let Ok(Some(exit_status)) = process.child.try_wait() {
            let exit_code = exit_status.code().unwrap_or(-1);
            status.exit_code = Some(exit_code);
            if exit_code == 0 {
                status.state = ProcessState::Completed;
            } else {
                status.state = ProcessState::Error;
            }
        }

        status.output_lines = process.output_buffer.len() as u64;
        status.started_at = Some(process.started_at.elapsed().as_secs().to_string());

        Ok(status)
    }

    /// Terminate an external agent
    pub async fn terminate(&self, agent_id: &str, force: bool) -> ExternalAgentResult<i32> {
        let mut processes = self.processes.write().await;
        let process = processes
            .get_mut(agent_id)
            .ok_or_else(|| ExternalAgentError::ProcessNotFound(agent_id.to_string()))?;

        if force {
            process
                .child
                .kill()
                .await
                .map_err(|e| ExternalAgentError::ProcessCrashed(e.to_string()))?;
        }

        let exit_code = process
            .child
            .wait()
            .await
            .map_err(|e| ExternalAgentError::ProcessCrashed(e.to_string()))?
            .code()
            .unwrap_or(-1);

        process.state = ProcessState::Killed;
        processes.remove(agent_id);

        info!("Terminated external agent: {} (exit code: {})", agent_id, exit_code);

        Ok(exit_code)
    }

    /// Interrupt an external agent (SIGINT)
    pub async fn interrupt(&self, agent_id: &str) -> ExternalAgentResult<()> {
        let processes = self.processes.read().await;
        let _process = processes
            .get(agent_id)
            .ok_or_else(|| ExternalAgentError::ProcessNotFound(agent_id.to_string()))?;

        // On Unix, we would send SIGINT
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            if let Err(e) = process.child.kill() {
                warn!("Failed to send interrupt to agent {}: {}", agent_id, e);
            }
        }

        debug!("Interrupted external agent: {}", agent_id);
        Ok(())
    }

    /// Wait for agent completion
    pub async fn wait_for_completion(
        &self,
        agent_id: &str,
        timeout_secs: u64,
    ) -> ExternalAgentResult<(Option<i32>, String)> {
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs);

        loop {
            let mut processes = self.processes.write().await;
            if let Some(process) = processes.get_mut(agent_id) {
                let elapsed = start_time.elapsed();
                if elapsed > timeout_duration {
                    return Err(ExternalAgentError::ProcessTimeout);
                }

                // Check if process has ended
                if let Ok(Some(status)) = process.child.try_wait() {
                    let exit_code = status.code();
                    let output = process.output_buffer.join("\n");
                    return Ok((exit_code, output));
                }

                // Check state
                match process.state {
                    ProcessState::Completed | ProcessState::Error | ProcessState::Killed => {
                        let exit_code = if let Ok(Some(status)) = process.child.try_wait() {
                            status.code().unwrap_or(-1)
                        } else {
                            -1
                        };
                        let output = process.output_buffer.join("\n");
                        return Ok((Some(exit_code), output));
                    }
                    _ => {}
                }
            } else {
                return Err(ExternalAgentError::ProcessNotFound(agent_id.to_string()));
            }

            drop(processes);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// List all managed agents
    pub async fn list_agents(&self) -> Vec<String> {
        let processes = self.processes.read().await;
        processes.keys().cloned().collect()
    }

    /// Check if agent exists
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let processes = self.processes.read().await;
        processes.contains_key(agent_id)
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let processes = self.processes.read().await;
        processes.len()
    }

    /// Validate input for dangerous patterns
    pub fn validate_input(&self, input: &str) -> ExternalAgentResult<()> {
        // Size limit
        if input.len() > 1_000_000 {
            return Err(ExternalAgentError::InvalidInput(
                "Input too large".to_string(),
            ));
        }

        // Dangerous patterns
        let dangerous = [
            "rm -rf /",
            "rm -rf /*",
            "format c:",
            "mkfs",
            ":(){:|:&};:",
        ];

        for pattern in dangerous {
            if input.contains(pattern) {
                return Err(ExternalAgentError::InvalidInput(format!(
                    "Dangerous pattern detected: {}",
                    pattern
                )));
            }
        }

        Ok(())
    }
}

impl Default for ExternalAgentManager {
    fn default() -> Self {
        Self::new()
    }
}
