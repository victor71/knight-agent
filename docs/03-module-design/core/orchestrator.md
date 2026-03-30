# Orchestrator (编排器)

## 1. 概述

### 1.1 职责描述

Orchestrator 负责整体系统的编排和调度，包括：

- Agent 生命周期管理（创建、启动、停止、销毁）
- 任务调度和分配
- 消息路由和广播
- 资源分配和负载均衡
- Agent 间协作协调

### 1.2 设计目标

1. **高效调度**: 合理分配任务到最优 Agent
2. **资源优化**: 控制并发数量，避免资源耗尽
3. **协作支持**: 支持多 Agent 协作模式
4. **可观测性**: 完整的状态追踪和监控

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 会话信息获取 |
| Agent Runtime | 依赖 | Agent 执行 |
| Task Manager | 依赖 | 任务状态管理 |
| Event Loop | 依赖 | 事件通知 |

---

## 2. 接口定义

### 2.1 对外接口

```yaml
# Orchestrator 接口定义
Orchestrator:
  # ========== Agent 管理 ==========
  create_agent:
    description: 创建并启动 Agent 实例
    inputs:
      definition:
        type: AgentDefinition
        required: true
      session_id:
        type: string
        required: true
      variant:
        type: string
        required: false
    outputs:
      agent_id:
        type: string

  start_agent:
    description: 启动已创建的 Agent
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
      graceful:
        type: boolean
        description: 优雅停止（等待当前任务完成）
        required: false
        default: true
    outputs:
      success:
        type: boolean

  destroy_agent:
    description: 销毁 Agent 释放资源
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_agents:
    description: 列出 Agent
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

  # ========== 任务调度 ==========
  submit_task:
    description: 提交任务到调度队列
    inputs:
      task:
        type: Task
        required: true
      priority:
        type: integer
        description: 任务优先级（越小越优先）
        required: false
        default: 100
    outputs:
      task_id:
        type: string

  assign_task:
    description: 分配任务到指定 Agent
    inputs:
      task_id:
        type: string
        required: true
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  get_task_status:
    description: 获取任务状态
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      status:
        type: TaskStatus

  list_tasks:
    description: 列出任务
    inputs:
      agent_id:
        type: string
        description: 过滤 Agent
        required: false
      status:
        type: string
        description: 过滤状态
        required: false
    outputs:
      tasks:
        type: array<TaskInfo>

  cancel_task:
    description: 取消任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

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

### 2.2 数据结构

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

# 任务
Task:
  id:
    type: string
  type:
    type: enum
    values: [message, workflow, skill, custom]
  priority:
    type: integer
  payload:
    type: object
    description: 任务数据
  requirements:
    type: TaskRequirements
    description: 任务需求
  status:
    type: enum
    values: [pending, assigned, running, completed, failed, cancelled]
  assigned_agent:
    type: string | null
  created_at:
    type: datetime
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null

# 任务需求
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

# 任务状态
TaskStatus:
  task_id:
    type: string
  status:
    type: string
  assigned_agent:
    type: string | null
  progress:
    type: float
    description: 进度 0-1
  result:
    type: object | null
  error:
    type: string | null
  created_at:
    type: datetime
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

### 2.3 配置选项

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

## 3. 核心流程

### 3.1 任务调度流程

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

### 3.2 调度策略

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

### 3.3 消息路由流程

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

### 3.4 协作模式

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

## 4. 模块交互

### 4.1 依赖关系图

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

### 4.2 消息流

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

## 5. 配置与部署

### 5.1 配置文件格式

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

### 5.2 环境变量

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

### 5.3 部署考虑

1. **资源规划**: 根据服务器资源调整 Agent 和任务限制
2. **调度策略**: 生产环境建议使用 least_busy 策略
3. **消息队列**: 考虑使用持久化队列防止消息丢失

---

## 6. 示例

### 6.1 使用场景

#### 场景 1: 创建并分配任务

```python
# 伪代码
agent_id = orchestrator.create_agent(
    definition=agent_def,
    session_id="abc123"
)

task_id = orchestrator.submit_task(
    task={
        "type": "message",
        "payload": {"content": "分析代码"}
    },
    priority=10
)
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

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 任务调度延迟 | < 10ms | 内存操作 |
| 消息投递延迟 | < 5ms | 内存操作 |
| Agent 创建延迟 | < 100ms | 不含初始化 |
| 吞吐量 | > 1000 tasks/s | 单机 |

### 7.2 错误处理

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

### 7.3 测试策略

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
