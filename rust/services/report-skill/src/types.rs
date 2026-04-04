//! Type definitions for report skill

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

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
