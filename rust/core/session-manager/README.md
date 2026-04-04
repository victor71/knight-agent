# Session Manager Module

Design Reference: `docs/03-module-design/core/session-manager.md`

## 概述

会话管理器负责管理 Knight-Agent 的所有会话生命周期，包括：
- 会话的创建、切换、删除和持久化
- Workspace 隔离和路径权限控制
- 上下文管理和自动压缩
- 消息历史存储和检索
- 多会话并行执行

## 导入

```rust
use session_manager::{
    SessionManagerImpl, Session, SessionStatus, SessionContext, SessionMetadata,
    SessionError, SessionResult, CreateSessionRequest, Message, MessageRole,
    CompressionPoint, CompressionMethod, SearchResult, PathAction, PathAccessResult,
    ProjectType, SessionStats,
};
```

## 核心类型

### SessionStatus
会话状态枚举：`Active`, `Paused`, `Archived`

### ProjectType
项目类型枚举：`Rust`, `Node`, `Python`, `Go`, `Java`, `Web`, `Other`, `Auto`

### Message
消息结构：
- `id`: 消息唯一标识
- `role`: 角色（System/User/Assistant/Tool）
- `content`: 消息内容
- `timestamp`: 时间戳
- `metadata`: 元数据

### Session
会话结构：
- `id`: 会话唯一标识
- `metadata`: 会话元数据（名称、工作区、项目类型等）
- `status`: 会话状态
- `context`: 会话上下文
- `stats`: 会话统计
- `created_at`: 创建时间
- `updated_at`: 更新时间

## 对外接口

### 创建会话

```rust
use session_manager::{SessionManagerImpl, CreateSessionRequest, ProjectType};

let manager = SessionManagerImpl::new();

// 基本创建
let session = manager.create_session(
    CreateSessionRequest::new("/workspace/project")
).await?;

// 完整配置
let session = manager.create_session(
    CreateSessionRequest::new("/workspace/project")
        .name("my-session")
        .project_type(ProjectType::Rust)
).await?;
```

### 获取和列出会话

```rust
// 获取单个会话
let session = manager.get_session("session_id").await?;

// 列出所有会话
let all = manager.list_sessions(None).await;

// 按状态过滤
let active = manager.list_sessions(Some(SessionStatus::Active)).await;
```

### 切换会话

```rust
// 切换到指定会话
manager.use_session("session_id").await?;

// 获取当前会话
let current = manager.get_current_session().await;
```

### 删除和归档

```rust
// 删除会话
manager.delete_session("session_id", false).await?;

// 强制删除（忽略未保存更改）
manager.delete_session("session_id", true).await?;

// 归档会话
manager.archive_session("session_id").await?;

// 恢复已归档的会话
manager.restore_session("session_id").await?;
```

### 消息和上下文管理

```rust
use session_manager::Message;

// 添加消息
let msg = Message::user("m1", "Hello world");
let should_compress = manager.add_message("session_id", msg).await?;

// 获取上下文
let ctx = manager.get_context("session_id", true).await?;

// 压缩上下文
let point = manager.compress_context(
    "session_id",
    CompressionMethod::Summary
).await?;
```

### 历史搜索

```rust
// 搜索指定会话的历史
let results = manager.search_history("error", Some("session_id"), 10).await?;

// 搜索所有会话
let results = manager.search_history("connection", None, 20).await?;
```

### 路径访问控制

```rust
use session_manager::PathAction;

// 检查路径访问权限
let result = manager.check_path_access(
    "session_id",
    "/workspace/project/src/main.rs",
    PathAction::Read
).await?;
if result.allowed {
    println!("Path is within workspace");
}

// 验证路径是否在 Workspace 内
let valid = manager.validate_path("session_id", "/etc/passwd").await?;
assert!(!valid); // 不在 workspace 内
```

### 会话统计

```rust
let stats = manager.get_stats("session_id").await?;
println!("Messages: {}", stats.total_messages);
println!("Compressions: {}", stats.compression_count);
```

### 持久化

```rust
// 保存会话
let path = manager.save_session(Some("session_id")).await?;

// 加载会话
let session = manager.load_session("session_id").await?;
```

### 管理和清空

```rust
// 会话数量
let count = manager.len().await;

// 清空所有会话
manager.clear().await;
```

## 完整示例

```rust
use session_manager::{
    SessionManagerImpl, CreateSessionRequest, Message,
    SessionStatus, ProjectType, PathAction,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = SessionManagerImpl::new();

    // 创建会话
    let session = manager.create_session(
        CreateSessionRequest::new("/workspace/myproject")
            .name("dev-session")
            .project_type(ProjectType::Rust)
    ).await?;

    println!("Created session: {}", session.id);

    // 添加消息
    manager.add_message(
        &session.id,
        Message::user("m1", "Hello, how are you?")
    ).await?;

    // 获取当前会话
    if let Some(current) = manager.get_current_session().await {
        println!("Current session: {}", current.id);
    }

    // 列出所有会话
    let sessions = manager.list_sessions(None).await;
    println!("Total sessions: {}", sessions.len());

    // 检查路径访问
    let result = manager.check_path_access(
        &session.id,
        "/workspace/myproject/src/main.rs",
        PathAction::Read
    ).await?;
    println!("Path accessible: {}", result.allowed);

    // 搜索历史
    let results = manager.search_history("hello", None, 10).await?;
    println!("Found {} matching messages", results.len());

    Ok(())
}
```

## 错误处理

SessionError 枚举定义：
- `NotFound(String)`: 会话不存在
- `AlreadyExists(String)`: 会话已存在
- `Expired`: 会话已过期
- `NotInitialized`: 会话管理器未初始化
- `InvalidState(String)`: 无效的会话状态
- `PersistenceError(String)`: 持久化错误
- `CompressionError(String)`: 压缩错误
