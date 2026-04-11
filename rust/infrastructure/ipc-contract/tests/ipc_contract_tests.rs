//! IPC Contract tests

use ipc_contract::{
    ErrorCode, IPCConfig, IPCContract, IPCContractImpl, QueryContext, QueryDependencies, QueryType,
    RequestMessage, RequestOptions, ResponseMessage, UserQueryMessage, UserResponseData,
};

#[tokio::test]
async fn test_ipc_contract_new() {
    let contract = IPCContractImpl::new().unwrap();
    assert_eq!(contract.name(), "ipc-contract");
    assert!(!contract.is_initialized());
}

#[tokio::test]
async fn test_initialize() {
    let contract = IPCContractImpl::new().unwrap();
    contract.initialize().await.unwrap();
    assert!(contract.is_initialized());
}

#[tokio::test]
async fn test_connect_disconnect() {
    let contract = IPCContractImpl::new().unwrap();
    contract.initialize().await.unwrap();
    contract.connect().await.unwrap();
    contract.disconnect().await.unwrap();
}

#[tokio::test]
async fn test_error_codes() {
    // Test error code properties
    assert_eq!(ErrorCode::Timeout.as_i32(), 5);
    assert_eq!(ErrorCode::SessionNotFound.as_i32(), 1000);
    assert_eq!(ErrorCode::AgentNotFound.as_i32(), 2000);
    assert!(ErrorCode::Timeout.is_retryable());
    assert!(!ErrorCode::ParseError.is_retryable());
}

#[tokio::test]
async fn test_base_message() {
    use ipc_contract::{BaseMessage, MessageType};

    let msg = BaseMessage::new(MessageType::Request);
    assert_eq!(msg.msg_type, MessageType::Request);
    assert!(msg.timestamp > 0);
    assert!(msg.id.starts_with("msg-"));

    let msg_with_session = msg.with_session_id("session-1".to_string());
    assert_eq!(msg_with_session.session_id, Some("session-1".to_string()));
}

#[tokio::test]
async fn test_request_message() {
    use ipc_contract::{BaseMessage, MessageType, RequestMessage};

    let base = BaseMessage::new(MessageType::Request);
    let request = RequestMessage {
        base,
        method: "session.create".to_string(),
        params: serde_json::json!({"name": "test"}),
        options: None,
    };

    assert_eq!(request.method, "session.create");
}

#[tokio::test]
async fn test_response_message_success() {
    let response = ResponseMessage::success(
        "req-1".to_string(),
        serde_json::json!({"session_id": "sess-1"}),
    );

    assert!(response.is_success());
    assert_eq!(response.request_id, "req-1");
    assert!(response.error.is_none());
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_response_message_error() {
    use ipc_contract::{ErrorResponse, ResponseMessage};

    let error = ErrorResponse::from_error_code(ErrorCode::SessionNotFound);
    let response = ResponseMessage::error("req-1".to_string(), error);

    assert!(!response.is_success());
    assert_eq!(response.request_id, "req-1");
    assert!(response.error.is_some());
    assert!(response.result.is_none());
}

#[tokio::test]
async fn test_error_response() {
    let error = ipc_contract::ErrorResponse::from_error_code(ErrorCode::Timeout);
    assert_eq!(error.code, 5);
    assert!(error.message.contains("timeout"));

    let with_details = error.with_details(serde_json::json!({"retry_after": 1000}));
    assert!(with_details.details.is_some());
}

#[tokio::test]
async fn test_query_type() {
    assert_eq!(QueryType::Permission.as_str(), "permission");
    assert_eq!(QueryType::Clarification.as_str(), "clarification");
    assert_eq!(QueryType::Confirmation.as_str(), "confirmation");
    assert_eq!(QueryType::Information.as_str(), "information");
}

#[tokio::test]
async fn test_user_query_message() {
    let query = UserQueryMessage::new(
        "agent-1".to_string(),
        QueryType::Permission,
        "Allow file access?".to_string(),
        30000,
    );

    assert_eq!(query.agent_id, "agent-1");
    assert_eq!(query.query_type, QueryType::Permission);
    assert_eq!(query.message, "Allow file access?");
    assert!(query.await_id.starts_with("await-"));

    let with_session = query.with_session_id("session-1".to_string());
    assert_eq!(with_session.base.session_id, Some("session-1".to_string()));

    let with_options = with_session.with_options(vec!["Yes".to_string(), "No".to_string()]);
    assert!(with_options.options.is_some());
}

#[tokio::test]
async fn test_user_response_data() {
    let accepted = UserResponseData::accepted();
    assert!(accepted.accepted);

    let rejected = UserResponseData::rejected();
    assert!(!rejected.accepted);

    let with_value = UserResponseData::accepted().with_value("option-a".to_string());
    assert_eq!(with_value.value, Some("option-a".to_string()));

    let with_custom = UserResponseData::accepted().with_custom_input("custom input".to_string());
    assert_eq!(with_custom.custom_input, Some("custom input".to_string()));
}

#[tokio::test]
async fn test_request_options_default() {
    let options = RequestOptions::default();
    assert!(options.timeout.is_none());
    assert!(options.stream.is_none());
    assert!(options.priority.is_none());
}

#[tokio::test]
async fn test_query_context() {
    let context = QueryContext {
        resource: Some("/path/to/file".to_string()),
        action: Some("delete".to_string()),
        reason: Some("User requested deletion".to_string()),
    };

    assert_eq!(context.resource, Some("/path/to/file".to_string()));
    assert_eq!(context.action, Some("delete".to_string()));
}

#[tokio::test]
async fn test_query_dependencies() {
    let deps = QueryDependencies {
        depends_on_agents: Some(vec!["agent-a".to_string(), "agent-b".to_string()]),
        depends_on_queries: Some(vec!["await-1".to_string()]),
        waiting_for_agent: Some("agent-c".to_string()),
    };

    assert_eq!(deps.depends_on_agents.as_ref().unwrap().len(), 2);
    assert_eq!(deps.waiting_for_agent, Some("agent-c".to_string()));
}

#[tokio::test]
async fn test_await_registry() {
    let registry = ipc_contract::AwaitRegistry::new();

    // Create a simple query for registration
    let query = UserQueryMessage::new(
        "agent-1".to_string(),
        QueryType::Permission,
        "Allow access?".to_string(),
        60000,
    );

    let await_id = registry.register(query).await.unwrap();
    assert!(await_id.starts_with("await-"));

    // Get the await info
    let info = registry.get(&await_id).await.unwrap();
    assert_eq!(info.agent_id, "agent-1");
    assert_eq!(info.query_type, QueryType::Permission);

    // List all
    let all = registry.list_all().await;
    assert_eq!(all.len(), 1);

    // List by agent
    let by_agent = registry.list_by_agent("agent-1").await;
    assert_eq!(by_agent.len(), 1);

    // Test count
    assert_eq!(registry.count().await, 1);
    assert_eq!(registry.count_by_agent("agent-1").await, 1);
}

#[tokio::test]
async fn test_await_info_timeout() {
    use ipc_contract::{AwaitInfo, QueryContext, QueryType};

    // Create an await info with an old timestamp
    let old_timestamp = chrono::Utc::now() - chrono::Duration::milliseconds(100);
    let info = AwaitInfo {
        await_id: "await-test-1".to_string(),
        agent_id: "agent-1".to_string(),
        session_id: None,
        query_type: QueryType::Permission,
        message: "Allow access?".to_string(),
        options: None,
        context: QueryContext::default(),
        dependencies: None,
        created_at: old_timestamp,
        timeout: 1, // 1ms timeout
    };

    // Should be timed out
    assert!(info.is_timeout());

    // Create an await info without timeout (timeout = 0)
    let no_timeout = AwaitInfo {
        timeout: 0,
        ..info.clone()
    };
    assert!(!no_timeout.is_timeout());

    // Create a fresh await info
    let fresh = AwaitInfo {
        created_at: chrono::Utc::now(),
        timeout: 60000,
        ..info.clone()
    };
    assert!(!fresh.is_timeout());
}

#[tokio::test]
async fn test_await_registry_count() {
    use ipc_contract::{AwaitRegistry, BaseMessage, MessageType};

    let registry = AwaitRegistry::new();

    let base = BaseMessage::new(MessageType::UserQuery);
    let mut query1 = UserQueryMessage::new(
        "agent-1".to_string(),
        QueryType::Permission,
        "Query 1".to_string(),
        60000,
    );
    query1.base = base.clone();

    let mut query2 = UserQueryMessage::new(
        "agent-1".to_string(),
        QueryType::Confirmation,
        "Query 2".to_string(),
        60000,
    );
    query2.base = base;

    registry.register(query1).await.unwrap();
    registry.register(query2).await.unwrap();

    assert_eq!(registry.count().await, 2);
    assert_eq!(registry.count_by_agent("agent-1").await, 2);
}

#[tokio::test]
async fn test_ipc_config_default() {
    let config = IPCConfig::default();
    assert_eq!(config.max_message_size, 10 * 1024 * 1024); // 10MB
    assert_eq!(config.message_timeout, 300000); // 5 minutes
    assert_eq!(config.queue_size, 1000);
    assert_eq!(config.default_query_timeout, 300000);
    assert_eq!(config.max_concurrent_queries, 10);
}

#[tokio::test]
async fn test_serialize_deserialize() {
    // Test RequestMessage
    let base = ipc_contract::BaseMessage::new(ipc_contract::MessageType::Request);
    let request = RequestMessage {
        base: base.clone(),
        method: "test.method".to_string(),
        params: serde_json::json!({"key": "value"}),
        options: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RequestMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.method, "test.method");

    // Test ResponseMessage
    let response =
        ResponseMessage::success("req-1".to_string(), serde_json::json!({"result": "ok"}));

    let json = serde_json::to_string(&response).unwrap();
    let parsed: ResponseMessage = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_success());

    // Test UserQueryMessage
    let query = UserQueryMessage::new(
        "agent-1".to_string(),
        QueryType::Permission,
        "Allow?".to_string(),
        30000,
    );

    let json = serde_json::to_string(&query).unwrap();
    let parsed: UserQueryMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.agent_id, "agent-1");

    // Test UserResponseData
    let response_data = UserResponseData::accepted().with_value("yes".to_string());
    let json = serde_json::to_string(&response_data).unwrap();
    let parsed: UserResponseData = serde_json::from_str(&json).unwrap();
    assert!(parsed.accepted);
    assert_eq!(parsed.value, Some("yes".to_string()));
}
