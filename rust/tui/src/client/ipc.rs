//! IPC-based Daemon Client
//!
//! Implements DaemonClient trait using IPC transport to communicate
//! with the daemon process.

use async_trait::async_trait;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use tokio::sync::{mpsc, Mutex};

use crate::event::{AppEvent, SystemStatusSnapshot};
use crate::state::SessionListItem;

use super::{DaemonClient, DaemonClientError, DaemonClientResult};

/// IPC-based daemon client
pub struct IpcDaemonClient {
    /// IPC client for communication (wrapped in Mutex for mutable access)
    ipc_client: Arc<Mutex<ipc_contract::IpcClient>>,
    /// Daemon address
    daemon_addr: String,
}

impl IpcDaemonClient {
    /// Create a new IPC daemon client and connect to daemon
    pub async fn new(daemon_addr: String) -> Result<Self, DaemonClientError> {
        // Parse address
        let socket_addr: std::net::SocketAddr = daemon_addr.parse()
            .map_err(|e| DaemonClientError::ConnectionFailed(format!("Invalid address: {}", e)))?;

        // Create client config
        let config = ipc_contract::IpcClientConfig {
            server_addr: socket_addr,
            connect_timeout_ms: 5000,
            request_timeout_ms: 30000,
            event_channel_size: 100,
        };

        // Create client
        let mut ipc_client = ipc_contract::IpcClient::new(config);

        // Connect to daemon
        ipc_client.connect()
            .await
            .map_err(|e| DaemonClientError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            ipc_client: Arc::new(Mutex::new(ipc_client)),
            daemon_addr,
        })
    }

    /// Try to connect to an existing daemon, returning error if failed
    pub async fn try_connect(daemon_addr: &str) -> Result<Self, DaemonClientError> {
        Self::new(daemon_addr.to_string()).await
    }
}

#[async_trait]
impl DaemonClient for IpcDaemonClient {
    fn handle_input(&self, _input: String, _session_id: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<router::HandleInputResult>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            // For now, return a placeholder response
            Err(DaemonClientError::InternalError("Not yet implemented".to_string()))
        })
    }

    fn send_message(&self, _session_id: &str, _content: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            Err(DaemonClientError::InternalError("Not yet implemented".to_string()))
        })
    }

    fn list_sessions(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<Vec<SessionListItem>>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            Ok(vec![])
        })
    }

    fn create_session(&self, _name: Option<String>, _workspace: String) -> Pin<Box<dyn Future<Output = DaemonClientResult<String>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            Ok("default".to_string())
        })
    }

    fn switch_session(&self, _session_id: &str) -> Pin<Box<dyn Future<Output = DaemonClientResult<()>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            Ok(())
        })
    }

    fn get_system_status(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<SystemStatusSnapshot>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement IPC call to daemon
            Ok(SystemStatusSnapshot::default())
        })
    }

    fn subscribe_events(&self) -> Pin<Box<dyn Future<Output = DaemonClientResult<mpsc::UnboundedReceiver<AppEvent>>> + Send>> {
        let client = self.ipc_client.clone();

        Box::pin(async move {
            // TODO: Phase 4 - Implement event subscription via IPC
            let (_tx, rx) = mpsc::unbounded_channel();
            Ok(rx)
        })
    }
}
