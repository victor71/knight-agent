# Agent Runtime (Agent 运行时)

## 概述

### 职责描述

Agent Runtime 负责单个 Agent 的执行逻辑，包括：

- Agent 生命周期管理（初始化、执行、停止）
- LLM 调用和响应处理
- Tool/Skill 调用执行
- 上下文管理和状态更新
- 错误处理和重试逻辑

### 设计目标

1. **隔离性**: 每个 Agent 实例独立运行，状态不共享
2. **可靠性**: 自动重试和错误恢复
3. **可观测性**: 完整的执行追踪和日志
4. **可扩展性**: 支持自定义 Agent 变体

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取会话上下文 |
| LLM Provider | 依赖 | LLM 调用 |
| Tool System | 依赖 | 工具执行 |
| Skill Engine | 依赖 | 技能触发 |
| Hook Engine | 依赖 | 生命周期钩子 |
| Orchestrator | 协作 | 创建 Agent 后注册到 Orchestrator 池。Agent 分配接口见 [Orchestrator 接口](../core/orchestrator.md#agent-分配-供-task-manager-调用)。 |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| Orchestrator | 被依赖 | Agent 创建后调用 [Orchestrator.register_agent](../core/orchestrator.md#agent-池管理) |
| Skill Engine | 被依赖 | Agent 可调用 Skill |
| External Agent | 被依赖 | 外部 Agent 基于内部 Agent 接口 |

---

## 接口定义

### 对外接口

```yaml
# Agent Runtime 接口定义
AgentRuntime:
  # ========== Agent 生命周期 ==========
  create_agent:
    description: |
      创建 Agent 实例

      注意: 创建成功后，会自动调用 Orchestrator.register_agent
      将 Agent 注册到池中，供 Task Manager 分配使用
    inputs:
      definition:
        type: AgentDefinition
        description: Agent 定义
        required: true
      session_id:
        type: string
        description: 所属会话 ID
        required: true
      variant:
        type: string
        description: Agent 变体名称
        required: false
    outputs:
      agent_id:
        type: string
      agent:
        type: Agent

  start_agent:
    description: 启动 Agent
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  stop_agent:
    description: 停止 Agent
    inputs:
      agent_id:
        type: string
        required: true
      force:
        type: boolean
        description: 强制停止
        required: false
        default: false
    outputs:
      success:
        type: boolean

  get_agent_state:
    description: 获取 Agent 状态
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      state:
        type: AgentState

  # ========== 消息处理 ==========
  send_message:
    description: 发送消息给 Agent
    inputs:
      agent_id:
        type: string
        required: true
      message:
        type: Message
        required: true
      stream:
        type: boolean
        description: 是否使用流式响应
        required: false
        default: true
    outputs:
      response_stream:
        type: stream<MessageChunk>
        description: 流式响应(当 stream=true 时)
      response_complete:
        type: Message
        description: 完整响应(当 stream=false 时)
      request_id:
        type: string
        description: 请求唯一标识

  # ========== 上下文管理 ==========
  get_context:
    description: 获取 Agent 上下文
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      context:
        type: AgentContext

  update_variables:
    description: 更新 Agent 变量
    inputs:
      agent_id:
        type: string
        required: true
      variables:
        type: map<string, any>
        required: true
    outputs:
      success:
        type: boolean

  # ========== 执行控制 ==========
  pause:
    description: 暂停 Agent 执行
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  resume:
    description: 恢复 Agent 执行
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 用户交互 ==========
  handle_user_response:
    description: |
      处理用户响应（由 IPC 层调用）
      当 Agent 处于 awaiting_user 状态时，接收用户响应并恢复执行
    inputs:
      agent_id:
        type: string
        required: true
      await_id:
        type: string
        required: true
        description: 等待 ID，必须与 awaiting_user 状态中的 await_id 匹配
      response:
        type: UserResponse
        required: true
        description: 用户响应内容，类型定义见 [IPC Contract](../infrastructure/ipc-contract.md#用户响应消息)
    outputs:
      success:
        type: boolean
        description: 是否成功处理响应
      resumed_state:
        type: string
        description: 恢复后的状态 (thinking/acting)

  cancel_operation:
    description: |
      取消当前操作
      - 当 Agent 处于 acting 状态时：中断正在执行的工具
      - 当 Agent 处于 awaiting_user 状态时：取消等待中的用户询问
      - 取消后 Agent 进入 idle 状态，可接收新消息
      - 与 stop_agent 的区别：cancel 只取消当前操作，stop_agent 停止整个 Agent
    inputs:
      agent_id:
        type: string
        required: true
      reason:
        type: string
        required: false
        description: 取消原因
    outputs:
      success:
        type: boolean
        description: 是否成功取消
      cancelled_await_id:
        type: string | null
        description: 如果取消了用户询问，返回对应的 await_id

  # ========== 工具调用代理 ==========
  call_tool:
    description: Agent 调用工具（内部接口）
    inputs:
      agent_id:
        type: string
        required: true
      tool_name:
        type: string
        required: true
      args:
        type: object
        required: true
    outputs:
      result:
        type: ToolResult
```

### 数据结构

```yaml
# Agent 定义
AgentDefinition:
  id:
    type: string
    description: Agent 唯一标识
  name:
    type: string
    description: Agent 名称
  role:
    type: string
    description: Agent 角色描述
  model:
    type: ModelConfig
    description: 模型配置
  instructions:
    type: string
    description: 系统指令
  tools:
    type: array<string>
    description: 可用工具列表
  skills:
    type: array<string>
    description: 可用技能列表
  permissions:
    type: PermissionConfig
    description: 权限配置
  variants:
    type: array<AgentVariant>
    description: 支持的变体

# Model 配置
ModelConfig:
  provider:
    type: string
    description: 提供者 (anthropic/openai/custom)
  model:
    type: string
    description: 模型名称
  temperature:
    type: float
    description: 温度参数
    default: 0.7
  max_tokens:
    type: integer
    description: 最大输出 Token
    default: 4096

# Agent 变体
AgentVariant:
  name:
    type: string
    description: 变体名称
  model:
    type: ModelConfig
    description: 覆盖的模型配置
  instructions:
    type: string
    description: 覆盖的系统指令

# Agent 实例
Agent:
  id:
    type: string
    description: 实例 ID
  definition:
    type: AgentDefinition
    description: 使用的定义
  session_id:
    type: string
    description: 所属会话
  variant:
    type: string | null
    description: 当前变体
  state:
    type: AgentState
    description: 运行状态
  context:
    type: AgentContext
    description: 运行上下文

# Agent 状态
AgentState:
  status:
    type: enum
    values: [idle, thinking, acting, paused, awaiting_user, error, stopped]
    description: |
      状态标识：
      - idle: 空闲，等待消息
      - thinking: 思考中，正在处理消息
      - acting: 执行中，正在执行工具
      - paused: 已暂停（用户主动暂停）
      - awaiting_user: 等待用户响应（UserQuery 场景）
      - error: 错误状态
      - stopped: 已停止
  current_action:
    type: string | null
    description: 当前执行的动作
  error:
    type: ErrorInfo | null
    description: 错误信息
  statistics:
    type: AgentStatistics
    description: 统计信息
  await_info:
    type: AwaitInfo | null
    description: 等待用户响应时的信息（当 status=awaiting_user 时有效）

# 等待用户响应信息
AwaitInfo:
  await_id:
    type: string
    description: 等待 ID，用于匹配 UserResponse
  query_type:
    type: string
    description: 询问类型 (permission/clarification/confirmation/information)
  message:
    type: string
    description: 询问内容
  created_at:
    type: datetime
    description: 创建时间

# Agent 上下文
AgentContext:
  messages:
    type: array<Message>
    description: 消息历史
  variables:
    type: map<string, any>
    description: 变量
  memory:
    type: array<MemoryItem>
    description: 记忆项

# 记忆项
MemoryItem:
  key:
    type: string
  value:
    type: any
  timestamp:
    type: datetime

# Agent 统计
AgentStatistics:
  messages_sent:
    type: integer
  messages_received:
    type: integer
  tools_called:
    type: integer
  llm_calls:
    type: integer
  total_tokens:
    type: integer
  errors:
    type: integer

# 错误信息
ErrorInfo:
  code:
    type: string
  message:
    type: string
  details:
    type: object
  retryable:
    type: boolean
    description: 是否可重试

# Message 数据结构
Message:
  role:
    type: enum
    values: [user, assistant, system, tool]
    description: |
      角色类型：
      - user: 用户消息
      - assistant: AI 助手消息
      - system: 系统消息
      - tool: 工具调用结果消息
  content:
    type: string | array<ContentBlock>
  timestamp:
    type: datetime
  metadata:
    type: map<string, any>

# ContentBlock
ContentBlock:
  type:
    type: enum
    values: [text, image, tool_use, tool_result]
  content:
    type: any

# Tool 调用结果
ToolResult:
  success:
    type: boolean
  data:
    type: any
  error:
    type: string | null
  duration_ms:
    type: integer
```

### 配置选项

```yaml
# config/agent.yaml
agent:
  # 执行配置
  execution:
    max_execution_time: 300      # 最大执行时间（秒）
    max_tool_calls: 50           # 最大工具调用次数
    max_llm_calls: 20            # 最大 LLM 调用次数

  # 重试配置
  retry:
    max_attempts: 3              # 最大重试次数
    delay: 1000                  # 重试延迟（毫秒）
    backoff: exponential         # 退避策略
    retryable_errors:
      - rate_limit
      - timeout
      - connection_error

  # 超时配置
  timeout:
    llm_call: 60                 # LLM 调用超时（秒）
    tool_call: 30                # 工具调用超时（秒）

  # 流式输出
  streaming:
    enabled: true
    chunk_size: 100              # 流式块大小
```

---

## 核心流程

### Agent 执行流程

```
接收用户消息
        │
        ▼
┌──────────────────────────────┐
│ 1. 触发 before hooks         │
│    - agent_execute hook      │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 构建对话上下文            │
│    - 获取历史消息            │
│    - 添加系统指令            │
│    - 插入工具定义            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 调用 LLM                  │
│    - 发送请求                │
│    - 流式接收响应            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 有工具  │
    │ 调用？  │
    └───┬────┘
        │ 是                │ 否
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 4. 执行工具调用  │   │ 6. 返回响应  │
│    - 解析工具名  │   └──────────────┘
│    - 调用工具    │          │
│    - 获取结果    │          ▼
└──────────────────┘   ┌──────────────┐
        │              │ 7. 触发      │
        ▼              │ after hooks  │
┌──────────────────┐   └──────────────┘
│ 5. 将结果加入    │          │
│    上下文        │          ▼
│    → 回到步骤3   │    完成
└──────────────────┘
```

### Tool 调用流程

```
Agent 请求调用工具
        │
        ▼
┌──────────────────────────────┐
│ 1. 触发 tool_call hook       │
│    - before hook             │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 权限检查                  │
│    - 检查工具是否允许        │
│    - 检查参数是否有效        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 通过？  │
    └───┬────┘
        │ 否
        ▼
┌──────────────────────────────┐
│ 2a. 检查是否可请求用户授权   │
│    - dangerous 操作         │
│    - 权限不足但可申请       │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 可申请？│
    └───┬────┘
        │ 否
        ▼
    返回权限错误
        │ 是
        ▼
┌──────────────────────────────┐
│ 2b. 调用 user.query()        │
│    - await_id 生成           │
│    - 状态转换为 awaiting_user │
└──────────────────────────────┘
        │
        ▼
    Agent 等待用户响应
        │
        ▼
    ┌───┴────┐
    │ 用户   │
    │ 授权？ │
    └───┬────┘
        │ 否
        ▼
    返回权限拒绝
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 执行工具                  │
│    - 调用 Tool System        │
│    - 传入参数                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 4. 错误处理      │   │ 5. 记录结果  │
│    - 检查可重试  │   │    更新统计  │
│    - 记录错误    │   └──────────────┘
└──────────────────┘          │
        │                     ▼
        ▼              ┌──────────────┐
┌──────────────────┐   │ 6. 触发      │
│ 5. 触发          │   │ tool_result  │
│ tool_result hook │   │ hook         │
└──────────────────┘   └──────────────┘
        │                     │
        ▼                     ▼
    返回错误          返回结果
```

### 状态机设计

```mermaid
stateDiagram-v2
    [*] --> Idle: create()
    Idle --> Thinking: receive message
    Thinking --> Thinking: LLM streaming
    Thinking --> Acting: tool use detected
    Thinking --> Idle: complete
    Thinking --> AwaitingUser: user.query() ⭐
    AwaitingUser --> Thinking: user.response() ✅
    AwaitingUser --> Idle: timeout/cancel ⭐
    Acting --> Thinking: tool result
    Acting --> Idle: tool error (non-retryable)
    Thinking --> Error: execution error
    Acting --> Error: execution error
    Error --> Idle: recoverable
    Error --> Stopped: unrecoverable
    Idle --> Paused: pause()
    Paused --> Idle: resume()
    Idle --> Stopped: stop()
    Paused --> Stopped: stop()
    AwaitingUser --> Stopped: stop()
    Acting --> Idle: cancel_operation() ⭐ NEW
    AwaitingUser --> Idle: cancel_operation() ⭐ NEW
```

**状态说明**:

| 状态 | 说明 | 可转换到 |
|------|------|---------|
| `idle` | 空闲，等待消息 | thinking, paused, stopped |
| `thinking` | 思考中，正在处理消息 | acting, idle, awaiting_user, error |
| `acting` | 执行中，正在执行工具 | thinking, idle, error, stopped |
| `awaiting_user` | 等待用户响应 | thinking, idle, stopped |
| `paused` | 已暂停（用户主动） | idle, stopped |
| `error` | 错误状态 | idle, stopped |
| `stopped` | 已停止 | - |

**操作说明**:

| 操作 | 可调用状态 | 行为 |
|------|----------|------|
| `pause()` | idle | Agent 进入 paused 状态，等待 resume() |
| `resume()` | paused | Agent 恢复到 idle 状态 |
| `stop()` | any | Agent 立即进入 stopped 状态 |
| `cancel_operation()` | acting, awaiting_user | 中断当前操作，Agent 进入 idle 状态 |
| `user.response()` | awaiting_user | Agent 继续执行 |

**注意**: `awaiting_user` 与 `paused` 的区别：
- `paused`: 用户主动暂停，任意时刻可恢复
- `awaiting_user`: Agent 等待用户响应，必须收到 UserResponse 或 cancel() 才能继续

### 错误处理与重试

```
执行过程中发生错误
        │
        ▼
┌──────────────────────────────┐
│ 1. 分类错误类型              │
│    - 可重试错误              │
│    - 不可重试错误            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 可重试？│
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 2. 记录错误      │   │ 检查重试次数 │
│    更新状态      │   └──────────────┘
└──────────────────┘          │
        │               ┌────┴────┐
        ▼               │ 超限？  │
┌──────────────────┐   └────┬────┘
│ 3. 触发          │        │ 否      │ 是
│ error hook       │        ▼         ▼
└──────────────────┘   ┌──────────┐ ┌──────────────┐
        │              │ 等待延迟  │ │ 2. 记录错误  │
        ▼              │ 重新执行  │ │    放弃重试  │
┌──────────────────┐   └──────────┘ └──────────────┘
│ 4. 返回错误响应  │        │             │
└──────────────────┘        └──────┬──────┘
                                   ▼
                           ┌──────────────┐
                           │ 3. 返回错误  │
                           └──────────────┘
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│           Agent Runtime                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Executor  │  │State Mgr │  │Monitor ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│LLM       │  │Tool      │  │Skill     │
│Provider  │  │System    │  │Engine    │
└──────────┘  └──────────┘  └──────────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Hook      │  │Session   │  │Context   │
│Engine    │  │Manager   │  │Compressor│
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
用户请求
    │
    ▼
┌─────────────────────────────┐
│ CLI / Web UI                │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Session Manager             │
│ - 获取/创建会话             │
│ - 检查权限                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Agent Runtime               │
│ - 创建/获取 Agent           │
│ - 发送消息                  │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ LLM Provider    │         │ Tool System     │
│ - 调用模型      │         │ - 执行工具      │
└─────────────────┘         └─────────────────┘
        │                             │
        └────────────┬────────────────┘
                     ▼
        ┌─────────────────────────────┐
        │ Agent Runtime               │
        │ - 处理响应                  │
        │ - 更新状态                  │
        └─────────────────────────────┘
                     │
                     ▼
              返回给用户
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/agent.yaml (agent 配置合并文件)
agent:
  # Agent 运行时配置
  runtime:
    # 执行限制
    maxExecutionTime: 300
    maxToolCalls: 50
    maxLlmCalls: 20

    # 重试策略
    retry:
      maxAttempts: 3
      delay: 1000
      backoff: exponential
      retryableErrors:
        - rate_limit
        - timeout
        - connection_error

    # 超时配置
    timeout:
      llmCall: 60
      toolCall: 30

    # 流式输出
    streaming:
      enabled: true
      chunkSize: 100
```

**配置说明**:
- Agent 运行时配置已合并到 `config/agent.yaml`
- LLM 提供者配置统一在 `knight.json` 中管理
- Agent Runtime 通过 `knight-config` 库获取 LLM 配置

### 环境变量

```bash
# Agent 配置
export KNIGHT_MAX_EXECUTION_TIME=300
export KNIGHT_MAX_TOOL_CALLS=50
export KNIGHT_MAX_LLM_CALLS=20

# 重试配置
export KNIGHT_RETRY_MAX_ATTEMPTS=3
export KNIGHT_RETRY_DELAY=1000

# 超时配置
export KNIGHT_TIMEOUT_LLM_CALL=60
export KNIGHT_TIMEOUT_TOOL_CALL=30
```

### 部署考虑

1. **资源限制**: 根据服务器资源调整执行时间和调用次数限制
2. **并发控制**: 同一会话内可能存在多个 Agent，需注意资源竞争
3. **监控**: 建议记录所有 Agent 执行日志用于调试

---

## 示例

### 使用场景

#### 场景 1: 创建并发送消息给 Agent

```bash
# CLI 命令
knight chat code-reviewer

# 内部调用
agent = agent_runtime.create_agent(
    definition=agent_definition,
    session_id="abc123"
)

response = agent_runtime.send_message(
    agent_id=agent.id,
    message={
        "role": "user",
        "content": "请审查 src/main.ts 文件"
    }
)
```

#### 场景 2: 使用 Agent 变体

```bash
# CLI 命令
knight chat code-reviewer:quick

# 内部调用
agent = agent_runtime.create_agent(
    definition=agent_definition,
    session_id="abc123",
    variant="quick"
)
```

#### 场景 3: 流式响应处理

```python
# 伪代码
async for chunk in agent_runtime.send_message(
    agent_id=agent.id,
    message=user_message
):
    print(chunk.content, end="", flush=True)
```

### 配置示例

#### 开发环境

```yaml
agent:
  execution:
    max_execution_time: 600  # 较长超时便于调试
  retry:
    max_attempts: 5
  timeout:
    llm_call: 120
```

#### 生产环境

```yaml
agent:
  execution:
    max_execution_time: 300
  retry:
    max_attempts: 3
  timeout:
    llm_call: 60
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| Agent 创建延迟 | < 50ms | 内存操作 |
| 消息处理首字延迟 | < 2s | TTFB |
| 工具调用延迟 | < 100ms | 不含工具执行时间 |
| 内存占用 | < 100MB | 单 Agent 实例 |

### 错误处理

```yaml
error_codes:
  AGENT_NOT_FOUND:
    code: 404
    message: "Agent 不存在"
    action: "检查 Agent ID"

  AGENT_ALREADY_RUNNING:
    code: 409
    message: "Agent 已在运行"
    action: "使用现有 Agent 或先停止"

  EXECUTION_TIMEOUT:
    code: 408
    message: "执行超时"
    action: "增加超时时间或优化任务"

  TOOL_EXECUTION_FAILED:
    code: 500
    message: "工具执行失败"
    action: "查看工具错误详情"

  LLM_CALL_FAILED:
    code: 502
    message: "LLM 调用失败"
    action: "检查 LLM 服务状态"

  RATE_LIMIT_EXCEEDED:
    code: 429
    message: "超过速率限制"
    action: "等待后重试"
    retryable: true
```

### 测试策略

```yaml
test_plan:
  unit_tests:
    - Agent 创建/销毁
    - 状态转换
    - 消息处理
    - 工具调用

  integration_tests:
    - 端到端对话
    - 多轮工具调用
    - 错误恢复
    - 流式输出

  performance_tests:
    - 并发 Agent
    - 长时间运行
    - 内存泄漏检测

  edge_cases:
    - 空消息处理
    - 超大响应处理
    - 工具失败处理
    - LLM 超时处理
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 1.1.0 | 2026-04-01 | 修复 send_message 接口定义,分离流式和完整响应 |

