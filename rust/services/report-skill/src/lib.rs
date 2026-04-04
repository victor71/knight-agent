//! Report Skill
//!
//! Generates usage reports including daily, weekly, monthly, and custom reports.
//!
//! Design Reference: docs/03-module-design/services/report-skill.md

// Re-export modules
pub mod error;
pub mod service;
pub mod trait_def;
pub mod types;

// Re-export public API
pub use error::{ReportSkillError, ReportSkillResult};
pub use service::ReportSkillImpl;
pub use trait_def::ReportSkill;
pub use types::{
    AgentModelStats, AgentReportSection, AgentUsageStats, GenerateReportRequest, HourlyTokenStats,
    ModelTokenStats, OutputFormat, Period, PeriodSerde, Report, ReportContent, ReportMetadata,
    ReportOutput, ReportSummary, ReportTemplate, ReportType, ScheduledReport, SessionReportSection,
    SystemReportSection, TopSession,
};
