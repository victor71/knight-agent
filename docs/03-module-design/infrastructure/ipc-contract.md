# IPC Contract (进程间通信契约)

## 概述

### 职责描述

IPC Contract 定义 Knight-Agent 系统中两个进程间通信边界的协议规范，包括：

- 两个 IPC 边界的消息格式定义 (Request/Response/Event)
- 通信协议规范 (Unix Socket / TCP / stdin-stdout)
- 错误处理和重试机制
- 类型安全保证 (Rust-to-Rust)
- 版本兼容性策略

### 架构背景

Knight-Agent 采用多进程架构，包含两个 IPC 边界：

```
┌─────────────────────────────────────────────────────────┐
│                     TUI Process                         │
│  (ratatui) - 用户界面、交互逻辑                         │
└─────────────────────────────────────────────────────────┘
                          │
                   IPC #1 (Unix Socket / TCP)
                   JSON-RPC style messages
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Daemon Process                        │
│  Session Manager, Router, Orchestrator                  │
│  Manages session lifecycle, routes messages             │
└─────────────────────────────────────────────────────────┘
                          │
                   IPC #2 (Unix Socket / stdin-stdout)
                   JSON-RPC style messages
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Session Process(es)                    │
│  Agent Runtime, Tool System                            │
│  Each session runs in its own process                  │
└─────────────────────────────────────────────────────────┘
```

### IPC 边界说明

| IPC 边界 | 参与方 | 传输层 | 用途 |
|----------|--------|--------|------|
| **IPC #1** | TUI <-> Daemon | Unix Socket / TCP | 用户输入、状态显示、会话管理 |
| **IPC #2** | Daemon <-> Session Process | Unix Socket / stdin-stdout | Agent 生命周期、消息路由、状态上报 |

### 设计目标

1. **类型安全**: Rust-to-Rust 类型一致性保证
2. **高性能**: 低延迟通信，支持流式传输
3. **可靠性**: 错误恢复、断线重连
4. **可维护性**: 清晰的版本管理和兼容策略
5. **安全性**: 消息验证、权限检查
6. **进程隔离**: 每个会话独立进程，故障不扩散

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 协作 | 会话状态同步。见 [Session Manager 接口](../core/session-manager.md#对外接口) |
| Router | 协作 | 请求路由和分发。见 [Router 接口](../core/router.md#对外接口) |
| Security Manager | 协作 | 权限验证。见 [Security Manager 接口](../security/security-manager.md#对外接口) |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| TUI (ratatui) | 被依赖 | 用户界面通过 IPC #1 与 Daemon 通信 |
| Session Process | 被依赖 | 会话进程通过 IPC #2 与 Daemon 通信 |

---

## 通信协议

### IPC #1: TUI <-> Daemon 传输层

| 协议 | 场景 | 优点 | 缺点 |
|------|------|------|------|
| **Unix Socket** | 本地模式 (默认) | 高性能、安全、无需端口 | 跨平台需适配 (Windows named pipe) |
| **TCP** | 远程模式 / 跨平台备选 | 跨平台、支持远程连接 | 需端口管理、安全性依赖防火墙 |

### IPC #2: Daemon <-> Session Process 传输层

| 协议 | 场景 | 优点 | 缺点 |
|------|------|------|------|
| **Unix Socket** | 本地模式 (默认) | 高性能、安全 | 跨平台需适配 |
| **stdin-stdout** | 简单模式 / 调试 | 无需额外端口、进程管道原生支持 | 仅支持请求-响应模式 |

### 传输层配置

```yaml
# 根据运行模式选择传输层
transport:
  ipc1_tui_daemon:
    default: unix_socket
    fallback: tcp
    unix_socket:
      path: /tmp/knight-agent/daemon.sock   # Linux/macOS
      # Windows: \\.\pipe\knight-agent-daemon
    tcp:
      host: "127.0.0.1"
      port: 0  # 自动分配

  ipc2_daemon_session:
    default: unix_socket
    fallback: stdin_stdout
    unix_socket:
      path_template: /tmp/knight-agent/session-{session_id}.sock
    stdin_stdout:
      format: json_rpc
      line_delimited: true   # 每行一条 JSON-RPC 消息
```

---

## 消息格式

### 基础消息结构

两个 IPC 边界共享相同的基础消息格式，以 JSON 作为序列化格式：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    pub id: String,           // 消息唯一 ID (UUID v4)
    pub r#type: MessageType,  // 消息类型
    pub timestamp: i64,       // Unix 时间戳 (毫秒)
    pub session_id: Option<String>,  // 会话 ID (可选)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    #[serde(rename = "request")]
    Request,
    #[serde(rename = "response")]
    Response,
    #[serde(rename = "notification")]
    Notification,
    #[serde(rename = "stream_chunk")]
    StreamChunk,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "user_query")]
    UserQuery,       // Agent 询问用户
    #[serde(rename = "user_response")]
    UserResponse,    // 用户响应
}
```

### 请求消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub method: String,
    pub params: serde_json::Value,
    pub options: Option<RequestOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestOptions {
    pub timeout: Option<u64>,
    pub stream: Option<bool>,
    pub priority: Option<i32>,
}
```

### 响应消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub request_id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorResponse>,
    pub streaming: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub stack: Option<String>,
}
```

### 通知消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub event: String,
    pub data: serde_json::Value,
}
```

### 流式数据消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunkMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub request_id: String,
    pub sequence: u64,
    pub chunk: String,
    pub done: bool,
}
```

### 用户询问消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct UserQueryMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub await_id: String,
    pub query_type: QueryType,
    pub agent_id: String,
    pub message: String,
    pub options: Option<Vec<String>>,
    pub context: serde_json::Value,
    pub timeout: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    #[serde(rename = "permission")]
    Permission,
    #[serde(rename = "clarification")]
    Clarification,
    #[serde(rename = "confirmation")]
    Confirmation,
    #[serde(rename = "information")]
    Information,
}
```

### 用户响应消息

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponseMessage {
    #[serde(flatten)]
    pub base: BaseMessage,

    pub await_id: String,
    pub response: UserResponseData,
    pub responded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponseData {
    pub accepted: bool,
    pub value: Option<String>,
    pub custom_input: Option<String>,
}
```

### 等待注册表 (Await Registry)

Daemon 内部维护等待注册表，用于路由用户响应到正确的 Agent：

```rust
use std::collections::HashMap;

/// Daemon 内部数据结构
pub struct AwaitRegistry {
    pub mappings: HashMap<String, AwaitInfo>,
}

pub struct AwaitInfo {
    pub agent_id: String,
    pub session_id: String,
    pub query_type: QueryType,
    pub created_at: i64,
    pub timeout: u64,
}
```

---

## IPC #1: TUI <-> Daemon 协议

TUI 进程通过 Unix Socket 或 TCP 与 Daemon 通信，使用 JSON-RPC 风格消息。

### TUI -> Daemon 请求

```yaml
tui_daemon_protocol:
  requests:
    - method: "session.list"
      description: 列出所有会话
      params: {}
      returns: { sessions: "array<SessionInfo>" }

    - method: "session.create"
      description: 创建新会话
      params:
        name: { type: string, required: false }
        workspace: { type: string, required: false }
      returns: { session_id: string }

    - method: "session.switch"
      description: 切换到指定会话
      params:
        session_id: { type: string, required: true }
      returns: { success: boolean }

    - method: "session.send_message"
      description: 向会话发送用户消息
      params:
        session_id: { type: string, required: true }
        content: { type: string, required: true }
      returns: { response: string }

    - method: "session.get_status"
      description: 获取会话状态快照
      params:
        session_id: { type: string, required: true }
      returns: SystemStatusSnapshot

    - method: "daemon.status"
      description: 获取 Daemon 整体状态
      params: {}
      returns: DaemonStatus

    - method: "daemon.stop"
      description: 停止 Daemon 进程
      params: {}
      returns: { success: boolean }
```

### Daemon -> TUI 事件 (推送)

```yaml
tui_daemon_protocol:
  events:
    - type: "agent.response_stream"
      description: Agent 响应流式数据块
      payload:
        session_id: string
        chunk: string

    - type: "agent.status_change"
      description: Agent 状态变更通知
      payload:
        session_id: string
        agent_id: string
        status: string

    - type: "session.status_update"
      description: 会话状态更新
      payload:
        session_id: string
        status: string

    - type: "system.metrics"
      description: 系统资源指标
      payload:
        token_usage: object
        memory: integer
        cpu: float
```

---

## IPC #2: Daemon <-> Session Process 协议

Daemon 与每个 Session Process 通过 Unix Socket 或 stdin-stdout 通信，使用 JSON-RPC 风格消息。

### Daemon -> Session 请求

```yaml
daemon_session_protocol:
  requests:
    - method: "agent.create"
      description: 在会话中创建 Agent
      params:
        agent_id: { type: string, required: true }
        definition: { type: AgentDefinition, required: true }
      returns: { success: boolean }

    - method: "agent.send_message"
      description: 向 Agent 发送消息
      params:
        agent_id: { type: string, required: true }
        content: { type: string, required: true }
      returns: { response: string }

    - method: "agent.list"
      description: 列出会话中的所有 Agent
      params: {}
      returns: { agents: "array<AgentInfo>" }

    - method: "task.list"
      description: 列出会话中的所有任务
      params: {}
      returns: { tasks: "array<TaskInfo>" }

    - method: "session.shutdown"
      description: 关闭会话进程
      params:
        graceful: { type: boolean, required: false, default: true }
      returns: { success: boolean }
```

### Session -> Daemon 事件 (推送)

```yaml
daemon_session_protocol:
  events:
    - type: "heartbeat"
      description: 会话进程心跳
      payload:
        session_id: string
        memory: integer
        agents: integer

    - type: "agent.response"
      description: Agent 响应结果
      payload:
        agent_id: string
        content: string

    - type: "task.status_change"
      description: 任务状态变更
      payload:
        task_id: string
        status: string

    - type: "error"
      description: 错误事件
      payload:
        code: integer
        message: string
```

---

## 方法命名规范

```
{entity}.{action}

IPC #1 (TUI <-> Daemon) 示例:
- session.list           // 列出会话
- session.create         // 创建会话
- session.switch         // 切换会话
- session.send_message   // 发送消息
- session.get_status     // 获取状态
- daemon.status          // Daemon 状态
- daemon.stop            // 停止 Daemon

IPC #2 (Daemon <-> Session) 示例:
- agent.create           // 创建 Agent
- agent.send_message     // 发送消息到 Agent
- agent.list             // 列出 Agent
- task.list              // 列出任务
- session.shutdown       // 关闭会话
```

---

## 错误处理

### 错误码定义

两个 IPC 边界共享统一的错误码体系：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorCode {
    // 通用错误 (1-999)
    UnknownError = 1,
    ParseError = 2,           // 消息解析失败
    InvalidRequest = 3,       // 无效请求
    MethodNotFound = 4,       // 方法不存在
    Timeout = 5,              // 超时

    // Session 错误 (1000-1999)
    SessionNotFound = 1000,
    SessionExpired = 1001,
    SessionDestroyed = 1002,

    // Agent 错误 (2000-2999)
    AgentNotFound = 2000,
    AgentSpawnFailed = 2001,
    AgentTimeout = 2002,

    // 工具错误 (3000-3999)
    ToolNotFound = 3000,
    ToolExecutionFailed = 3001,

    // 安全错误 (4000-4999)
    Unauthorized = 4001,
    Forbidden = 4002,
    PermissionDenied = 4003,

    // 用户交互错误 (6000-6999)
    AwaitTimeout = 6000,           // 等待用户响应超时
    AwaitCancelled = 6001,         // 等待被取消
    InvalidUserResponse = 6002,    // 无效的用户响应
    AwaitNotFound = 6003,          // 等待 ID 不存在

    // 系统错误 (5000-5999)
    InternalError = 5000,
    ResourceExhausted = 5001,

    // IPC 连接错误 (7000-7999)
    ConnectionRefused = 7000,      // 连接被拒绝
    ConnectionReset = 7001,        // 连接被重置
    SessionProcessCrashed = 7002,  // Session 进程崩溃
    DaemonUnavailable = 7003,      // Daemon 不可用
}
```

**说明**: 以上为 IPC 层的错误码规范。内部模块（如 Session Manager、Security Manager）的错误码在传播到 IPC 层时应映射到上述错误码。IPC 连接错误 (7000+) 专用于两个 IPC 边界的连接管理。

### 错误处理策略

```yaml
# 客户端重试策略
retry:
  # 可重试的错误码
  retryable_codes:
    - Timeout
    - InternalError
    - ResourceExhausted

  # 重试配置
  max_attempts: 3
  backoff:
    type: exponential
    initial_delay: 1000  # 毫秒
    max_delay: 10000
    multiplier: 2

  # 不可重试的错误
  non_retryable_codes:
    - ParseError
    - InvalidRequest
    - MethodNotFound
    - Unauthorized
    - Forbidden
```

### 用户交互流程

当 Agent 需要与用户交互（权限确认、信息补充、危险操作确认等）时，消息通过两个 IPC 边界传递：

```
┌─────────────────────────────────────────────────────────────────┐
│                      Session Process                             │
│                    (Agent Runtime)                                │
│                                                                   │
│  Agent 执行中遇到需要用户确认的场景                               │
│        │                                                         │
│        ▼                                                         │
│  ┌──────────────────────────────┐                                │
│  │ 发出 user_query 事件         │                                │
│  │ - await_id: 唯一标识         │                                │
│  │ - query_type: 询问类型       │                                │
│  │ - message: 询问内容          │                                │
│  │ - context: 上下文信息        │                                │
│  └──────────────────────────────┘                                │
│        │                                                         │
└────────┼─────────────────────────────────────────────────────────┘
         │ IPC #2 (Session -> Daemon event: user_query)
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Daemon Process                               │
│                                                                   │
│  1. 接收 user_query 事件                                         │
│  2. 注册到 AwaitRegistry (await_id -> agent_id, session_id)      │
│  3. 转发到 TUI                                                   │
│        │                                                         │
└────────┼─────────────────────────────────────────────────────────┘
         │ IPC #1 (Daemon -> TUI event: user_query)
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                       TUI (ratatui)                              │
│                                                                   │
│  接收 UserQuery 消息                                             │
│        │                                                         │
│        ▼                                                         │
│  显示询问对话框/面板给用户                                       │
│        │                                                         │
│        ▼                                                         │
│  用户输入响应（接受/拒绝/输入值）                                 │
│        │                                                         │
└────────┼─────────────────────────────────────────────────────────┘
         │ IPC #1 (TUI -> Daemon request: user.respond)
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Daemon Process                               │
│                                                                   │
│  1. 接收 user.respond 请求                                       │
│  2. 查询 AwaitRegistry[await_id]                                 │
│  3. 路由到目标 Session Process                                    │
│        │                                                         │
└────────┼─────────────────────────────────────────────────────────┘
         │ IPC #2 (Daemon -> Session request: agent.user_response)
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Session Process                             │
│                    (Agent Runtime)                                │
│                                                                   │
│  接收 UserResponse 消息                                          │
│        │                                                         │
│        ▼                                                         │
│  ┌──────────────────────────────┐                                │
│  │ 匹配 await_id               │                                │
│  │ 恢复 Agent 执行              │                                │
│  └──────────────────────────────┘                                │
│        │                                                         │
│        ▼                                                         │
│  Agent 继续执行流程                                               │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

#### 询问类型说明

| QueryType | 触发场景 | 期望响应 |
|-----------|----------|----------|
| `permission` | 权限不足，请求用户授权 | `accepted: true/false` |
| `confirmation` | 危险操作确认 | `accepted: true/false` |
| `clarification` | 信息不足，需要用户补充 | `value: 补充的信息` |
| `information` | 一般性信息请求 | `value: 请求的信息` |

#### 超时处理

```yaml
# 用户交互配置
user_interaction:
  default_timeout: 300000  # 默认 5 分钟超时
  max_timeout: 3600000      # 最大 1 小时超时
  auto_reject_on_timeout: false  # 超时后自动拒绝
```

| 超时配置 | 行为 |
|----------|------|
| `timeout = 0` | 无限等待 |
| `auto_reject_on_timeout = true` | 超时后自动拒绝 |
| `auto_reject_on_timeout = false` | 超时后返回 AwaitTimeout 错误 |

#### 多并发询问

当多个 Agent 并行运行时，可能同时存在多个待处理的用户询问：

```
Agent A ─┬─ await_id_1 ──▶ 用户询问 1
         └─ await_id_2 ──▶ 用户询问 2
Agent B ─┬─ await_id_3 ──▶ 用户询问 3
         └─ await_id_4 ──▶ 用户询问 4
```

**Daemon 维护等待表 (Await Registry)**：

```rust
use std::collections::HashMap;

pub struct AwaitRegistry {
    // await_id -> 映射信息
    pub mappings: HashMap<String, AwaitMapping>,
}

pub struct AwaitMapping {
    pub agent_id: String,       // 目标 Agent ID
    pub session_id: String,     // 会话 ID
    pub query_type: QueryType,  // 询问类型
    pub created_at: i64,        // 创建时间
    pub timeout: u64,           // 超时时间
}

pub struct PendingQuery {
    pub await_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub query_type: QueryType,
    pub message: String,
    pub created_at: i64,
    pub timeout: u64,
}
```

**消息路由流程**：

```
1. Session Process 发出 user_query 事件 (IPC #2)
   └─> Daemon 接收，创建 await_id，注册到 AwaitRegistry
   └─> Daemon 转发 UserQuery 事件到 TUI (IPC #1)

2. TUI 收到 UserQuery 事件
   └─> 显示给用户
   └─> 用户选择响应（选择对应的 await_id）

3. TUI 调用 user.respond(await_id, response) (IPC #1)
   └─> Daemon 查询 AwaitRegistry[await_id]
   └─> 获取目标 agent_id 和 session_id
   └─> Daemon 路由到目标 Session Process (IPC #2)

4. Session Process 收到响应，恢复 Agent 执行
```

**用户响应匹配**：

用户响应时，`await_id` 必须精确匹配。IPC 层会验证：
- `await_id` 是否存在
- 是否已超时
- 响应格式是否正确

**超时处理**：

当某个 `await_id` 超时：
```
超时检查器
    │
    ▼
检测到 await_id_2 超时
    │
    ▼
查询 AwaitRegistry[await_id_2] → agent_id = "Agent A"
    │
    ▼
调用 AgentRuntime.handle_user_response(
    agent_id = "Agent A",
    await_id = "await_id_2",
    response = { accepted: false, value: "timeout" }
)
    │
    ▼
Agent A 继续执行（收到拒绝/超时响应）
```

---

## 类型同步

### 共享类型定义策略

由于两个 IPC 边界都是 Rust-to-Rust 通信，类型定义通过共享 crate 实现：

```yaml
# 类型定义目录结构
crates/
├── knight-ipc/                  # 共享 IPC 类型 crate
│   ├── src/
│   │   ├── lib.rs               # 公共导出
│   │   ├── message.rs           # 基础消息类型
│   │   ├── ipc1_tui_daemon.rs   # IPC #1 消息类型
│   │   ├── ipc2_daemon_session.rs # IPC #2 消息类型
│   │   ├── error_codes.rs       # 错误码定义
│   │   └── events.rs            # 事件类型定义
│   └── Cargo.toml
```

### 使用方式

```rust
// TUI 依赖 knight-ipc
// Daemon 依赖 knight-ipc
// Session Process 依赖 knight-ipc

use knight_ipc::ipc1::{TuiDaemonRequest, TuiDaemonEvent};
use knight_ipc::ipc2::{DaemonSessionRequest, SessionDaemonEvent};
use knight_ipc::{BaseMessage, ResponseMessage, ErrorCode};
```

### 序列化保证

所有消息类型使用 `serde::Serialize` / `serde::Deserialize`，确保 JSON 序列化一致：

```rust
// 消息在传输前序列化为 JSON
let json = serde_json::to_string(&message)?;

// 接收后反序列化
let message: RequestMessage = serde_json::from_str(&json)?;
```

---

## 安全性

### 消息验证

```rust
// Rust 端验证
pub struct MessageValidator {
    max_size: usize,
    max_depth: usize,
}

impl MessageValidator {
    pub fn validate(&self, message: &BaseMessage) -> Result<(), ValidationError> {
        // 1. 大小检查
        let serialized = serde_json::to_vec(message)?;
        if serialized.len() > self.max_size {
            return Err(ValidationError::MessageTooLarge);
        }

        // 2. 深度检查 (防止嵌套攻击)
        self.check_depth(&message.params, 0)?;

        // 3. 类型检查
        self.validate_types(message)?;

        Ok(())
    }
}
```

### 权限检查

```yaml
# IPC #1: TUI <-> Daemon 权限矩阵
ipc1_permissions:
  session.list:
    level: user
    description: TUI 可以列出会话

  session.create:
    level: user
    description: TUI 可以创建会话

  session.switch:
    level: user
    description: TUI 可以切换会话

  session.send_message:
    level: user
    description: TUI 可以发送用户消息

  session.get_status:
    level: user
    description: TUI 可以查询会话状态

  daemon.status:
    level: user
    description: TUI 可以查询 Daemon 状态

  daemon.stop:
    level: admin
    description: 需要 admin 权限停止 Daemon

# IPC #2: Daemon <-> Session Process 权限矩阵
ipc2_permissions:
  agent.create:
    level: daemon
    description: 仅 Daemon 可以请求创建 Agent

  agent.send_message:
    level: daemon
    description: 仅 Daemon 可以发送消息到 Agent

  agent.list:
    level: daemon
    description: 仅 Daemon 可以查询 Agent 列表

  task.list:
    level: daemon
    description: 仅 Daemon 可以查询任务列表

  session.shutdown:
    level: daemon
    description: 仅 Daemon 可以关闭会话进程

  # Session -> Daemon 事件 (无需权限检查，由 Daemon 验证来源)
  heartbeat:
    level: session
    description: Session Process 定期发送心跳

  agent.response:
    level: session
    description: Session Process 上报 Agent 响应

  task.status_change:
    level: session
    description: Session Process 上报任务状态

  error:
    level: session
    description: Session Process 上报错误
```

---

## 版本兼容性

### 版本号规范

```
<major>.<minor>.<patch>

- major: 破坏性变更
- minor: 新增 API，向后兼容
- patch: Bug 修复
```

### 兼容性策略

两个 IPC 边界独立版本化：

```yaml
versioning:
  # IPC #1 协议版本 (TUI <-> Daemon)
  ipc1_protocol_version: "1.0.0"
  ipc1_supported_range:
    min: "1.0.0"
    max: "2.0.0"  # 不包含 2.0.0

  # IPC #2 协议版本 (Daemon <-> Session Process)
  ipc2_protocol_version: "1.0.0"
  ipc2_supported_range:
    min: "1.0.0"
    max: "2.0.0"

  # 版本协商 (IPC #1: TUI 连接 Daemon 时)
  ipc1_handshake:
    tui_send:
      type: "hello"
      version: "1.0.0"
      protocol_version: "1.0.0"
    daemon_respond:
      type: "hello_ack"
      daemon_version: "1.0.0"
      compatible: true
      selected_protocol: "1.0.0"

  # 版本协商 (IPC #2: Daemon 启动 Session Process 时)
  ipc2_handshake:
    daemon_send:
      type: "init"
      protocol_version: "1.0.0"
      session_id: "<session_id>"
    session_respond:
      type: "init_ack"
      session_version: "1.0.0"
      compatible: true
      selected_protocol: "1.0.0"

  # 废弃 API 处理
  deprecation:
    warning_field: "deprecated"
    sunset_field: "sunset_version"
    alternative_method: "<new_method>"
```

---

## 性能优化

### 消息压缩

```yaml
compression:
  enabled: true
  threshold: 1024  # 大于 1KB 的消息才压缩
  algorithm: gzip  # 或 zstd

  # 不压缩的消息类型
  exclude_types:
    - stream_chunk  # 流式数据通常已经是分块的
    - notification # 通知通常很小
```

### 批量处理

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    pub requests: Vec<BatchRequestItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequestItem {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResponse {
    pub responses: Vec<BatchResponseItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResponseItem {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorResponse>,
}
```

---

## 测试策略

### 契约测试

```rust
// tests/ipc_contract.rs

#[cfg(test)]
mod tests {
    use knight_ipc::*;

    #[test]
    fn test_ipc1_request_serialization_roundtrip() {
        // IPC #1 请求消息序列化/反序列化往返测试
        let request = RequestMessage {
            base: BaseMessage::new(MessageType::Request),
            method: "session.create".to_string(),
            params: serde_json::json!({"name": "test", "workspace": "/tmp"}),
            options: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: RequestMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(request.method, decoded.method);
    }

    #[test]
    fn test_ipc2_event_serialization_roundtrip() {
        // IPC #2 事件消息序列化/反序列化往返测试
        let event = NotificationMessage {
            base: BaseMessage::new(MessageType::Notification),
            event: "heartbeat".to_string(),
            data: serde_json::json!({"session_id": "s1", "memory": 1024, "agents": 2}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: NotificationMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event, decoded.event);
    }

    #[test]
    fn test_error_codes_consistency() {
        // 验证错误码在两个 IPC 边界中一致
        let ipc1_codes = get_all_error_codes();
        let ipc2_codes = get_all_error_codes();
        assert_eq!(ipc1_codes, ipc2_codes);
    }
}
```

---

## 配置

```yaml
# config/ipc.yaml
ipc:
  # IPC #1: TUI <-> Daemon 传输配置
  ipc1_tui_daemon:
    transport:
      protocol: unix_socket           # unix_socket | tcp
      unix_socket:
        path: /tmp/knight-agent/daemon.sock
      tcp:
        host: "127.0.0.1"
        port: 0                       # 自动分配
      compression: true

    # 消息配置 (传输层限制)
    message:
      max_size: 10485760              # 10MB - 单条消息最大大小
      timeout: 300000                 # 5 分钟 - 消息处理超时
      queue_size: 1000                # 消息队列大小

    # 安全配置
    security:
      enable_validation: true
      max_message_depth: 100
      rate_limit:
        enabled: true
        max_per_minute: 1000

  # IPC #2: Daemon <-> Session Process 传输配置
  ipc2_daemon_session:
    transport:
      protocol: unix_socket           # unix_socket | stdin_stdout
      unix_socket:
        path_template: /tmp/knight-agent/session-{session_id}.sock
      stdin_stdout:
        line_delimited: true
      compression: false              # 本地短距离通信无需压缩

    # 消息配置
    message:
      max_size: 52428800              # 50MB - Session 可能有更大的消息
      timeout: 600000                 # 10 分钟 - Agent 操作可能较长时间
      queue_size: 500

    # 心跳配置
    heartbeat:
      interval: 5000                  # 5 秒心跳间隔
      timeout: 15000                  # 15 秒无心跳视为断开
      max_missed: 3                   # 连续 3 次未响应则标记为不可用

    # 安全配置
    security:
      enable_validation: true
      max_message_depth: 100

  # 版本配置
  version:
    ipc1_current: "1.0.0"
    ipc1_min_compatible: "1.0.0"
    ipc1_max_compatible: "2.0.0"
    ipc2_current: "1.0.0"
    ipc2_min_compatible: "1.0.0"
    ipc2_max_compatible: "2.0.0"
```

**配置说明**:

| 配置路径 | 说明 | 作用域 |
|---------|------|--------|
| `ipc1_tui_daemon.message.max_size` | IPC #1 单条消息最大大小 | IPC #1 传输层 |
| `ipc1_tui_daemon.message.timeout` | IPC #1 消息处理超时时间 | IPC #1 传输层 |
| `ipc1_tui_daemon.message.queue_size` | IPC #1 消息队列大小 | IPC #1 传输层 |
| `ipc2_daemon_session.message.max_size` | IPC #2 单条消息最大大小 | IPC #2 传输层 |
| `ipc2_daemon_session.message.timeout` | IPC #2 消息处理超时时间 | IPC #2 传输层 |
| `ipc2_daemon_session.heartbeat.interval` | Session 心跳间隔 | IPC #2 连接管理 |
| `ipc2_daemon_session.heartbeat.timeout` | 心跳超时时间 | IPC #2 连接管理 |
| `session.limits.max_sessions` | 最大会话数 | Session Manager (见 session-manager.md) |
| `session.limits.max_message_count` | 单会话最大消息数 | Session Manager (见 session-manager.md) |

**说明**: IPC 配置负责传输层限制，Session Manager 配置负责应用层限制。两者作用域不同，但共同影响系统性能和资源使用。

### 用户交互配置

```yaml
# config/ipc.yaml
ipc:
  # 用户交互配置
  user_interaction:
    enabled: true                    # 是否启用用户交互功能
    default_timeout: 300000         # 默认超时（毫秒），5 分钟
    max_timeout: 3600000             # 最大超时（毫秒），1 小时
    max_concurrent_queries: 10       # 最大并发询问数
    auto_reject_on_timeout: false   # 超时后自动拒绝

    # TUI 模式配置
    tui_mode:
      serial_queue: true             # TUI 模式下串行处理询问
      queue_priority:
        - permission                 # 权限询问优先级最高
        - confirmation
        - clarification
        - information
      show_pending_count: true       # 显示等待中的询问数量
      allow_batch_approve: false     # 是否允许批量批准（相同操作）

  # 危险操作确认配置
  dangerous_operation:
    auto_confirm_on_ci: false       # CI 环境下自动确认
    require_reason_on_deny: false   # 拒绝时是否需要填写原因
```

| 配置路径 | 说明 | 默认值 |
|---------|------|--------|
| `user_interaction.enabled` | 启用用户交互功能 | true |
| `user_interaction.default_timeout` | 默认等待超时 | 300000ms (5分钟) |
| `user_interaction.max_timeout` | 最大等待超时 | 3600000ms (1小时) |
| `user_interaction.max_concurrent_queries` | 最大并发询问数 | 10 |
| `user_interaction.auto_reject_on_timeout` | 超时后自动拒绝 | false |
| `user_interaction.tui_mode.serial_queue` | TUI 串行队列模式 | true |
| `user_interaction.tui_mode.show_pending_count` | 显示等待中的询问数量 | true |
| `dangerous_operation.auto_confirm_on_ci` | CI 环境自动确认 | false |

### TUI 模式串行队列

TUI 模式使用串行队列处理用户询问，因为终端界面本质上是顺序的：

```
多个 Agent 同时发起询问
         │
         ▼
┌────────────────────────────────┐
│  TUI 串行队列                   │
│  - 按优先级排序                 │
│  - permission > confirmation   │
│    > clarification > info      │
└────────────────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  显示当前询问                  │
│  "[1/3] code-reviewer 询问:   │
│   是否允许删除文件？            │
│   (还有 2 个等待中的询问)"     │
└────────────────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  等待用户输入                  │
│  - y/n/选项                    │
│  - 或 "batch" 批量处理         │
└────────────────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  处理响应                      │
│  - 发送 UserResponse           │
│  - 取出下一个询问              │
└────────────────────────────────┘
```

**批量处理**: 用户可输入 `batch` 批量批准相同类型的询问（如多个 "删除文件" 的权限询问）。

### 跨 Agent 依赖检测

当多个 Agent 同时询问用户时，系统检测可能的依赖关系并发出警告：

```
多个 Agent 同时发起询问
         │
         ▼
┌────────────────────────────────┐
│  检查依赖关系                  │
│  - 查询间的依赖               │
│  - Agent 间的依赖             │
└────────────────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  检测到循环依赖？              │
│  Agent A 等待 Agent B          │
│  Agent B 等待 Agent A          │
└────────────────────────────────┘
         │
    ┌────┴────┐
    │         │
   是         否
    │         │
    ▼         ▼
发出警告    正常处理
```

**依赖示例**：

```yaml
# Agent A 的询问
agent_a_query:
  agent_id: "agent_a"
  message: "是否批准 API 设计方案？"
  dependencies:
    waiting_for_agent: "agent_b"      # 等待 Agent B 的技术分析
    depends_on_agents: ["agent_b"]

# Agent B 的询问
agent_b_query:
  agent_id: "agent_b"
  message: "使用哪个数据库？"
  dependencies:
    waiting_for_agent: "agent_a"      # 等待 Agent A 的架构决策
    depends_on_agents: ["agent_a"]

# 检测结果：循环依赖警告
warning: "检测到循环依赖：agent_a ↔ agent_b"
recommendation: "先回答架构决策，再回答技术选型"
```

**警告 UI 示例**：

```
⚠️  检测到 Agent 间依赖

当前询问顺序可能导致等待：
  1. [agent_a] 是否批准 API 设计方案？
     → 依赖: agent_b 的技术分析

  2. [agent_b] 使用哪个数据库？
     → 依赖: agent_a 的架构决策

建议：先回答 [agent_a] 的架构决策，
     然后再回答 [agent_b] 的技术选型。

[按推荐顺序] [保持当前顺序]
```

---

## 附录

### 性能指标

| 指标 | IPC #1 目标值 | IPC #2 目标值 | 说明 |
|------|--------------|--------------|------|
| 消息延迟 | < 5ms | < 2ms | 本地 Unix Socket 通信 |
| 吞吐量 | > 10000 msg/s | > 20000 msg/s | 单连接 |
| 连接建立 | < 50ms | < 10ms | Unix Socket 连接 |
| 内存占用 | < 10MB | < 5MB | 消息缓冲 |

### 错误处理示例

```rust
use std::time::Duration;
use tokio::time::sleep;

/// 带重试的 IPC 调用
pub async fn call_ipc_with_retry(
    transport: &dyn Transport,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, IpcError> {
    let max_retries = 3;

    for attempt in 0..max_retries {
        let response = transport.send_request(method, params.clone()).await?;

        if let Some(error) = response.error {
            // 检查是否可重试
            if is_retryable_error(error.code) && attempt < max_retries - 1 {
                let delay = Duration::from_millis(1000 * 2u64.pow(attempt as u32));
                sleep(delay).await;
                continue;
            }
            return Err(IpcError::from(error));
        }

        return Ok(response.result.unwrap_or(serde_json::Value::Null));
    }

    Err(IpcError::MaxRetriesExceeded)
}

/// 判断错误码是否可重试
fn is_retryable_error(code: i32) -> bool {
    matches!(
        code,
        5 | 5000 | 5001  // Timeout | InternalError | ResourceExhausted
    )
}
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0.0 | 2026-04-08 | 重构为两个 IPC 边界 (TUI<->Daemon, Daemon<->Session)；移除 TypeScript/WebSocket，改为 Rust/Unix Socket |
| 1.0.0 | 2026-04-02 | 初始版本 |
