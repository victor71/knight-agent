# IPC Contract (进程间通信契约)

## 概述

### 职责描述

IPC Contract 定义 Knight-Agent 系统中 Rust 核心服务与 TypeScript UI 层之间的进程间通信协议，包括：

- 消息格式定义 (Request/Response)
- 通信协议规范 (WebSocket/stdio)
- 错误处理和重试机制
- 类型安全保证
- 版本兼容性策略

### 架构背景

Knight-Agent 采用混合架构：

```
┌─────────────────────────────────────────────────────────┐
│                    TypeScript UI                        │
│  (Electron/Vite/Web) - 用户界面、交互逻辑               │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    IPC Boundary                         │
│  - WebSocket / stdio / JSON-RPC                         │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Rust Core                            │
│  Session Manager, Orchestrator, Agent Runtime, etc.     │
└─────────────────────────────────────────────────────────┘
```

### 设计目标

1. **类型安全**: 跨语言类型一致性保证
2. **高性能**: 低延迟通信，支持流式传输
3. **可靠性**: 错误恢复、断线重连
4. **可维护性**: 清晰的版本管理和兼容策略
5. **安全性**: 消息验证、权限检查

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 协作 | 会话状态同步。见 [Session Manager 接口](../core/session-manager.md#对外接口) |
| Router | 协作 | 请求路由和分发。见 [Router 接口](../core/router.md#对外接口) |
| Security Manager | 协作 | 权限验证。见 [Security Manager 接口](../security/security-manager.md#对外接口) |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| TypeScript UI | 被依赖 | UI 层调用 Rust 核心 |
| CLI Frontend | 被依赖 | 命令行接口 |

---

## 通信协议

### 传输层选择

| 协议 | 用途 | 优点 | 缺点 |
|------|------|------|------|
| **WebSocket** | UI 模式 | 双向通信、低延迟、支持流式 | 需要 HTTP 升级 |
| **stdio (JSON-RPC)** | CLI 模式 | 简单、无端口占用 | 单向请求响应 |
| **Unix Socket** | 本地通信 | 高性能、安全 | 跨平台复杂 |

### 推荐方案

```yaml
# 根据运行模式选择传输层
transport:
  ui_mode:
    protocol: websocket
    port: 0  # 自动分配
    path: /ws/knight

  cli_mode:
    protocol: stdio
    format: json_rpc

  embedded_mode:
    protocol: direct_call  # 直接函数调用（编译时链接）
```

---

## 消息格式

### 基础消息结构

```typescript
// TypeScript 端
interface BaseMessage {
  id: string;           // 消息唯一 ID
  type: MessageType;    // 消息类型
  timestamp: number;    // Unix 时间戳 (毫秒)
  session_id?: string;  // 会话 ID (可选)
}

enum MessageType {
  Request = "request",
  Response = "response",
  Notification = "notification",
  StreamChunk = "stream_chunk",
  Error = "error",
  UserQuery = "user_query",       // Agent 询问用户
  UserResponse = "user_response", // 用户响应
}
```

```rust
// Rust 端
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    pub id: String,
    pub r#type: MessageType,
    pub timestamp: i64,
    pub session_id: Option<String>,
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

```typescript
interface RequestMessage extends BaseMessage {
  type: MessageType.Request;
  method: string;        // 方法名 (如 "session.create")
  params: unknown;       // 参数
  options?: RequestOptions;
}

interface RequestOptions {
  timeout?: number;      // 超时时间 (毫秒)
  stream?: boolean;      // 是否流式响应
  priority?: number;     // 优先级
}
```

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

```typescript
interface ResponseMessage extends BaseMessage {
  type: MessageType.Response;
  request_id: string;    // 关联的请求 ID
  result?: unknown;      // 成功结果
  error?: ErrorResponse; // 错误信息
  streaming?: boolean;   // 是否还有后续流式数据
}

interface ErrorResponse {
  code: number;          // 错误码
  message: string;       // 错误消息
  details?: unknown;     // 详细信息
  stack?: string;        // 堆栈跟踪
}
```

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

```typescript
interface NotificationMessage extends BaseMessage {
  type: MessageType.Notification;
  event: string;         // 事件名
  data: unknown;         // 事件数据
}
```

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

```typescript
interface StreamChunkMessage extends BaseMessage {
  type: MessageType.StreamChunk;
  request_id: string;    // 关联的请求 ID
  sequence: number;      // 序列号
  chunk: string;         // 数据块
  done: boolean;         // 是否结束
}
```

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

```typescript
interface UserQueryMessage extends BaseMessage {
  type: MessageType.UserQuery;
  await_id: string;        // 等待 ID，用于匹配响应
  query_type: QueryType;   // 询问类型
  agent_id: string;        // 发起询问的 Agent
  message: string;          // 询问内容
  options?: string[];      // 可选的回答选项
  context: {
    // 触发询问的上下文信息
    resource?: string;      // 相关资源路径
    action?: string;       // 正在执行的操作
    reason?: string;       // 询问原因
  };
  timeout: number;         // 超时时间（毫秒），0 表示不超时
  created_at: number;      // 创建时间戳
}

enum QueryType {
  Permission = "permission",       // 权限询问
  Clarification = "clarification", // 澄清询问
  Confirmation = "confirmation",   // 确认询问
  Information = "information",     // 信息请求
}
```

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

```typescript
interface UserResponseMessage extends BaseMessage {
  type: MessageType.UserResponse;
  await_id: string;        // 对应的等待 ID
  response: UserResponseData;
  responded_at: number;    // 响应时间戳
}

interface UserResponseData {
  accepted: boolean;        // 用户是否接受
  value?: string;          // 用户输入的值（当有选项时）
  custom_input?: string;   // 用户的自定义输入
}
```

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

---

## API 契约定义

### 方法命名规范

```
{module}.{entity}.{action}

例如:
- session.create       // 创建会话
- session.get          // 获取会话
- session.destroy      // 销毁会话
- agent.spawn          // 启动 Agent
- agent.send_message   // 发送消息到 Agent
- agent.list           // 列出 Agent
```

### 核心 API

```yaml
# Session 管理
session.create:
  description: 创建新会话
  params:
    name:
      type: string
      required: false
    workspace:
      type: string
      required: false
  result:
    session_id: string
    session: Session
    description: |
      会话对象，类型定义见 [Session Manager](../core/session-manager.md#session-数据结构)

session.get:
  description: 获取会话信息
  params:
    session_id:
      type: string
      required: true
  result:
    session: Session | null
    description: |
      会话对象，类型定义见 [Session Manager](../core/session-manager.md#session-数据结构)

session.destroy:
  description: 销毁会话
  params:
    session_id:
      type: string
      required: true
  result:
    success: boolean

# Agent 管理
agent.spawn:
  description: 启动 Agent
  params:
    agent_definition:
      type: object
      required: true
    session_id:
      type: string
      required: true
  result:
    agent_id: string

agent.send_message:
  description: 发送消息到 Agent
  params:
    agent_id:
      type: string
      required: true
    message:
      type: string
      required: true
    options:
      type: object
      required: false
  result:
    response_stream: boolean  # 是否流式响应

agent.list:
  description: 列出 Agent
  params:
    session_id:
      type: string
      required: true
  result:
    agents: array<AgentInfo>
    description: |
      Agent 信息数组，类型定义见 [Orchestrator](../core/orchestrator.md#agentinfo)

# 工具调用
tools.call:
  description: 调用工具
  params:
    tool_name:
      type: string
      required: true
    args:
      type: object
      required: true
  result:
    result: ToolResult
    description: |
      工具执行结果，类型定义见 [Tool System](../tools/tool-system.md#工具结果)

# 流式订阅
stream.subscribe:
  description: 订阅流式输出
  params:
    request_id:
      type: string
      required: true
  result:
    subscribed: boolean

stream.unsubscribe:
  description: 取消订阅
  params:
    request_id:
      type: string
      required: true
  result:
    success: boolean

# 用户交互
user.query:
  description: |
    Agent 发起用户询问（内部由 Agent Runtime 调用，UI 层订阅此类型消息）
    此方法不会直接返回响应，而是将询问通过 IPC 消息发送，UI 通过 stream.subscribe 接收
    响应处理：UI 调用 user.respond()，最终由 Agent Runtime.handle_user_response() 处理
    见 [Agent Runtime 用户交互流程](../agent/agent-runtime.md#tool-调用流程)
  params:
    agent_id:
      type: string
      required: true
      description: 发起询问的 Agent ID
    query_type:
      type: string
      enum: [permission, clarification, confirmation, information]
      required: true
    message:
      type: string
      required: true
      description: 询问内容
    options:
      type: array<string>
      required: false
      description: 可选的选项列表
    context:
      type: object
      required: false
      description: 触发询问的上下文信息
    timeout:
      type: integer
      required: false
      default: 0
      description: 超时时间（毫秒），0 表示不超时
  result:
    await_id: string
    description: 等待 ID，UI 响应时需要携带此 ID

user.respond:
  description: 用户响应询问
  params:
    await_id:
      type: string
      required: true
      description: 对应的等待 ID
    accepted:
      type: boolean
      required: true
      description: 用户是否接受
    value:
      type: string
      required: false
      description: 用户选择的选项值
    custom_input:
      type: string
      required: false
      description: 用户的自定义输入
  result:
    success: boolean
    description: 响应是否成功投递到 Agent

user.cancel:
  description: 取消等待用户响应
  params:
    await_id:
      type: string
      required: true
      description: 对应的等待 ID
  result:
    success: boolean
    description: 取消是否成功

user.list_pending:
  description: 列出当前等待用户响应的事件
  result:
    queries: array<PendingQuery>
    description: 待处理的询问列表

PendingQuery:
  await_id: string
  agent_id: string
  query_type: string
  message: string
  created_at: string
  timeout: integer
```

---

## 错误处理

### 错误码定义

```typescript
enum ErrorCode {
  // 通用错误 (1-999)
  UnknownError = 1,
  ParseError = 2,           // 消息解析失败
  InvalidRequest = 3,       // 无效请求
  MethodNotFound = 4,       // 方法不存在
  Timeout = 5,             // 超时

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
  AwaitCancelled = 6001,           // 等待被取消
  InvalidUserResponse = 6002,      // 无效的用户响应
  AwaitNotFound = 6003,            // 等待 ID 不存在

  // 系统错误 (5000-5999)
  InternalError = 5000,
  ResourceExhausted = 5001,
}
```

**说明**: 以上为 IPC 层的错误码规范。内部模块（如 Session Manager、Security Manager）的错误码在传播到 IPC 层时应映射到上述错误码。各模块的错误码定义仅供参考，实际 IPC 通信统一使用本节定义的错误码。

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

当 Agent 需要与用户交互（权限确认、信息补充、危险操作确认等）时，通过以下流程进行：

```
┌─────────────────────────────────────────────────────────────────┐
│                         Agent Runtime                             │
│                                                                   │
│  Agent 执行中遇到需要用户确认的场景                               │
│        │                                                         │
│        ▼                                                         │
│  ┌──────────────────────────────┐                                │
│  │ 调用 user.query()            │                                │
│  │ - await_id: 唯一标识         │                                │
│  │ - query_type: 询问类型       │                                │
│  │ - message: 询问内容          │                                │
│  │ - context: 上下文信息        │                                │
│  └──────────────────────────────┘                                │
│        │                                                         │
│        ▼                                                         │
│  Agent 进入 AwaitingUser 状态                                    │
│        │                                                         │
└────────┼─────────────────────────────────────────────────────────┘
         │
         ▼ IPC Message (UserQuery)
┌─────────────────────────────────────────────────────────────────┐
│                         TypeScript UI                             │
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
         │
         ▼ IPC Message (UserResponse)
┌─────────────────────────────────────────────────────────────────┐
│                         Agent Runtime                             │
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

Agent 可以同时发起多个用户询问：

```
Agent A ─┬─ await_id_1 ──▶ 用户询问 1
         ├─ await_id_2 ──▶ 用户询问 2
         └─ await_id_3 ──▶ 用户询问 3
```

UI 层通过 `await_id` 区分不同的询问，用户可以按任意顺序响应。

---

## 类型同步

### 代码生成策略

使用 TypeScript/Rust 类型定义作为单一真实来源 (Single Source of Truth)，自动生成对应语言的类型代码：

```yaml
# 类型定义目录结构
types/
├── shared/
│   ├── message-types.ts    # TypeScript 类型定义
│   ├── api-contracts.ts    # API 契约定义
│   └── error-codes.ts      # 错误码定义
├── generated/
│   ├── rust/
│   │   ├── message_types.rs
│   │   ├── api_contracts.rs
│   │   └── error_codes.rs
│   └── ts/
│       └── (符号链接到 ../shared)
```

### 生成工具

```typescript
// scripts/generate-rust-types.ts
import { compile } from 'json-schema-to-typescript';
import { writeFileSync } from 'fs';

// 从 TypeScript 类型生成 JSON Schema
async function generateRustTypes() {
  // 1. 解析 TypeScript 类型
  // 2. 生成 JSON Schema
  // 3. 使用 serde-rs/jsonschema_codegen 生成 Rust 类型
  // 4. 输出到 generated/rust/
}

generateRustTypes();
```

```bash
# cargo generate --type=typescript --input=types/shared --output=types/generated/rust
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
# API 权限矩阵
permissions:
  session.create:
    level: public
    description: 任何人都可以创建会话

  agent.spawn:
    level: user
    description: 需要用户权限

  tools.call:
    level: restricted
    description: 需要 Security Manager 验证
    validator: security_manager.check_tool_permission

  # 用户交互 API
  user.query:
    level: agent
    description: 仅限 Agent 调用（由 Agent Runtime 内部使用）

  user.respond:
    level: user
    description: 仅限用户调用

  user.cancel:
    level: user
    description: 仅限用户调用

  user.list_pending:
    level: user
    description: 仅限用户调用
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

```yaml
versioning:
  # 协议版本
  protocol_version: "1.0.0"

  # 支持的客户端版本范围
  supported_client_range:
    min: "1.0.0"
    max: "2.0.0"  # 不包含 2.0.0

  # 版本协商
  handshake:
    # 连接时交换版本信息
    client_send:
      type: "hello"
      version: "1.2.3"
      protocol_version: "1.0.0"

    server_respond:
      type: "hello_ack"
      server_version: "1.0.5"
      compatible: true
      selected_protocol: "1.0.0"

  # 废弃 API 处理
  deprecation:
    warning_header: "X-API-Deprecated"
    sunset_header: "X-API-Sunset"
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

```typescript
interface BatchRequest {
  requests: Array<{
    id: string;
    method: string;
    params: unknown;
  }>;
}

interface BatchResponse {
  responses: Array<{
    id: string;
    result?: unknown;
    error?: ErrorResponse;
  }>;
}
```

---

## 测试策略

### 契约测试

```typescript
// tests/contract/ipc-contract.test.ts
describe('IPC Contract', () => {
  it('should match Rust types', async () => {
    // 1. 从 Rust 端获取类型定义
    const rustTypes = await fetchRustTypeDefinitions();

    // 2. 从 TypeScript 端获取类型定义
    const tsTypes = getTSTypeDefinitions();

    // 3. 验证一致性
    expect(compareTypes(rustTypes, tsTypes)).toBe(true);
  });

  it('should handle all error codes', () => {
    const rustErrorCodes = getRustErrorCodes();
    const tsErrorCodes = getTSErrorCodes();

    expect(rustErrorCodes).toEqual(tsErrorCodes);
  });
});
```

---

## 配置

```yaml
# config/ipc.yaml
ipc:
  # 传输配置
  transport:
    ui_mode:
      protocol: websocket
      port: 0
      path: /ws/knight
      compression: true

    cli_mode:
      protocol: stdio
      format: json_rpc

  # 消息配置 (传输层限制)
  message:
    max_size: 10485760  # 10MB - 单条消息最大大小
    timeout: 300000     # 5 分钟 - 消息处理超时
    queue_size: 1000    # 消息队列大小

  # 安全配置
  security:
    enable_validation: true
    max_message_depth: 100
    rate_limit:
      enabled: true
      max_per_minute: 1000

  # 版本配置
  version:
    current: "1.0.0"
    min_compatible: "1.0.0"
    max_compatible: "2.0.0"
```

**配置说明**:

| 配置路径 | 说明 | 作用域 |
|---------|------|--------|
| `message.max_size` | 单条消息最大大小 | IPC 传输层 |
| `message.timeout` | 消息处理超时时间 | IPC 传输层 |
| `message.queue_size` | 消息队列大小 | IPC 传输层 |
| `security.max_message_depth` | 消息最大嵌套深度 | IPC 传输层 |
| `session.limits.max_sessions` | 最大会话数 | Session Manager (见 session-manager.md) |
| `session.limits.max_message_count` | 单会话最大消息数 | Session Manager (见 session-manager.md) |

**说明**: IPC 层配置负责传输层限制，Session Manager 配置负责应用层限制。两者作用域不同，但共同影响系统性能和资源使用。

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
    notification:
      sound_enabled: true            # 收到询问时播放提示音
      desktop_notification: true     # 发送桌面通知

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
| `dangerous_operation.auto_confirm_on_ci` | CI 环境自动确认 | false |

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 消息延迟 | < 5ms | 本地通信 |
| 吞吐量 | > 10000 msg/s | 单连接 |
| 连接建立 | < 100ms | WebSocket 握手 |
| 内存占用 | < 10MB | 消息缓冲 |

### 错误处理示例

```typescript
// TypeScript 端错误处理
async function callIPC<T>(
  method: string,
  params: unknown,
): Promise<T> {
  const maxRetries = 3;
  let attempt = 0;

  while (attempt < maxRetries) {
    try {
      const response = await sendRequest(method, params);

      if (response.error) {
        // 检查是否可重试
        if (isRetryableError(response.error.code) && attempt < maxRetries - 1) {
          await delay(Math.pow(2, attempt) * 1000);
          attempt++;
          continue;
        }

        throw new IPCError(
          response.error.code,
          response.error.message,
          response.error.details,
        );
      }

      return response.result as T;
    } catch (error) {
      if (attempt === maxRetries - 1) throw error;
      attempt++;
    }
  }

  throw new Error('Max retries exceeded');
}

// 判断错误码是否可重试
function isRetryableError(code: ErrorCode): boolean {
  const retryableCodes: ErrorCode[] = [
    ErrorCode.Timeout,
    ErrorCode.InternalError,
    ErrorCode.ResourceExhausted,
  ];
  return retryableCodes.includes(code);
}
}
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-04-02 | 初始版本 |
