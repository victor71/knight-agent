//! Await registry for user interaction management

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{IPCError, IPCResult};
use crate::types::{PendingQuery, QueryContext, QueryDependencies, QueryType, UserQueryMessage};

/// Information stored in the await registry
#[derive(Debug, Clone)]
pub struct AwaitInfo {
    pub await_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub query_type: QueryType,
    pub message: String,
    pub options: Option<Vec<String>>,
    pub context: QueryContext,
    pub dependencies: Option<QueryDependencies>,
    pub created_at: DateTime<Utc>,
    pub timeout: u64,
}

impl AwaitInfo {
    /// Check if this await has timed out
    pub fn is_timeout(&self) -> bool {
        if self.timeout == 0 {
            return false; // No timeout
        }
        let elapsed = Utc::now().signed_duration_since(self.created_at);
        elapsed.num_milliseconds() as u64 > self.timeout
    }

    /// Convert to pending query
    pub fn to_pending_query(&self) -> PendingQuery {
        PendingQuery {
            await_id: self.await_id.clone(),
            agent_id: self.agent_id.clone(),
            session_id: self.session_id.clone(),
            query_type: self.query_type,
            message: self.message.clone(),
            options: self.options.clone(),
            created_at: self.created_at.timestamp_millis(),
            timeout: self.timeout,
            context: Some(self.context.clone()),
            dependencies: self.dependencies.clone(),
        }
    }
}

impl From<UserQueryMessage> for AwaitInfo {
    fn from(query: UserQueryMessage) -> Self {
        Self {
            await_id: query.await_id,
            agent_id: query.agent_id,
            session_id: query.base.session_id,
            query_type: query.query_type,
            message: query.message,
            options: query.options,
            context: query.context,
            dependencies: query.dependencies,
            created_at: DateTime::<Utc>::from_timestamp_millis(query.created_at)
                .unwrap_or(Utc::now()),
            timeout: query.timeout,
        }
    }
}

/// Await registry for managing user interaction queries
#[derive(Clone)]
pub struct AwaitRegistry {
    // await_id -> AwaitInfo
    entries: Arc<RwLock<HashMap<String, AwaitInfo>>>,
    // Agent index: agent_id -> Vec await_id
    by_agent: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // Session index: session_id -> Vec await_id
    by_session: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AwaitRegistry {
    /// Create new await registry
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            by_agent: Arc::new(RwLock::new(HashMap::new())),
            by_session: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new await
    pub async fn register(&self, query: UserQueryMessage) -> IPCResult<String> {
        let await_id = query.await_id.clone();
        let agent_id = query.agent_id.clone();
        let session_id = query.base.session_id.clone();
        let info = AwaitInfo::from(query);

        // Store entry
        self.entries
            .write()
            .await
            .insert(await_id.clone(), info.clone());

        // Update agent index
        self.by_agent
            .write()
            .await
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(await_id.clone());

        // Update session index
        if let Some(session_id) = session_id {
            self.by_session
                .write()
                .await
                .entry(session_id)
                .or_insert_with(Vec::new)
                .push(await_id.clone());
        }

        tracing::debug!("Registered await: {}", await_id);
        Ok(await_id)
    }

    /// Get await info
    pub async fn get(&self, await_id: &str) -> IPCResult<AwaitInfo> {
        self.entries
            .read()
            .await
            .get(await_id)
            .cloned()
            .ok_or_else(|| IPCError::UserInteractionError(format!("Await not found: {}", await_id)))
    }

    /// Remove await (when user responds or times out)
    pub async fn remove(&self, await_id: &str) -> IPCResult<AwaitInfo> {
        let info = self.get(await_id).await?;

        // Remove from entries
        self.entries.write().await.remove(await_id);

        // Remove from agent index
        if let Some(ids) = self.by_agent.write().await.get_mut(&info.agent_id) {
            ids.retain(|id| id != await_id);
            if ids.is_empty() {
                self.by_agent.write().await.remove(&info.agent_id);
            }
        }

        // Remove from session index
        if let Some(session_id) = &info.session_id {
            if let Some(ids) = self.by_session.write().await.get_mut(session_id) {
                ids.retain(|id| id != await_id);
                if ids.is_empty() {
                    self.by_session.write().await.remove(session_id);
                }
            }
        }

        tracing::debug!("Removed await: {}", await_id);
        Ok(info)
    }

    /// List all pending queries
    pub async fn list_all(&self) -> Vec<PendingQuery> {
        self.entries
            .read()
            .await
            .values()
            .map(|info| info.to_pending_query())
            .collect()
    }

    /// List pending queries by agent
    pub async fn list_by_agent(&self, agent_id: &str) -> Vec<PendingQuery> {
        let await_ids = self
            .by_agent
            .read()
            .await
            .get(agent_id)
            .cloned()
            .unwrap_or_default();

        let entries = self.entries.read().await;
        await_ids
            .iter()
            .filter_map(|id| entries.get(id).map(|info| info.to_pending_query()))
            .collect()
    }

    /// List pending queries by session
    pub async fn list_by_session(&self, session_id: &str) -> Vec<PendingQuery> {
        let await_ids = self
            .by_session
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();

        let entries = self.entries.read().await;
        await_ids
            .iter()
            .filter_map(|id| entries.get(id).map(|info| info.to_pending_query()))
            .collect()
    }

    /// Cancel an await (returns removed info)
    pub async fn cancel(&self, await_id: &str) -> IPCResult<AwaitInfo> {
        self.remove(await_id).await
    }

    /// Get count of pending queries
    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Get count of pending queries by agent
    pub async fn count_by_agent(&self, agent_id: &str) -> usize {
        self.by_agent
            .read()
            .await
            .get(agent_id)
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// Clean up timed out awaits
    pub async fn cleanup_timeouts(&self) -> Vec<AwaitInfo> {
        let timed_out: Vec<String> = self
            .entries
            .read()
            .await
            .iter()
            .filter(|(_, info)| info.is_timeout())
            .map(|(id, _)| id.clone())
            .collect();

        let mut removed = Vec::new();
        for await_id in timed_out {
            if let Ok(info) = self.remove(&await_id).await {
                removed.push(info);
            }
        }

        removed
    }

    /// Check for circular dependencies in awaits
    pub async fn detect_circular_dependencies(&self) -> Vec<(String, String)> {
        // Returns pairs of (agent_a, agent_b) that have circular dependency
        let mut circular = Vec::new();
        let entries = self.entries.read().await;

        // Build dependency graph
        let mut waiting_for: HashMap<String, Vec<String>> = HashMap::new();
        for info in entries.values() {
            if let Some(deps) = &info.dependencies {
                if let Some(waiting) = &deps.waiting_for_agent {
                    waiting_for
                        .entry(info.agent_id.clone())
                        .or_default()
                        .push(waiting.clone());
                }
            }
        }

        // Detect cycles (simple pair detection)
        let agents: Vec<_> = waiting_for.keys().cloned().collect();
        for agent_a in &agents {
            if let Some(waiting) = waiting_for.get(agent_a) {
                for agent_b in waiting {
                    if let Some(other_waiting) = waiting_for.get(agent_b) {
                        if other_waiting.contains(agent_a) {
                            circular.push((agent_a.clone(), agent_b.clone()));
                        }
                    }
                }
            }
        }

        circular
    }
}

impl Default for AwaitRegistry {
    fn default() -> Self {
        Self::new()
    }
}
