# Storage Service Module

持久化存储服务，提供会话、消息、任务、工作流等数据的 SQLite 存储能力。

Design Reference: `docs/03-module-design/services/storage-service.md`

## 特性

- SQLite 本地存储，支持 WAL 模式
- 会话、消息、任务、工作流完整 CRUD 操作
- 压缩点管理，支持会话历史压缩
- Token 使用量统计和 LLM 调用记录
- 统计快照和日报表生成
- 数据库备份和恢复
- 自动 vacuum 和 reindex 维护

## 依赖

```toml
[dependencies]
storage-service = { path = "./rust/services/storage-service" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## 快速开始

```rust
use storage_service::{StorageService, StorageServiceImpl, Session, SessionStatus};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建存储服务实例
    let storage = StorageServiceImpl::new()?;

    // 初始化数据库
    storage.init().await?;

    // 创建会话
    let session = Session {
        id: "session-1".to_string(),
        name: "My Session".to_string(),
        status: SessionStatus::Active,
        workspace_root: "/path/to/workspace".to_string(),
        project_type: Some("rust".to_string()),
        created_at: chrono::Utc::now().timestamp(),
        last_active_at: chrono::Utc::now().timestamp(),
        metadata: HashMap::new(),
    };

    storage.save_session(session).await?;

    // 加载会话
    let loaded = storage.load_session("session-1").await?;
    println!("Loaded session: {:?}", loaded);

    Ok(())
}
```

## API 接口

### 初始化

| 方法 | 说明 |
|------|------|
| `new()` | 创建默认配置的存储服务 |
| `with_config(config)` | 使用自定义配置创建存储服务 |
| `init()` | 初始化数据库连接 |
| `is_initialized()` | 检查是否已初始化 |
| `name()` | 获取服务名称 |

### 会话操作 (Session)

```rust
// 保存会话
async fn save_session(&self, session: Session) -> Result<bool, StorageError>

// 加载会话
async fn load_session(&self, session_id: &str) -> Result<Option<Session>, StorageError>

// 删除会话
async fn delete_session(&self, session_id: &str) -> Result<bool, StorageError>

// 列出会话（支持分页和过滤）
async fn list_sessions(
    &self,
    filter: SessionFilter,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<Session>, StorageError>
```

**SessionFilter 过滤条件：**

```rust
pub struct SessionFilter {
    pub status: Option<SessionStatus>,      // 按状态过滤 (Active/Archived/Deleted)
    pub created_after: Option<i64>,          // 创建时间下限
    pub created_before: Option<i64>,          // 创建时间上限
    pub workspace: Option<String>,            // 工作空间路径过滤
}
```

### 消息操作 (Message)

```rust
// 添加消息
async fn append_message(&self, message: Message) -> Result<bool, StorageError>

// 获取消息列表
async fn get_messages(
    &self,
    session_id: &str,
    limit: Option<usize>,      // 返回条数限制
    offset: Option<usize>,     // 偏移量
    after: Option<&str>,        // 获取指定ID之后的消息
) -> Result<Vec<Message>, StorageError>

// 删除消息（删除指定ID之前的所有消息）
async fn delete_messages(&self, session_id: &str, before: &str) -> Result<i64, StorageError>
```

**Message 结构：**

```rust
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,       // User / Assistant / System
    pub content: String,
    pub timestamp: i64,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### 压缩点操作 (Compression Point)

当会话消息过多时，可创建压缩点对历史消息进行摘要压缩。

```rust
// 保存压缩点
async fn save_compression_point(&self, point: CompressionPoint) -> Result<bool, StorageError>

// 获取会话的所有压缩点
async fn get_compression_points(&self, session_id: &str) -> Result<Vec<CompressionPoint>, StorageError>

// 删除压缩点
async fn delete_compression_point(&self, point_id: &str) -> Result<bool, StorageError>
```

### 任务操作 (Task)

```rust
// 保存任务
async fn save_task(&self, task: Task) -> Result<bool, StorageError>

// 加载任务
async fn load_task(&self, task_id: &str) -> Result<Option<Task>, StorageError>

// 更新任务
async fn update_task(&self, task_id: &str, updates: TaskUpdate) -> Result<bool, StorageError>

// 列出任务（支持过滤）
async fn list_tasks(&self, filter: TaskFilter, limit: Option<usize>) -> Result<Vec<Task>, StorageError>
```

**TaskUpdate 更新字段：**

```rust
pub struct TaskUpdate {
    pub status: Option<TaskStatus>,                              // 状态更新
    pub input: Option<HashMap<String, serde_json::Value>>,       // 输入更新
    pub output: Option<HashMap<String, serde_json::Value>>,      // 输出更新
    pub error: Option<String>,                                   // 错误信息
    pub started_at: Option<i64>,                                  // 开始时间
    pub completed_at: Option<i64>,                               // 完成时间
}
```

**TaskFilter 过滤条件：**

```rust
pub struct TaskFilter {
    pub workflow_id: Option<String>,     // 按工作流ID过滤
    pub status: Option<TaskStatus>,      // 按状态过滤
    pub task_type: Option<String>,       // 按类型过滤
    pub created_after: Option<i64>,       // 创建时间下限
    pub created_before: Option<i64>,     // 创建时间上限
}
```

### 工作流操作 (Workflow)

```rust
// 保存工作流
async fn save_workflow(&self, workflow: WorkflowDefinition) -> Result<bool, StorageError>

// 加载工作流
async fn load_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>, StorageError>

// 列出所有工作流
async fn list_workflows(&self) -> Result<Vec<WorkflowDefinition>, StorageError>
```

### 配置操作 (Config)

键值对配置存储。

```rust
// 保存配置
async fn save_config(&self, key: &str, value: serde_json::Value) -> Result<bool, StorageError>

// 加载配置
async fn load_config(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError>

// 删除配置
async fn delete_config(&self, key: &str) -> Result<bool, StorageError>
```

### 统计和监控

```rust
// 获取存储统计
async fn get_stats(&self) -> Result<StorageStats, StorageError>

// 保存统计快照
async fn save_stats_snapshot(&self, snapshot: StatsSnapshot) -> Result<bool, StorageError>

// 查询时间范围统计
async fn query_stats_range(
    &self,
    start_time: i64,
    end_time: i64,
    granularity: Option<&str>,
) -> Result<Vec<StatsSnapshot>, StorageError>

// 保存 Token 使用记录
async fn save_token_usage(&self, usage: TokenUsageRecord) -> Result<bool, StorageError>

// 保存 LLM 调用记录
async fn save_llm_call(&self, call: LLMCallRecord) -> Result<bool, StorageError>

// 保存会话事件
async fn save_session_event(&self, event: SessionEvent) -> Result<bool, StorageError>

// 获取日报表
async fn get_daily_report(&self, date: &str) -> Result<Option<DailyReport>, StorageError>
```

### 备份和恢复

```rust
// 备份数据库
async fn backup(&self, path: &str) -> Result<bool, StorageError>

// 恢复数据库
async fn restore(&self, path: &str) -> Result<bool, StorageError>

// 导出数据
async fn export_data(&self, format: &str, output_path: &str) -> Result<bool, StorageError>
```

### 维护操作

```rust
// Vacuum 压缩数据库
async fn vacuum(&self) -> Result<i64, StorageError>

// Reindex 重建索引
async fn reindex(&self) -> Result<bool, StorageError>
```

## 数据类型

### SessionStatus

```rust
pub enum SessionStatus {
    Active,    // 活跃会话
    Archived,  // 已归档
    Deleted,   // 已删除
}
```

### TaskStatus

```rust
pub enum TaskStatus {
    Pending,    // 待处理
    Running,    // 执行中
    Completed,  // 已完成
    Failed,     // 失败
    Cancelled,  // 已取消
}
```

### MessageRole

```rust
pub enum MessageRole {
    User,       // 用户消息
    Assistant,  // 助手消息
    System,     // 系统消息
}
```

## 配置选项

```rust
pub struct StorageConfig {
    pub database_path: String,         // 数据库路径 (默认: "./storage/knight-agent.db")
    pub wal_enabled: bool,             // 启用 WAL 模式 (默认: true)
    pub cache_size: i64,               // 缓存大小 (默认: 10000)
    pub page_size: i64,                // 页面大小 (默认: 4096)
    pub backup_enabled: bool,           // 启用自动备份 (默认: true)
    pub backup_interval_secs: i64,     // 备份间隔 (默认: 86400 秒)
    pub backup_retention_days: i64,     // 备份保留天数 (默认: 7)
    pub backup_path: String,            // 备份存储路径 (默认: "./storage/backups")
    pub vacuum_interval_secs: i64,      // Vacuum 间隔 (默认: 604800 秒)
    pub reindex_interval_secs: i64,    // Reindex 间隔 (默认: 1209600 秒)
    pub auto_vacuum: bool,             // 自动 vacuum (默认: true)
}
```

## 错误处理

所有操作返回 `Result<T, StorageError>`，错误类型包括：

```rust
pub enum StorageError {
    Database(String),       // 数据库错误
    NotFound(String),       // 资源不存在
    InvalidData(String),    // 无效数据
    WriteFailed(String),     // 写入失败
    ReadFailed(String),      // 读取失败
    AlreadyExists(String),   // 资源已存在
}
```
