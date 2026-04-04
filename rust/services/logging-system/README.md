# Logging System Module

Design Reference: `docs/03-module-design/services/logging-system.md`

## 概述

高性能、结构化的日志系统，支持多种输出目标和日志级别。

## 导入

```rust
use logging_system::{
    LoggingSystem, LoggingSystemImpl, LogLevel, LogFilter, LogEntry,
    LogContext, LogStats, LogOutput, ExportFormat, LoggerGuard,
};
```

## 核心类型

### LogLevel
日志级别枚举：`Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`

### LogEntry
日志条目结构：
- `id`: 日志唯一标识
- `timestamp`: 时间戳
- `level`: 日志级别
- `module`: 来源模块
- `session_id`: 会话ID（可选）
- `message`: 日志消息
- `context`: 上下文字段（HashMap）
- `error`: 错误信息（可选）

### LogFilter
日志过滤器，用于查询日志：
- `level`: 按级别过滤
- `module`: 按模块过滤
- `session_id`: 按会话过滤
- `since`/`until`: 按时间范围过滤
- `message_pattern`: 消息内容匹配

### LogOutput
日志输出目标枚举：
- `LogOutput::Console { colored: bool }`: 控制台输出
- `LogOutput::File { path, rotation_max_size, rotation_max_files, compress }`: 文件输出

## 对外接口

### LoggingSystemImpl 创建与初始化

```rust
// 创建实例（默认控制台输出）
let logging = LoggingSystemImpl::new()?;

// 配置控制台输出（带颜色）
let logging = LoggingSystemImpl::new()?.with_console(true);

// 配置文件输出
use std::path::PathBuf;
let logging = LoggingSystemImpl::new()?
    .with_file(PathBuf::from("./logs/app.log"), 10 * 1024 * 1024, 5);

// 初始化（全局tracing subscriber）
logging.init().await?;
```

### 基础日志记录

```rust
// 记录不同级别的日志
logging.debug("Debug message".to_string()).await?;
logging.info("Info message".to_string()).await?;
logging.warn("Warning message".to_string()).await?;
logging.error("Error message".to_string(), None).await?;
logging.fatal("Fatal message".to_string(), Some(error_info)).await?;
```

### 详细日志记录（带上下文）

```rust
use logging_system::{LogContext, ErrorInfo};

let context = LogContext {
    fields: HashMap::from([
        ("user_id".to_string(), serde_json::json!("123")),
        ("action".to_string(), serde_json::json!("login")),
    ]),
};

let error_info = ErrorInfo {
    code: "E001".to_string(),
    message: "Connection failed".to_string(),
    stack_trace: None,
};

logging.log_message(
    LogLevel::Error,
    "Request failed".to_string(),
    Some(context),
    Some("auth-module".to_string()),  // module
    Some("session-abc".to_string()),   // session_id
    Some(error_info),
).await?;
```

### 日志查询

```rust
use logging_system::LogFilter;

// 创建过滤器
let filter = LogFilter {
    level: Some(LogLevel::Error),
    module: Some("auth-module".to_string()),
    session_id: None,
    since: Some(start_time),
    until: Some(end_time),
    message_pattern: Some("failed".to_string()),
};

// 查询日志
let logs = logging.get_logs(filter).await?;
```

### 日志搜索

```rust
// 搜索包含关键词的日志
let results = logging.search(
    "connection error".to_string(),  // query
    None,                           // start_time
    None,                           // end_time
    Some(LogLevel::Error),          // level filter
    Some(50),                       // limit
).await?;
```

### 日志导出

```rust
use logging_system::ExportFormat;

// 导出为 JSON
let json = logging.export(ExportFormat::Json, filter).await?;

// 导出为 CSV
let csv = logging.export(ExportFormat::Csv, filter).await?;

// 导出为纯文本
let text = logging.export(ExportFormat::Text, filter).await?;
```

### 日志级别管理

```rust
// 设置全局日志级别
logging.set_level(LogLevel::Debug).await?;

// 获取当前全局级别
let level = logging.get_level().await;

// 设置特定模块的日志级别
logging.set_module_level("auth-module".to_string(), LogLevel::Trace).await?;

// 获取特定模块的级别
let module_level = logging.get_module_level("auth-module").await;
```

### 日志统计

```rust
// 获取日志统计信息
let stats = logging.get_stats().await;
println!("Total entries: {}", stats.total_entries);
println!("By level: {:?}", stats.entries_by_level);
println!("By module: {:?}", stats.entries_by_module);
```

### 日志轮转与清理

```rust
// 触发日志轮转
logging.rotate().await?;

// 清空日志缓冲区
logging.clear().await?;
```

### 获取系统信息

```rust
// 获取系统名称
println!("System name: {}", logging.name());

// 检查是否已初始化
if logging.is_initialized() {
    println!("Logging system is ready");
}
```

## 完整示例

```rust
use logging_system::{LoggingSystemImpl, LogLevel, LogFilter};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    let logging = LoggingSystemImpl::new()?;
    logging.init().await?;

    // 记录日志
    logging.info("Application started".to_string()).await?;

    // 记录带上下文的日志
    let mut ctx = HashMap::new();
    ctx.insert("port".to_string(), serde_json::json!(8080));

    logging
        .log_message(
            LogLevel::Info,
            "Server listening".to_string(),
            Some(LogContext { fields: ctx }),
            Some("server".to_string()),
            None,
            None,
        )
        .await?;

    // 查询错误日志
    let filter = LogFilter {
        level: Some(LogLevel::Error),
        ..Default::default()
    };
    let errors = logging.get_logs(filter).await?;
    println!("Found {} error entries", errors.len());

    Ok(())
}
```
