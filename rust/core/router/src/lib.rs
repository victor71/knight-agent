//! Router
//!
//! Design Reference: docs/03-module-design/core/router.md

#![allow(unused)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Router not initialized")]
    NotInitialized,
    #[error("Route not found: {0}")]
    RouteNotFound(String),
    #[error("Routing failed: {0}")]
    RoutingFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub handler: String,
    pub middleware: Vec<String>,
}

#[async_trait]
pub trait Router: Send + Sync {
    fn new() -> Result<Self, RouterError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn register_route(&self, route: Route) -> Result<(), RouterError>;
    async fn route(&self, path: &str) -> Result<Route, RouterError>;
}

pub struct RouterImpl;

impl Router for RouterImpl {
    fn new() -> Result<Self, RouterError> {
        Ok(RouterImpl)
    }

    fn name(&self) -> &str {
        "router"
    }

    fn is_initialized(&self) -> bool {
        false
    }

    async fn register_route(&self, _route: Route) -> Result<(), RouterError> {
        Ok(())
    }

    async fn route(&self, path: &str) -> Result<Route, RouterError> {
        Err(RouterError::RouteNotFound(path.to_string()))
    }
}
