//! Report skill trait definition

use crate::error::{ReportSkillError, ReportSkillResult};
use crate::types::{GenerateReportRequest, Report, ReportTemplate};

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
