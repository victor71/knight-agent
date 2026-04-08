//! LLM Router
//!
//! Routes requests to the appropriate LLM provider based on model name.

use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::types::{
    ChatCompletionRequest, ChatCompletionResponse, CostEstimate,
    ModelInfo, ProviderStatus, Usage,
};
use crate::{CompletionStream, TokenCount};
use crate::llm_trait::{LLMError, LLMProvider, LLMResult};
use crate::provider::GenericLLMProvider;

/// LLM Router - routes requests to appropriate providers based on model
pub struct LLMRouter {
    /// Providers by name
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    /// Model to provider name mapping
    model_to_provider: HashMap<String, String>,
    /// Default provider name
    default_provider: Option<String>,
    /// Default model per provider
    provider_default_models: HashMap<String, String>,
}

impl LLMRouter {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            model_to_provider: HashMap::new(),
            default_provider: None,
            provider_default_models: HashMap::new(),
        }
    }

    /// Initialize router from environment variables (backward compatible)
    pub fn initialize(&mut self) -> LLMResult<()> {
        match GenericLLMProvider::from_env() {
            Ok(provider) => {
                let name = provider.name().to_string();
                let models = provider.config().models.clone();
                let default_model = provider.config().default_model().to_string();

                for model in &models {
                    self.model_to_provider.insert(model.clone(), name.clone());
                }

                self.provider_default_models.insert(name.clone(), default_model);
                self.providers.insert(name.clone(), Arc::new(provider));
                self.default_provider = Some(name);

                info!("LLM Router initialized with default provider");
                Ok(())
            }
            Err(e) => {
                info!("No LLM provider configured (env vars not set): {}", e);
                Ok(())
            }
        }
    }

    /// Add a provider to the router
    pub fn add_provider(&mut self, name: String, provider: GenericLLMProvider) -> LLMResult<()> {
        let models = provider.config().models.clone();
        let default_model = provider.config().default_model().to_string();

        for model in &models {
            self.model_to_provider.insert(model.clone(), name.clone());
        }

        self.provider_default_models.insert(name.clone(), default_model);
        self.providers.insert(name.clone(), Arc::new(provider));

        if self.default_provider.is_none() {
            self.default_provider = Some(name.clone());
        }

        info!("Added provider '{}' with models: {:?}", name, models);
        Ok(())
    }

    /// Set the default provider
    pub fn set_default_provider(&mut self, name: String) {
        if self.providers.contains_key(&name) {
            self.default_provider = Some(name);
        }
    }

    /// Get the provider for a specific model
    fn get_provider_for_model(&self, model: &str) -> Option<Arc<dyn LLMProvider>> {
        if let Some(provider_name) = self.model_to_provider.get(model) {
            if let Some(provider) = self.providers.get(provider_name) {
                return Some(provider.clone());
            }
        }

        if let Some(ref default) = self.default_provider {
            if let Some(provider) = self.providers.get(default) {
                return Some(provider.clone());
            }
        }

        None
    }

    /// Get default model for a provider
    fn get_default_model(&self, provider_name: &str) -> Option<String> {
        self.provider_default_models.get(provider_name).cloned()
    }

    /// Check if router is empty
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Get list of all configured models
    pub fn models(&self) -> Vec<String> {
        self.model_to_provider.keys().cloned().collect()
    }

    /// Get list of all provider names
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
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
        let mut router = Self::new();
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
            if let Some(ref default) = self.default_provider {
                if let Some(default_model) = self.get_default_model(default) {
                    default_model
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
            if let Some(ref default) = self.default_provider {
                if let Some(default_model) = self.get_default_model(default) {
                    default_model
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
        } else if let Some(ref default) = self.default_provider {
            if let Some(provider) = self.providers.get(default) {
                provider.count_tokens(text, model).await
            } else {
                Err(LLMError::ModelNotFound(model.to_string()))
            }
        } else {
            Err(LLMError::ModelNotFound(model.to_string()))
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
        let mut all_healthy = true;
        let mut max_latency = 0u64;

        for provider in self.providers.values() {
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
            providers: self.providers.clone(),
            model_to_provider: self.model_to_provider.clone(),
            default_provider: self.default_provider.clone(),
            provider_default_models: self.provider_default_models.clone(),
        }
    }
}
