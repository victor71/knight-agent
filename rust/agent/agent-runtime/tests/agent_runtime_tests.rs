//! Agent Runtime Tests
//!
//! Unit tests for the agent-runtime module.

use agent_runtime::{
    AgentRuntimeImpl, AgentStatus, AgentState, AgentStatistics,
    ErrorInfo, AwaitInfo, Message, ToolResult, UserResponse,
    AgentRuntimeError, RuntimeConfig,
};

#[tokio::test]
async fn test_runtime_new() {
    let runtime = AgentRuntimeImpl::new();
    assert!(!runtime.is_initialized());
    assert_eq!(runtime.agent_count().await, 0);
}

#[tokio::test]
async fn test_runtime_initialize() {
    let mut runtime = AgentRuntimeImpl::new();
    assert!(!runtime.is_initialized());

    runtime.initialize().await.unwrap();
    assert!(runtime.is_initialized());
}

#[tokio::test]
async fn test_runtime_with_config() {
    let config = RuntimeConfig {
        max_execution_time: 600,
        max_tool_calls: 100,
        max_llm_calls: 50,
        max_retry_attempts: 5,
        retry_delay_ms: 2000,
        llm_timeout_secs: 120,
        tool_timeout_secs: 60,
    };

    let runtime = AgentRuntimeImpl::with_config(config.clone());
    assert!(!runtime.is_initialized());
}

#[tokio::test]
async fn test_create_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    assert!(agent.id.contains("test-agent"));
    assert_eq!(agent.definition_id, "test-agent");
    assert_eq!(agent.session_id, "session-1");
    assert!(agent.variant.is_none());
    assert_eq!(agent.state.status, AgentStatus::Idle);
}

#[tokio::test]
async fn test_create_agent_with_variant() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent(
            "code-reviewer".to_string(),
            "session-1".to_string(),
            Some("quick".to_string()),
        )
        .await
        .unwrap();

    assert_eq!(agent.variant, Some("quick".to_string()));
}

#[tokio::test]
async fn test_get_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let created = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    let retrieved = runtime.get_agent(&created.id).await.unwrap();
    assert_eq!(retrieved.id, created.id);
    assert_eq!(retrieved.definition_id, created.definition_id);
}

#[tokio::test]
async fn test_get_agent_not_found() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let result = runtime.get_agent("nonexistent").await;
    assert!(result.is_err());
    match result {
        Err(AgentRuntimeError::AgentNotFound(id)) => assert_eq!(id, "nonexistent"),
        _ => panic!("Expected AgentNotFound error"),
    }
}

#[tokio::test]
async fn test_start_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.start_agent(&agent.id).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Thinking);
}

#[tokio::test]
async fn test_start_agent_invalid_state() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    // Start the agent first
    runtime.start_agent(&agent.id).await.unwrap();

    // Try to start again - should fail
    let result = runtime.start_agent(&agent.id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.start_agent(&agent.id).await.unwrap();
    runtime.stop_agent(&agent.id, false).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Stopped);
}

#[tokio::test]
async fn test_pause_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.pause_agent(&agent.id).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Paused);
}

#[tokio::test]
async fn test_resume_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.pause_agent(&agent.id).await.unwrap();
    runtime.resume_agent(&agent.id).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Idle);
}

#[tokio::test]
async fn test_send_message() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    let msg = Message::user("Hello");
    let response = runtime.send_message(&agent.id, msg, false).await.unwrap();

    assert_eq!(response.role, agent_runtime::MessageRole::Assistant);
}

#[tokio::test]
async fn test_update_variables() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    let mut vars = serde_json::Map::new();
    vars.insert("name".to_string(), serde_json::json!("test"));
    vars.insert("count".to_string(), serde_json::json!(42));

    runtime.update_variables(&agent.id, vars).await.unwrap();

    let context = runtime.get_context(&agent.id).await.unwrap();
    assert_eq!(context.get_variable("name"), Some(&serde_json::json!("test")));
    assert_eq!(context.get_variable("count"), Some(&serde_json::json!(42)));
}

#[tokio::test]
async fn test_list_agents() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    runtime
        .create_agent("agent-1".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime
        .create_agent("agent-2".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    let agents = runtime.list_agents().await;
    assert_eq!(agents.len(), 2);
}

#[tokio::test]
async fn test_has_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    assert!(runtime.has_agent(&agent.id).await);
    assert!(!runtime.has_agent("nonexistent").await);
}

#[tokio::test]
async fn test_remove_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    assert!(runtime.has_agent(&agent.id).await);

    runtime.remove_agent(&agent.id).await.unwrap();
    assert!(!runtime.has_agent(&agent.id).await);
}

#[tokio::test]
async fn test_cancel_operation() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.start_agent(&agent.id).await.unwrap();

    let cancelled = runtime.cancel_operation(&agent.id).await.unwrap();
    assert!(cancelled.is_none());

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Idle);
}

#[tokio::test]
async fn test_handle_user_response() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    // First put agent in awaiting state
    let await_id = runtime
        .await_user(&agent.id, "confirmation", "Approve this action?")
        .await
        .unwrap();

    let response = UserResponse::new(&await_id, serde_json::json!("yes"), true);
    let resumed_state = runtime.handle_user_response(&agent.id, response).await.unwrap();

    assert_eq!(resumed_state, "thinking");

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Thinking);
}

#[tokio::test]
async fn test_record_llm_call() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime
        .record_llm_call(&agent.id, 100)
        .await
        .unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.statistics.llm_calls, 1);
    assert_eq!(state.statistics.total_tokens, 100);
}

#[tokio::test]
async fn test_complete_agent() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    runtime.start_agent(&agent.id).await.unwrap();
    runtime.complete(&agent.id).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Idle);
    assert!(state.current_action.is_none());
}

#[tokio::test]
async fn test_set_error() {
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await.unwrap();

    let agent = runtime
        .create_agent("test-agent".to_string(), "session-1".to_string(), None)
        .await
        .unwrap();

    let error = ErrorInfo::new("TEST_ERROR", "This is a test error");
    runtime.set_error(&agent.id, error).await.unwrap();

    let state = runtime.get_agent_state(&agent.id).await.unwrap();
    assert_eq!(state.status, AgentStatus::Error);
    assert!(state.error.is_some());
    assert_eq!(state.statistics.errors, 1);
}

#[tokio::test]
async fn test_agent_state_transitions() {
    let state = AgentState::new();
    assert_eq!(state.status, AgentStatus::Idle);

    let state = state.with_status(AgentStatus::Thinking);
    assert_eq!(state.status, AgentStatus::Thinking);

    let state = state.with_action("test action".to_string());
    assert_eq!(state.current_action, Some("test action".to_string()));
}

#[tokio::test]
async fn test_message_roles() {
    let user_msg = Message::user("Hello");
    assert_eq!(user_msg.role, agent_runtime::MessageRole::User);

    let assistant_msg = Message::assistant("Hi there");
    assert_eq!(assistant_msg.role, agent_runtime::MessageRole::Assistant);

    let system_msg = Message::system("System prompt");
    assert_eq!(system_msg.role, agent_runtime::MessageRole::System);

    let tool_msg = Message::tool("Tool result");
    assert_eq!(tool_msg.role, agent_runtime::MessageRole::Tool);
}

#[tokio::test]
async fn test_tool_result() {
    let success = ToolResult::success(serde_json::json!({"result": "ok"}));
    assert!(success.success);
    assert!(success.error.is_none());

    let failure = ToolResult::failure("Something went wrong");
    assert!(!failure.success);
    assert_eq!(failure.error, Some("Something went wrong".to_string()));

    let with_duration = ToolResult::success(serde_json::json!({})).with_duration(100);
    assert_eq!(with_duration.duration_ms, 100);
}

#[tokio::test]
async fn test_error_info() {
    let error = ErrorInfo::new("CODE", "Message");
    assert_eq!(error.code, "CODE");
    assert_eq!(error.message, "Message");
    assert!(!error.retryable);

    let with_details = error.with_details(serde_json::json!({"key": "value"}));
    assert!(with_details.details.is_some());

    let retryable = ErrorInfo::new("RATE_LIMIT", "Rate limited").with_retryable(true);
    assert!(retryable.retryable);
}

#[tokio::test]
async fn test_await_info() {
    let await_info = AwaitInfo::new("await-123", "confirmation", "Continue?");
    assert_eq!(await_info.await_id, "await-123");
    assert_eq!(await_info.query_type, "confirmation");
    assert_eq!(await_info.message, "Continue?");
    assert!(!await_info.created_at.is_empty());
}

#[tokio::test]
async fn test_agent_statistics() {
    let mut stats = AgentStatistics::new();
    stats.increment_messages_sent();
    stats.increment_messages_received();
    stats.increment_tools_called();
    stats.increment_llm_calls();
    stats.add_tokens(500);
    stats.increment_errors();

    assert_eq!(stats.messages_sent, 1);
    assert_eq!(stats.messages_received, 1);
    assert_eq!(stats.tools_called, 1);
    assert_eq!(stats.llm_calls, 1);
    assert_eq!(stats.total_tokens, 500);
    assert_eq!(stats.errors, 1);
}
