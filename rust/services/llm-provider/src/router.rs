//! LLM Router
//!
//! Routes requests to the appropriate LLM provider based on model name.
//! Supports hot-reload via `reload_config()` method.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::info;

use crate::types::{
    ChatCompletionRequest, ChatCompletionResponse, CostEstimate,
    ModelInfo, ProviderStatus, Usage,
};
use crate::{CompletionStream, TokenCount};
use crate::llm_trait::{LLMError, LLMProvider, LLMResult};
use crate::provider::{GenericLLMProvider, LLMProtocol, ProviderConfig};

/// Resolve environment variable reference in config values
/// Supports ${ENV_VAR} syntax
fn resolve_env_var(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') {
        let env_var = &value[2..value.len() - 1];
        std::env::var(env_var).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}

/// Inner state of LLM Router (protected by RwLock)
struct LLMRouterInner {
    /// Providers by name
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    /// Model to provider name mapping
    model_to_provider: HashMap<String, String>,
    /// Default provider name
    default_provider: Option<String>,
    /// Default model per provider
    provider_default_models: HashMap<String, String>,
}

impl Default for LLMRouterInner {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            model_to_provider: HashMap::new(),
            default_provider: None,
            provider_default_models: HashMap::new(),
        }
    }
}

/// LLM Router - routes requests to appropriate providers based on model
/// Supports hot-reload when configuration changes
pub struct LLMRouter {
    /// Inner state protected by RwLock for thread-safe updates
    inner: Arc<RwLock<LLMRouterInner>>,
}

impl LLMRouter {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(LLMRouterInner::default())),
        }
    }

    /// Initialize router - tries global config first, then env vars
    /// This can be called multiple times to reload configuration
    pub fn initialize(&self) -> LLMResult<()> {
        // Try to load from configuration module's global storage
        if let Some(llm_config) = configuration::get_llm_config() {
            if !llm_config.providers.is_empty() {
                info!("Loading LLM config from configuration module");
                self.initialize_from_config(&llm_config)?;
                return Ok(());
            }
        }

        // Fall back to env vars
        match GenericLLMProvider::from_env() {
            Ok(provider) => {
                let name = provider.name().to_string();
                let models = provider.config().models.clone();
                let default_model = provider.config().default_model().to_string();

                let mut inner = self.inner.write();
                for model in &models {
                    inner.model_to_provider.insert(model.clone(), name.clone());
                }

                inner.provider_default_models.insert(name.clone(), default_model);
                inner.providers.insert(name.clone(), Arc::new(provider));
                inner.default_provider = Some(name);

                info!("LLM Router initialized with env vars provider");
                Ok(())
            }
            Err(e) => {
                info!("No LLM provider configured (env vars not set): {}", e);
                Ok(())
            }
        }
    }

    /// Initialize router from configuration module's LlmConfig
    fn initialize_from_config(&self, config: &configuration::LlmConfig) -> LLMResult<()> {
        if config.providers.is_empty() {
            info!("No LLM providers configured in knight.json");
            return Ok(());
        }

        let default_provider = config.default_provider.clone();
        let mut inner = self.inner.write();

        // Clear existing providers
        inner.providers.clear();
        inner.model_to_provider.clear();
        inner.provider_default_models.clear();
        inner.default_provider = None;

        for (name, provider_config) in &config.providers {
            // Resolve API key (supports ${ENV_VAR} syntax)
            let api_key = resolve_env_var(&provider_config.api_key);

            // Map provider type string to LLMProtocol
            let protocol = match provider_config.provider_type.to_lowercase().as_str() {
                "anthropic" => LLMProtocol::Anthropic,
                _ => LLMProtocol::OpenAI,
            };

            // Extract model IDs from LlmModelConfig list
            let models: Vec<String> = provider_config.models
                .iter()
                .map(|m| m.id.clone())
                .collect();

            let provider_cfg = ProviderConfig {
                name: name.clone(),
                api_key,
                base_url: provider_config.base_url.clone(),
                protocol,
                models: models.clone(),
                default_model: Some(provider_config.default_model.clone()),
                timeout_secs: provider_config.timeout_secs,
                model_pricing: HashMap::new(),
            };

            let provider = GenericLLMProvider::new(provider_cfg)?;
            let default_model = provider_config.default_model.clone();

            for model in &models {
                inner.model_to_provider.insert(model.clone(), name.clone());
            }

            inner.provider_default_models.insert(name.clone(), default_model);
            inner.providers.insert(name.clone(), Arc::new(provider));
        }

        // Set default provider
        if let Some(ref default_name) = default_provider {
            if inner.providers.contains_key(default_name) {
                inner.default_provider = Some(default_name.clone());
                info!("LLM Router initialized with default provider: {}", default_name);
            } else {
                info!("Configured default provider '{}' not found in providers", default_name);
                inner.default_provider = inner.providers.keys().next().cloned();
            }
        } else {
            inner.default_provider = inner.providers.keys().next().cloned();
        }

        info!("LLM Router initialized from config with {} providers", inner.providers.len());
        Ok(())
    }

    /// Reload configuration from the configuration module
    /// This should be called when configuration changes are detected
    /// Can be triggered via event loop, hook, or direct call
    pub fn reload_config(&self) -> LLMResult<()> {
        info!("LLM Router: Reloading configuration");
        self.initialize()
    }

    /// Add a provider to the router
    pub fn add_provider(&self, name: String, provider: GenericLLMProvider) -> LLMResult<()> {
        let models = provider.config().models.clone();
        let default_model = provider.config().default_model().to_string();

        let mut inner = self.inner.write();
        for model in &models {
            inner.model_to_provider.insert(model.clone(), name.clone());
        }

        inner.provider_default_models.insert(name.clone(), default_model);
        inner.providers.insert(name.clone(), Arc::new(provider));

        if inner.default_provider.is_none() {
            inner.default_provider = Some(name.clone());
        }

        info!("Added provider '{}' with models: {:?}", name, models);
        Ok(())
    }

    /// Set the default provider
    pub fn set_default_provider(&self, name: String) {
        let mut inner = self.inner.write();
        if inner.providers.contains_key(&name) {
            inner.default_provider = Some(name);
        }
    }

    /// Get the provider for a specific model
    fn get_provider_for_model(&self, model: &str) -> Option<Arc<dyn LLMProvider>> {
        let inner = self.inner.read();
        if let Some(provider_name) = inner.model_to_provider.get(model) {
            if let Some(provider) = inner.providers.get(provider_name) {
                return Some(provider.clone());
            }
        }

        if let Some(ref default) = inner.default_provider {
            if let Some(provider) = inner.providers.get(default) {
                return Some(provider.clone());
            }
        }

        None
    }

    /// Get default model for a provider
    fn get_default_model(&self, provider_name: &str) -> Option<String> {
        let inner = self.inner.read();
        inner.provider_default_models.get(provider_name).cloned()
    }

    /// Check if router is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().providers.is_empty()
    }

    /// Get list of all configured models
    pub fn models(&self) -> Vec<String> {
        self.inner.read().model_to_provider.keys().cloned().collect()
    }

    /// Get list of all provider names
    pub fn provider_names(&self) -> Vec<String> {
        self.inner.read().providers.keys().cloned().collect()
    }
}

impl Default for LLMRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LLMProvider for LLMRouter {
    fn new() -> LLMResult<Self>
    where
        Self: Sized,
    {
        let router = Self::new();
        router.initialize()?;
        Ok(router)
    }

    fn name(&self) -> &str {
        "router"
    }

    fn is_initialized(&self) -> bool {
        !self.is_empty()
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<ChatCompletionResponse> {
        let model = if request.model.is_empty() {
            let inner = self.inner.read();
            if let Some(ref default) = inner.default_provider {
                if let Some(default_model) = inner.provider_default_models.get(default) {
                    default_model.clone()
                } else {
                    return Err(LLMError::NotInitialized);
                }
            } else {
                return Err(LLMError::NotInitialized);
            }
        } else {
            request.model.clone()
        };

        let provider = self.get_provider_for_model(&model)
            .ok_or_else(|| LLMError::ModelNotFound(model.clone()))?;

        let model_request = ChatCompletionRequest {
            model: model.clone(),
            ..request
        };

        info!("Routing LLM request for model '{}' to provider: {}", model, provider.name());
        provider.chat_completion(model_request).await
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> LLMResult<CompletionStream> {
        let model = if request.model.is_empty() {
            let inner = self.inner.read();
            if let Some(ref default) = inner.default_provider {
                if let Some(default_model) = inner.provider_default_models.get(default) {
                    default_model.clone()
                } else {
                    return Err(LLMError::NotInitialized);
                }
            } else {
                return Err(LLMError::NotInitialized);
            }
        } else {
            request.model.clone()
        };

        let provider = self.get_provider_for_model(&model)
            .ok_or_else(|| LLMError::ModelNotFound(model.clone()))?;

        let model_request = ChatCompletionRequest {
            model: model.clone(),
            ..request
        };

        info!("Routing LLM streaming request for model '{}' to provider: {}", model, provider.name());
        provider.stream_completion(model_request).await
    }

    async fn count_tokens(&self, text: &str, model: &str) -> LLMResult<TokenCount> {
        if let Some(provider) = self.get_provider_for_model(model) {
            provider.count_tokens(text, model).await
        } else {
            let provider = {
                let inner = self.inner.read();
                if let Some(ref default) = inner.default_provider {
                    inner.providers.get(default).cloned()
                } else {
                    None
                }
            };

            if let Some(provider) = provider {
                provider.count_tokens(text, model).await
            } else {
                Err(LLMError::ModelNotFound(model.to_string()))
            }
        }
    }

    async fn estimate_cost(&self, request: &ChatCompletionRequest) -> LLMResult<CostEstimate> {
        let model = &request.model;
        if let Some(provider) = self.get_provider_for_model(model) {
            provider.estimate_cost(request).await
        } else {
            Err(LLMError::ModelNotFound(model.clone()))
        }
    }

    async fn calculate_cost(&self, usage: &Usage, model: &str) -> LLMResult<CostEstimate> {
        if let Some(provider) = self.get_provider_for_model(model) {
            provider.calculate_cost(usage, model).await
        } else {
            Err(LLMError::ModelNotFound(model.to_string()))
        }
    }

    async fn list_models(&self) -> LLMResult<Vec<String>> {
        Ok(self.models())
    }

    async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo> {
        if let Some(provider) = self.get_provider_for_model(model) {
            provider.get_model_info(model).await
        } else {
            Err(LLMError::ModelNotFound(model.to_string()))
        }
    }

    async fn health_check(&self) -> LLMResult<ProviderStatus> {
        let providers: Vec<Arc<dyn LLMProvider>> = {
            let inner = self.inner.read();
            inner.providers.values().cloned().collect()
        };

        let mut all_healthy = true;
        let mut max_latency = 0u64;

        for provider in &providers {
            if let Ok(status) = provider.health_check().await {
                if !status.healthy {
                    all_healthy = false;
                }
                max_latency = max_latency.max(status.latency_ms);
            } else {
                all_healthy = false;
            }
        }

        Ok(ProviderStatus {
            name: "router".to_string(),
            healthy: all_healthy,
            latency_ms: max_latency,
            error_rate: if all_healthy { 0.0 } else { 1.0 },
            last_check: chrono::Utc::now().to_rfc3339(),
        })
    }
}

impl Clone for LLMRouter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
