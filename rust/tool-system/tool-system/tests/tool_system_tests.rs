//! Tool System Unit Tests

use std::collections::HashMap;
use tool_system::{
    ExecuteRequest, HandlerType, JsonSchema, JsonSchemaProperty, McpToolDefinition,
    ToolContext, ToolDefinition, ToolExecutionResult, ToolHandler, ToolSystemImpl,
    ToolSystemTrait, ValidationResult,
};

fn create_test_context() -> ToolContext {
    ToolContext {
        session_id: "test-session".to_string(),
        agent_id: "test-agent".to_string(),
        workspace: ".".to_string(),
        variables: HashMap::new(),
    }
}

fn create_test_tool(name: &str, category: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        display_name: name.to_string(),
        description: format!("Test tool: {}", name),
        category: category.to_string(),
        parameters: Default::default(),
        handler: ToolHandler {
            handler_type: HandlerType::Builtin,
            target: String::new(),
            timeout_secs: 30,
        },
        permissions: vec![],
        dangerous: false,
    }
}

// Tool system creation tests

#[test]
fn test_tool_system_creation() {
    let ts = ToolSystemImpl::new().unwrap();
    assert!(ts.is_initialized());
    assert_eq!(ts.name(), "tool-system");
}

#[test]
fn test_tool_system_empty() {
    let ts = ToolSystemImpl::empty().unwrap();
    assert!(ts.is_initialized());
}

// Tool registration tests

#[tokio::test]
async fn test_register_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let tool = create_test_tool("test_tool", "test");

    ts.register_tool(tool).await.unwrap();

    let tools = ts.list_tools().await.unwrap();
    assert!(tools.iter().any(|t| t.name == "test_tool"));
}

#[tokio::test]
async fn test_register_duplicate_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let tool = create_test_tool("duplicate_tool", "test");

    ts.register_tool(tool.clone()).await.unwrap();
    let result = ts.register_tool(tool).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_unregister_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let tool = create_test_tool("unregister_tool", "test");

    ts.register_tool(tool).await.unwrap();
    ts.unregister_tool("unregister_tool").await.unwrap();

    let tool_info = ts.get_tool("unregister_tool").await.unwrap();
    assert!(tool_info.is_none());
}

#[tokio::test]
async fn test_unregister_nonexistent_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let result = ts.unregister_tool("nonexistent").await;
    assert!(result.is_err());
}

// Tool listing tests

#[tokio::test]
async fn test_list_tools() {
    let ts = ToolSystemImpl::new().unwrap();
    let tools = ts.list_tools().await.unwrap();
    // Should include built-in tools
    assert!(!tools.is_empty());
}

#[tokio::test]
async fn test_list_tools_by_category() {
    let ts = ToolSystemImpl::new().unwrap();

    // Register a custom tool
    let tool = create_test_tool("custom_search", "search");
    ts.register_tool(tool).await.unwrap();

    let search_tools = ts.list_tools_by_category("search").await.unwrap();
    assert!(!search_tools.is_empty());
    assert!(search_tools.iter().any(|t| t.name == "custom_search"));
}

#[tokio::test]
async fn test_get_categories() {
    let ts = ToolSystemImpl::new().unwrap();
    let categories = ts.get_categories().await.unwrap();
    assert!(categories.contains(&"builtin".to_string()));
}

// Tool info tests

#[tokio::test]
async fn test_get_tool_existing() {
    let ts = ToolSystemImpl::new().unwrap();
    let info = ts.get_tool("read").await.unwrap();
    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.name, "read");
}

#[tokio::test]
async fn test_get_tool_nonexistent() {
    let ts = ToolSystemImpl::new().unwrap();
    let info = ts.get_tool("nonexistent_tool").await.unwrap();
    assert!(info.is_none());
}

#[tokio::test]
async fn test_get_tool_custom() {
    let ts = ToolSystemImpl::new().unwrap();
    let tool = create_test_tool("custom_get_tool", "custom");
    ts.register_tool(tool).await.unwrap();

    let info = ts.get_tool("custom_get_tool").await.unwrap();
    assert!(info.is_some());
    assert_eq!(info.unwrap().category, "custom");
}

// Built-in tool execution tests

#[tokio::test]
async fn test_execute_builtin_read_missing_args() {
    let ts = ToolSystemImpl::new().unwrap();
    let context = create_test_context();
    let request = ExecuteRequest {
        name: "read".to_string(),
        args: serde_json::json!({}),
        context,
    };

    let result = ts.execute(request).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.error_code, Some("INVALID_ARGS".to_string()));
}

#[tokio::test]
async fn test_execute_nonexistent_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let context = create_test_context();
    let request = ExecuteRequest {
        name: "nonexistent_tool".to_string(),
        args: serde_json::json!({}),
        context,
    };

    let result = ts.execute(request).await;
    assert!(result.is_err());
}

// Validation tests

#[tokio::test]
async fn test_validate_args_valid() {
    let ts = ToolSystemImpl::new().unwrap();
    let args = serde_json::json!({
        "file_path": "/tmp/test.txt"
    });

    let result = ts.validate_args("read", &args).await.unwrap();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[tokio::test]
async fn test_validate_args_invalid_missing_required() {
    let ts = ToolSystemImpl::new().unwrap();
    let args = serde_json::json!({
        "offset": 10
    });

    let result = ts.validate_args("read", &args).await.unwrap();
    assert!(!result.valid);
    assert!(!result.errors.is_empty());
}

#[tokio::test]
async fn test_validate_args_nonexistent_tool() {
    let ts = ToolSystemImpl::new().unwrap();
    let args = serde_json::json!({});

    let result = ts.validate_args("nonexistent", &args).await;
    assert!(result.is_err());
}

// MCP tools tests

#[tokio::test]
async fn test_register_mcp_tools() {
    let ts = ToolSystemImpl::new().unwrap();
    let mcp_tools = vec![
        McpToolDefinition {
            name: "mcp_tool1".to_string(),
            description: "MCP tool 1".to_string(),
            input_schema: serde_json::json!({}),
            server_name: "mcp_server".to_string(),
        },
        McpToolDefinition {
            name: "mcp_tool2".to_string(),
            description: "MCP tool 2".to_string(),
            input_schema: serde_json::json!({}),
            server_name: "mcp_server".to_string(),
        },
    ];

    let count = ts.register_mcp_tools("mcp_server", mcp_tools).await.unwrap();
    assert_eq!(count, 2);
}

// Tool execution result tests

#[test]
fn test_tool_execution_result_success() {
    let result = ToolExecutionResult::success(serde_json::json!({"data": "test"}));
    assert!(result.success);
    assert!(result.data.is_some());
    assert!(result.error.is_none());
    assert!(result.error_code.is_none());
}

#[test]
fn test_tool_execution_result_error() {
    let result = ToolExecutionResult::error("TEST_ERROR", "Something went wrong");
    assert!(!result.success);
    assert!(result.data.is_none());
    assert!(result.error.is_some());
    assert_eq!(result.error_code, Some("TEST_ERROR".to_string()));
}

#[test]
fn test_tool_execution_result_with_duration() {
    let result = ToolExecutionResult::success(serde_json::json!({})).with_duration(100);
    assert_eq!(result.duration_ms, 100);
}

// JSON Schema tests

#[test]
fn test_json_schema_default() {
    let schema = JsonSchema::default();
    assert!(schema.properties.is_empty());
    assert!(schema.required.is_empty());
    assert!(!schema.additional_properties); // default is false
}

#[test]
fn test_json_schema_with_properties() {
    let mut schema = JsonSchema::default();
    schema.schema_type = "object".to_string();
    schema.properties.insert(
        "name".to_string(),
        JsonSchemaProperty {
            property_type: "string".to_string(),
            description: "Name field".to_string(),
            enum_values: None,
        },
    );
    schema.required.push("name".to_string());

    assert_eq!(schema.properties.len(), 1);
    assert!(schema.required.contains(&"name".to_string()));
}

// Validation result tests

#[test]
fn test_validation_result_valid() {
    let result = ValidationResult::valid();
    assert!(result.valid);
    assert!(result.errors.is_empty());
}

#[test]
fn test_validation_result_invalid() {
    let errors = vec![];
    let result = ValidationResult::invalid(errors);
    assert!(!result.valid);
}

#[test]
fn test_validation_result_add_error() {
    let mut result = ValidationResult::valid();
    result.add_error("field1", "Field is required");
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].field, "field1");
}

// ToolDefinition tests

#[test]
fn test_tool_definition_serialization() {
    let tool = create_test_tool("serial_test", "test");
    let json = serde_json::to_string(&tool).unwrap();
    let parsed: ToolDefinition = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "serial_test");
    assert_eq!(parsed.category, "test");
}

#[test]
fn test_tool_definition_dangerous_flag() {
    let tool = ToolDefinition {
        name: "dangerous_tool".to_string(),
        display_name: "Dangerous Tool".to_string(),
        description: "A dangerous tool".to_string(),
        category: "test".to_string(),
        parameters: Default::default(),
        handler: ToolHandler {
            handler_type: HandlerType::Command,
            target: "rm -rf".to_string(),
            timeout_secs: 30,
        },
        permissions: vec![],
        dangerous: true,
    };

    assert!(tool.dangerous);
    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains("dangerous"));
}

// Handler type tests

#[test]
fn test_handler_type_serialization() {
    assert_eq!(
        serde_json::to_string(&HandlerType::Builtin).unwrap(),
        "\"builtin\""
    );
    assert_eq!(
        serde_json::to_string(&HandlerType::Command).unwrap(),
        "\"command\""
    );
    assert_eq!(
        serde_json::to_string(&HandlerType::Skill).unwrap(),
        "\"skill\""
    );
    assert_eq!(
        serde_json::to_string(&HandlerType::Mcp).unwrap(),
        "\"mcp\""
    );
    assert_eq!(
        serde_json::to_string(&HandlerType::Wasm).unwrap(),
        "\"wasm\""
    );
}

// ToolContext tests

#[test]
fn test_tool_context_serialization() {
    let mut variables = HashMap::new();
    variables.insert("key".to_string(), serde_json::json!("value"));

    let context = ToolContext {
        session_id: "session123".to_string(),
        agent_id: "agent456".to_string(),
        workspace: "/workspace".to_string(),
        variables,
    };

    let json = serde_json::to_string(&context).unwrap();
    let parsed: ToolContext = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.session_id, "session123");
    assert_eq!(parsed.agent_id, "agent456");
}
