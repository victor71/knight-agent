# Monitor Module

Design Reference: `docs/03-module-design/core/monitor.md`

## 概述

Monitor 模块负责系统的实时状态收集和统计，包括：
- Token 使用统计
- 会话状态监控
- Agent 状态跟踪
- 系统资源监控
- 实时指标查询

## 导入

```rust
use monitor::{
    MonitorImpl, MonitorError, MonitorResult,
    SystemStats, TokenUsage, SessionStats, AgentStats,
    SystemStatus, SystemResourceStats, Metrics,
    StatusUpdate, HistoricalStats,
    StatScope, StatusScope,
};
```

## 核心类型

### StatScope
统计范围枚举：`All`, `Session`, `Agent`

### TokenUsage
Token 使用统计：
- `total`: 总消耗 Token 数
- `by_model`: 各模型消耗统计
- `by_type`: 按类型统计 (input/output)

### SessionStats
会话统计：
- `active_count`: 活跃会话数
- `total_count`: 总会话数
- `archived_count`: 归档会话数
- `total_messages`: 总消息数

### AgentStats
Agent 统计：
- `active_count`: 活跃 Agent 数
- `total_created`: 总创建数
- `total_tasks_completed`: 总完成任务数

### SystemStats
系统统计（综合）：
- `tokens`: Token 统计
- `sessions`: 会话统计
- `agents`: Agent 统计
- `resources`: 系统资源统计
- `uptime_seconds`: 运行时间

## 对外接口

### 初始化

```rust
use monitor::MonitorImpl;

let monitor = MonitorImpl::new();
monitor.initialize().await?;
```

### 启动/停止监控

```rust
monitor.start_monitoring().await?;
monitor.stop_monitoring().await?;
```

### 获取统计信息

```rust
use monitor::StatScope;

// 获取所有统计
let stats = monitor.get_stats(None, None).await?;

// 获取特定范围的统计
let session_stats = monitor.get_stats(Some(StatScope::Session), None).await?;
```

### 获取 Token 使用

```rust
let usage = monitor.get_token_usage(
    Some("session_id"),  // session_id (可选)
    None,                // start_time (可选)
    None,                // end_time (可选)
).await?;

println!("Total tokens used: {}", usage.total);
```

### 记录 Token 使用

```rust
monitor.record_token_usage(100, "claude", "input").await;
monitor.record_token_usage(50, "claude", "output").await;
```

### 获取系统状态

```rust
use monitor::StatusScope;

let status = monitor.get_status(None, None).await?;
println!("Running: {}", status.running);
println!("Initialized: {}", status.initialized);
println!("Uptime: {} seconds", status.stats.uptime_seconds);
```

### 获取指标快照

```rust
let metrics = monitor.collect_metrics().await?;
println!("CPU: {}%", metrics.cpu_usage);
println!("Memory: {}%", metrics.memory_usage);
println!("Active sessions: {}", metrics.active_sessions);
```

### 获取摘要信息

```rust
let summary = monitor.get_summary().await;
println!("{}", summary);
// Output: Sessions: 2/10 active, Tokens: 5000 used, Agents: 1/5 active
```

### 重置统计

```rust
monitor.reset_stats().await;
```

## 完整示例

```rust
use monitor::{MonitorImpl, StatScope};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化
    let monitor = MonitorImpl::new();
    monitor.initialize().await?;
    monitor.start_monitoring().await?;

    // 记录 token 使用
    monitor.record_token_usage(1000, "claude-3", "input").await;
    monitor.record_token_usage(500, "claude-3", "output").await;

    // 获取统计
    let stats = monitor.get_stats(None, None).await?;
    println!("Total tokens: {}", stats.tokens.total);
    println!("Active sessions: {}", stats.sessions.active_count);

    // 获取状态
    let status = monitor.get_status(None, None).await?;
    println!("System running: {}", status.running);

    // 获取摘要
    let summary = monitor.get_summary().await;
    println!("{}", summary);

    monitor.stop_monitoring().await?;
    Ok(())
}
```

## 错误处理

MonitorError 枚举定义：
- `NotInitialized`: 监控未初始化
- `CollectionFailed(String)`: 指标收集失败
- `StatsNotFound(String)`: 统计未找到
- `InvalidScope(String)`: 无效的范围
