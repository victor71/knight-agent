//! IPC Transport Layer
//!
//! Defines the transport abstraction and TCP implementation.

use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;

use crate::codec::FrameCodec;
use crate::error::{IPCError, IPCResult};

/// Transport trait for IPC communication
#[async_trait]
pub trait Transport: Send + Sync {
    /// Type of the incoming connection
    type Incoming: Stream<Item = IPCResult<Self::Connection>> + Send + Unpin;

    /// Type of the connection
    type Connection: Connection + Send + Unpin;

    /// Bind to address and return incoming stream
    async fn bind(&self, addr: SocketAddr) -> IPCResult<Self::Incoming>;

    /// Connect to remote address
    async fn connect(&self, addr: &SocketAddr) -> IPCResult<Self::Connection>;
}

/// Connection trait for bidirectional messaging
#[async_trait]
pub trait Connection: Send + Sync {
    /// Send a message
    async fn send(&mut self, msg: String) -> IPCResult<()>;

    /// Receive a message
    async fn recv(&mut self) -> IPCResult<Option<String>>;

    /// Get peer address
    fn peer_addr(&self) -> IPCResult<SocketAddr>;

    /// Close the connection
    async fn close(&mut self) -> IPCResult<()>;
}

/// Framed TCP connection using length-prefixed JSON frames
pub type FramedTcpStream = Framed<TcpStream, FrameCodec>;

/// TCP transport implementation
#[derive(Debug, Clone)]
pub struct TcpTransport;

impl TcpTransport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP connection wrapper
pub struct TcpConnection {
    framed: FramedTcpStream,
    peer_addr: SocketAddr,
}

impl TcpConnection {
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            framed: Framed::new(stream, FrameCodec::new()),
            peer_addr,
        }
    }

    /// Get the underlying framed stream
    pub fn framed(&mut self) -> &mut FramedTcpStream {
        &mut self.framed
    }
}

#[async_trait]
impl Connection for TcpConnection {
    async fn send(&mut self, msg: String) -> IPCResult<()> {
        self.framed
            .send(msg)
            .await
            .map_err(|e| IPCError::SendFailed(e.to_string()))?;
        Ok(())
    }

    async fn recv(&mut self) -> IPCResult<Option<String>> {
        self.framed
            .next()
            .await
            .transpose()
            .map_err(|e| IPCError::ReceiveFailed(e.to_string()))
    }

    fn peer_addr(&self) -> IPCResult<SocketAddr> {
        Ok(self.peer_addr)
    }

    async fn close(&mut self) -> IPCResult<()> {
        use tokio::io::AsyncWriteExt;
        self.framed.get_mut().shutdown().await
            .map_err(|e| IPCError::SendFailed(e.to_string()))?;
        Ok(())
    }
}

/// Incoming connections stream for TCP
pub struct TcpIncoming {
    pub listener: TcpListener,
}

impl TcpIncoming {
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }
}

impl futures::Stream for TcpIncoming {
    type Item = IPCResult<TcpConnection>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.listener.poll_accept(cx) {
            std::task::Poll::Ready(Ok((stream, addr))) => {
                std::task::Poll::Ready(Some(Ok(TcpConnection::new(stream, addr))))
            }
            std::task::Poll::Ready(Err(e)) => {
                std::task::Poll::Ready(Some(Err(IPCError::ConnectionFailed(e.to_string()))))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    type Incoming = TcpIncoming;
    type Connection = TcpConnection;

    async fn bind(&self, addr: SocketAddr) -> IPCResult<Self::Incoming> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| IPCError::ConnectionFailed(format!("Failed to bind {}: {}", addr, e)))?;
        Ok(TcpIncoming::new(listener))
    }

    async fn connect(&self, addr: &SocketAddr) -> IPCResult<Self::Connection> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| IPCError::ConnectionFailed(format!("Failed to connect {}: {}", addr, e)))?;
        Ok(TcpConnection::new(stream, *addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_echo() {
        let transport = TcpTransport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        // Bind and get actual address
        let mut incoming = transport.bind(addr).await.unwrap();
        let actual_addr = incoming.listener.local_addr().unwrap();

        // Spawn acceptor task
        tokio::spawn(async move {
            if let Some(Ok(mut conn)) = incoming.next().await {
                if let Ok(Some(msg)) = conn.recv().await {
                    conn.send(format!("echo: {}", msg)).await.unwrap();
                }
            }
        });

        // Connect and send
        let mut client = transport.connect(&actual_addr).await.unwrap();
        client.send("hello".to_string()).await.unwrap();

        let response = client.recv().await.unwrap().unwrap();
        assert_eq!(response, "echo: hello");
    }

    #[tokio::test]
    async fn test_tcp_multiple_messages() {
        let transport = TcpTransport::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let mut incoming = transport.bind(addr).await.unwrap();
        let actual_addr = incoming.listener.local_addr().unwrap();

        // Spawn echo server
        tokio::spawn(async move {
            if let Some(Ok(mut conn)) = incoming.next().await {
                while let Ok(Some(msg)) = conn.recv().await {
                    if conn.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        });

        let mut client = transport.connect(&actual_addr).await.unwrap();

        for i in 0..5 {
            let msg = format!("message_{}", i);
            client.send(msg.clone()).await.unwrap();
            let response = client.recv().await.unwrap().unwrap();
            assert_eq!(response, msg);
        }
    }

    #[tokio::test]
    async fn test_connection_refused() {
        let transport = TcpTransport::new();
        let addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();

        let result = transport.connect(&addr).await;
        assert!(result.is_err());
    }
}
