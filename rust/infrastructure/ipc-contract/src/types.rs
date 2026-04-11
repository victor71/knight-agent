//! IPC message and data types

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

/// Message type enum
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Request,
    Response,
    Notification,
    StreamChunk,
    Error,
    UserQuery,
    UserResponse,
}

impl MessageType {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageType::Request => "request",
            MessageType::Response => "response",
            MessageType::Notification => "notification",
            MessageType::StreamChunk => "stream_chunk",
            MessageType::Error => "error",
            MessageType::UserQuery => "user_query",
            MessageType::UserResponse => "user_response",
        }
    }
}

/// Base message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub timestamp: i64,
    pub session_id: Option<String>,
}

impl BaseMessage {
    /// Create a new base message with current timestamp
    pub fn new(msg_type: MessageType) -> Self {
        Self {
            id: format!("msg-{}", uuid::Uuid::new_v4()),
            msg_type,
            timestamp: Utc::now().timestamp_millis(),
            session_id: None,
        }
    }

    /// With session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Request options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestOptions {
    pub timeout: Option<u64>,
    pub stream: Option<bool>,
    pub priority: Option<i32>,
}

/// Request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub method: String,
    pub params: serde_json::Value,
    pub options: Option<RequestOptions>,
}

/// Error response details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub stack: Option<String>,
}

impl ErrorResponse {
    /// Create from ErrorCode
    pub fn from_error_code(code: ErrorCode) -> Self {
        Self {
            code: code.as_i32(),
            message: code.message().to_string(),
            details: None,
            stack: None,
        }
    }

    /// With custom details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// With stack trace
    pub fn with_stack(mut self, stack: String) -> Self {
        self.stack = Some(stack);
        self
    }
}

/// Response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub request_id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorResponse>,
    pub streaming: Option<bool>,
}

impl ResponseMessage {
    /// Create success response
    pub fn success(request_id: String, result: serde_json::Value) -> Self {
        Self {
            base: BaseMessage::new(MessageType::Response),
            request_id,
            result: Some(result),
            error: None,
            streaming: None,
        }
    }

    /// Create error response
    pub fn error(request_id: String, error: ErrorResponse) -> Self {
        Self {
            base: BaseMessage::new(MessageType::Response),
            request_id,
            result: None,
            error: Some(error),
            streaming: None,
        }
    }

    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub event: String,
    pub data: serde_json::Value,
}

impl NotificationMessage {
    /// Create new notification
    pub fn new(event: String, data: serde_json::Value) -> Self {
        Self {
            base: BaseMessage::new(MessageType::Notification),
            event,
            data,
        }
    }
}

/// Stream chunk message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub request_id: String,
    pub sequence: u64,
    pub chunk: String,
    pub done: bool,
}

impl StreamChunkMessage {
    /// Create new stream chunk
    pub fn new(request_id: String, sequence: u64, chunk: String, done: bool) -> Self {
        Self {
            base: BaseMessage::new(MessageType::StreamChunk),
            request_id,
            sequence,
            chunk,
            done,
        }
    }
}

/// Query type for user interaction
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    Permission,
    Clarification,
    Confirmation,
    Information,
}

impl QueryType {
    pub fn as_str(self) -> &'static str {
        match self {
            QueryType::Permission => "permission",
            QueryType::Clarification => "clarification",
            QueryType::Confirmation => "confirmation",
            QueryType::Information => "information",
        }
    }
}

/// User query dependencies (for cross-agent dependency detection)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryDependencies {
    /// Agents this query depends on
    pub depends_on_agents: Option<Vec<String>>,
    /// Other queries this query depends on
    pub depends_on_queries: Option<Vec<String>>,
    /// Agent this query is waiting for
    pub waiting_for_agent: Option<String>,
}

/// User query context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryContext {
    pub resource: Option<String>,
    pub action: Option<String>,
    pub reason: Option<String>,
}

/// User query message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQueryMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub await_id: String,
    pub query_type: QueryType,
    pub agent_id: String,
    pub message: String,
    pub options: Option<Vec<String>>,
    pub context: QueryContext,
    pub dependencies: Option<QueryDependencies>,
    pub timeout: u64,
    pub created_at: i64,
}

impl UserQueryMessage {
    /// Create new user query
    pub fn new(agent_id: String, query_type: QueryType, message: String, timeout: u64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            base: BaseMessage::new(MessageType::UserQuery),
            await_id: format!("await-{}", uuid::Uuid::new_v4()),
            agent_id,
            query_type,
            message,
            options: None,
            context: QueryContext::default(),
            dependencies: None,
            timeout,
            created_at: now,
        }
    }

    /// With session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.base.session_id = Some(session_id);
        self
    }

    /// With options
    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = Some(options);
        self
    }

    /// With context
    pub fn with_context(mut self, context: QueryContext) -> Self {
        self.context = context;
        self
    }

    /// With dependencies
    pub fn with_dependencies(mut self, dependencies: QueryDependencies) -> Self {
        self.dependencies = Some(dependencies);
        self
    }
}

/// User response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponseData {
    pub accepted: bool,
    pub value: Option<String>,
    pub custom_input: Option<String>,
}

impl UserResponseData {
    /// Create accepted response
    pub fn accepted() -> Self {
        Self {
            accepted: true,
            value: None,
            custom_input: None,
        }
    }

    /// Create rejected response
    pub fn rejected() -> Self {
        Self {
            accepted: false,
            value: None,
            custom_input: None,
        }
    }

    /// With value
    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }

    /// With custom input
    pub fn with_custom_input(mut self, input: String) -> Self {
        self.custom_input = Some(input);
        self
    }
}

/// User response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponseMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub await_id: String,
    pub response: UserResponseData,
    pub responded_at: i64,
}

impl UserResponseMessage {
    /// Create new user response
    pub fn new(await_id: String, response: UserResponseData) -> Self {
        Self {
            base: BaseMessage::new(MessageType::UserResponse),
            await_id,
            response,
            responded_at: Utc::now().timestamp_millis(),
        }
    }

    /// Create timeout response
    pub fn timeout(await_id: String) -> Self {
        Self {
            base: BaseMessage::new(MessageType::UserResponse),
            await_id,
            response: UserResponseData {
                accepted: false,
                value: Some("timeout".to_string()),
                custom_input: None,
            },
            responded_at: Utc::now().timestamp_millis(),
        }
    }

    /// Create cancelled response
    pub fn cancelled(await_id: String) -> Self {
        Self {
            base: BaseMessage::new(MessageType::UserResponse),
            await_id,
            response: UserResponseData {
                accepted: false,
                value: Some("cancelled".to_string()),
                custom_input: None,
            },
            responded_at: Utc::now().timestamp_millis(),
        }
    }
}

/// Pending query info (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuery {
    pub await_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub query_type: QueryType,
    pub message: String,
    pub options: Option<Vec<String>>,
    pub created_at: i64,
    pub timeout: u64,
    pub context: Option<QueryContext>,
    pub dependencies: Option<QueryDependencies>,
}
