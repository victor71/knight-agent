//! Agent Runtime Implementation
//!
//! Core implementation of the agent runtime.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info, warn};
use futures::StreamExt;

use crate::types::*;
use llm_provider::{LLMProvider, LLMRouter, CompletionStream};

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
    /// LLM Router for chat completions
    llm_router: Option<Arc<LLMRouter>>,
    /// Default model to use when agent doesn't specify one
    default_model: Option<String>,
}

impl AgentRuntimeImpl {
    /// Create a new runtime
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
            initialized: false,
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            execution_tracking: Arc::new(AsyncRwLock::new(HashMap::new())),
            llm_router: None,
            default_model: None,
        }
    }

    /// Create with custom config
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self {
            config,
            initialized: false,
            agents: Arc::new(AsyncRwLock::new(HashMap::new())),
            execution_tracking: Arc::new(AsyncRwLock::new(HashMap::new())),
            llm_router: None,
            default_model: None,
        }
    }

    /// Initialize the LLM router from environment variables
    fn initialize_llm_router(&mut self) -> RuntimeResult<()> {
        let router = LLMRouter::new();
        router.initialize()
            .map_err(|e| AgentRuntimeError::InitializationFailed(format!("failed to initialize LLM router: {}", e)))?;

        if !router.is_empty() {
            let router_arc = Arc::new(router);
            self.llm_router = Some(router_arc);
            if let Some(r) = &self.llm_router {
                let models = r.models();
                if !models.is_empty() {
                    self.default_model = models.first().cloned();
                }
            }
            info!("LLM Router initialized");
        } else {
            info!("No LLM provider configured");
        }
        Ok(())
    }

    /// Initialize the runtime
    pub async fn initialize(&mut self) -> RuntimeResult<()> {
        // Initialize LLM router from env vars
        self.initialize_llm_router()?;

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

    /// Get or create a session agent
    /// Returns the agent ID for the session, creating a new one if needed
    pub async fn get_or_create_session_agent(&self, session_id: String) -> RuntimeResult<String> {
        // Use a deterministic agent ID based on session_id
        let agent_id = format!("session:{}", session_id);

        // Check if agent already exists
        {
            let agents = self.agents.read().await;
            if agents.contains_key(&agent_id) {
                debug!("Session agent exists: {}", agent_id);
                return Ok(agent_id);
            }
        }

        // Create new agent for this session
        info!("Creating session agent: {}", agent_id);
        let mut agent = Agent::new(agent_id.clone(), "default".to_string(), session_id.clone());
        agent.state.status = AgentStatus::Idle;

        let mut agents = self.agents.write().await;
        agents.insert(agent_id.clone(), agent.clone());

        info!("Session agent created: {}", agent_id);
        Ok(agent_id)
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
        stream: bool,
    ) -> RuntimeResult<Message> {
        self.send_message_streaming_with_callback(agent_id, message, stream, None).await
    }

    /// Send a message with optional streaming callback
    pub async fn send_message_streaming_with_callback(
        &self,
        agent_id: &str,
        message: Message,
        _stream: bool,
        stream_callback: Option<Box<dyn Fn(String) -> bool + Send + Sync>>,
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
            "Message sent to agent {}, status: {:?}, has_callback={}",
            agent_id, agent.state.status, stream_callback.is_some()
        );

        // Call LLM router if available
        let response = if let Some(ref router) = self.llm_router {
            info!("[AGENT-RUNTIME] LLM router available for agent {}, starting streaming", agent_id);
            info!("Calling LLM router for agent {} with streaming", agent_id);

            // Convert agent-runtime Message to LLM provider format
            let llm_messages = self.convert_to_llm_messages(&agent.context.messages)?;

            let request = llm_provider::ChatCompletionRequest {
                model: self.default_model.clone().unwrap_or_else(|| "gpt-4o".to_string()),
                messages: llm_messages,
                temperature: 0.7,
                max_tokens: 4096,
                stream: true,  // Always use streaming now
                ..Default::default()
            };

            // Use streaming with callback if provided
            self.handle_streaming_with_callback(router, request, stream_callback).await?
        } else {
            // No LLM router - return placeholder
            warn!("No LLM provider available for agent {}", agent_id);
            Message::assistant("Message received (no LLM provider configured)".to_string())
        };

        agent.context.add_message(response.clone());
        agent.state.statistics.increment_messages_sent();

        Ok(response)
    }

    /// Handle streaming LLM request
    async fn handle_streaming_request(
        &self,
        router: &LLMRouter,
        request: llm_provider::ChatCompletionRequest,
    ) -> RuntimeResult<Message> {
        self.handle_streaming_with_callback(router, request, None).await
    }

    /// Handle streaming LLM request with optional callback
    async fn handle_streaming_with_callback(
        &self,
        router: &LLMRouter,
        request: llm_provider::ChatCompletionRequest,
        stream_callback: Option<Box<dyn Fn(String) -> bool + Send + Sync>>,
    ) -> RuntimeResult<Message> {
        info!("[AGENT-RUNTIME] handle_streaming_with_callback: has_callback={}", stream_callback.is_some());

        let stream_result = router.stream_completion(request.clone()).await;

        match stream_result {
            Ok(stream) => {
                info!("[AGENT-RUNTIME] Stream created successfully, starting to receive chunks");
                // Collect all chunks from the stream
                let mut full_content = String::new();
                let mut chunk_count = 0;

                use futures::StreamExt;
                let mut stream = stream;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // Extract text from chunk
                            if let Some(text) = self.extract_text_from_chunk(&chunk) {
                                // Call callback if provided
                                if let Some(ref callback) = stream_callback {
                                    chunk_count += 1;
                                    info!("[AGENT-RUNTIME] Stream chunk {}: {} chars, calling callback", chunk_count, text.len());
                                    if !callback(text.clone()) {
                                        info!("[AGENT-RUNTIME] Callback returned false, stopping stream");
                                        break;
                                    }
                                }
                                full_content.push_str(&text);
                                if stream_callback.is_none() {
                                    chunk_count += 1;
                                }
                                debug!("Received stream chunk {}: {} chars", chunk_count, text.len());
                            }
                        }
                        Err(e) => {
                            warn!("[AGENT-RUNTIME] Stream chunk error: {:?}", e);
                            break;
                        }
                    }
                }

                info!("[AGENT-RUNTIME] Streaming complete: {} chunks, {} total chars", chunk_count, full_content.len());

                if full_content.is_empty() {
                    Ok(Message::assistant("No response from LLM".to_string()))
                } else {
                    Ok(Message::assistant(full_content))
                }
            }
            Err(e) => {
                warn!("[AGENT-RUNTIME] LLM streaming failed: {:?}", e);
                // Fall back to regular request
                info!("[AGENT-RUNTIME] Falling back to regular chat completion");
                let mut fallback_request = request;
                fallback_request.stream = false;
                self.handle_regular_request(router, fallback_request).await
            }
        }
    }

    /// Handle regular (non-streaming) LLM request
    async fn handle_regular_request(
        &self,
        router: &LLMRouter,
        request: llm_provider::ChatCompletionRequest,
    ) -> RuntimeResult<Message> {
        match router.chat_completion(request).await {
            Ok(response) => {
                // Extract content from LLM response
                let content = response.content
                    .or_else(|| {
                        response.choices.first().and_then(|c| {
                            c.message.content.as_ref().map(|m| {
                                if let llm_provider::Content::Text(s) = m {
                                    s.clone()
                                } else {
                                    serde_json::to_string(m).unwrap_or_default()
                                }
                            })
                        })
                    })
                    .unwrap_or_else(|| "No response from LLM".to_string());

                Ok(Message::assistant(content))
            }
            Err(e) => {
                warn!("LLM call failed: {:?}", e);
                Ok(Message::assistant(format!("Error calling LLM: {:?}", e)))
            }
        }
    }

    /// Extract text content from a stream chunk
    fn extract_text_from_chunk(&self, chunk: &llm_provider::ChatCompletionChunk) -> Option<String> {
        use llm_provider::Delta;

        // Skip thinking chunks - they should not be shown to users
        if chunk.is_thinking == Some(true) {
            debug!("Skipping thinking chunk");
            return None;
        }

        // Try to get text from choices (Anthropic/SSE format)
        for choice in &chunk.choices {
            match &choice.delta {
                Delta::MessageDelta { delta, .. } => {
                    if let Some(ref text) = delta.content {
                        return Some(text.clone());
                    }
                }
                _ => {}
            }
        }

        // Try direct content field (some APIs put content directly)
        if let Some(ref content) = chunk.content {
            if !content.is_empty() {
                return Some(content.clone());
            }
        }

        None
    }

    /// Convert agent-runtime messages to LLM provider messages
    fn convert_to_llm_messages(
        &self,
        messages: &[Message],
    ) -> RuntimeResult<Vec<llm_provider::Message>> {
        use llm_provider::{Message as LlmMessage, MessageRole as LlmRole, Content, ContentBlock};

        let mut llm_messages = Vec::new();

        for msg in messages {
            let role = match msg.role {
                MessageRole::User => LlmRole::User,
                MessageRole::Assistant => LlmRole::Assistant,
                MessageRole::System => LlmRole::System,
                MessageRole::Tool => LlmRole::Tool,
            };

            let content = match &msg.content {
                serde_json::Value::String(s) => Some(Content::Text(s.clone())),
                serde_json::Value::Object(_) => {
                    // Try to parse as content blocks
                    if let Ok(blocks) = serde_json::from_value::<Vec<ContentBlock>>(msg.content.clone()) {
                        Some(Content::Blocks(blocks))
                    } else {
                        Some(Content::Text(msg.content.to_string()))
                    }
                }
                _ => Some(Content::Text(msg.content.to_string())),
            };

            llm_messages.push(LlmMessage {
                role,
                content,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        Ok(llm_messages)
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

#[async_trait::async_trait]
impl crate::AgentHandle for AgentRuntimeImpl {
    async fn send_message(
        &self,
        agent_id: &str,
        message: Message,
        stream: bool,
    ) -> RuntimeResult<Message> {
        AgentRuntimeImpl::send_message(self, agent_id, message, stream).await
    }

    async fn create_agent(
        &self,
        definition_id: String,
        session_id: String,
        variant: Option<String>,
    ) -> RuntimeResult<Agent> {
        AgentRuntimeImpl::create_agent(self, definition_id, session_id, variant).await
    }

    async fn get_agent(&self, agent_id: &str) -> RuntimeResult<Agent> {
        AgentRuntimeImpl::get_agent(self, agent_id).await
    }

    async fn get_or_create_session_agent(&self, session_id: String) -> RuntimeResult<String> {
        AgentRuntimeImpl::get_or_create_session_agent(self, session_id).await
    }

    fn is_initialized(&self) -> bool {
        AgentRuntimeImpl::is_initialized(self)
    }
}
