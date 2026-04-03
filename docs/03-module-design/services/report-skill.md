# Report Skill (报告技能)

## 概述

### 职责描述

Report Skill 是一个内置技能，负责生成各种使用报告，包括：

- 每日使用报告（会话、消息、Token、LLM 调用）
- 每周/每月汇总报告
- 自定义时间范围报告
- 报告输出（Markdown、JSON、邮件）

### 设计目标

1. **自动化**: 通过 Timer System 自动触发
2. **灵活**: 支持多种报告类型和输出格式
3. **高效**: 利用持久化数据，避免重复计算
4. **可扩展**: 支持自定义报告模板

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Storage Service | 依赖 | 查询历史统计数据。见 [Storage Service 接口](./storage-service.md) |
| Monitor | 可选 | 仅在需要实时数据时使用，大部分报告使用 Storage Service 的历史数据 |
| Timer System | 依赖 | 定时触发报告生成。见 [Timer System 接口](./timer-system.md) |
| Logging System | 依赖 | 记录报告生成日志。见 [Logging System 接口](./logging-system.md) |

**注意**：Report Skill 主要使用 Storage Service 的 `query_stats_range` 接口获取历史统计数据，Monitor 仅在需要实时数据时才使用。

---

## 接口定义

### 对外接口

```yaml
# Report Skill 接口定义
ReportSkill:
  # ========== 报告生成 ==========
  generate_report:
    description: 生成报告
    inputs:
      type:
        type: string
        enum: [daily, weekly, monthly, custom]
        required: true
      date:
        type: date
        description: 报告日期（daily 时使用）
      start_date:
        type: date
        description: 开始日期（custom 时使用）
      end_date:
        type: date
        description: 结束日期（custom 时使用）
      format:
        type: string
        enum: [markdown, json, html]
        default: markdown
      output:
        type: string
        description: 输出路径（可选）
    outputs:
      report:
        type: Report

  # ========== 定时报告 ==========
  schedule_report:
    description: 创建定时报告任务
    inputs:
      type:
        type: string
        enum: [daily, weekly, monthly]
      schedule:
        type: string
        description: Cron 表达式（如 "0 9 * * *" 表示每天 9 点）
      output:
        type: string
        description: 输出路径或邮件地址
    outputs:
      task_id:
        type: string

  cancel_scheduled_report:
    description: 取消定时报告
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 报告模板 ==========
  register_template:
    description: 注册自定义报告模板
    inputs:
      name:
        type: string
        required: true
      template:
        type: ReportTemplate
        required: true
    outputs:
      success:
        type: boolean
```

---

## 数据结构

### Report 结构

```yaml
Report:
  # 元数据
  metadata:
    type:
      type: string
      description: 报告类型 (daily/weekly/monthly/custom)
    period:
      type: Period
      description: 报告时间段
    generated_at:
      type: datetime
      description: 生成时间

  # 内容
  content:
    summary:
      type: ReportSummary
      description: 报告摘要
    tokens:
      type: TokenReportSection
    sessions:
      type: SessionReportSection
    agents:
      type: AgentReportSection
    system:
      type: SystemReportSection

  # 输出
  output:
    format:
      type: string
      description: 输出格式
    content:
      type: string
      description: 格式化后的内容
    path:
      type: string
      description: 输出文件路径（如保存到文件）
```

### ReportSummary 结构

```yaml
ReportSummary:
  total_sessions:
    type: integer
    description: 总会话数
  total_messages:
    type: integer
    description: 总消息数
  total_llm_calls:
    type: integer
    description: 总 LLM 调用次数
  total_tokens:
    type: integer
    description: 总消耗 Token
  estimated_cost:
    type: float
    description: 预估成本（美元）
```

### TokenReportSection 结构

```yaml
TokenReportSection:
  total:
    type: integer
  input:
    type: integer
  output:
    type: integer
  cost_estimate:
    type: float
  by_model:
    type: array<ModelTokenStats>
  by_session:
    type: array<SessionTokenStats>
  by_hour:
    type: array<HourlyTokenStats>
    description: 每小时 Token 使用（daily 报告）

ModelTokenStats:
  model:
    type: string
  total:
    type: integer
  input:
    type: integer
  output:
    type: integer
  cost_estimate:
    type: float
  calls:
    type: integer

SessionTokenStats:
  session_id:
    type: string
  session_name:
    type: string
  total:
    type: integer

HourlyTokenStats:
  hour:
    type: integer
  total:
    type: integer
```

### SessionReportSection 结构

```yaml
SessionReportSection:
  new_count:
    type: integer
  active_count:
    type: integer
  archived_count:
    type: integer
  total_messages:
    type: integer
  avg_messages_per_session:
    type: float
  top_sessions:
    type: array<TopSession>

TopSession:
  session_id:
    type: string
  name:
    type: string
  message_count:
    type: integer
  token_usage:
    type: integer
```

### AgentReportSection 结构

```yaml
AgentReportSection:
  total_llm_calls:
    type: integer
  successful_calls:
    type: integer
  failed_calls:
    type: integer
  avg_latency_ms:
    type: float
  total_tokens:
    type: integer
  by_model:
    type: array<AgentModelStats>
  by_agent:
    type: array<AgentUsageStats>

AgentModelStats:
  model:
    type: string
  calls:
    type: integer
  tokens:
    type: integer
  avg_latency_ms:
    type: float

AgentUsageStats:
  agent_name:
    type: string
  calls:
    type: integer
  tokens:
    type: integer
```

### SystemReportSection 结构

```yaml
SystemReportSection:
  uptime_seconds:
    type: integer
  avg_memory_mb:
    type: float
  peak_memory_mb:
    type: integer
  avg_cpu_percent:
    type: float
```

---

## 报告格式

### Markdown 格式

```markdown
# Knight Agent 每日报告

**日期**: 2026-04-02
**生成时间**: 2026-04-02 09:00:00

## 摘要

| 指标 | 数值 |
|------|------|
| 总会话数 | 15 |
| 总消息数 | 234 |
| LLM 调用次数 | 89 |
| 总 Token | 45,678 |
| 预估成本 | $0.23 |

## Token 使用

### 总体统计

- **总 Token**: 45,678
- **输入 Token**: 32,100
- **输出 Token**: 13,578
- **预估成本**: $0.23

### 按模型统计

| 模型 | 调用次数 | Token 数 | 成本 |
|------|----------|----------|------|
| claude-sonnet-4-6 | 67 | 38,234 | $0.19 |
| claude-haiku-4-5 | 22 | 7,444 | $0.04 |

### 每小时趋势

```
00:00 ████ 2,345
01:00 ██ 1,234
02:00 █ 567
...
23:00 ██████ 3,456
```

## 会话统计

- **新建会话**: 5
- **活跃会话**: 8
- **归档会话**: 2
- **总消息数**: 234
- **平均消息/会话**: 15.6

### Top 会话

| 会话 | 消息数 | Token |
|------|--------|-------|
| frontend-dev | 45 | 8,901 |
| backend-api | 38 | 7,234 |
| code-review | 32 | 5,678 |

## Agent 统计

- **LLM 调用次数**: 89
- **成功**: 87
- **失败**: 2
- **平均延迟**: 1,234 ms

### 按 Agent 统计

| Agent | 调用次数 | Token |
|-------|----------|-------|
| coder | 45 | 23,456 |
| reviewer | 28 | 15,234 |
| planner | 16 | 6,988 |

## 系统资源

- **运行时长**: 24h 00m
- **平均内存**: 245 MB
- **峰值内存**: 312 MB
- **平均 CPU**: 3.2%
```

### JSON 格式

```json
{
  "metadata": {
    "type": "daily",
    "period": {
      "start": "2026-04-02T00:00:00Z",
      "end": "2026-04-02T23:59:59Z"
    },
    "generated_at": "2026-04-02T09:00:00Z"
  },
  "summary": {
    "total_sessions": 15,
    "total_messages": 234,
    "total_llm_calls": 89,
    "total_tokens": 45678,
    "estimated_cost": 0.23
  },
  "tokens": {
    "total": 45678,
    "input": 32100,
    "output": 13578,
    "cost_estimate": 0.23,
    "by_model": [...],
    "by_hour": [...]
  },
  "sessions": {...},
  "agents": {...},
  "system": {...}
}
```

---

## 配置

### 报告配置

```yaml
# config/report.yaml
report:
  # 默认配置
  defaults:
    format: markdown
    output_dir: "./reports"
    include_hourly: true
    include_top_n: 10

  # 每日报告
  daily:
    enabled: true
    schedule: "0 9 * * *"          # 每天 9 点
    template: default
    output:
      - type: file
        path: "./reports/daily/{date}.md"
      - type: console              # 同时输出到控制台

  # 每周报告
  weekly:
    enabled: true
    schedule: "0 9 * * 1"          # 每周一 9 点
    template: weekly_summary

  # 每月报告
  monthly:
    enabled: true
    schedule: "0 9 1 * *"          # 每月 1 号 9 点
    template: monthly_summary

  # 数据保留
  retention:
    reports: 90                    # 保留 90 天
    raw_data: 365                  # 原始数据保留 1 年
```

---

## 实现逻辑

### 报告生成流程

```
触发报告生成
    ↓
确定报告时间段
    ↓
查询历史数据
    ├─→ Token 使用统计
    ├─→ 会话统计
    ├─→ Agent 统计
    └─→ 系统资源统计
    ↓
聚合计算
    ├─→ 按模型分组
    ├─→ 按会话分组
    ├─→ 按时间分组
    └─→ 计算 Top N
    ↓
应用模板
    ↓
格式化输出
    ├─→ Markdown
    ├─→ JSON
    └─→ HTML（可选）
    ↓
保存/发送报告
    ├─→ 写入文件
    ├─→ 发送邮件（可选）
    └─→ 输出到控制台
```

### 数据查询

```rust
impl ReportSkill {
    /// 查询每日统计数据
    async fn query_daily_stats(&self, date: NaiveDate) -> Result<DailyStats> {
        let start = date.and_hms(0, 0, 0).and_utc();
        let end = date.and_hms(23, 59, 59).and_utc();

        // 并行查询各类数据
        let (tokens, sessions, agents, system) = tokio::try_join!(
            self.query_tokens(start, end),
            self.query_sessions(start, end),
            self.query_agents(start, end),
            self.query_system(start, end),
        )?;

        Ok(DailyStats {
            date,
            tokens,
            sessions,
            agents,
            system,
        })
    }

    /// 查询 Token 使用
    async fn query_tokens(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<TokenReportSection> {
        // 从 storage 查询
        let snapshots = self.storage
            .query_stats_range(start, end, Granularity::Hourly)
            .await?;

        // 聚合数据
        let mut by_model = HashMap::new();
        let mut by_hour = Vec::new();

        for snapshot in snapshots {
            // 按模型聚合
            // 按小时聚合
        }

        Ok(TokenReportSection {
            total: ...,
            by_model,
            by_hour,
        })
    }
}
```

### Markdown 模板引擎

```rust
impl ReportSkill {
    /// 渲染 Markdown 报告
    fn render_markdown(&self, report: &Report) -> String {
        let mut md = String::new();

        // 标题
        md.push_str(&format!("# Knight Agent {}报告\n\n",
            match report.metadata.type.as_str() {
                "daily" => "每日",
                "weekly" => "每周",
                "monthly" => "每月",
                _ => "自定义",
            }
        ));

        // 摘要表格
        md.push_str("## 摘要\n\n");
        md.push_str("| 指标 | 数值 |\n");
        md.push_str("|------|------|\n");
        md.push_str(&format!("| 总会话数 | {} |\n", report.content.summary.total_sessions));
        md.push_str(&format!("| 总消息数 | {} |\n", report.content.summary.total_messages));
        // ...

        md
    }
}
```

---

## CLI 集成

### /report 命令

```bash
# 生成今日报告
knight> /report --type daily

✅ 报告已生成: ./reports/daily/2026-04-02.md

# 生成自定义范围报告
knight> /report --type custom --start 2026-03-01 --end 2026-03-31

✅ 报告已生成: ./reports/custom/2026-03-01_to_2026-03-31.md

# 设置定时报告
knight> /report --schedule daily --time 09:00

✅ 定时报告已创建: 每天早上 9 点生成

# 查看已设置的报告任务
knight> /report --list

Scheduled Reports:
  - Daily (0 9 * * *) → ./reports/daily/{date}.md
  - Weekly (0 9 * * 1) → ./reports/weekly/{date}.md
```

---

## 测试要点

### 单元测试

- [ ] 报告数据查询正确性
- [ ] 数据聚合计算正确性
- [ ] Markdown 格式输出正确性
- [ ] JSON 格式输出正确性
- [ ] 模板渲染正确性

### 集成测试

- [ ] 与 Storage Service 集成
- [ ] 与 Timer System 集成
- [ ] 定时报告触发
- [ ] 报告文件生成

### 测试用例示例

```rust
#[tokio::test]
async fn test_daily_report_generation() {
    let skill = create_test_report_skill();
    let date = NaiveDate::from_ymd_opt(2026, 4, 2).unwrap();

    let report = skill.generate_report(
        ReportType::Daily,
        Some(date),
        None,
        None,
        OutputFormat::Markdown,
    ).await.unwrap();

    assert_eq!(report.metadata.period.start.date(), date);
    assert!(report.content.summary.total_tokens > 0);
}

#[tokio::test]
async fn test_report_markdown_rendering() {
    let skill = create_test_report_skill();
    let report = create_mock_report();

    let md = skill.render_markdown(&report);

    assert!(md.contains("# Knight Agent"));
    assert!(md.contains("| 总会话数 |"));
}
```

---

## 性能考虑

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 数据查询时间 | < 100ms | 单日报告 |
| 报告生成时间 | < 500ms | Markdown 格式 |
| 内存占用 | < 50MB | 单个报告 |
| 并发报告 | 10+ | 同时处理多个报告 |

---

## 未来扩展

- [ ] HTML 格式报告（带图表）
- [ ] 邮件发送集成
- [ ] Webhook 通知
- [ ] 自定义报告模板语言
- [ ] 报告对比功能（同比/环比）
- [ ] 异常检测和告警
- [ ] 多维度数据分析
