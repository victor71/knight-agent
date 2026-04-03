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
