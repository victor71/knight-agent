# Report Skill Module

Generates usage reports including daily, weekly, monthly, and custom reports.

Design Reference: `docs/03-module-design/services/report-skill.md`

## Features

- **Report Generation**: Generate reports for daily, weekly, monthly, or custom date ranges
- **Multiple Formats**: Output reports in Markdown, JSON, or HTML format
- **Scheduled Reports**: Schedule reports to run automatically via Timer System integration
- **Template Support**: Register custom report templates
- **Comprehensive Statistics**: Token usage, session stats, agent stats, and system metrics

## API

### ReportSkill Trait

```rust
pub trait ReportSkill: Send + Sync {
    fn new() -> Result<Self, ReportSkillError>;
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
```

### Report Types

```rust
// Daily report
let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);

// Weekly report
let request = GenerateReportRequest::weekly(date, OutputFormat::Markdown);

// Monthly report
let request = GenerateReportRequest::monthly(date, OutputFormat::Markdown);

// Custom date range
let request = GenerateReportRequest::custom(start_date, end_date, OutputFormat::Json);

// With output path
let request = GenerateReportRequest::daily(date, OutputFormat::Markdown)
    .with_output_path("/path/to/report.md");
```

### Output Formats

```rust
pub enum OutputFormat {
    Markdown,  // Default, human-readable format
    Json,      // Structured JSON data
    Html,      // HTML format (renders as Markdown)
}
```

### Report Structure

```rust
pub struct Report {
    pub id: String,
    pub metadata: ReportMetadata,
    pub content: ReportContent,
    pub output: ReportOutput,
}

pub struct ReportContent {
    pub summary: ReportSummary,       // Overall statistics
    pub tokens: TokenReportSection,   // Token usage breakdown
    pub sessions: SessionReportSection, // Session statistics
    pub agents: AgentReportSection,   // Agent call statistics
    pub system: SystemReportSection,  // System resource usage
}
```

## Usage Example

```rust
use report_skill::{ReportSkill, ReportSkillImpl, GenerateReportRequest, OutputFormat};
use chrono::NaiveDate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let skill = ReportSkillImpl::new();
    skill.initialize().await?;

    // Generate daily report
    let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();
    let request = GenerateReportRequest::daily(date, OutputFormat::Markdown);
    let report = skill.generate_report(request).await?;

    println!("Generated report: {}", report.id);
    println!("{}", report.output.content);

    Ok(())
}
```

## Dependencies

- **Storage Service**: Query historical statistics (for future integration)
- **Timer System**: Schedule recurring reports (for future integration)
- **Logging System**: Log report generation events
