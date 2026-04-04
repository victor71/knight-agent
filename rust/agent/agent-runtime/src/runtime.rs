//! Agent Runtime Implementation
//!
//! Core implementation of the agent runtime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info, warn};

use crate::types::*;

/// Agent runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub max_execution_time: u64,
    pub max_tool_calls: usize,
    pub max_llm_calls: usize,
    pub max_retry_attempts: u32,
    pub retry_delay_ms: u64,
    pub llm_timeout_secs: u64,
    pub tool_timeout_secs: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_execution_time: 300,
            max_tool_calls: 50,
            max_llm_calls: 20,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
            llm_timeout_secs: 60,
            tool_timeout_secs: 30,
        }
    }
}

/// Agent runtime implementation
pub struct AgentRuntimeImpl {
    /// Configuration
    config: RuntimeConfig,
    /// Whether initialized
    initialized: bool,
    /// Active agents (agent_id -> Agent)
    agents: Arc<AsyncRwLock<HashMap<String, Agent>>>,
    /// Execution tracking (agent_id -> start_time)
    execution_tracking: Arc<AsyncRwLock<HashMap<String, Instant>>>,
}

impl AgentRuntimeImpl {
    /// Create a new runtime
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            initialized: false,
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            execution_tracking: Arc::new(AsyncRwLock::new(HashMap::new())),
        }
    }

    /// Create with custom config
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            config,
            initialized: false,
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            execution_tracking: Arc::new(AsyncRwLock::new(HashMap::new())),
        }
    }

    /// Initialize the runtime
    pub async fn initialize(&mut self) -> RuntimeResult<()> {
        self.initialized = true;
        info!("AgentRuntime initialized");
        Ok(())
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Create a new agent
    pub async fn create_agent(
        &self,
        definition_id: String,
        session_id: String,
        variant: Option<String>,
    ) -> RuntimeResult<Agent> {
        if !self.initialized {
            return Err(AgentRuntimeError::NotInitialized);
        }

        let agent_id = format!("{}-{}", definition_id, uuid::Uuid::new_v4());

        let mut agent = Agent::new(agent_id.clone(), definition_id, session_id);
        agent.variant = variant;

        let mut agents = self.agents.write().await;
        agents.insert(agent_id.clone(), agent.clone());

        debug!("Created agent: {}", agent_id);
        Ok(agent)
    }

    /// Get an agent by ID
    pub async fn get_agent(&self, agent_id: &str) -> RuntimeResult<Agent> {
        let agents = self.agents.read().await;
        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))
    }

    /// Start an agent (transition from idle to thinking)
    pub async fn start_agent(&self, agent_id: &str) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        if agent.state.status != AgentStatus::Idle {
            return Err(AgentRuntimeError::InvalidStateTransition(format!(
                "Cannot start agent in {:?} state",
                agent.state.status
            )));
        }

        agent.state.status = AgentStatus::Thinking;
        agent.state.current_action = Some("initializing".to_string());

        // Start execution tracking
        drop(agents);
        let mut tracking = self.execution_tracking.write().await;
        tracking.insert(agent_id.to_string(), Instant::now());

        info!("Started agent: {}", agent_id);
        Ok(())
    }

    /// Stop an agent
    pub async fn stop_agent(&self, agent_id: &str, force: bool) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        let can_stop = matches!(
            agent.state.status,
            AgentStatus::Idle
                | AgentStatus::Thinking
                | AgentStatus::Acting
                | AgentStatus::AwaitingUser
                | AgentStatus::Paused
                | AgentStatus::Error
        );

        if !can_stop && !force {
            return Err(AgentRuntimeError::InvalidStateTransition(format!(
                "Cannot stop agent in {:?} state",
                agent.state.status
            )));
        }

        agent.state.status = AgentStatus::Stopped;
        agent.state.current_action = None;

        // Remove from execution tracking
        drop(agents);
        let mut tracking = self.execution_tracking.write().await;
        tracking.remove(agent_id);

        info!("Stopped agent: {}", agent_id);
        Ok(())
    }

    /// Pause an agent
    pub async fn pause_agent(&self, agent_id: &str) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        if agent.state.status != AgentStatus::Idle {
            return Err(AgentRuntimeError::InvalidStateTransition(format!(
                "Cannot pause agent in {:?} state",
                agent.state.status
            )));
        }

        agent.state.status = AgentStatus::Paused;
        info!("Paused agent: {}", agent_id);
        Ok(())
    }

    /// Resume a paused agent
    pub async fn resume_agent(&self, agent_id: &str) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        if agent.state.status != AgentStatus::Paused {
            return Err(AgentRuntimeError::InvalidStateTransition(format!(
                "Cannot resume agent in {:?} state",
                agent.state.status
            )));
        }

        agent.state.status = AgentStatus::Idle;
        info!("Resumed agent: {}", agent_id);
        Ok(())
    }

    /// Send a message to an agent
    pub async fn send_message(
        &self,
        agent_id: &str,
        message: Message,
        _stream: bool,
    ) -> RuntimeResult<Message> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        // Add message to context
        agent.context.add_message(message.clone());
        agent.state.statistics.increment_messages_received();

        // Transition to thinking if idle
        if agent.state.status == AgentStatus::Idle {
            agent.state.status = AgentStatus::Thinking;
            agent.state.current_action = Some("processing".to_string());
        }

        debug!(
            "Message sent to agent {}, status: {:?}",
            agent_id, agent.state.status
        );

        // For now, return a placeholder response
        // In a full implementation, this would call the LLM
        let response = Message::assistant("Message received".to_string());
        agent.context.add_message(response.clone());
        agent.state.statistics.increment_messages_sent();

        Ok(response)
    }

    /// Get agent state
    pub async fn get_agent_state(&self, agent_id: &str) -> RuntimeResult<AgentState> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;
        Ok(agent.state.clone())
    }

    /// Get agent context
    pub async fn get_context(&self, agent_id: &str) -> RuntimeResult<AgentContext> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;
        Ok(agent.context.clone())
    }

    /// Update agent variables
    pub async fn update_variables(
        &self,
        agent_id: &str,
        variables: serde_json::Map<String, serde_json::Value>,
    ) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        let vars: Vec<(String, serde_json::Value)> = variables.into_iter().collect();
        for (key, value) in vars {
            agent.context.set_variable(&key, value);
        }

        debug!("Updated variables for agent: {}", agent_id);
        Ok(())
    }

    /// Cancel current operation
    pub async fn cancel_operation(&self, agent_id: &str) -> RuntimeResult<Option<String>> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        let cancelled_await_id = match agent.state.status {
            AgentStatus::Acting => {
                // If acting, we could interrupt the tool call
                agent.state.current_action = None;
                None
            }
            AgentStatus::AwaitingUser => {
                // If awaiting user, return the await_id
                let await_id = agent.state.await_info.as_ref().map(|a| a.await_id.clone());
                agent.state.await_info = None;
                await_id
            }
            _ => {
                warn!("cancel_operation called but agent is not in cancellable state");
                None
            }
        };

        agent.state.status = AgentStatus::Idle;
        info!("Cancelled operation for agent: {}", agent_id);
        Ok(cancelled_await_id)
    }

    /// Handle user response
    pub async fn handle_user_response(
        &self,
        agent_id: &str,
        user_response: UserResponse,
    ) -> RuntimeResult<String> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        if agent.state.status != AgentStatus::AwaitingUser {
            return Err(AgentRuntimeError::InvalidStateTransition(format!(
                "Agent is not awaiting user response, status: {:?}",
                agent.state.status
            )));
        }

        // Verify await_id matches
        if let Some(await_info) = &agent.state.await_info {
            if await_info.await_id != user_response.await_id {
                return Err(AgentRuntimeError::ExecutionFailed(
                    "Await ID mismatch".to_string(),
                ));
            }
        }

        // Transition back to thinking
        agent.state.status = AgentStatus::Thinking;
        agent.state.await_info = None;
        agent.state.current_action = Some("resuming".to_string());

        // Add user response to context
        let msg = Message::user(serde_json::json!({
            "await_id": user_response.await_id,
            "response": user_response.response,
            "approved": user_response.approved,
        }));
        agent.context.add_message(msg);

        info!(
            "Handled user response for agent: {}, approved: {}",
            agent_id, user_response.approved
        );

        Ok("thinking".to_string())
    }

    /// Call a tool (internal interface)
    pub async fn call_tool(
        &self,
        agent_id: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> RuntimeResult<ToolResult> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        // Check execution limits
        if agent.state.statistics.tools_called >= self.config.max_tool_calls as u64 {
            return Err(AgentRuntimeError::ExecutionFailed(
                "Max tool calls exceeded".to_string(),
            ));
        }

        // Transition to acting if not already
        if agent.state.status == AgentStatus::Thinking {
            agent.state.status = AgentStatus::Acting;
        }

        agent.state.current_action = Some(format!("tool:{}", tool_name));
        agent.state.statistics.increment_tools_called();

        debug!(
            "Tool call: {} on agent {} with args: {:?}",
            tool_name, agent_id, args
        );

        // In a full implementation, this would call the Tool System
        // For now, return a placeholder result
        let result = ToolResult::success(serde_json::json!({
            "tool": tool_name,
            "args": args,
            "message": "Tool execution would happen here"
        }));

        // Transition back to thinking
        agent.state.status = AgentStatus::Thinking;
        agent.state.current_action = Some("processing".to_string());

        Ok(result)
    }

    /// List all agents
    pub async fn list_agents(&self) -> Vec<Agent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
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

    /// Remove an agent
    pub async fn remove_agent(&self, agent_id: &str) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let mut tracking = self.execution_tracking.write().await;

        if agents.remove(agent_id).is_none() {
            return Err(AgentRuntimeError::AgentNotFound(agent_id.to_string()));
        }

        tracking.remove(agent_id);
        debug!("Removed agent: {}", agent_id);
        Ok(())
    }

    /// Update agent state directly
    pub async fn update_agent_state(&self, agent_id: &str, status: AgentStatus) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        agent.state.status = status;
        Ok(())
    }

    /// Record LLM call
    pub async fn record_llm_call(&self, agent_id: &str, tokens: u64) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        agent.state.statistics.increment_llm_calls();
        agent.state.statistics.add_tokens(tokens);

        Ok(())
    }

    /// Transition to awaiting user state
    pub async fn await_user(
        &self,
        agent_id: &str,
        query_type: &str,
        message: &str,
    ) -> RuntimeResult<String> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        let await_id = uuid::Uuid::new_v4().to_string();
        agent.state.status = AgentStatus::AwaitingUser;
        agent.state.current_action = Some(format!("awaiting:{}", query_type));
        agent.state.await_info = Some(AwaitInfo::new(&await_id, query_type, message));

        info!(
            "Agent {} awaiting user response, await_id: {}",
            agent_id, await_id
        );

        Ok(await_id)
    }

    /// Set agent error state
    pub async fn set_error(&self, agent_id: &str, error: ErrorInfo) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        let new_state = agent.state.clone().with_error(error);
        agent.state = new_state;
        agent.state.statistics.increment_errors();

        Ok(())
    }

    /// Complete agent execution (transition to idle)
    pub async fn complete(&self, agent_id: &str) -> RuntimeResult<()> {
        let mut agents = self.agents.write().await;
        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentRuntimeError::AgentNotFound(agent_id.to_string()))?;

        agent.state.status = AgentStatus::Idle;
        agent.state.current_action = None;

        // Remove from execution tracking
        drop(agents);
        let mut tracking = self.execution_tracking.write().await;
        tracking.remove(agent_id);

        debug!("Agent {} execution completed", agent_id);
        Ok(())
    }
}

impl Default for AgentRuntimeImpl {
    fn default() -> Self {
        Self::new()
    }
}
