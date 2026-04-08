//! IPC Server
//!
//! Listens for connections and dispatches requests to handlers.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_stream::StreamExt;

use crate::dispatch::MethodDispatcher;
use crate::error::{IPCError, IPCResult};
use crate::transport::{Connection, TcpTransport, Transport};
use crate::types::{ErrorResponse, RequestMessage, ResponseMessage};

/// Pending response callback
type ResponseSender = oneshot::Sender<ResponseMessage>;

/// IPC server configuration
#[derive(Debug, Clone)]
pub struct IpcServerConfig {
    /// Bind address
    pub bind_addr: SocketAddr,
    /// Max concurrent connections
    pub max_connections: usize,
    /// Request queue size per connection
    pub request_queue_size: usize,
}

impl Default for IpcServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            max_connections: 100,
            request_queue_size: 100,
        }
    }
}

/// IPC server state
struct IpcServerState {
    dispatcher: MethodDispatcher,
    active_connections: usize,
}

/// IPC server
pub struct IpcServer {
    config: IpcServerConfig,
    state: Arc<RwLock<IpcServerState>>,
    transport: TcpTransport,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

/// Server events
#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// Client connected
    ClientConnected { addr: SocketAddr },
    /// Client disconnected
    ClientDisconnected { addr: SocketAddr },
    /// Request received
    RequestReceived {
        addr: SocketAddr,
        method: String,
        request_id: String,
    },
    /// Response sent
    ResponseSent {
        addr: SocketAddr,
        request_id: String,
    },
    /// Error occurred
    Error { addr: SocketAddr, error: String },
}

impl IpcServer {
    /// Create new IPC server
    pub fn new(config: IpcServerConfig) -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();

        Self {
            config,
            state: Arc::new(RwLock::new(IpcServerState {
                dispatcher: MethodDispatcher::new(),
                active_connections: 0,
            })),
            transport: TcpTransport::new(),
            event_tx,
            shutdown_tx: None,
        }
    }

    /// Register a method handler
    pub async fn register<F, Fut>(&self, method: &str, handler: F) -> IPCResult<()>
    where
        F: (Fn(serde_json::Value) -> Fut) + Send + Sync + 'static,
        Fut: std::future::Future<Output = IPCResult<serde_json::Value>> + Send + 'static,
    {
        let mut state = self.state.write().await;
        state.dispatcher.register(method, handler)
    }

    /// Subscribe to server events
    pub fn subscribe_events(&self) -> mpsc::UnboundedReceiver<ServerEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // Note: In a full implementation, this would use broadcast channel
        // to support multiple subscribers
        rx
    }

    /// Get bound address (after start)
    pub async fn bound_addr(&self) -> Option<SocketAddr> {
        // Would need to store this in state
        None
    }

    /// Start the server
    pub async fn start(&mut self) -> IPCResult<SocketAddr> {
        let mut incoming = self
            .transport
            .bind(self.config.bind_addr)
            .await?;

        // Get actual bound address
        let bound_addr = incoming
            .listener
            .local_addr()
            .map_err(|e: std::io::Error| IPCError::ConnectionFailed(e.to_string()))?;

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let state = self.state.clone();
        let event_tx = self.event_tx.clone();
        let _request_queue_size = self.config.request_queue_size;

        // Spawn accept loop
        tokio::spawn(async move {
            let _connection_count = 0;

            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        tracing::info!("IPC server shutdown requested");
                        break;
                    }
                    result = incoming.next() => {
                        match result {
                            Some(Ok(conn)) => {
                                let peer_addr = conn.peer_addr().unwrap();
                                let _connection_count = 0; // Track for future use

                                let _ = event_tx.send(ServerEvent::ClientConnected {
                                    addr: peer_addr,
                                });

                                let state = state.clone();
                                let event_tx = event_tx.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = handle_connection(
                                        conn,
                                        state,
                                        event_tx,
                                    ).await {
                                        tracing::error!("Connection handler error: {}", e);
                                    }
                                });
                            }
                            Some(Err(e)) => {
                                tracing::error!("Accept error: {}", e);
                            }
                            None => {
                                tracing::info!("Incoming connections stream ended");
                                break;
                            }
                        }
                    }
                }
            }
        });

        tracing::info!("IPC server listening on {}", bound_addr);
        Ok(bound_addr)
    }

    /// Shutdown the server
    pub async fn shutdown(&mut self) -> IPCResult<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }
}

/// Handle a single client connection
async fn handle_connection(
    mut conn: impl Connection,
    state: Arc<RwLock<IpcServerState>>,
    event_tx: mpsc::UnboundedSender<ServerEvent>,
) -> IPCResult<()> {
    let peer_addr = conn.peer_addr()?;

    loop {
        match conn.recv().await {
            Ok(Some(msg_str)) => {
                // Parse request
                let req: RequestMessage = serde_json::from_str(&msg_str)
                    .map_err(|e| IPCError::ParseError(format!("Invalid request: {}", e)))?;

                let request_id = req.base.id.clone();
                let method = req.method.clone();

                let _ = event_tx.send(ServerEvent::RequestReceived {
                    addr: peer_addr,
                    method: method.clone(),
                    request_id: request_id.clone(),
                });

                // Dispatch request
                let result = {
                    let state = state.read().await;
                    state.dispatcher.dispatch(&method, req.params).await
                };

                // Create response
                let response = match result {
                    Ok(result_value) => ResponseMessage::success(request_id.clone(), result_value),
                    Err(e) => ResponseMessage::error(
                        request_id.clone(),
                        ErrorResponse::from_error_code(e.error_code())
                            .with_details(e.to_string().into()),
                    ),
                };

                // Send response
                let response_str = serde_json::to_string(&response)
                    .map_err(|e| IPCError::ParseError(format!("JSON error: {}", e)))?;

                conn.send(response_str).await?;

                let _ = event_tx.send(ServerEvent::ResponseSent {
                    addr: peer_addr,
                    request_id,
                });
            }
            Ok(None) => {
                // Connection closed
                break;
            }
            Err(e) => {
                let _ = event_tx.send(ServerEvent::Error {
                    addr: peer_addr,
                    error: e.to_string(),
                });
                break;
            }
        }
    }

    let _ = event_tx.send(ServerEvent::ClientDisconnected { addr: peer_addr });
    conn.close().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BaseMessage, MessageType};

    #[tokio::test]
    async fn test_server_start() {
        let config = IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);
        let bound_addr = server.start().await.unwrap();

        assert!(bound_addr.ip().is_loopback());
        assert!(bound_addr.port() > 0);

        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_register_handler() {
        let config = IpcServerConfig {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            ..Default::default()
        };

        let mut server = IpcServer::new(config);

        server
            .register("echo", |params| async move { Ok(params) })
            .await
            .unwrap();

        let bound_addr = server.start().await.unwrap();

        // Connect client
        let transport = TcpTransport::new();
        let mut client = transport.connect(&bound_addr).await.unwrap();

        // Send request
        let request = RequestMessage {
            base: BaseMessage::new(MessageType::Request),
            method: "echo".to_string(),
            params: serde_json::json!("hello"),
            options: None,
        };

        let request_str = serde_json::to_string(&request).unwrap();
        client.send(request_str).await.unwrap();

        // Receive response
        let response_str = client.recv().await.unwrap().unwrap();
        let response: ResponseMessage = serde_json::from_str(&response_str).unwrap();

        assert!(response.is_success());
        assert_eq!(response.result, Some(serde_json::json!("hello")));

        server.shutdown().await.unwrap();
    }
}
