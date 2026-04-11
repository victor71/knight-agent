//! IPC Contract implementation

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::contract::IPCContract;
use crate::error::{IPCError, IPCResult};
use crate::registry::AwaitRegistry;
use crate::types::{
    NotificationMessage, PendingQuery, RequestMessage, ResponseMessage, StreamChunkMessage,
    UserQueryMessage, UserResponseData,
};

/// Message envelope for internal queue
#[derive(Debug, Clone)]
enum Envelope {
    Request(RequestMessage),
    Response(ResponseMessage),
    Notification(NotificationMessage),
    StreamChunk(StreamChunkMessage),
    UserQuery(UserQueryMessage),
    UserResponse(String, UserResponseData), // await_id, response
}

/// IPC configuration
#[derive(Debug, Clone)]
pub struct IPCConfig {
    pub max_message_size: usize,
    pub message_timeout: u64,
    pub queue_size: usize,
    pub default_query_timeout: u64,
    pub max_concurrent_queries: usize,
}

impl Default for IPCConfig {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            message_timeout: 300000,            // 5 minutes
            queue_size: 1000,
            default_query_timeout: 300000, // 5 minutes
            max_concurrent_queries: 10,
        }
    }
}

/// IPC Contract implementation
#[derive(Clone)]
pub struct IPCContractImpl {
    name: String,
    config: IPCConfig,
    initialized: Arc<RwLock<bool>>,
    connected: Arc<RwLock<bool>>,
    await_registry: AwaitRegistry,
    outbound_tx: Arc<RwLock<Option<mpsc::Sender<Envelope>>>>,
    inbound_tx: Arc<RwLock<Option<mpsc::Sender<Vec<u8>>>>>,
}

impl IPCContractImpl {
    /// Create a new IPC contract implementation
    pub fn new() -> Result<Self, IPCError> {
        Ok(Self {
            name: "ipc-contract".to_string(),
            config: IPCConfig::default(),
            initialized: Arc::new(RwLock::new(false)),
            connected: Arc::new(RwLock::new(false)),
            await_registry: AwaitRegistry::new(),
            outbound_tx: Arc::new(RwLock::new(None)),
            inbound_tx: Arc::new(RwLock::new(None)),
        })
    }

    /// Create with custom config
    pub fn with_config(config: IPCConfig) -> Result<Self, IPCError> {
        Ok(Self {
            name: "ipc-contract".to_string(),
            config,
            initialized: Arc::new(RwLock::new(false)),
            connected: Arc::new(RwLock::new(false)),
            await_registry: AwaitRegistry::new(),
            outbound_tx: Arc::new(RwLock::new(None)),
            inbound_tx: Arc::new(RwLock::new(None)),
        })
    }

    /// Get await registry
    pub fn await_registry(&self) -> &AwaitRegistry {
        &self.await_registry
    }

    /// Get config
    pub fn config(&self) -> &IPCConfig {
        &self.config
    }

    /// Serialize message to JSON
    fn serialize_message(&self, envelope: &Envelope) -> IPCResult<Vec<u8>> {
        let value = match envelope {
            Envelope::Request(req) => serde_json::to_value(req)?,
            Envelope::Response(resp) => serde_json::to_value(resp)?,
            Envelope::Notification(notif) => serde_json::to_value(notif)?,
            Envelope::StreamChunk(chunk) => serde_json::to_value(chunk)?,
            Envelope::UserQuery(query) => serde_json::to_value(query)?,
            Envelope::UserResponse(await_id, response) => {
                let mut map = serde_json::Map::new();
                map.insert("await_id".to_string(), serde_json::json!(await_id));
                map.insert("response".to_string(), serde_json::to_value(response)?);
                serde_json::Value::Object(map)
            }
        };

        let bytes = serde_json::to_vec(&value)?;
        if bytes.len() > self.config.max_message_size {
            return Err(IPCError::InvalidRequest(format!(
                "Message too large: {} bytes",
                bytes.len()
            )));
        }

        Ok(bytes)
    }

    /// Deserialize message from JSON
    fn deserialize_message(&self, data: &[u8]) -> IPCResult<Envelope> {
        if data.len() > self.config.max_message_size {
            return Err(IPCError::ParseError(format!(
                "Message too large: {} bytes",
                data.len()
            )));
        }

        let value: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| IPCError::ParseError(format!("JSON parse error: {}", e)))?;

        // Determine message type
        let msg_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IPCError::ParseError("Missing message type".to_string()))?;

        match msg_type {
            "request" => {
                let req: RequestMessage = serde_json::from_value(value)
                    .map_err(|e| IPCError::ParseError(format!("Request parse error: {}", e)))?;
                Ok(Envelope::Request(req))
            }
            "response" => {
                let resp: ResponseMessage = serde_json::from_value(value)
                    .map_err(|e| IPCError::ParseError(format!("Response parse error: {}", e)))?;
                Ok(Envelope::Response(resp))
            }
            "notification" => {
                let notif: NotificationMessage = serde_json::from_value(value).map_err(|e| {
                    IPCError::ParseError(format!("Notification parse error: {}", e))
                })?;
                Ok(Envelope::Notification(notif))
            }
            "stream_chunk" => {
                let chunk: StreamChunkMessage = serde_json::from_value(value)
                    .map_err(|e| IPCError::ParseError(format!("StreamChunk parse error: {}", e)))?;
                Ok(Envelope::StreamChunk(chunk))
            }
            "user_query" => {
                let query: UserQueryMessage = serde_json::from_value(value)
                    .map_err(|e| IPCError::ParseError(format!("UserQuery parse error: {}", e)))?;
                Ok(Envelope::UserQuery(query))
            }
            "user_response" => {
                // Handle user response from client
                let await_id = value
                    .get("await_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| IPCError::ParseError("Missing await_id".to_string()))?
                    .to_string();
                let response: UserResponseData = serde_json::from_value(
                    value
                        .get("response")
                        .ok_or_else(|| IPCError::ParseError("Missing response".to_string()))?
                        .clone(),
                )
                .map_err(|e| {
                    IPCError::ParseError(format!("UserResponseData parse error: {}", e))
                })?;
                Ok(Envelope::UserResponse(await_id, response))
            }
            _ => Err(IPCError::ParseError(format!(
                "Unknown message type: {}",
                msg_type
            ))),
        }
    }
}

#[async_trait::async_trait]
impl IPCContract for IPCContractImpl {
    fn new() -> Result<Self, IPCError> {
        Self::new()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_initialized(&self) -> bool {
        self.initialized
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    async fn initialize(&self) -> IPCResult<()> {
        if *self.initialized.read().await {
            return Ok(());
        }

        // Set up message channels
        let (outbound_tx, _) = mpsc::channel(self.config.queue_size);
        *self.outbound_tx.write().await = Some(outbound_tx);

        let (inbound_tx, _) = mpsc::channel(self.config.queue_size);
        *self.inbound_tx.write().await = Some(inbound_tx);

        *self.initialized.write().await = true;
        tracing::info!("IPC Contract initialized");
        Ok(())
    }

    async fn connect(&self) -> IPCResult<()> {
        if !*self.initialized.read().await {
            return Err(IPCError::NotInitialized);
        }

        *self.connected.write().await = true;
        tracing::info!("IPC Contract connected");
        Ok(())
    }

    async fn disconnect(&self) -> IPCResult<()> {
        *self.connected.write().await = false;
        tracing::info!("IPC Contract disconnected");
        Ok(())
    }

    async fn send_request(&self, request: RequestMessage) -> IPCResult<ResponseMessage> {
        if !*self.connected.read().await {
            return Err(IPCError::ConnectionFailed("Not connected".to_string()));
        }

        // Serialize and queue the request
        let envelope = Envelope::Request(request);
        let data = self.serialize_message(&envelope)?;

        // In a real implementation, this would send to transport and wait for response
        // For now, we'll return a mock response
        tracing::debug!("Sent request: {}", data.len());

        // Mock response
        Ok(ResponseMessage::success(
            "msg-1".to_string(),
            serde_json::json!({"status": "ok"}),
        ))
    }

    async fn send_notification(&self, notification: NotificationMessage) -> IPCResult<()> {
        if !*self.connected.read().await {
            return Err(IPCError::ConnectionFailed("Not connected".to_string()));
        }

        let envelope = Envelope::Notification(notification);
        let data = self.serialize_message(&envelope)?;

        tracing::debug!("Sent notification: {}", data.len());
        Ok(())
    }

    async fn subscribe_stream(&self, request_id: String) -> IPCResult<()> {
        tracing::debug!("Subscribed to stream: {}", request_id);
        Ok(())
    }

    async fn unsubscribe_stream(&self, request_id: String) -> IPCResult<()> {
        tracing::debug!("Unsubscribed from stream: {}", request_id);
        Ok(())
    }

    async fn send_user_query(&self, query: UserQueryMessage) -> IPCResult<String> {
        if !*self.connected.read().await {
            return Err(IPCError::ConnectionFailed("Not connected".to_string()));
        }

        // Register the await
        let await_id = self.await_registry.register(query.clone()).await?;

        // Send the query message
        let envelope = Envelope::UserQuery(query);
        let _data = self.serialize_message(&envelope)?;

        tracing::debug!("Sent user query: {}", await_id);
        Ok(await_id)
    }

    async fn send_user_response(
        &self,
        await_id: String,
        response: UserResponseData,
    ) -> IPCResult<()> {
        if !*self.connected.read().await {
            return Err(IPCError::ConnectionFailed("Not connected".to_string()));
        }

        // Verify await exists
        self.await_registry.get(&await_id).await?;

        // Send response
        let envelope = Envelope::UserResponse(await_id.clone(), response);
        let _data = self.serialize_message(&envelope)?;

        // Remove from registry (user has responded)
        let _ = self.await_registry.remove(&await_id).await;

        tracing::debug!("Sent user response: {}", await_id);
        Ok(())
    }

    async fn cancel_user_query(&self, await_id: String) -> IPCResult<()> {
        self.await_registry.cancel(&await_id).await?;
        tracing::debug!("Cancelled user query: {}", await_id);
        Ok(())
    }

    async fn list_pending_queries(
        &self,
        session_id: Option<String>,
    ) -> IPCResult<Vec<PendingQuery>> {
        let queries = match session_id {
            Some(session_id) => self.await_registry.list_by_session(&session_id).await,
            None => self.await_registry.list_all().await,
        };
        Ok(queries)
    }

    async fn handle_message(&self, data: &[u8]) -> IPCResult<()> {
        let envelope = self.deserialize_message(data)?;

        match envelope {
            Envelope::Request(request) => {
                tracing::debug!("Received request: {}", request.method);
                // Handle request...
            }
            Envelope::Response(response) => {
                tracing::debug!("Received response for: {}", response.request_id);
                // Handle response...
            }
            Envelope::Notification(notification) => {
                tracing::debug!("Received notification: {}", notification.event);
                // Handle notification...
            }
            Envelope::StreamChunk(chunk) => {
                tracing::debug!("Received stream chunk: {}", chunk.sequence);
                // Handle stream chunk...
            }
            Envelope::UserQuery(query) => {
                tracing::debug!("Received user query: {}", query.await_id);
                // Register user query (from agent)
                let _ = self.await_registry.register(query).await;
            }
            Envelope::UserResponse(await_id, _response) => {
                tracing::debug!("Received user response: {}", await_id);
                // Route response to waiting agent
                // In real implementation, this would notify the agent runtime
                let _ = self.await_registry.remove(&await_id).await;
            }
        }

        Ok(())
    }

    async fn next_message(&self) -> IPCResult<Vec<u8>> {
        Err(IPCError::ReceiveFailed("No message available".to_string()))
    }
}
