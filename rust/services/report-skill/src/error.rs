//! Error types for report skill

use thiserror::Error;

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
