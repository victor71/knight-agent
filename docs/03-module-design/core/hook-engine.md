# Hook Engine (Hook 引擎)

## 概述

### 职责描述

Hook Engine 负责管理系统扩展钩子，允许在关键事件点注入自定义逻辑：

- Hook 注册和生命周期管理
- 事件触发和 Hook 执行
- 优先级排序和链式调用
- 阻断、修改和替换能力
- 多种 Hook 处理器类型

### 设计目标

1. **灵活扩展**: 支持命令、技能、RPC 等多种处理器
2. **精细控制**: 支持阻断、修改、跳过等控制能力
3. **高性能**: 最小化对主流程的性能影响
4. **安全隔离**: Hook 执行失败不影响主流程

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Event Loop | 依赖 | 事件分发。见 [Event Loop 接口](./event-loop.md) |
| Skill Engine | 可选 | Skill 类型 Hook。见 [Skill Engine 接口](../agent/skill-engine.md) |
| Session Manager | 依赖 | 会话上下文。见 [Session Manager 接口](./session-manager.md) |

---

## 接口定义

### 对外接口

```yaml
# Hook Engine 接口定义
HookEngine:
  # ========== Hook 管理 ==========
  register_hook:
    description: 注册 Hook
    inputs:
      hook:
        type: HookDefinition
        required: true
    outputs:
      hook_id:
        type: string

  unregister_hook:
    description: 注销 Hook
    inputs:
      hook_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_hooks:
    description: 列出 Hook
    inputs:
      event:
        type: string
        description: 事件过滤
        required: false
      phase:
        type: string
        description: 阶段过滤 (before/after/replace)
        required: false
    outputs:
      hooks:
        type: array<HookInfo>

  get_hook:
    description: 获取 Hook 详情
    inputs:
      hook_id:
        type: string
        required: true
    outputs:
      hook:
        type: HookDefinition | null

  enable_hook:
    description: 启用 Hook
    inputs:
      hook_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  disable_hook:
    description: 禁用 Hook
    inputs:
      hook_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== Hook 执行 ==========
  trigger_hooks:
    description: 触发 Hook 执行
    inputs:
      event:
        type: string
        required: true
      phase:
        type: string
        required: true
      context:
        type: HookContext
        required: true
    outputs:
      result:
        type: HookResult

  # ========== Hook 组管理 ==========
  create_hook_group:
    description: 创建 Hook 组（批量管理）
    inputs:
      name:
        type: string
        required: true
      description:
        type: string
        required: false
    outputs:
      group_id:
        type: string

  add_to_group:
    description: 添加 Hook 到组
    inputs:
      group_id:
        type: string
        required: true
      hook_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  remove_from_group:
    description: 从组移除 Hook
    inputs:
      group_id:
        type: string
        required: true
      hook_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  enable_group:
    description: 启用 Hook 组
    inputs:
      group_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  disable_group:
    description: 禁用 Hook 组
    inputs:
      group_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 事件点管理 ==========
  list_events:
    description: 列出所有可用事件点
    outputs:
      events:
        type: array<EventPoint>

  get_event_info:
    description: 获取事件点详情
    inputs:
      event:
        type: string
        required: true
    outputs:
      info:
        type: EventPoint
```

### 数据结构

```yaml
# Hook 定义
HookDefinition:
  id:
    type: string
  name:
    type: string
  description:
    type: string
  enabled:
    type: boolean
    default: true

  # 触发配置
  event:
    type: string
    description: 监听的事件
  phase:
    type: enum
    values: [before, after, replace]
    description: 执行阶段
  priority:
    type: integer
    description: 优先级（越小越先执行）
    default: 100

  # 过滤条件
  filter:
    type: HookFilter
    description: 触发条件过滤

  # 处理器
  handler:
    type: HookHandler
    description: Hook 处理器

  # 控制能力
  control:
    type: HookControl
    description: 控制选项

  # 错误处理
  error_handling:
    type: HookErrorHandling
    description: 错误处理策略

# Hook 过滤器
HookFilter:
  agent:
    type: string | null
    description: Agent ID 过滤
  session:
    type: string | null
    description: 会话 ID 过滤
  tool:
    type: string | null
    description: 工具名过滤
  custom:
    type: map<string, any>
    description: 自定义过滤条件

# Hook 处理器
HookHandler:
  type:
    type: enum
    values: [command, skill, rpc, wasm, callback]
    description: 处理器类型

  # 命令处理器
  command:
    type: object
    properties:
      executable:
        type: string
        description: 可执行文件路径
      args:
        type: array<string>
        description: 命令参数
      env:
        type: map<string, string>
        description: 环境变量

  # 技能处理器
  skill:
    type: object
    properties:
      skill_id:
        type: string
      args:
        type: map<string, any>

  # RPC 处理器
  rpc:
    type: object
    properties:
      endpoint:
        type: string
        description: RPC 端点 URL
      method:
        type: string
        description: RPC 方法名
      timeout:
        type: integer
        description: 超时时间（秒）

  # WASM 处理器
  wasm:
    type: object
    properties:
      module:
        type: string
        description: WASM 模块路径
      function:
        type: string
        description: 导出函数名

  # 回调处理器
  callback:
    type: function
    description: 直接回调函数（内部使用）

# Hook 控制选项
HookControl:
  can_block:
    type: boolean
    description: 可以阻断操作
    default: false
  can_modify:
    type: boolean
    description: 可以修改数据
    default: false
  can_skip:
    type: boolean
    description: 可以跳过后续 Hook
    default: false
  continue_on_error:
    type: boolean
    description: 错误时继续执行
    default: false

# Hook 错误处理
HookErrorHandling:
  retry:
    type: boolean
    default: false
  max_retries:
    type: integer
    default: 3
  retry_delay:
    type: integer
    default: 1000
  fallback:
    type: string | null
    description: 降级处理

# Hook 上下文
HookContext:
  event:
    type: string
  phase:
    type: string
  timestamp:
    type: datetime

  session:
    type: object
    properties:
      id:
        type: string
      workspace:
        type: string
      variables:
        type: map<string, any>

  agent:
    type: object
    properties:
      id:
        type: string
      name:
        type: string
      state:
        type: string

  request:
    type: object
    properties:
      method:
        type: string
      params:
        type: object
      headers:
        type: map<string, string>

  response:
    type: object | null
    properties:
      data:
        type: any
      error:
        type: string | null
      duration_ms:
        type: integer

  # 控制配置（来自 HookDefinition.control）
  # 运行时控制能力由 HookEngine 内部实现，不在接口中暴露
  control:
    type: HookControl
    description: Hook 的控制权限配置（哪些操作是允许的）

# Hook 控制接口（运行时实现）
# 注意：这是运行时接口，不是序列化接口。实现时由 HookEngine 提供具体函数
HookControlRuntime:
  block:
    type: function
    description: 阻断操作 block(reason: string)
  modify:
    type: function
    description: 修改数据 modify(data: any)
  skip:
    type: function
    description: 跳过后续 Hook

**实现说明**：
- `HookControl` 是可序列化的配置，定义在 `HookDefinition.control` 中
- `HookControlRuntime` 是运行时接口，由 HookEngine 在触发 Hook 时提供
- Hook 处理器通过 `HookContext.control` 获取控制能力，但控制能力本身由 HookDefinition.control 的值决定

# Hook 执行结果
HookResult:
  blocked:
    type: boolean
  block_reason:
    type: string | null
  modified:
    type: boolean
  modified_data:
    type: any
  skipped:
    type: boolean
  hooks_executed:
    type: integer
  hooks_failed:
    type: integer
  duration_ms:
    type: integer

# Hook 信息
HookInfo:
  id:
    type: string
  name:
    type: string
  event:
    type: string
  phase:
    type: string
  priority:
    type: integer
  enabled:
    type: boolean
  execution_count:
    type: integer
  last_executed:
    type: datetime | null

# 事件点
EventPoint:
  name:
    type: string
  category:
    type: string
    description: 事件类别
  phases:
    type: array<string>
    description: 支持的阶段
  context:
    type: object
    description: 上下文结构
```

### 配置选项

```yaml
# config/hooks.yaml
hooks:
  # Hook 目录
  directories:
    - "./hooks"
    - "~/.knight-agent/hooks"

  # 执行配置
  execution:
    timeout: 30
    max_concurrent: 10
    parallel: false

  # 错误处理
  error_handling:
    log_errors: true
    continue_on_error: false
    default_retry: 3

  # 内置 Hooks
  builtin:
    audit_log:
      enabled: true
    sensitive_confirm:
      enabled: true
```

---

## 核心流程

### Hook 执行流程

```
触发事件
        │
        ▼
┌──────────────────────────────┐
│ 1. 查找匹配的 Hook           │
│    - 按 event 过滤           │
│    - 按 phase 过滤            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 应用过滤条件              │
│    - agent/session/tool      │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 按优先级排序              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 依次执行 Hook             │
│    for hook in hooks:        │
│      - 执行处理器            │
│      - 检查控制信号          │
│      - 处理错误              │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 阻断？  │
    └───┬────┘
        │ 是
        ▼
    停止执行，返回阻断
        │ 否
        ▼
    ┌───┴────┐
    │ 跳过？  │
    └───┬────┘
        │ 是
        ▼
    跳过后续 Hook
        │ 否
        ▼
    完成
```

### Hook 阶段

```
原始操作流程
    │
    ▼
┌─────────────────────────────────────────┐
│ Before Phase (priority: 1 → N)          │
│ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────┐│
│ │Hook 1│→│Hook 2│→│Hook N│→│检查阻断  ││
│ └──────┘ └──────┘ └──────┘ │          ││
│                            └────┬─────┘│
│                                 │       │
│                       ┌─────────┴───────┤
│                       │ 未被阻断        │
│                       ▼                 │
│                ┌─────────────┐          │
│                │ 执行原始操作 │          │
│                └─────────────┘          │
│                       │                 │
│                       ▼                 │
│ ┌────────────────────────────────────┐ │
│ │ After Phase (priority: 1 → N)      │ │
│ │ ┌──────┐ ┌──────┐ ┌──────┐         │ │
│ │ │Hook 1│→│Hook 2│→│Hook N│         │ │
│ │ └──────┘ └──────┘ └──────┘         │ │
│ └────────────────────────────────────┘ │
│                       │                 │
│                       ▼                 │
│                   返回结果              │
└─────────────────────────────────────────┘
```

### Hook 处理器类型

#### 命令处理器

```yaml
handler:
  type: command
  command:
    executable: "./hooks/confirm.sh"
    args: ["{{ context.event }}", "{{ context.request.tool }}"]
    env:
      SESSION_ID: "{{ context.session.id }}"
```

#### 技能处理器

```yaml
handler:
  type: skill
  skill:
    skill_id: "audit-logger"
    args:
      event: "{{ context.event }}"
      data: "{{ context }}"
```

#### RPC 处理器

```yaml
handler:
  type: rpc
  rpc:
    endpoint: "http://localhost:8080/hooks/validate"
    method: "POST"
    timeout: 5
```

---

## Hook 事件点

### 事件类型注册表

以下为系统所有 Hook 事件点的完整注册表：

```yaml
# 事件注册表
EventRegistry:
  # ========== Agent 事件 ==========
  agent:
    category: agent
    description: Agent 生命周期事件
    events:
      - name: agent_create
        phase: [before]
        can_block: true
        description: Agent 创建前
      - name: agent_created
        phase: [after]
        can_block: false
        description: Agent 创建后
      - name: agent_execute
        phase: [before]
        can_block: true
        description: Agent 执行前
      - name: agent_executed
        phase: [after]
        can_block: false
        description: Agent 执行后
      - name: agent_error
        phase: [after]
        can_block: false
        description: Agent 执行错误
      - name: agent_destroy
        phase: [before]
        can_block: true
        description: Agent 销毁前

  # ========== Session 事件 ==========
  session:
    category: session
    description: 会话生命周期事件
    events:
      - name: session_create
        phase: [before]
        can_block: true
        description: 会话创建前
      - name: session_created
        phase: [after]
        can_block: false
        description: 会话创建后
      - name: session_switch
        phase: [after]
        can_block: false
        description: 会话切换后
      - name: session_close
        phase: [before]
        can_block: true
        description: 会话关闭前
      - name: context_compress
        phase: [before]
        can_block: true
        description: 上下文压缩前
      - name: context_compressed
        phase: [after]
        can_block: false
        description: 上下文压缩后

  # ========== Tool 事件 ==========
  tool:
    category: tool
    description: 工具调用事件
    events:
      - name: tool_call
        phase: [before]
        can_block: true
        description: 工具调用前
      - name: tool_result
        phase: [after]
        can_block: false
        description: 工具返回后
      - name: file_access
        phase: [before]
        can_block: true
        description: 文件访问前
      - name: command_execute
        phase: [before]
        can_block: true
        description: 命令执行前

  # ========== LLM 事件 ==========
  llm:
    category: llm
    description: LLM 调用事件
    events:
      - name: llm_request
        phase: [before]
        can_block: true
        description: LLM 请求前
      - name: llm_response
        phase: [after]
        can_block: false
        description: LLM 响应后
      - name: prompt_build
        phase: [before]
        can_block: true
        can_modify: true
        description: Prompt 构建时（可修改）

  # ========== Message 事件 ==========
  message:
    category: message
    description: 消息事件
    events:
      - name: message_send
        phase: [before]
        can_block: true
        description: 消息发送前
      - name: message_received
        phase: [after]
        can_block: false
        description: 消息接收后
      - name: message_modify
        phase: [before]
        can_block: true
        can_modify: true
        description: 消息修改时
```

### Agent 生命周期事件

```yaml
agent:
  - agent_create          # Agent 创建前
  - agent_created         # Agent 创建后
  - agent_execute         # Agent 执行前
  - agent_executed        # Agent 执行后
  - agent_error           # Agent 错误
  - agent_destroy         # Agent 销毁前
```

### 会话生命周期事件

```yaml
session:
  - session_create        # 会话创建前
  - session_created       # 会话创建后
  - session_switch        # 会话切换
  - session_close         # 会话关闭前
  - context_compress      # 上下文压缩前
  - context_compressed    # 上下文压缩后
```

### 工具调用事件

```yaml
tool:
  - tool_call             # 工具调用前（可阻断）
  - tool_result           # 工具返回后
  - file_access           # 文件访问（可阻断）
  - command_execute       # 命令执行（可阻断）
```

### LLM 调用事件

```yaml
llm:
  - llm_request           # LLM 请求前
  - llm_response          # LLM 响应后
  - prompt_build          # Prompt 构建时（可修改）
```

### 消息事件

```yaml
message:
  - message_send          # 消息发送前
  - message_received      # 消息接收后
  - message_modify        # 消息修改时
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Hook Engine                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Registry  │  │Executor  │  │Control ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Skill     │  │Command   │  │RPC       │
│Engine    │  │Executor  │  │Client    │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
主流程
    │
    ▼
┌─────────────────────────────┐
│ Hook Engine                │
│ - 触发 before hooks         │
│ - 执行原始操作              │
│ - 触发 after hooks          │
└─────────────────────────────┘
        │
        ▼
    返回结果
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/hooks.yaml
hooks:
  # Hook 目录
  directories:
    - "./hooks"
    - "~/.knight-agent/hooks"

  # 执行配置
  execution:
    timeout: 30
    max_concurrent: 10
    parallel: false

  # 错误处理
  error_handling:
    log_errors: true
    continue_on_error: false
    default_retry: 3

  # 内置 Hooks
  builtin:
    audit_log:
      enabled: true
    sensitive_confirm:
      enabled: true
```

### 环境变量

```bash
# Hook 目录
export KNIGHT_HOOK_DIRS="./hooks:~/.knight-agent/hooks"

# 执行配置
export KNIGHT_HOOK_TIMEOUT=30
export KNIGHT_HOOK_MAX_CONCURRENT=10
```

---

## 示例

### 敏感操作确认 Hook

```yaml
# hooks/sensitive-confirm.yaml
name: sensitive_confirm
event: tool_call
phase: before
priority: 100
filter:
  tool: "delete|rm|reset|format"
handler:
  type: command
  command:
    executable: "./hooks/confirm.sh"
control:
  can_block: true
  continue_on_error: false
```

### 审计日志 Hook

```yaml
# hooks/audit-log.yaml
name: audit_log
event: tool_call
phase: after
priority: 999
filter:
  tool: "*"  # 所有工具
handler:
  type: skill
  skill:
    skill_id: audit-logger
    args:
      log_to_file: true
control:
  continue_on_error: true
error_handling:
  continue_on_error: true
```

### Prompt 修改 Hook

```yaml
# hooks/prompt-modifier.yaml
name: prompt_modifier
event: prompt_build
phase: before
priority: 50
handler:
  type: command
  command:
    executable: "./hooks/modify-prompt.sh"
control:
  can_modify: true
```

### Prompt 修改流程

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Runtime                             │
│                                                             │
│  1. 准备发送消息到 LLM                                      │
│  2. 构建 Prompt 初始版本                                    │
│     - system_prompt                                        │
│     - user_messages                                        │
│     - context                                             │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 触发 prompt_build Hook
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Hook Engine                               │
│                                                             │
│  1. 匹配 prompt_build 事件的 Hooks                          │
│  2. 按 priority 顺序执行 before 阶段                        │
│  3. 每个 Hook 可以修改 prompt 内容                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 传递 Hook 上下文
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                  Hook Handler (prompt_modifier)              │
│                                                             │
│  输入上下文:                                                │
│  - event: "prompt_build"                                   │
│  - phase: "before"                                         │
│  - context.prompt: {原始 prompt}                           │
│  - control: {modify, block, skip} 函数                      │
│                                                             │
│  Hook 处理:                                                 │
│  1. 读取原始 prompt                                         │
│  2. 应用修改逻辑                                            │
│     - 添加系统提示                                          │
│     - 修改用户消息                                          │
│     - 注入额外上下文                                        │
│  3. 调用 control.modify(modified_prompt)                   │
│                                                             │
│  输出结果:                                                   │
│  - modified: true                                          │
│  - modified_data: {修改后的 prompt}                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 返回修改结果
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Hook Engine                               │
│                                                             │
│  1. 收集所有 Hook 的修改结果                                │
│  2. 检查是否有 Hook 阻断操作                               │
│  3. 合并所有修改(按优先级顺序应用)                          │
│  4. 返回最终 prompt                                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 返回最终 prompt
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Agent Runtime                             │
│                                                             │
│  1. 检查 Hook 执行结果                                      │
│  2. 如果被阻断: 取消 LLM 调用                               │
│  3. 如果被修改: 使用修改后的 prompt                         │
│  4. 调用 LLM Provider                                       │
│                                                             │
│  LLM Provider 调用:                                         │
│  llm_provider.send_message(final_prompt)                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 发送到 LLM
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    LLM Provider                              │
│                                                             │
│  1. 接收最终 prompt                                         │
│  2. 发送到 LLM API                                          │
│  3. 返回响应                                               │
└─────────────────────────────────────────────────────────────┘
```

### Prompt 修改数据流

```yaml
# Hook 接收的上下文
PromptBuildHookContext:
  event:
    type: string
    value: "prompt_build"
  phase:
    type: string
    value: "before"
  prompt:
    type: object
    properties:
      system:
        type: string
        description: "系统提示词"
      messages:
        type: array
        description: "用户消息列表"
      context:
        type: object
        description: "额外上下文"
  control:
    type: object
    properties:
      modify:
        type: function
        description: "修改 prompt"
      block:
        type: function
        description: "阻断操作"

# Hook 返回的修改结果
PromptBuildHookResult:
  modified:
    type: boolean
    description: "是否修改了 prompt"
  modified_data:
    type: object
    properties:
      system:
        type: string
        description: "修改后的系统提示词"
      messages:
        type: array
        description: "修改后的消息列表"
      context:
        type: object
        description: "修改后的上下文"
  blocked:
    type: boolean
    description: "是否阻断操作"
  block_reason:
    type: string
    description: "阻断原因"

# Prompt 修改示例
prompt_modification_example:
  original:
    system: "You are a code reviewer."
    messages:
      - role: "user"
        content: "Review this code."

  modified_by_hook:
    system: "You are a senior code reviewer with 10 years of experience. Focus on security, performance, and maintainability."
    messages:
      - role: "user"
        content: "Review this code for security issues, performance bottlenecks, and code quality."
      - role: "system"
        content: "Additional context: This is a production API endpoint that handles user authentication."
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| Hook 匹配 | < 1ms | 内存查找 |
| 命令执行 | < 5s | 默认超时 |
| RPC 调用 | < 1s | 网络调用 |
| 技能执行 | < 10s | 复杂技能 |

### 错误处理

```yaml
error_codes:
  HOOK_NOT_FOUND:
    code: 404
    message: "Hook 不存在"
    action: "检查 Hook ID"

  HOOK_EXECUTION_FAILED:
    code: 500
    message: "Hook 执行失败"
    action: "查看错误详情"

  HOOK_TIMEOUT:
    code: 408
    message: "Hook 执行超时"
    action: "增加超时时间"

  HOOK_BLOCKED:
    code: 403
    message: "操作被 Hook 阻断"
    action: "查看阻断原因"
```

### 内置 Hooks

| Hook | 事件 | 描述 |
|------|------|------|
| audit_log | tool_call | 记录所有工具调用 |
| sensitive_confirm | tool_call | 敏感操作确认 |
| prompt_modifier | prompt_build | 修改 Prompt |
| context_logger | message_send | 记录消息上下文 |
| error_notifier | agent_error | Agent 错误通知 |

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 1.1.0 | 2026-04-01 | 添加 prompt_build 事件详细流程和数据结构说明 |

