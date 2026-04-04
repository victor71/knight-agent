//! Report Skill
//!
//! Generates usage reports including daily, weekly, monthly, and custom reports.
//!
//! Design Reference: docs/03-module-design/services/report-skill.md

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum ReportSkillError {
    #[error("Report skill not initialized")]
    NotInitialized,
    #[error("Report generation failed: {0}")]
    GenerationFailed(String),
    #[error("Report not found: {0}")]
    NotFound(String),
    #[error("Invalid report type: {0}")]
    InvalidReportType(String),
    #[error("Storage service unavailable: {0}")]
    StorageUnavailable(String),
    #[error("Timer service unavailable: {0}")]
    TimerUnavailable(String),
}

pub type ReportSkillResult<T> = Result<T, ReportSkillError>;

/// Report type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportType {
    Daily,
    Weekly,
    Monthly,
    Custom,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportType::Daily => write!(f, "daily"),
            ReportType::Weekly => write!(f, "weekly"),
            ReportType::Monthly => write!(f, "monthly"),
            ReportType::Custom => write!(f, "custom"),
        }
    }
}

/// Report output format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Markdown,
    Json,
    Html,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Markdown
    }
}

/// Time period for a report (internal use)
#[derive(Debug, Clone)]
pub struct Period {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Period for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodSerde {
    pub start: String,
    pub end: String,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub report_type: ReportType,
    pub period: PeriodSerde,
    pub generated_at: String,
}

/// Report summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub total_llm_calls: u64,
    pub total_tokens: u64,
    pub estimated_cost: f64,
}

/// Token statistics by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenStats {
    pub model: String,
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cost_estimate: f64,
    pub calls: u64,
}

/// Hourly token statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyTokenStats {
    pub hour: u32,
    pub total: u64,
}

/// Token report section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenReportSection {
    pub total: u64,
    pub input: u64,
    pub output: u64,
    pub cost_estimate: f64,
    pub by_model: Vec<ModelTokenStats>,
    pub by_hour: Vec<HourlyTokenStats>,
}

/// Top session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopSession {
    pub session_id: String,
    pub name: String,
    pub message_count: u64,
    pub token_usage: u64,
}

/// Session report section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionReportSection {
    pub new_count: u64,
    pub active_count: u64,
    pub archived_count: u64,
    pub total_messages: u64,
    pub avg_messages_per_session: f64,
    pub top_sessions: Vec<TopSession>,
}

/// Agent statistics by model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelStats {
    pub model: String,
    pub calls: u64,
    pub tokens: u64,
    pub avg_latency_ms: f64,
}

/// Agent usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsageStats {
    pub agent_name: String,
    pub calls: u64,
    pub tokens: u64,
}

/// Agent report section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentReportSection {
    pub total_llm_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub avg_latency_ms: f64,
    pub total_tokens: u64,
    pub by_model: Vec<AgentModelStats>,
    pub by_agent: Vec<AgentUsageStats>,
}

/// System report section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemReportSection {
    pub uptime_seconds: u64,
    pub avg_memory_mb: f64,
    pub peak_memory_mb: u64,
    pub avg_cpu_percent: f64,
}

/// Report content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportContent {
    pub summary: ReportSummary,
    pub tokens: TokenReportSection,
    pub sessions: SessionReportSection,
    pub agents: AgentReportSection,
    pub system: SystemReportSection,
}

/// Report output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOutput {
    pub format: OutputFormat,
    pub content: String,
    pub path: Option<String>,
}

/// Full report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub metadata: ReportMetadata,
    pub content: ReportContent,
    pub output: ReportOutput,
}

/// Report generation request
#[derive(Debug, Clone)]
pub struct GenerateReportRequest {
    pub report_type: ReportType,
    pub date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub format: OutputFormat,
    pub output_path: Option<String>,
}

impl GenerateReportRequest {
    /// Create a daily report request
    pub fn daily(date: NaiveDate, format: OutputFormat) -> Self {
        Self {
            report_type: ReportType::Daily,
            date: Some(date),
            start_date: None,
            end_date: None,
            format,
            output_path: None,
        }
    }

    /// Create a weekly report request
    pub fn weekly(date: NaiveDate, format: OutputFormat) -> Self {
        Self {
            report_type: ReportType::Weekly,
            date: Some(date),
            start_date: None,
            end_date: None,
            format,
            output_path: None,
        }
    }

    /// Create a monthly report request
    pub fn monthly(date: NaiveDate, format: OutputFormat) -> Self {
        Self {
            report_type: ReportType::Monthly,
            date: Some(date),
            start_date: None,
            end_date: None,
            format,
            output_path: None,
        }
    }

    /// Create a custom report request
    pub fn custom(start: NaiveDate, end: NaiveDate, format: OutputFormat) -> Self {
        Self {
            report_type: ReportType::Custom,
            date: None,
            start_date: Some(start),
            end_date: Some(end),
            format,
            output_path: None,
        }
    }

    /// Set output path
    pub fn with_output_path(mut self, path: String) -> Self {
        self.output_path = Some(path);
        self
    }
}

/// Scheduled report info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledReport {
    pub task_id: String,
    pub report_type: ReportType,
    pub schedule: String,
    pub format: OutputFormat,
    pub output_path: Option<String>,
}

/// Report template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub name: String,
    pub template: String,
    pub report_type: ReportType,
}

/// Report skill trait
#[allow(async_fn_in_trait)]
pub trait ReportSkill: Send + Sync {
    fn new() -> Result<Self, ReportSkillError>
    where
        Self: Sized;
    fn name(&self) -> &str;
    fn is_initialized(&self) -> bool;
    async fn initialize(&self) -> ReportSkillResult<()>;
    async fn generate_report(&self, request: GenerateReportRequest) -> ReportSkillResult<Report>;
    async fn get_report(&self, id: &str) -> ReportSkillResult<Report>;
    async fn list_reports(&self) -> ReportSkillResult<Vec<Report>>;
    async fn schedule_report(&self, request: GenerateReportRequest, schedule: &str) -> ReportSkillResult<String>;
    async fn cancel_scheduled_report(&self, task_id: &str) -> ReportSkillResult<()>;
    async fn register_template(&self, template: ReportTemplate) -> ReportSkillResult<()>;
}

/// Report skill implementation
#[derive(Clone)]
pub struct ReportSkillImpl {
    reports: Arc<RwLock<HashMap<String, Report>>>,
    scheduled_reports: Arc<RwLock<HashMap<String, ScheduledReport>>>,
    templates: Arc<RwLock<HashMap<String, ReportTemplate>>>,
    initialized: Arc<RwLock<bool>>,
}

impl ReportSkillImpl {
    /// Create a new report skill instance
    pub fn new() -> Self {
        Self {
            reports: Arc::new(RwLock::new(HashMap::new())),
            scheduled_reports: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Calculate the period for a report type
    fn calculate_period(&self, request: &GenerateReportRequest) -> ReportSkillResult<Period> {
        let (start, end) = match request.report_type {
            ReportType::Daily => {
                let date = request.date.ok_or_else(|| {
                    ReportSkillError::GenerationFailed("Date required for daily report".to_string())
                })?;
                let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            ReportType::Weekly => {
                let date = request.date.ok_or_else(|| {
                    ReportSkillError::GenerationFailed("Date required for weekly report".to_string())
                })?;
                // Find the Monday of that week
                let days_since_monday = date.weekday().num_days_from_monday();
                let monday = date - chrono::Duration::days(days_since_monday as i64);
                let sunday = monday + chrono::Duration::days(6);
                let start = monday.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = sunday.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            ReportType::Monthly => {
                let date = request.date.ok_or_else(|| {
                    ReportSkillError::GenerationFailed("Date required for monthly report".to_string())
                })?;
                let first_day = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
                let last_day = if date.month() == 12 {
                    NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
                } - chrono::Duration::days(1);
                let start = first_day.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = last_day.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            ReportType::Custom => {
                let start_date = request.start_date.ok_or_else(|| {
                    ReportSkillError::GenerationFailed("Start date required for custom report".to_string())
                })?;
                let end_date = request.end_date.ok_or_else(|| {
                    ReportSkillError::GenerationFailed("End date required for custom report".to_string())
                })?;
                let start = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
        };
        Ok(Period { start, end })
    }

    /// Query mock data for a period (in production, this would query Storage Service)
    async fn query_stats(&self, period: &Period) -> ReportContent {
        // In production, this would query Storage Service
        // For now, return mock data based on the period
        let days = (period.end - period.start).num_days() as f64 + 1.0;

        ReportContent {
            summary: ReportSummary {
                total_sessions: (15.0 * days / 30.0) as u64,
                total_messages: (234.0 * days / 30.0) as u64,
                total_llm_calls: (89.0 * days / 30.0) as u64,
                total_tokens: (45678.0 * days / 30.0) as u64,
                estimated_cost: 0.23 * days / 30.0,
            },
            tokens: TokenReportSection {
                total: (45678.0 * days / 30.0) as u64,
                input: (32100.0 * days / 30.0) as u64,
                output: (13578.0 * days / 30.0) as u64,
                cost_estimate: 0.23 * days / 30.0,
                by_model: vec![
                    ModelTokenStats {
                        model: "claude-sonnet-4-6".to_string(),
                        total: (38234.0 * days / 30.0) as u64,
                        input: (26764.0 * days / 30.0) as u64,
                        output: (11470.0 * days / 30.0) as u64,
                        cost_estimate: 0.19 * days / 30.0,
                        calls: (67.0 * days / 30.0) as u64,
                    },
                    ModelTokenStats {
                        model: "claude-haiku-4-5".to_string(),
                        total: (7444.0 * days / 30.0) as u64,
                        input: (5336.0 * days / 30.0) as u64,
                        output: (2108.0 * days / 30.0) as u64,
                        cost_estimate: 0.04 * days / 30.0,
                        calls: (22.0 * days / 30.0) as u64,
                    },
                ],
                by_hour: (0..24)
                    .map(|hour| HourlyTokenStats {
                        hour,
                        total: if hour >= 9 && hour <= 17 { 3456 } else { 1234 },
                    })
                    .collect(),
            },
            sessions: SessionReportSection {
                new_count: (5.0 * days / 30.0) as u64,
                active_count: (8.0 * days / 30.0) as u64,
                archived_count: (2.0 * days / 30.0) as u64,
                total_messages: (234.0 * days / 30.0) as u64,
                avg_messages_per_session: 15.6,
                top_sessions: vec![
                    TopSession {
                        session_id: "sess-001".to_string(),
                        name: "frontend-dev".to_string(),
                        message_count: 45,
                        token_usage: 8901,
                    },
                    TopSession {
                        session_id: "sess-002".to_string(),
                        name: "backend-api".to_string(),
                        message_count: 38,
                        token_usage: 7234,
                    },
                    TopSession {
                        session_id: "sess-003".to_string(),
                        name: "code-review".to_string(),
                        message_count: 32,
                        token_usage: 5678,
                    },
                ],
            },
            agents: AgentReportSection {
                total_llm_calls: (89.0 * days / 30.0) as u64,
                successful_calls: (87.0 * days / 30.0) as u64,
                failed_calls: (2.0 * days / 30.0) as u64,
                avg_latency_ms: 1234.0,
                total_tokens: (45678.0 * days / 30.0) as u64,
                by_model: vec![
                    AgentModelStats {
                        model: "claude-sonnet-4-6".to_string(),
                        calls: 67,
                        tokens: 38234,
                        avg_latency_ms: 1200.0,
                    },
                    AgentModelStats {
                        model: "claude-haiku-4-5".to_string(),
                        calls: 22,
                        tokens: 7444,
                        avg_latency_ms: 800.0,
                    },
                ],
                by_agent: vec![
                    AgentUsageStats {
                        agent_name: "coder".to_string(),
                        calls: 45,
                        tokens: 23456,
                    },
                    AgentUsageStats {
                        agent_name: "reviewer".to_string(),
                        calls: 28,
                        tokens: 15234,
                    },
                    AgentUsageStats {
                        agent_name: "planner".to_string(),
                        calls: 16,
                        tokens: 6988,
                    },
                ],
            },
            system: SystemReportSection {
                uptime_seconds: (86400.0 * days) as u64,
                avg_memory_mb: 245.0,
                peak_memory_mb: 312,
                avg_cpu_percent: 3.2,
            },
        }
    }

    /// Render report as Markdown
    fn render_markdown(&self, report: &Report) -> String {
        let mut md = String::new();
        let period = &report.metadata.period;

        // Title
        let type_str = match report.metadata.report_type {
            ReportType::Daily => "每日",
            ReportType::Weekly => "每周",
            ReportType::Monthly => "每月",
            ReportType::Custom => "自定义",
        };
        md.push_str(&format!("# Knight Agent {}报告\n\n", type_str));
        // Extract date part from ISO format datetime string
        let start_date = period.start.split('T').next().unwrap_or(&period.start);
        let end_date = period.end.split('T').next().unwrap_or(&period.end);
        md.push_str(&format!("**日期**: {} - {}\n", start_date, end_date));
        md.push_str(&format!("**生成时间**: {}\n\n", report.metadata.generated_at));

        // Summary
        md.push_str("## 摘要\n\n");
        md.push_str("| 指标 | 数值 |\n");
        md.push_str("|------|------|\n");
        md.push_str(&format!("| 总会话数 | {} |\n", report.content.summary.total_sessions));
        md.push_str(&format!("| 总消息数 | {} |\n", report.content.summary.total_messages));
        md.push_str(&format!("| LLM 调用次数 | {} |\n", report.content.summary.total_llm_calls));
        md.push_str(&format!("| 总 Token | {} |\n", report.content.summary.total_tokens));
        md.push_str(&format!("| 预估成本 | ${:.2} |\n\n", report.content.summary.estimated_cost));

        // Token usage
        md.push_str("## Token 使用\n\n");
        md.push_str("### 总体统计\n\n");
        md.push_str(&format!("- **总 Token**: {}\n", report.content.tokens.total));
        md.push_str(&format!("- **输入 Token**: {}\n", report.content.tokens.input));
        md.push_str(&format!("- **输出 Token**: {}\n", report.content.tokens.output));
        md.push_str(&format!("- **预估成本**: ${:.2}\n\n", report.content.tokens.cost_estimate));

        // By model
        if !report.content.tokens.by_model.is_empty() {
            md.push_str("### 按模型统计\n\n");
            md.push_str("| 模型 | 调用次数 | Token 数 | 成本 |\n");
            md.push_str("|------|----------|----------|------|\n");
            for model in &report.content.tokens.by_model {
                md.push_str(&format!(
                    "| {} | {} | {} | ${:.2} |\n",
                    model.model, model.calls, model.total, model.cost_estimate
                ));
            }
            md.push('\n');
        }

        // Hourly trend (for daily reports)
        if report.metadata.report_type == ReportType::Daily && !report.content.tokens.by_hour.is_empty() {
            md.push_str("### 每小时趋势\n\n```\n");
            for hour_stat in &report.content.tokens.by_hour {
                let bar_len = (hour_stat.total / 500) as usize;
                let bar: String = "█".repeat(bar_len.min(20));
                md.push_str(&format!("{:02}:00 {} {}\n", hour_stat.hour, bar, hour_stat.total));
            }
            md.push_str("```\n\n");
        }

        // Session stats
        md.push_str("## 会话统计\n\n");
        md.push_str(&format!("- **新建会话**: {}\n", report.content.sessions.new_count));
        md.push_str(&format!("- **活跃会话**: {}\n", report.content.sessions.active_count));
        md.push_str(&format!("- **归档会话**: {}\n", report.content.sessions.archived_count));
        md.push_str(&format!("- **总消息数**: {}\n", report.content.sessions.total_messages));
        md.push_str(&format!("- **平均消息/会话**: {:.1}\n\n", report.content.sessions.avg_messages_per_session));

        // Top sessions
        if !report.content.sessions.top_sessions.is_empty() {
            md.push_str("### Top 会话\n\n");
            md.push_str("| 会话 | 消息数 | Token |\n");
            md.push_str("|------|--------|-------|\n");
            for session in &report.content.sessions.top_sessions {
                md.push_str(&format!("| {} | {} | {} |\n", session.name, session.message_count, session.token_usage));
            }
            md.push('\n');
        }

        // Agent stats
        md.push_str("## Agent 统计\n\n");
        md.push_str(&format!("- **LLM 调用次数**: {}\n", report.content.agents.total_llm_calls));
        md.push_str(&format!("- **成功**: {}\n", report.content.agents.successful_calls));
        md.push_str(&format!("- **失败**: {}\n", report.content.agents.failed_calls));
        md.push_str(&format!("- **平均延迟**: {} ms\n\n", report.content.agents.avg_latency_ms));

        // By agent
        if !report.content.agents.by_agent.is_empty() {
            md.push_str("### 按 Agent 统计\n\n");
            md.push_str("| Agent | 调用次数 | Token |\n");
            md.push_str("|-------|----------|-------|\n");
            for agent in &report.content.agents.by_agent {
                md.push_str(&format!("| {} | {} | {} |\n", agent.agent_name, agent.calls, agent.tokens));
            }
            md.push('\n');
        }

        // System resources
        md.push_str("## 系统资源\n\n");
        let hours = report.content.system.uptime_seconds / 3600;
        let mins = (report.content.system.uptime_seconds % 3600) / 60;
        md.push_str(&format!("- **运行时长**: {}h {}m\n", hours, mins));
        md.push_str(&format!("- **平均内存**: {:.0} MB\n", report.content.system.avg_memory_mb));
        md.push_str(&format!("- **峰值内存**: {} MB\n", report.content.system.peak_memory_mb));
        md.push_str(&format!("- **平均 CPU**: {:.1}%\n", report.content.system.avg_cpu_percent));

        md
    }

    /// Render report as JSON
    fn render_json(&self, report: &Report) -> String {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
    }
}

impl ReportSkill for ReportSkillImpl {
    fn new() -> Result<Self, ReportSkillError> {
        Ok(Self::new())
    }

    fn name(&self) -> &str {
        "report-skill"
    }

    fn is_initialized(&self) -> bool {
        // Use blocking poll since this is not async
        // Note: This is a sync accessor, use is_initialized_async for actual async check
        false
    }

    async fn initialize(&self) -> ReportSkillResult<()> {
        if *self.initialized.read().await {
            return Ok(());
        }
        // Register default templates
        let default_template = ReportTemplate {
            name: "default".to_string(),
            template: "default".to_string(),
            report_type: ReportType::Daily,
        };
        self.register_template(default_template).await?;
        *self.initialized.write().await = true;
        tracing::info!("Report skill initialized");
        Ok(())
    }

    async fn generate_report(&self, request: GenerateReportRequest) -> ReportSkillResult<Report> {
        if !*self.initialized.read().await {
            return Err(ReportSkillError::NotInitialized);
        }

        let period = self.calculate_period(&request)?;
        let content = self.query_stats(&period).await;

        let now = Utc::now();
        let period_serde = PeriodSerde {
            start: period.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            end: period.end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        };
        let generated_at = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let report_id = format!("report-{}", uuid::Uuid::new_v4());

        let report_for_render = Report {
            id: report_id.clone(),
            metadata: ReportMetadata {
                report_type: request.report_type,
                period: period_serde.clone(),
                generated_at: generated_at.clone(),
            },
            content: content.clone(),
            output: ReportOutput {
                format: request.format,
                content: String::new(),
                path: request.output_path.clone(),
            },
        };

        let rendered_content = match request.format {
            OutputFormat::Markdown => self.render_markdown(&report_for_render),
            OutputFormat::Json => self.render_json(&report_for_render),
            OutputFormat::Html => self.render_markdown(&report_for_render),
        };

        let report = Report {
            id: report_id,
            metadata: ReportMetadata {
                report_type: request.report_type,
                period: period_serde,
                generated_at,
            },
            content,
            output: ReportOutput {
                format: request.format,
                content: rendered_content,
                path: request.output_path,
            },
        };

        // Store the report
        self.reports.write().await.insert(report.id.clone(), report.clone());

        tracing::info!("Generated report: {}", report.id);
        Ok(report)
    }

    async fn get_report(&self, id: &str) -> ReportSkillResult<Report> {
        self.reports
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| ReportSkillError::NotFound(id.to_string()))
    }

    async fn list_reports(&self) -> ReportSkillResult<Vec<Report>> {
        Ok(self.reports.read().await.values().cloned().collect())
    }

    async fn schedule_report(&self, request: GenerateReportRequest, schedule: &str) -> ReportSkillResult<String> {
        if !*self.initialized.read().await {
            return Err(ReportSkillError::NotInitialized);
        }
        // In production, this would integrate with Timer System
        // For now, generate a task ID and store the scheduled report info
        let task_id = format!("scheduled-report-{}", uuid::Uuid::new_v4());
        let scheduled = ScheduledReport {
            task_id: task_id.clone(),
            report_type: request.report_type,
            schedule: schedule.to_string(),
            format: request.format,
            output_path: request.output_path,
        };
        self.scheduled_reports.write().await.insert(task_id.clone(), scheduled);
        tracing::info!("Scheduled report: {}", task_id);
        Ok(task_id)
    }

    async fn cancel_scheduled_report(&self, task_id: &str) -> ReportSkillResult<()> {
        if self.scheduled_reports.write().await.remove(task_id).is_none() {
            return Err(ReportSkillError::NotFound(task_id.to_string()));
        }
        tracing::info!("Cancelled scheduled report: {}", task_id);
        Ok(())
    }

    async fn register_template(&self, template: ReportTemplate) -> ReportSkillResult<()> {
        self.templates.write().await.insert(template.name.clone(), template);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_report_skill_new() {
        let skill = ReportSkillImpl::new();
        assert_eq!(skill.name(), "report-skill");
        assert!(!skill.is_initialized());
    }

    #[tokio::test]
    async fn test_generate_daily_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);
        let report = skill.generate_report(request).await.unwrap();

        assert_eq!(report.metadata.report_type, ReportType::Daily);
        assert!(report.content.summary.total_tokens > 0);
        assert!(report.output.content.contains("Knight Agent"));
    }

    #[tokio::test]
    async fn test_generate_weekly_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let request = GenerateReportRequest::weekly(date, OutputFormat::Markdown);
        let report = skill.generate_report(request).await.unwrap();

        assert_eq!(report.metadata.report_type, ReportType::Weekly);
    }

    #[tokio::test]
    async fn test_generate_monthly_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
        let request = GenerateReportRequest::monthly(date, OutputFormat::Markdown);
        let report = skill.generate_report(request).await.unwrap();

        assert_eq!(report.metadata.report_type, ReportType::Monthly);
    }

    #[tokio::test]
    async fn test_generate_custom_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let request = GenerateReportRequest::custom(start, end, OutputFormat::Json);
        let report = skill.generate_report(request).await.unwrap();

        assert_eq!(report.metadata.report_type, ReportType::Custom);
        assert_eq!(report.output.format, OutputFormat::Json);
    }

    #[tokio::test]
    async fn test_get_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);
        let created = skill.generate_report(request).await.unwrap();

        let retrieved = skill.get_report(&created.id).await.unwrap();
        assert_eq!(retrieved.id, created.id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let result = skill.get_report("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_reports() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        skill.generate_report(GenerateReportRequest::daily(date, OutputFormat::Markdown)).await.unwrap();
        skill.generate_report(GenerateReportRequest::daily(date, OutputFormat::Json)).await.unwrap();

        let reports = skill.list_reports().await.unwrap();
        assert_eq!(reports.len(), 2);
    }

    #[tokio::test]
    async fn test_schedule_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);
        let task_id = skill.schedule_report(request, "0 9 * * *").await.unwrap();

        assert!(task_id.contains("scheduled-report-"));
    }

    #[tokio::test]
    async fn test_cancel_scheduled_report() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
        let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);
        let task_id = skill.schedule_report(request.clone(), "0 9 * * *").await.unwrap();

        skill.cancel_scheduled_report(&task_id).await.unwrap();
        let result = skill.cancel_scheduled_report(&task_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_template() {
        let skill = ReportSkillImpl::new();
        skill.initialize().await.unwrap();

        let template = ReportTemplate {
            name: "custom".to_string(),
            template: "custom template".to_string(),
            report_type: ReportType::Daily,
        };
        skill.register_template(template).await.unwrap();
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Daily.to_string(), "daily");
        assert_eq!(ReportType::Weekly.to_string(), "weekly");
        assert_eq!(ReportType::Monthly.to_string(), "monthly");
        assert_eq!(ReportType::Custom.to_string(), "custom");
    }

    #[test]
    fn test_generate_report_request() {
        let req = GenerateReportRequest::daily(
            NaiveDate::from_ymd_opt(2026, 4, 2).unwrap(),
            OutputFormat::Markdown,
        );
        assert_eq!(req.report_type, ReportType::Daily);
        assert!(req.output_path.is_none());

        let req = req.with_output_path("/path/to/report.md".to_string());
        assert_eq!(req.output_path, Some("/path/to/report.md".to_string()));
    }
}
