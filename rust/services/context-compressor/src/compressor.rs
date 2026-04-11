//! Context compressor implementation

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::ContextCompressor;
use crate::content::ContentType;
use crate::error::{ContextCompressorError, ContextCompressorResult};
use crate::types::{
    CompressionConfig, CompressionJob, CompressionJobStatus, CompressionOptions, CompressionPoint,
    CompressionStats, CompressionStrategy, Message, MessageRange, ShouldCompressResponse,
    TokenEstimation,
};

/// Context compressor implementation
#[derive(Clone)]
pub struct ContextCompressorImpl {
    // Compression points storage: point_id -> CompressionPoint
    points: Arc<RwLock<HashMap<String, CompressionPoint>>>,
    // Session compression history: session_id -> Vec<CompressionPoint>
    session_points: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // Async compression jobs: job_id -> CompressionJob
    jobs: Arc<RwLock<HashMap<String, CompressionJob>>>,
    // Compression statistics by session: session_id -> CompressionStats
    stats: Arc<RwLock<HashMap<String, CompressionStats>>>,
    // Configuration
    config: Arc<RwLock<CompressionConfig>>,
    // Initialization state
    initialized: Arc<RwLock<bool>>,
}

impl ContextCompressorImpl {
    /// Create a new context compressor
    pub fn new() -> Self {
        Self {
            points: Arc::new(RwLock::new(HashMap::new())),
            session_points: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(CompressionConfig::default())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Simple token estimation (characters / 4 as rough approximation)
    fn estimate_token_count(messages: &[Message]) -> TokenEstimation {
        let mut total = 0;
        let mut by_message = Vec::with_capacity(messages.len());

        for msg in messages {
            // Rough estimation: 1 token ≈ 4 characters
            let count = ((msg.content.len() + 2) / 4).max(1);
            total += count;
            by_message.push(count);
        }

        TokenEstimation {
            count: total,
            by_message,
        }
    }

    /// Generate summary using LLM (mock implementation)
    async fn generate_summary(
        &self,
        messages: &[Message],
        _strategy: CompressionStrategy,
    ) -> ContextCompressorResult<String> {
        // In production, this would call LLM Provider
        // For testing: generate a compact summary proportional to original
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();

        // Mock LLM summary: ~25% of original size, minimum 1 char
        let summary_chars = ((total_chars as f64) * 0.25).max(1.0) as usize;
        // Generate short placeholder text that's definitely shorter than original
        let placeholder = "x".repeat(summary_chars.min(total_chars.saturating_sub(1)).max(1));

        Ok(placeholder)
    }

    /// Preserve important messages based on content type
    fn select_preserved_messages(
        messages: &[Message],
        options: &CompressionOptions,
    ) -> Vec<Message> {
        let mut preserved = Vec::new();

        for msg in messages {
            let content_type = ContentType::detect(&msg.content, &msg.role);

            // Keep system messages if configured
            if content_type == ContentType::System && options.keep_system {
                preserved.push(msg.clone());
                continue;
            }

            // Keep messages with preserved keys
            if let Some(id) = msg.metadata.get("id").and_then(|v| v.as_str()) {
                if options.preserve_keys.contains(&id.to_string()) {
                    preserved.push(msg.clone());
                    continue;
                }
            }

            // Keep code and config types
            if content_type == ContentType::Code || content_type == ContentType::Config {
                preserved.push(msg.clone());
            }
        }

        preserved
    }

    /// Perform actual compression
    async fn perform_compression(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        strategy: CompressionStrategy,
        options: CompressionOptions,
    ) -> ContextCompressorResult<CompressionPoint> {
        let token_est = Self::estimate_token_count(&messages);
        let original_size = token_est.count;
        let message_count = messages.len();

        // Determine how many messages to compress (exclude recent ones)
        let to_compress_count = message_count.saturating_sub(options.keep_recent_messages);

        // If nothing to compress (all messages are "recent"), skip compression
        if to_compress_count == 0 {
            return Err(ContextCompressorError::CompressionFailed(
                "No messages to compress: all messages are within keep_recent_messages threshold"
                    .to_string(),
            ));
        }

        let range = MessageRange {
            start: 0,
            end: to_compress_count,
        };

        // Get messages to compress
        let to_compress: Vec<Message> = messages.into_iter().take(to_compress_count).collect();

        let preserved = Self::select_preserved_messages(&to_compress, &options);

        // Generate summary
        let summary = self.generate_summary(&to_compress, strategy).await?;

        // Calculate compression statistics
        let summary_tokens = (summary.len() + 2) / 4;
        let preserved_tokens: usize = preserved.iter().map(|m| (m.content.len() + 2) / 4).sum();

        // Token count for messages actually being compressed (not preserved)
        let to_compress_tokens: usize = to_compress
            .iter()
            .filter(|m| !preserved.iter().any(|p| p.id == m.id))
            .map(|m| (m.content.len() + 2) / 4)
            .sum();

        // compressed_size = summary + preserved (these replace the original messages)
        let compressed_size = summary_tokens + preserved_tokens;
        // token_saved = tokens from compressed messages - summary tokens (preserved are kept as-is)
        let token_saved = to_compress_tokens.saturating_sub(summary_tokens);
        let compression_ratio = if to_compress_tokens > 0 {
            token_saved as f64 / to_compress_tokens as f64
        } else {
            0.0
        };

        // Check minimum compression ratio
        if compression_ratio < options.min_compression_ratio {
            return Err(ContextCompressorError::CompressionFailed(format!(
                "Compression ratio {} below minimum {}",
                compression_ratio, options.min_compression_ratio
            )));
        }

        let replaced_ids: Vec<String> =
            to_compress.iter().map(|m| m.id.clone()).collect::<Vec<_>>();

        let point = CompressionPoint {
            id: format!("cp-{}", uuid::Uuid::new_v4()),
            session_id: session_id.to_string(),
            created_at: Utc::now(),
            strategy,
            message_range: range,
            message_count: to_compress.len(),
            token_count: original_size,
            summary,
            preserved_messages: preserved,
            original_size,
            compressed_size,
            token_saved,
            compression_ratio,
            replaced_message_ids: replaced_ids,
        };

        // Store the compression point
        let point_id = point.id.clone();
        self.points
            .write()
            .await
            .insert(point_id.clone(), point.clone());

        // Add to session's points
        self.session_points
            .write()
            .await
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(point_id.clone());

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            let stats_entry = stats
                .entry(session_id.to_string())
                .or_insert_with(CompressionStats::default);

            stats_entry.total_compressions += 1;
            stats_entry.total_tokens_saved += token_saved;
            stats_entry.avg_compression_ratio =
                stats_entry.total_tokens_saved as f64 / stats_entry.total_compressions as f64;
            stats_entry.last_compression = Some(Utc::now());
            stats_entry.tokens_used += original_size;
            stats_entry.budget_remaining = stats_entry
                .budget_limit
                .saturating_sub(stats_entry.tokens_used);

            use crate::types::CompressionHistoryEntry;
            stats_entry
                .compression_history
                .push(CompressionHistoryEntry {
                    point_id: point_id.clone(),
                    timestamp: Utc::now(),
                    strategy,
                    token_saved,
                    compression_ratio,
                });

            // Limit history size
            if stats_entry.compression_history.len() > 100 {
                stats_entry.compression_history.remove(0);
            }
        }

        tracing::info!(
            "Created compression point {} for session {}: {} tokens saved ({:.1}% ratio)",
            point_id,
            session_id,
            token_saved,
            compression_ratio * 100.0
        );

        Ok(point)
    }
}

impl Default for ContextCompressorImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextCompressor for ContextCompressorImpl {
    fn new() -> Result<Self, ContextCompressorError> {
        Ok(Self::new())
    }

    fn name(&self) -> &str {
        "context-compressor"
    }

    fn is_initialized(&self) -> bool {
        // This is a sync method but we have async internals
        // Use try_read to avoid blocking, returns false if locked
        self.initialized
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    async fn initialize(&self) -> ContextCompressorResult<()> {
        if *self.initialized.read().await {
            return Ok(());
        }

        // Initialize with default configuration
        let config = CompressionConfig::default();
        *self.config.write().await = config;

        *self.initialized.write().await = true;
        tracing::info!("Context compressor initialized");
        Ok(())
    }

    async fn should_compress(
        &self,
        session_id: &str,
    ) -> ContextCompressorResult<ShouldCompressResponse> {
        let stats = self
            .stats
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();

        // Check if compression is paused due to budget
        if let Some(paused_until) = stats.compression_paused_until {
            if Utc::now() < paused_until {
                return Ok(ShouldCompressResponse {
                    should: false,
                    reason: "Compression paused due to token budget limit".to_string(),
                    estimated_tokens: 0,
                });
            }
        }

        let config = self.config.read().await.clone();
        let trigger = &config.trigger;

        // For now, return that compression is not needed based on stats
        // In production, this would check actual message count and token count
        Ok(ShouldCompressResponse {
            should: trigger.auto_compress && stats.budget_remaining > 0,
            reason: if trigger.auto_compress {
                "Auto compression enabled".to_string()
            } else {
                "Auto compression disabled".to_string()
            },
            estimated_tokens: stats.budget_remaining,
        })
    }

    async fn compress(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        strategy: CompressionStrategy,
        options: CompressionOptions,
    ) -> ContextCompressorResult<CompressionPoint> {
        if !*self.initialized.read().await {
            return Err(ContextCompressorError::NotInitialized);
        }

        self.perform_compression(session_id, messages, strategy, options)
            .await
    }

    async fn compress_async(
        &self,
        session_id: &str,
        messages: Vec<Message>,
        strategy: CompressionStrategy,
    ) -> ContextCompressorResult<String> {
        if !*self.initialized.read().await {
            return Err(ContextCompressorError::NotInitialized);
        }

        let job_id = format!("job-{}", uuid::Uuid::new_v4());
        let job = CompressionJob {
            job_id: job_id.clone(),
            session_id: session_id.to_string(),
            status: CompressionJobStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        };

        self.jobs.write().await.insert(job_id.clone(), job);

        // Spawn async compression task
        let points = Arc::clone(&self.points);
        let session_points = Arc::clone(&self.session_points);
        let _stats = Arc::clone(&self.stats);
        let options = self.config.read().await.default_options.clone();
        let session_id_owned = session_id.to_string();
        let strategy_owned = strategy;
        let messages_owned = messages;

        tokio::spawn(async move {
            // Simulate some processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // In production, this would call perform_compression with real LLM
            let token_est = ContextCompressorImpl::estimate_token_count(&messages_owned);
            let original_size = token_est.count;

            let point = CompressionPoint {
                id: format!("cp-{}", uuid::Uuid::new_v4()),
                session_id: session_id_owned.clone(),
                created_at: Utc::now(),
                strategy: strategy_owned,
                message_range: MessageRange {
                    start: 0,
                    end: messages_owned
                        .len()
                        .saturating_sub(options.keep_recent_messages),
                },
                message_count: messages_owned.len(),
                token_count: original_size,
                summary: format!("[Compressed {} messages]", messages_owned.len()),
                preserved_messages: Vec::new(),
                original_size,
                compressed_size: original_size / 2,
                token_saved: original_size / 2,
                compression_ratio: 0.5,
                replaced_message_ids: messages_owned.iter().map(|m| m.id.clone()).collect(),
            };

            // Store point
            let point_id = point.id.clone();
            points.write().await.insert(point_id.clone(), point.clone());

            // Update session points
            session_points
                .write()
                .await
                .entry(session_id_owned.clone())
                .or_insert_with(Vec::new)
                .push(point_id.clone());

            // Update job status
            // Note: In real implementation, we'd update the job status
            tracing::info!(
                "Async compression completed for session {}",
                session_id_owned
            );
        });

        Ok(job_id)
    }

    async fn get_compression_job_status(
        &self,
        job_id: &str,
    ) -> ContextCompressorResult<CompressionJob> {
        self.jobs.read().await.get(job_id).cloned().ok_or_else(|| {
            ContextCompressorError::CompressionFailed(format!("Job not found: {}", job_id))
        })
    }

    async fn get_compression_points(
        &self,
        session_id: &str,
    ) -> ContextCompressorResult<Vec<CompressionPoint>> {
        let point_ids = self
            .session_points
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default();

        let mut points = Vec::new();
        let points_guard = self.points.read().await;

        for id in point_ids {
            if let Some(point) = points_guard.get(&id) {
                points.push(point.clone());
            }
        }

        Ok(points)
    }

    async fn get_compression_point(
        &self,
        point_id: &str,
    ) -> ContextCompressorResult<CompressionPoint> {
        self.points
            .read()
            .await
            .get(point_id)
            .cloned()
            .ok_or_else(|| ContextCompressorError::PointNotFound(point_id.to_string()))
    }

    async fn restore_compression_point(
        &self,
        point_id: &str,
    ) -> ContextCompressorResult<Vec<Message>> {
        let point = self.get_compression_point(point_id).await?;

        let mut messages = point.preserved_messages.clone();

        // Add back the summary as a special message
        messages.push(Message {
            id: format!("summary-{}", point_id),
            role: "system".to_string(),
            content: format!("[Compressed summary]: {}", point.summary),
            metadata: HashMap::new(),
        });

        Ok(messages)
    }

    async fn delete_compression_point(&self, point_id: &str) -> ContextCompressorResult<()> {
        let point = self.get_compression_point(point_id).await?;

        // Remove from points
        self.points.write().await.remove(point_id);

        // Remove from session points
        if let Some(ids) = self.session_points.write().await.get_mut(&point.session_id) {
            ids.retain(|id| id != point_id);
        }

        Ok(())
    }

    async fn estimate_tokens(
        &self,
        messages: &[Message],
    ) -> ContextCompressorResult<TokenEstimation> {
        Ok(Self::estimate_token_count(messages))
    }

    async fn get_compression_stats(
        &self,
        session_id: &str,
    ) -> ContextCompressorResult<CompressionStats> {
        Ok(self
            .stats
            .read()
            .await
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_compression_config(
        &self,
        _session_id: Option<&str>,
    ) -> ContextCompressorResult<CompressionConfig> {
        Ok(self.config.read().await.clone())
    }

    async fn update_compression_config(
        &self,
        config: CompressionConfig,
    ) -> ContextCompressorResult<()> {
        *self.config.write().await = config;
        Ok(())
    }
}
