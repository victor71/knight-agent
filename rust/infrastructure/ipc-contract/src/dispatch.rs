//! Method Dispatcher
//!
//! Routes incoming IPC requests to handler functions by method name.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{IPCError, IPCResult};
use crate::server::StreamingContext;

/// Method handler function signature
pub type MethodHandler = Arc<
    dyn (Fn(Value) -> Pin<Box<dyn Future<Output = IPCResult<Value>> + Send>>)
        + Send
        + Sync,
>;

/// Streaming method handler function signature
pub type StreamingMethodHandler = Arc<
    dyn (Fn(Value, StreamingContext) -> Pin<Box<dyn Future<Output = IPCResult<Value>> + Send>>)
        + Send
        + Sync,
>;

/// Method dispatcher for routing requests to handlers
pub struct MethodDispatcher {
    handlers: HashMap<String, MethodHandler>,
    streaming_handlers: HashMap<String, StreamingMethodHandler>,
}

impl MethodDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            streaming_handlers: HashMap::new(),
        }
    }

    /// Register a method handler
    pub fn register<F, Fut>(&mut self, method: &str, handler: F) -> IPCResult<()>
    where
        F: (Fn(Value) -> Fut) + Send + Sync + 'static,
        Fut: Future<Output = IPCResult<Value>> + Send + 'static,
    {
        if self.handlers.contains_key(method) || self.streaming_handlers.contains_key(method) {
            return Err(IPCError::MethodNotFound(format!(
                "Method {} already registered",
                method
            )));
        }

        self.handlers.insert(
            method.to_string(),
            Arc::new(move |params| Box::pin(handler(params))),
        );

        Ok(())
    }

    /// Register a streaming method handler
    pub fn register_streaming<F, Fut>(&mut self, method: &str, handler: F) -> IPCResult<()>
    where
        F: (Fn(Value, StreamingContext) -> Fut) + Send + Sync + 'static,
        Fut: Future<Output = IPCResult<Value>> + Send + 'static,
    {
        if self.handlers.contains_key(method) || self.streaming_handlers.contains_key(method) {
            return Err(IPCError::MethodNotFound(format!(
                "Method {} already registered",
                method
            )));
        }

        self.streaming_handlers.insert(
            method.to_string(),
            Arc::new(move |params, ctx| Box::pin(handler(params, ctx))),
        );

        Ok(())
    }

    /// Unregister a method handler
    pub fn unregister(&mut self, method: &str) -> bool {
        self.handlers.remove(method).is_some() || self.streaming_handlers.remove(method).is_some()
    }

    /// Check if method is registered
    pub fn has_method(&self, method: &str) -> bool {
        self.handlers.contains_key(method) || self.streaming_handlers.contains_key(method)
    }

    /// Check if method has streaming handler
    pub fn has_streaming_handler(&self, method: &str) -> bool {
        self.streaming_handlers.contains_key(method)
    }

    /// List all registered methods
    pub fn list_methods(&self) -> Vec<String> {
        let mut methods = self.handlers.keys().cloned().collect::<Vec<_>>();
        methods.extend(self.streaming_handlers.keys().cloned());
        methods
    }

    /// Dispatch a request to the appropriate handler
    pub async fn dispatch(&self, method: &str, params: Value) -> IPCResult<Value> {
        let handler = self
            .handlers
            .get(method)
            .ok_or_else(|| IPCError::MethodNotFound(method.to_string()))?;

        handler(params).await
    }

    /// Dispatch a streaming request to the appropriate handler
    pub async fn dispatch_streaming(&self, method: &str, params: Value, ctx: StreamingContext) -> IPCResult<Value> {
        let handler = self
            .streaming_handlers
            .get(method)
            .ok_or_else(|| IPCError::MethodNotFound(method.to_string()))?;

        handler(params, ctx).await
    }

    /// Get handler count
    pub fn len(&self) -> usize {
        self.handlers.len() + self.streaming_handlers.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty() && self.streaming_handlers.is_empty()
    }
}

impl Default for MethodDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_dispatch() {
        let mut dispatcher = MethodDispatcher::new();

        dispatcher
            .register("echo", |params| async move {
                Ok(params)
            })
            .unwrap();

        let result = dispatcher
            .dispatch("echo", serde_json::json!("hello"))
            .await
            .unwrap();

        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let dispatcher = MethodDispatcher::new();

        let result = dispatcher.dispatch("unknown", serde_json::json!(null)).await;

        assert!(matches!(result, Err(IPCError::MethodNotFound(_))));
    }

    #[tokio::test]
    async fn test_handler_error() {
        let mut dispatcher = MethodDispatcher::new();

        dispatcher
            .register("fail", |_params| async move {
                Err(IPCError::InternalError("handler failed".to_string()))
            })
            .unwrap();

        let result = dispatcher.dispatch("fail", serde_json::json!(null)).await;

        assert!(matches!(result, Err(IPCError::InternalError(_))));
    }

    #[tokio::test]
    async fn test_multiple_handlers() {
        let mut dispatcher = MethodDispatcher::new();

        dispatcher
            .register("add", |params| async move {
                let arr = params
                    .as_array()
                    .ok_or_else(|| IPCError::InvalidRequest("expected array".to_string()))?;
                if arr.len() != 2 {
                    return Err(IPCError::InvalidRequest("expected 2 numbers".to_string()));
                }
                let a = arr[0]
                    .as_i64()
                    .ok_or_else(|| IPCError::InvalidRequest("first not number".to_string()))?;
                let b = arr[1]
                    .as_i64()
                    .ok_or_else(|| IPCError::InvalidRequest("second not number".to_string()))?;
                Ok(serde_json::json!(a + b))
            })
            .unwrap();

        dispatcher
            .register("concat", |params| async move {
                let arr = params
                    .as_array()
                    .ok_or_else(|| IPCError::InvalidRequest("expected array".to_string()))?;
                let strings: Vec<&str> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect();
                Ok(serde_json::json!(strings.join("")))
            })
            .unwrap();

        assert_eq!(
            dispatcher
                .dispatch("add", serde_json::json!([3, 5]))
                .await
                .unwrap(),
            8
        );

        assert_eq!(
            dispatcher
                .dispatch("concat", serde_json::json!(["hello", " ", "world"]))
                .await
                .unwrap(),
            "hello world"
        );

        assert_eq!(dispatcher.len(), 2);
        assert!(dispatcher.has_method("add"));
        assert!(dispatcher.has_method("concat"));
        assert!(!dispatcher.has_method("unknown"));

        let methods = dispatcher.list_methods();
        assert!(methods.contains(&"add".to_string()));
        assert!(methods.contains(&"concat".to_string()));
    }

    #[tokio::test]
    async fn test_unregister() {
        let mut dispatcher = MethodDispatcher::new();

        dispatcher
            .register("temp", |_params| async move { Ok(serde_json::json!(null)) })
            .unwrap();

        assert!(dispatcher.has_method("temp"));
        assert!(dispatcher.unregister("temp"));
        assert!(!dispatcher.has_method("temp"));
        assert!(!dispatcher.unregister("temp")); // Already removed
    }

    #[tokio::test]
    async fn test_duplicate_register() {
        let mut dispatcher = MethodDispatcher::new();

        dispatcher
            .register("dup", |_params| async move { Ok(serde_json::json!(null)) })
            .unwrap();

        let result = dispatcher
            .register("dup", |_params| async move { Ok(serde_json::json!(null)) });

        assert!(matches!(result, Err(IPCError::MethodNotFound(_))));
    }
}
