//! Integration tests for sandbox module
//!
//! This file contains all tests for the sandbox module, extracted from
//! the individual source files to maintain clean separation of concerns.

use sandbox::{
    Sandbox, SandboxImpl, SandboxConfig, SandboxStatus, FileAction,
    Violation, ViolationType, ViolationSeverity, PermissionChecker,
    glob_match,
};

use std::collections::HashMap;

// ============================================================================
// Tests from sandbox.rs
// ============================================================================

#[test]
fn test_sandbox_impl_new() {
    let sandbox = SandboxImpl::new();
    assert_eq!(sandbox.name(), "sandbox");
    assert!(!sandbox.is_initialized());
}

#[tokio::test]
async fn test_create_sandbox() {
    let sandbox = SandboxImpl::new();
    let config = SandboxConfig::default();

    let id = sandbox.create_sandbox(config).await.unwrap();
    assert!(!id.is_empty());

    let info = sandbox.get_sandbox(&id).await.unwrap();
    assert!(info.is_some());
    assert_eq!(info.unwrap().status, SandboxStatus::Active);
}

#[tokio::test]
async fn test_destroy_sandbox() {
    let sandbox = SandboxImpl::new();
    let config = SandboxConfig::default();

    let id = sandbox.create_sandbox(config).await.unwrap();
    sandbox.destroy_sandbox(&id).await.unwrap();

    let info = sandbox.get_sandbox(&id).await.unwrap();
    assert_eq!(info.unwrap().status, SandboxStatus::Terminated);
}

#[tokio::test]
async fn test_check_file_access() {
    let sandbox = SandboxImpl::new();
    let config = SandboxConfig::default();

    let id = sandbox.create_sandbox(config).await.unwrap();
    let result = sandbox.check_file_access(&id, "/tmp/test.txt", FileAction::Read).await.unwrap();
    assert!(result.allowed);
}

#[tokio::test]
async fn test_check_command_access() {
    let sandbox = SandboxImpl::new();
    let config = SandboxConfig::default();

    let id = sandbox.create_sandbox(config).await.unwrap();

    // Safe command should be allowed
    let result = sandbox.check_command_access(&id, "git", &["status".to_string()]).await.unwrap();
    assert!(result.allowed);

    // Dangerous command should be denied
    let result = sandbox.check_command_access(&id, "rm -rf /", &[]).await.unwrap();
    assert!(!result.allowed);
}

#[tokio::test]
async fn test_violation_reporting() {
    let sandbox = SandboxImpl::new();
    let config = SandboxConfig::default();

    let id = sandbox.create_sandbox(config).await.unwrap();
    let violation = Violation {
        id: "test-vio-1".to_string(),
        sandbox_id: id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        violation_type: ViolationType::FileAccessDenied,
        severity: ViolationSeverity::Medium,
        description: "Test violation".to_string(),
        details: HashMap::new(),
    };

    sandbox.report_violation(&id, violation).await.unwrap();

    let violations = sandbox.get_violations(&id, None).await.unwrap();
    assert_eq!(violations.len(), 1);
}

// ============================================================================
// Tests from checker.rs
// ============================================================================

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
