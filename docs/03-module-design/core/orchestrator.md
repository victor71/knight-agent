# Orchestrator (编排器)

## 概述

### 职责描述

Orchestrator 负责**多 Agent 编排和资源管理**，包括：

- Agent 资源分配和负载均衡（为 Task Manager 分配可用 Agent）
- Agent 消息路由和广播（跨 Agent 通信）
- Agent 间协作协调（主题订阅/发布、协作组管理）
- Agent 池管理和容量控制
- **为 Task Manager 提供 Agent 分配接口**

**注意**: Orchestrator **不直接负责**单个 Agent 实例的创建/销毁。单个 Agent 的生命周期由 Agent Runtime 管理。

### 与 Agent Runtime 的职责划分

| 职责 | Agent Runtime | Orchestrator |
|------|--------------|--------------|
| 单个 Agent 创建/销毁 | ✅ 负责 | ❌ 不负责 |
| 单个 Agent 状态管理 | ✅ 负责 | ❌ 仅监控 |
| Agent 池管理 | ❌ 不负责 | ✅ 负责 |
| Agent 资源分配 | ❌ 不负责 | ✅ 负责 |
| 跨 Agent 消息路由 | ❌ 不负责 | ✅ 负责 |
| Agent 协作模式 | ❌ 不负责 | ✅ 负责 |

### 设计目标

1. **高效调度**: 管理并发 Agent 数量，优化资源使用
2. **消息可靠**: 确保消息正确路由到目标 Agent
3. **协作支持**: 支持多 Agent 协作模式（主从、流水线、投票）
4. **可观测性**: 完整的 Agent 状态追踪和监控

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 会话信息获取 |
| Agent Runtime | 依赖 | Agent 执行 |
| Event Loop | 依赖 | 事件通知 |
| Hook Engine | 协作 | Agent 生命周期 Hook |
| Task Manager | 协作 | 使用 Task Manager 的 Task 数据结构定义 |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| Task Manager | 被依赖 | 调用 Agent 分配接口 |
| External Agent | 被依赖 | 外部 Agent 通过 Agent Runtime 注册到 Orchestrator 池 |

**注意**:
- Orchestrator 不定义自己的 Task 数据结构，Task 相关类型由 Task Manager 统一定义。
- 外部 Agent (external 类型) 通过 Agent Runtime 创建并注册到 Orchestrator 池中。

---

## 接口定义

### 对外接口

```yaml
# Orchestrator 接口定义
Orchestrator:
  # ========== Agent 池管理 ==========
  register_agent:
    description: 注册已创建的 Agent 到池中（由 Agent Runtime 调用）
    inputs:
      agent_id:
        type: string
        required: true
      session_id:
        type: string
        required: true
      capabilities:
        type: array<string>
        description: Agent 能力列表
        required: false
    outputs:
      success:
        type: boolean

  unregister_agent:
    description: 从池中注销 Agent（由 Agent Runtime 调用）
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_agents:
    description: 列出 Agent（监控用）
    inputs:
      session_id:
        type: string
        description: 过滤会话
        required: false
      status:
        type: string
        description: 过滤状态
        required: false
    outputs:
      agents:
        type: array<AgentInfo>

  get_agent_info:
    description: 获取 Agent 详细信息
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      info:
        type: AgentInfo

  # ========== Agent 分配 (供 Task Manager 调用) ==========
  allocate_agent:
    description: 为任务分配可用的 Agent
    inputs:
      task_requirements:
        type: TaskRequirements
        description: 任务对 Agent 的要求（能力、负载等）
        required: true
    outputs:
      agent_id:
        type: string
        description: 分配的 Agent ID

  get_available_agents:
    description: 获取可用 Agent 列表
    inputs:
      filter:
        type: AgentFilter
        description: Agent 过滤条件（能力、状态等）
        required: false
    outputs:
      agents:
        type: array<AgentInfo>

  # ========== 消息路由 ==========
  send_message:
    description: 发送消息到指定 Agent
    inputs:
      to:
        type: string
        description: Agent ID
        required: true
      message:
        type: Message
        required: true
    outputs:
      success:
        type: boolean

  broadcast:
    description: 广播消息到多个 Agent
    inputs:
      recipients:
        type: array<string>
        description: Agent ID 列表
        required: true
      message:
        type: Message
        required: true
    outputs:
      results:
        type: array<SendResult>

  publish:
    description: 发布消息到主题
    inputs:
      topic:
        type: string
        required: true
      message:
        type: Message
        required: true
    outputs:
      delivered_count:
        type: integer

  subscribe:
    description: 订阅主题
    inputs:
      agent_id:
        type: string
        required: true
      topic:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  unsubscribe:
    description: 取消订阅
    inputs:
      agent_id:
        type: string
        required: true
      topic:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 资源管理 ==========
  get_resource_usage:
    description: 获取资源使用情况
    outputs:
      usage:
        type: ResourceUsage

  set_resource_limit:
    description: 设置资源限制
    inputs:
      resource_type:
        type: string
        required: true
      limit:
        type: integer
        required: true
    outputs:
      success:
        type: boolean

  # ========== 协作模式 ==========
  create_collaboration:
    description: 创建协作组
    inputs:
      name:
        type: string
        required: true
      agents:
        type: array<string>
        required: true
      mode:
        type: string
        description: 协作模式 (master-worker/pipeline/voting)
        required: false
        default: "master-worker"
    outputs:
      collaboration_id:
        type: string

  dissolve_collaboration:
    description: 解散协作组
    inputs:
      collaboration_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
```

### 数据结构

```yaml
# Agent 信息
AgentInfo:
  id:
    type: string
  name:
    type: string
  definition_id:
    type: string
  session_id:
    type: string
  variant:
    type: string | null
  status:
    type: enum
    values: [idle, busy, paused, error, stopped]
  current_task:
    type: string | null
  statistics:
    type: AgentStatistics
  created_at:
    type: datetime
  last_active_at:
    type: datetime

# 任务分配需求 (用于 allocate_agent 接口)
# 注意: 完整的 Task 数据结构定义见 Task Manager 模块
TaskRequirements:
  agent_type:
    type: string | null
    description: 需要的 Agent 类型
  capabilities:
    type: array<string>
    description: 需要的能力
  min_memory:
    type: integer
    description: 最小内存（MB）
  max_duration:
    type: integer
    description: 最大执行时间（秒）
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null

# 资源使用
ResourceUsage:
  total_agents:
    type: integer
  active_agents:
    type: integer
  pending_tasks:
    type: integer
  running_tasks:
    type: integer
  memory_mb:
    type: integer
  cpu_percent:
    type: float

# 协作组
Collaboration:
  id:
    type: string
  name:
    type: string
  agents:
    type: array<string>
  mode:
    type: string
  master:
    type: string | null
    description: 主 Agent（master-worker 模式）
  pipeline:
    type: array<string>
    description: 流水线顺序（pipeline 模式）
  created_at:
    type: datetime

# 发送结果
SendResult:
  agent_id:
    type: string
  success:
    type: boolean
  error:
    type: string | null
```

### 配置选项

```yaml
# config/orchestrator.yaml
orchestrator:
  # Agent 限制
  limits:
    max_agents: 50
    max_agents_per_session: 10
    max_concurrent_tasks: 100

  # 调度策略
  scheduling:
    strategy: round_robin    # round_robin/least_busy/priority/custom
    queue_size: 1000
    timeout: 300

  # 资源限制
  resources:
    max_memory_mb: 4096
    max_cpu_percent: 80
    max_task_duration: 3600

  # 消息配置
  messaging:
    max_message_size: 10485760    # 10MB
    message_ttl: 3600             # 消息存活时间
    topic_retention: 1000         # 主题保留消息数
```

---

## 核心流程

### 任务调度流程

```
任务提交
        │
        ▼
┌──────────────────────────────┐
│ 1. 任务验证                  │
│    - 检查任务格式            │
│    - 检查需求                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 加入队列                  │
│    - 根据优先级排序          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 选择 Agent                │
│    - 根据调度策略            │
│    - 检查 Agent 可用性        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 有可用 │
    │ Agent？│
    └───┬────┘
        │ 否
        ▼
    等待可用 Agent
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 分配任务                  │
│    - 更新任务状态            │
│    - 通知 Agent              │
└──────────────────────────────┘
        │
        ▼
    监控执行
        │
        ▼
┌──────────────────────────────┐
│ 5. 任务完成                  │
│    - 更新状态                │
│    - 释放资源                │
└──────────────────────────────┘
```

### 调度策略

#### Round Robin (轮询)

```
Agent Pool: [A1, A2, A3]
Task Queue:  [T1, T2, T3, T4]

T1 → A1
T2 → A2
T3 → A3
T4 → A1  (循环)
```

#### Least Busy (最闲优先)

```
Agent Pool: [A1(busy:3), A2(busy:1), A3(busy:2)]
Task: T

选择 A2 (最闲)
```

#### Priority (优先级)

```
Task Queue (priority):
  T1 (10)  ← 高优先级
  T2 (50)
  T3 (100) ← 低优先级

先执行 T1，再 T2，再 T3
```

### 消息路由流程

```
发送消息
        │
        ▼
┌──────────────────────────────┐
│ 1. 解析目标                  │
│    - Agent ID                │
│    - 主题订阅                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 查找目标                  │
│    - Agent 存在性检查        │
│    - 主题订阅者查找          │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 找到？  │
    └───┬────┘
        │ 否
        ▼
    返回目标不存在
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 投递消息                  │
│    - 单播/广播/发布          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 确认投递                  │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 协作模式

#### Master-Worker (主从模式)

```
Master Agent
    │
    ├──→ Worker 1: 任务 A
    ├──→ Worker 2: 任务 B
    ├──→ Worker 3: 任务 C
    │
    └──→ 汇总结果
```

#### Pipeline (流水线模式)

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│ Spec    │───→│ Design  │───→│ Code    │───→│ Test    │
│ Agent   │    │ Agent   │    │ Agent   │    │ Agent   │
└─────────┘    └─────────┘    └─────────┘    └─────────┘
     │                              │
     └──────────── 输出 ─────────────┘
```

#### Voting (投票模式)

```
┌────────────────────────────────┐
│         Agent A                │
│         结果: Yes              │
└────────────────────────────────┘
            │
┌────────────────────────────────┤        ┌────────────────────────────────┐
│         Agent B                │←───────│         Agent C                │
│         结果: No               │        │         结果: Yes              │
└────────────────────────────────┘        └────────────────────────────────┘
            │                                       │
            └──────────────┬────────────────────────┘
                           ▼
                    ┌─────────────┐
                    │  投票统计   │
                    │  Yes: 2     │
                    │  No: 1      │
                    └─────────────┘
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│           Orchestrator                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Scheduler │  │Router    │  │Collab  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Agent     │  │Task      │  │Event     │
│Runtime   │  │Manager   │  │Loop      │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
用户请求
    │
    ▼
┌─────────────────────────────┐
│ Orchestrator                │
│ - 接收请求                   │
│ - 查找/创建 Agent            │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Scheduler                   │
│ - 分配任务                   │
│ - 选择最优 Agent             │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Agent Runtime               │
│ - 执行任务                   │
│ - 返回结果                   │
└─────────────────────────────┘
        │
        ▼
    返回结果
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/orchestrator.yaml
orchestrator:
  # Agent 限制
  limits:
    max_agents: 50
    max_agents_per_session: 10
    max_concurrent_tasks: 100

  # 调度策略
  scheduling:
    strategy: round_robin
    queue_size: 1000
    timeout: 300

  # 资源限制
  resources:
    max_memory_mb: 4096
    max_cpu_percent: 80
    max_task_duration: 3600

  # 消息配置
  messaging:
    max_message_size: 10485760
    message_ttl: 3600
    topic_retention: 1000

  # 协作配置
  collaboration:
    enabled: true
    max_group_size: 10
    default_timeout: 600
```

### 环境变量

```bash
# 调度配置
export KNIGHT_SCHEDULING_STRATEGY="round_robin"
export KNIGHT_QUEUE_SIZE=1000

# 资源限制
export KNIGHT_MAX_AGENTS=50
export KNIGHT_MAX_CONCURRENT_TASKS=100

# 消息配置
export KNIGHT_MAX_MESSAGE_SIZE=10485760
export KNIGHT_MESSAGE_TTL=3600
```

### 部署考虑

1. **资源规划**: 根据服务器资源调整 Agent 和任务限制
2. **调度策略**: 生产环境建议使用 least_busy 策略
3. **消息队列**: 考虑使用持久化队列防止消息丢失

---

## 示例

### 使用场景

#### 场景 1: 创建 Agent 并分配给任务

```python
# 伪代码 - 由 Task Manager 调用
agent_id = orchestrator.allocate_agent(
    task_requirements={
        "agent_type": "coder",
        "capabilities": ["code_analysis", "writing"],
        "max_duration": 300
    }
)

# Agent Runtime 创建 Agent 后会自动注册
# agent_runtime.create_agent() 内部调用 orchestrator.register_agent()
```

#### 场景 2: 创建协作组

```python
# 伪代码
collab_id = orchestrator.create_collaboration(
    name="code-review",
    agents=["agent-1", "agent-2", "agent-3"],
    mode="master-worker"
)
```

#### 场景 3: 发布订阅消息

```python
# 伪代码
# Agent 订阅主题
orchestrator.subscribe("agent-1", topic="code-changes")

# 发布消息到主题
orchestrator.publish(
    topic="code-changes",
    message={"file": "main.ts", "change": "added function"}
)
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 任务调度延迟 | < 10ms | 内存操作 |
| 消息投递延迟 | < 5ms | 内存操作 |
| Agent 创建延迟 | < 100ms | 不含初始化 |
| 吞吐量 | > 1000 tasks/s | 单机 |

### 错误处理

```yaml
error_codes:
  AGENT_NOT_FOUND:
    code: 404
    message: "Agent 不存在"
    action: "检查 Agent ID"

  NO_AVAILABLE_AGENT:
    code: 503
    message: "没有可用 Agent"
    action: "等待或创建新 Agent"

  TASK_FAILED:
    code: 500
    message: "任务执行失败"
    action: "查看错误详情"

  RESOURCE_LIMIT_EXCEEDED:
    code: 429
    message: "超过资源限制"
    action: "等待或增加资源"
```

### 测试策略

```yaml
test_plan:
  unit_tests:
    - Agent 创建/销毁
    - 任务调度
    - 消息路由
    - 资源管理

  integration_tests:
    - 多 Agent 协作
    - 高并发调度
    - 消息广播
    - 故障恢复

  performance_tests:
    - 大规模 Agent 管理
    - 高吞吐任务调度
    - 内存使用优化
```
