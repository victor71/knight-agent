//! Orchestrator Manager
//!
//! Manages agent pool, task allocation, and message routing.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info};

use crate::types::*;

/// Orchestrator implementation
pub struct OrchestratorImpl {
    /// Registered agents (agent_id -> AgentInfo)
    agents: Arc<AsyncRwLock<HashMap<String, AgentInfo>>>,
    /// Agent ID list for round-robin scheduling
    agent_queue: Arc<AsyncRwLock<VecDeque<String>>>,
    /// Topic subscriptions (topic -> Vec<agent_id>)
    subscriptions: Arc<AsyncRwLock<HashMap<String, Vec<String>>>>,
    /// Collaboration groups (collab_id -> Collaboration)
    collaborations: Arc<AsyncRwLock<HashMap<String, Collaboration>>>,
    /// Pending messages for agents
    message_queue: Arc<AsyncRwLock<HashMap<String, Vec<AgentMessage>>>>,
    /// Configuration
    config: Arc<Mutex<OrchestratorConfig>>,
    /// Next agent index for round-robin
    round_robin_index: Arc<AsyncRwLock<usize>>,
}

impl OrchestratorImpl {
    /// Create a new orchestrator
    pub fn new() -> Self {
        Self {
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            agent_queue: Arc::new(AsyncRwLock::new(VecDeque::new())),
            subscriptions: Arc::new(AsyncRwLock::new(HashMap::new())),
            collaborations: Arc::new(AsyncRwLock::new(HashMap::new())),
            message_queue: Arc::new(AsyncRwLock::new(HashMap::new())),
            config: Arc::new(Mutex::new(OrchestratorConfig::default())),
            round_robin_index: Arc::new(AsyncRwLock::new(0)),
        }
    }

    /// Create with custom config
    pub fn with_config(config: OrchestratorConfig) -> Self {
        Self {
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            agent_queue: Arc::new(AsyncRwLock::new(VecDeque::new())),
            subscriptions: Arc::new(AsyncRwLock::new(HashMap::new())),
            collaborations: Arc::new(AsyncRwLock::new(HashMap::new())),
            message_queue: Arc::new(AsyncRwLock::new(HashMap::new())),
            config: Arc::new(Mutex::new(config)),
            round_robin_index: Arc::new(AsyncRwLock::new(0)),
        }
    }

    // ========== Agent Pool Management ==========

    /// Register an agent to the pool
    pub async fn register_agent(&self, agent_info: AgentInfo) -> OrchestratorResult<()> {
        // Get config values before any locks to avoid holding mutex across await
        let max_agents;
        let max_agents_per_session;
        {
            let config = self.config.lock().unwrap();
            max_agents = config.max_agents;
            max_agents_per_session = config.max_agents_per_session;
        }

        let mut agents = self.agents.write().await;
        let agent_id = agent_info.id.clone();

        // Check if agent already exists
        if agents.contains_key(&agent_id) {
            return Err(OrchestratorError::RegistrationFailed(
                format!("Agent {} already registered", agent_id),
            ));
        }

        // Check agent limit
        if agents.len() >= max_agents {
            return Err(OrchestratorError::ResourceLimitExceeded(
                format!("Max agents {} reached", max_agents),
            ));
        }

        // Check per-session limit
        let session_agents = agents.values().filter(|a| a.session_id == agent_info.session_id).count();
        if session_agents >= max_agents_per_session {
            return Err(OrchestratorError::ResourceLimitExceeded(
                format!("Max agents per session {} reached", max_agents_per_session),
            ));
        }

        agents.insert(agent_id.clone(), agent_info);

        // Add to scheduling queue
        let mut queue = self.agent_queue.write().await;
        queue.push_back(agent_id.clone());

        info!("Registered agent: {}", agent_id);
        Ok(())
    }

    /// Unregister an agent from the pool
    pub async fn unregister_agent(&self, agent_id: &str) -> OrchestratorResult<()> {
        let mut agents = self.agents.write().await;

        if agents.remove(agent_id).is_none() {
            return Err(OrchestratorError::AgentNotFound(agent_id.to_string()));
        }

        // Remove from scheduling queue
        let mut queue = self.agent_queue.write().await;
        queue.retain(|id| id != agent_id);

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().await;
        for agents_list in subs.values_mut() {
            agents_list.retain(|id| id != agent_id);
        }

        info!("Unregistered agent: {}", agent_id);
        Ok(())
    }

    /// List agents with optional filtering
    pub async fn list_agents(&self, filter: Option<AgentFilter>) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;

        match filter {
            Some(f) => agents
                .values()
                .filter(|a| {
                    if let Some(session_id) = &f.session_id {
                        if &a.session_id != session_id {
                            return false;
                        }
                    }
                    if let Some(status) = f.status {
                        if a.status != status {
                            return false;
                        }
                    }
                    if let Some(caps) = &f.capabilities {
                        if !caps.iter().all(|c| a.capabilities.contains(c)) {
                            return false;
                        }
                    }
                    if let Some(def_id) = &f.definition_id {
                        if &a.definition_id != def_id {
                            return false;
                        }
                    }
                    if let Some(ref variant) = f.variant {
                        if a.variant.as_ref() != Some(variant) {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect(),
            None => agents.values().cloned().collect(),
        }
    }

    /// Get agent info
    pub async fn get_agent_info(&self, agent_id: &str) -> OrchestratorResult<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))
    }

    /// Update agent status
    pub async fn update_agent_status(&self, agent_id: &str, status: AgentStatus) -> OrchestratorResult<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = status;
            agent.last_active_at = Some(chrono::Utc::now().to_rfc3339());
            Ok(())
        } else {
            Err(OrchestratorError::AgentNotFound(agent_id.to_string()))
        }
    }

    /// Update agent's current task
    pub async fn update_agent_task(&self, agent_id: &str, task_id: Option<String>) -> OrchestratorResult<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.current_task = task_id;
            Ok(())
        } else {
            Err(OrchestratorError::AgentNotFound(agent_id.to_string()))
        }
    }

    // ========== Agent Allocation ==========

    /// Get available agents matching requirements
    pub async fn get_available_agents(&self, filter: Option<AgentFilter>) -> Vec<AgentInfo> {
        let mut agents = self.list_agents(filter).await;
        agents.retain(|a| a.status == AgentStatus::Idle);
        agents
    }

    /// Allocate an available agent for a task
    pub async fn allocate_agent(&self, requirements: &TaskRequirements) -> OrchestratorResult<String> {
        let available = self.get_available_agents(None).await;

        // Filter by requirements
        let candidates: Vec<AgentInfo> = available
            .into_iter()
            .filter(|a| {
                if let Some(ref variant) = requirements.agent_variant {
                    if a.variant.as_ref() != Some(variant) {
                        return false;
                    }
                }
                if !requirements.capabilities.is_empty()
                    && !requirements.capabilities.iter().all(|c| a.capabilities.contains(c)) {
                        return false;
                    }
                true
            })
            .collect();

        if candidates.is_empty() {
            return Err(OrchestratorError::AgentNotAvailable(
                "No available agent matching requirements".to_string(),
            ));
        }

        // Select based on scheduling strategy
        let scheduling_strategy = {
            let config = self.config.lock().unwrap();
            config.scheduling_strategy
        };
        let agent_id = match scheduling_strategy {
            SchedulingStrategy::RoundRobin => {
                let queue = self.agent_queue.write().await;
                let mut index = self.round_robin_index.write().await;
                *index = (*index + 1) % queue.len().max(1);
                queue[*index % queue.len()].clone()
            }
            SchedulingStrategy::LeastBusy => {
                // Find agent with least tasks completed
                let mut sorted = candidates.clone();
                sorted.sort_by_key(|a| a.statistics.tasks_completed);
                sorted[0].id.clone()
            }
            SchedulingStrategy::Priority => {
                // For now, just pick the first one
                candidates[0].id.clone()
            }
        };

        // Update agent status
        self.update_agent_status(&agent_id, AgentStatus::Busy).await?;

        debug!("Allocated agent: {} for task", agent_id);
        Ok(agent_id)
    }

    // ========== Message Routing ==========

    /// Send a message to a specific agent
    pub async fn send_message(&self, to: &str, message: AgentMessage) -> OrchestratorResult<bool> {
        let agents = self.agents.read().await;

        if !agents.contains_key(to) {
            return Err(OrchestratorError::AgentNotFound(to.to_string()));
        }

        // Queue the message
        let mut queue = self.message_queue.write().await;
        queue.entry(to.to_string()).or_insert_with(Vec::new).push(message);

        Ok(true)
    }

    /// Broadcast message to multiple agents
    pub async fn broadcast(&self, recipients: &[String], message: AgentMessage) -> Vec<SendResult> {
        let agents = self.agents.read().await.clone();
        drop(agents);

        let mut results = Vec::new();
        for agent_id in recipients {
            if self.agents.read().await.contains_key(agent_id) {
                let mut queue = self.message_queue.write().await;
                queue.entry(agent_id.clone()).or_insert_with(Vec::new).push(message.clone());
                results.push(SendResult::success(agent_id));
            } else {
                results.push(SendResult::failure(agent_id, "Agent not found"));
            }
        }
        results
    }

    /// Publish message to a topic
    pub async fn publish(&self, topic: &str, message: TopicMessage) -> OrchestratorResult<usize> {
        let subs = self.subscriptions.read().await;

        let agents = subs.get(topic).ok_or_else(|| {
            OrchestratorError::TopicNotFound(topic.to_string())
        })?;

        let count = agents.len();

        // Queue message for each subscriber
        let mut queue = self.message_queue.write().await;
        for agent_id in agents {
            let agent_msg = AgentMessage {
                from: message.from.clone(),
                to: agent_id.clone(),
                content: message.content.clone(),
                message_type: "topic".to_string(),
                timestamp: message.timestamp.clone(),
            };
            queue.entry(agent_id.clone()).or_insert_with(Vec::new).push(agent_msg);
        }

        debug!("Published message to topic {}: {} recipients", topic, count);
        Ok(count)
    }

    /// Subscribe an agent to a topic
    pub async fn subscribe(&self, agent_id: &str, topic: &str) -> OrchestratorResult<()> {
        // Verify agent exists
        let agents = self.agents.read().await;
        if !agents.contains_key(agent_id) {
            return Err(OrchestratorError::AgentNotFound(agent_id.to_string()));
        }
        drop(agents);

        let mut subs = self.subscriptions.write().await;
        subs.entry(topic.to_string()).or_insert_with(Vec::new).push(agent_id.to_string());

        debug!("Agent {} subscribed to topic {}", agent_id, topic);
        Ok(())
    }

    /// Unsubscribe an agent from a topic
    pub async fn unsubscribe(&self, agent_id: &str, topic: &str) -> OrchestratorResult<()> {
        let mut subs = self.subscriptions.write().await;

        if let Some(agents) = subs.get_mut(topic) {
            agents.retain(|id| id != agent_id);
            if agents.is_empty() {
                subs.remove(topic);
            }
        }

        debug!("Agent {} unsubscribed from topic {}", agent_id, topic);
        Ok(())
    }

    // ========== Resource Management ==========

    /// Get resource usage
    pub async fn get_resource_usage(&self) -> ResourceUsage {
        let agents = self.agents.read().await;

        let total = agents.len();
        let active = agents.values().filter(|a| a.status == AgentStatus::Busy).count();
        let running = agents.values().filter(|a| a.current_task.is_some()).count();

        ResourceUsage {
            total_agents: total,
            active_agents: active,
            pending_tasks: 0,
            running_tasks: running,
            memory_mb: agents.values().map(|a| a.statistics.memory_mb).sum::<f64>() as u64,
            cpu_percent: agents.values().map(|a| a.statistics.cpu_percent).sum::<f64>() / total.max(1) as f64,
        }
    }

    /// Set resource limit
    pub async fn set_resource_limit(&self, resource_type: &str, limit: usize) -> OrchestratorResult<()> {
        let mut config = self.config.lock().unwrap();
        match resource_type {
            "max_agents" => {
                config.max_agents = limit;
                Ok(())
            }
            "max_agents_per_session" => {
                config.max_agents_per_session = limit;
                Ok(())
            }
            "max_concurrent_tasks" => {
                config.max_concurrent_tasks = limit;
                Ok(())
            }
            _ => Err(OrchestratorError::InvalidRequest(format!(
                "Unknown resource type: {}",
                resource_type
            ))),
        }
    }

    // ========== Collaboration ==========

    /// Create a collaboration group
    pub async fn create_collaboration(
        &self,
        name: &str,
        agents: Vec<String>,
        mode: CollaborationMode,
    ) -> OrchestratorResult<String> {
        let collab_id = format!("collab-{}", uuid::Uuid::new_v4());

        let mut collab = Collaboration::new(&collab_id, name, agents.clone(), mode);

        // Set master for master-worker mode
        if mode == CollaborationMode::MasterWorker && !agents.is_empty() {
            collab = collab.with_master(&agents[0]);
        }

        // Set pipeline order for pipeline mode
        if mode == CollaborationMode::Pipeline {
            collab = collab.with_pipeline(agents.clone());
        }

        let mut collaborations = self.collaborations.write().await;
        collaborations.insert(collab_id.clone(), collab);

        info!("Created collaboration: {} with {} agents", collab_id, agents.len());
        Ok(collab_id)
    }

    /// Dissolve a collaboration group
    pub async fn dissolve_collaboration(&self, collaboration_id: &str) -> OrchestratorResult<()> {
        let mut collaborations = self.collaborations.write().await;

        if collaborations.remove(collaboration_id).is_none() {
            return Err(OrchestratorError::CollaborationNotFound(collaboration_id.to_string()));
        }

        info!("Dissolved collaboration: {}", collaboration_id);
        Ok(())
    }

    /// Get collaboration info
    pub async fn get_collaboration(&self, collaboration_id: &str) -> OrchestratorResult<Collaboration> {
        let collaborations = self.collaborations.read().await;
        collaborations
            .get(collaboration_id)
            .cloned()
            .ok_or_else(|| OrchestratorError::CollaborationNotFound(collaboration_id.to_string()))
    }

    // ========== Message Retrieval ==========

    /// Get pending messages for an agent
    pub async fn get_messages(&self, agent_id: &str) -> Vec<AgentMessage> {
        let mut queue = self.message_queue.write().await;
        queue.remove(agent_id).unwrap_or_default()
    }

    // ========== Statistics ==========

    /// Record task completion for an agent
    pub async fn record_task_completion(&self, agent_id: &str, execution_time_ms: u64) -> OrchestratorResult<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.statistics.tasks_completed += 1;
            agent.statistics.total_execution_time_ms += execution_time_ms;
            agent.status = AgentStatus::Idle;
            agent.current_task = None;
            Ok(())
        } else {
            Err(OrchestratorError::AgentNotFound(agent_id.to_string()))
        }
    }

    /// Record task failure for an agent
    pub async fn record_task_failure(&self, agent_id: &str) -> OrchestratorResult<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.statistics.tasks_failed += 1;
            agent.status = AgentStatus::Error;
            Ok(())
        } else {
            Err(OrchestratorError::AgentNotFound(agent_id.to_string()))
        }
    }

    /// Get agent count
    pub async fn agent_count(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if agent exists
    pub async fn has_agent(&self, agent_id: &str) -> bool {
        let agents = self.agents.read().await;
        agents.contains_key(agent_id)
    }
}

impl Default for OrchestratorImpl {
    fn default() -> Self {
        Self::new()
    }
}
