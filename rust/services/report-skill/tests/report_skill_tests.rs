//! Unit tests for report-skill

use chrono::NaiveDate;
use report_skill::{GenerateReportRequest, OutputFormat, ReportSkill, ReportSkillImpl, ReportType};

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

    let template = report_skill::ReportTemplate {
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
