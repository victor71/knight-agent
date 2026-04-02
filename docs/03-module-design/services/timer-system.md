# Timer System (定时器系统)

## 概述

### 职责描述

Timer System 负责管理 Knight-Agent 的所有定时任务和调度功能,包括:

- 一次性定时器 (延迟执行)
- 周期性定时器 (间隔执行)
- Cron 定时器 (复杂调度规则)
- 定时器生命周期管理 (创建、取消、暂停、恢复)
- 定时器持久化 (跨重启保持)
- 定时器回调触发 (Hook/Skill/Callback)
- 定时器统计和监控

### 设计目标

1. **精确调度**: 毫秒级调度精度
2. **高并发**: 支持大量同时运行的定时器
3. **可靠性**: 定时器不丢失,跨重启恢复
4. **灵活性**: 支持多种调度模式
5. **可观测**: 完整的定时器状态追踪

### 核心需求

| 需求 | 描述 | 优先级 |
|------|------|--------|
| **一次性定时器** | 延迟指定时间后执行一次 | P0 |
| **周期性定时器** | 按固定间隔重复执行 | P0 |
| **Cron 定时器** | 支持标准 Cron 表达式 | P1 |
| **定时器持久化** | 重启后恢复定时器 | P1 |
| **定时器回调** | 支持 Hook/Skill/Callback 触发 | P1 |

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Event Loop | 协作 | 定时器事件通过 Event Loop 分发 |
| Hook Engine | 依赖 | 触发 Hook 回调 |
| Skill Engine | 依赖 | 触发 Skill 执行 |
| Storage Service | 依赖 | 定时器持久化 |

**被依赖模块**:

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| Event Loop | 被依赖 | Timer System 向 Event Loop 发送 timer_triggered 事件 |
| Tool System | 被依赖 | 通过工具调用 Timer System |
| Agent Runtime | 被依赖 | LLM 通过工具创建定时器 |

**架构说明**:
- Timer System 独立运行,有自己的调度循环
- **定时器到期时,Timer System 向 Event Loop 发送 `timer_triggered` 事件**
- Event Loop 将事件分发到注册的监听器(Hook/Skill/Callback)
- Event Loop **不管理**定时任务调度,仅负责事件分发
- 避免循环依赖:Timer System → Event Loop → Hook/Skill Engine
- 自然语言处理由 Agent Runtime + LLM 完成,通过 Tool System 调用 Timer System

---

## 接口定义

### 对外接口

```yaml
# Timer System 接口定义
TimerSystem:
  # ========== 定时器创建 ==========
  create_timer:
    description: 创建一次性定时器
    inputs:
      name:
        type: string
        description: 定时器名称
        required: false
      delay_ms:
        type: integer
        description: 延迟时间(毫秒)
        required: true
      callback:
        type: TimerCallback
        description: 回调函数/钩子/技能
        required: true
      metadata:
        type: object
        description: 元数据
        required: false
      persistent:
        type: boolean
        description: 是否持久化
        required: false
        default: false
    outputs:
      timer_id:
        type: string
        description: 定时器唯一标识

  create_interval:
    description: 创建周期性定时器
    inputs:
      name:
        type: string
        description: 定时器名称
        required: false
      interval_ms:
        type: integer
        description: 执行间隔(毫秒)
        required: true
      callback:
        type: TimerCallback
        description: 回调函数/钩子/技能
        required: true
      max_executions:
        type: integer
        description: 最大执行次数(-1表示无限)
        required: false
        default: -1
      metadata:
        type: object
        required: false
      persistent:
        type: boolean
        required: false
        default: false
    outputs:
      timer_id:
        type: string

  create_cron:
    description: 创建 Cron 定时器
    inputs:
      name:
        type: string
        description: 定时器名称
        required: false
      cron_expression:
        type: string
        description: Cron 表达式
        required: true
      timezone:
        type: string
        description: 时区
        required: false
        default: UTC
      callback:
        type: TimerCallback
        description: 回调函数/钩子/技能
        required: true
      metadata:
        type: object
        required: false
      persistent:
        type: boolean
        required: false
        default: true
    outputs:
      timer_id:
        type: string

  # ========== 定时器控制 ==========
  cancel:
    description: 取消定时器
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
      next_execution:
        type: datetime
        description: 下次执行时间(如果已调度)

  pause:
    description: 暂停定时器
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  resume:
    description: 恢复定时器
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
      next_execution:
        type: datetime

  reset:
    description: 重置定时器(重新开始计时)
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
      next_execution:
        type: datetime

  # ========== 定时器查询 ==========
  get_timer:
    description: 获取定时器信息
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      timer:
        type: TimerInfo

  list_timers:
    description: 列出定时器
    inputs:
      filter:
        type: TimerFilter
        description: 过滤条件
        required: false
    outputs:
      timers:
        type: array<TimerInfo>

  get_next_execution:
    description: 获取下次执行时间
    inputs:
      timer_id:
        type: string
        required: true
    outputs:
      next_execution:
        type: datetime

  # ========== 批量操作 ==========
  cancel_all:
    description: 取消所有定时器
    inputs:
      filter:
        type: TimerFilter
        description: 过滤条件
        required: false
    outputs:
      cancelled_count:
        type: integer

  pause_all:
    description: 暂停所有定时器
    inputs:
      filter:
        type: TimerFilter
        required: false
    outputs:
      paused_count:
        type: integer

  # ========== 统计和监控 ==========
  get_stats:
    description: 获取定时器统计
    outputs:
      stats:
        type: TimerStats

  get_executions:
    description: 获取执行历史
    inputs:
      timer_id:
        type: string
        required: true
      limit:
        type: integer
        description: 返回数量
        required: false
        default: 100
    outputs:
      executions:
        type: array<TimerExecution>

  # ========== 持久化 ==========
  save_timers:
    description: 保存定时器到存储
    inputs:
      filter:
        type: TimerFilter
        required: false
    outputs:
      saved_count:
        type: integer

  load_timers:
    description: 从存储加载定时器
    outputs:
      loaded_count:
        type: integer
```

### 数据结构

```yaml
# 定时器类型
TimerType:
  type: enum
  values:
    - oneshot
    - interval
    - cron

# 定时器状态
TimerStatus:
  type: enum
  values:
    - pending
    - active
    - paused
    - completed
    - cancelled

# 定时器回调类型
TimerCallback:
  type:
    type: enum
    values: [callback, hook, skill, webhook, message_queue]
    description: 回调类型

  # 回调函数
  callback:
    type: function
    description: 直接回调函数

  # Hook 触发
  hook:
    type: object
    properties:
      hook_id:
        type: string
        description: Hook ID
      args:
        type: object
        description: Hook 参数

  # Skill 触发
  skill:
    type: object
    properties:
      skill_id:
        type: string
        description: Skill ID
      args:
        type: object
        description: Skill 参数

  # Webhook 调用
  webhook:
    type: object
    properties:
      url:
        type: string
        description: Webhook URL
      method:
        type: string
        enum: [POST, PUT, PATCH]
        default: POST
      headers:
        type: object
        description: HTTP 头
      body:
        type: object
        description: 请求体

  # 消息队列
  message_queue:
    type: object
    properties:
      provider:
        type: string
        enum: [redis, rabbitmq, kafka, sqs]
        description: 消息队列提供商
      connection:
        type: object
        description: 连接配置
        properties:
          host:
            type: string
          port:
            type: integer
          username:
            type: string
          password:
            type: string
      topic:
        type: string
        description: 主题/队列名称
      message:
        type: object
        description: 消息内容
        properties:
          timer_id:
            type: string
          timer_name:
            type: string
          executed_at:
            type: datetime
          metadata:
            type: object

# 定时器
Timer:
  id:
    type: string
    description: 定时器唯一标识
  name:
    type: string
    description: 定时器名称
  type:
    type: TimerType
    description: 定时器类型
  status:
    type: TimerStatus
    description: 当前状态
  callback:
    type: TimerCallback
    description: 回调配置
  created_at:
    type: datetime
    description: 创建时间
  updated_at:
    type: datetime
    description: 更新时间

  # 一次性定时器
  oneshot:
    type: object
    properties:
      delay_ms:
        type: integer
        description: 延迟时间
      execute_at:
        type: datetime
        description: 执行时间

  # 周期性定时器
  interval:
    type: object
    properties:
      interval_ms:
        type: integer
        description: 执行间隔
      max_executions:
        type: integer
        description: 最大执行次数
      execution_count:
        type: integer
        description: 已执行次数
      next_execution:
        type: datetime
        description: 下次执行时间

  # Cron 定时器
  cron:
    type: object
    properties:
      expression:
        type: string
        description: Cron 表达式
      timezone:
        type: string
        description: 时区
      next_execution:
        type: datetime
        description: 下次执行时间

  # 通用属性
  metadata:
    type: object
    description: 元数据
  persistent:
    type: boolean
    description: 是否持久化
  last_execution:
    type: datetime
    description: 最后执行时间
  last_result:
    type: TimerExecutionResult
    description: 最后执行结果

# 定时器信息(查询返回)
TimerInfo:
  id:
    type: string
  name:
    type: string
  type:
    type: TimerType
  status:
    type: TimerStatus
  created_at:
    type: datetime
  next_execution:
    type: datetime
  last_execution:
    type: datetime
  execution_count:
    type: integer

# 定时器过滤条件
TimerFilter:
  type:
    type: TimerType
    description: 按类型过滤
  status:
    type: TimerStatus
    description: 按状态过滤
  name_pattern:
    type: string
    description: 按名称模式过滤
  created_after:
    type: datetime
    description: 创建时间之后
  created_before:
    type: datetime
    description: 创建时间之前
  persistent:
    type: boolean
    description: 是否持久化

# 定时器统计
TimerStats:
  total_timers:
    type: integer
    description: 总定时器数
  active_timers:
    type: integer
    description: 活跃定时器数
  paused_timers:
    type: integer
    description: 暂停定时器数
  completed_timers:
    type: integer
    description: 已完成定时器数

  by_type:
    type: object
    description: 按类型统计
    properties:
      oneshot:
        type: integer
      interval:
        type: integer
      cron:
        type: integer

  total_executions:
    type: integer
    description: 总执行次数
  successful_executions:
    type: integer
    description: 成功执行次数
  failed_executions:
    type: integer
    description: 失败执行次数

  avg_execution_time_ms:
    type: number
    description: 平均执行时间

# 定时器执行记录
TimerExecution:
  id:
    type: string
  timer_id:
    type: string
  executed_at:
    type: datetime
    description: 执行时间
  scheduled_at:
    type: datetime
    description: 计划时间
  delay_ms:
    type: integer
    description: 实际延迟(毫秒)
  result:
    type: TimerExecutionResult
    description: 执行结果

# 定时器执行结果
TimerExecutionResult:
  status:
    type: enum
    values: [success, failed, timeout]
  error:
    type: string
    description: 错误信息
  duration_ms:
    type: integer
    description: 执行耗时
  output:
    type: object
    description: 输出数据
```

### 配置选项

```yaml
# config/timer-system.yaml
timer_system:
  # 调度器配置
  scheduler:
    workers: 4
    queue_size: 10000
    resolution_ms: 10

  # 持久化配置
  persistence:
    enabled: true
    sync_interval_ms: 60000
    storage_path: "./data/timers"

  # 执行配置
  execution:
    timeout_ms: 300000
    retry_on_failure: false
    max_retries: 3
    retry_delay_ms: 1000

  # 监控配置
  monitoring:
    metrics_enabled: true
    execution_history_limit: 1000
    log_executions: true

  # Cron 配置
  cron:
    timezone: UTC
    enable_seconds: false
```

---

## 核心流程

### 定时器调度主流程

```
启动定时器系统
        │
        ▼
┌──────────────────────────────┐
│ 1. 加载持久化定时器          │
│    - 从存储加载              │
│    - 计算下次执行时间        │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 启动调度循环              │
│    while (running) {         │
│      - 检查到期定时器        │
│      - 执行回调              │
│      - 调度下次执行          │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    停止
```

### 一次性定时器流程

```
创建定时器
        │
        ▼
┌──────────────────────────────┐
│ 1. 验证参数                  │
│    - delay_ms > 0            │
│    - callback 有效           │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 计算执行时间              │
│    execute_at = now + delay   │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 加入调度队列              │
│    - 按执行时间排序          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 等待执行                  │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 5. 执行回调                  │
│    - 调用 callback/hook/skill │
└──────────────────────────────┘
        │
        ▼
    完成/删除
```

### 周期性定时器流程

```
创建周期定时器
        │
        ▼
┌──────────────────────────────┐
│ 1. 验证参数                  │
│    - interval_ms > 0         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 计算首次执行时间          │
│    next = now + interval     │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 加入调度队列              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 执行循环                  │
│    while (count < max) {     │
│      - 等待执行时间          │
│      - 执行回调              │
│      - 计算下次执行时间      │
│      - count++               │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    完成/删除
```

### Cron 定时器流程

```
创建 Cron 定时器
        │
        ▼
┌──────────────────────────────┐
│ 1. 解析 Cron 表达式          │
│    - 验证语法                │
│    - 构建调度器              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 计算下次执行时间          │
│    - 基于当前时区            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 加入调度队列              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 执行循环                  │
│    while (active) {          │
│      - 等待执行时间          │
│      - 执行回调              │
│      - 计算下次执行时间      │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    停止/删除
```

### 定时器执行流程

```
定时器到期
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查状态                  │
│    - 是否暂停?               │
│    - 是否已取消?             │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 记录执行开始              │
│    - actual_execute_time     │
│    - delay = actual - scheduled│
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 执行回调                  │
│    - callback/hook/skill     │
│    - 超时控制                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 处理结果                  │
│    - 记录执行状态            │
│    - 更新统计信息            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 5. 调度下次执行(如需要)      │
│    - interval: 更新 next     │
│    - cron: 计算新的 next     │
│    - oneshot: 删除           │
└──────────────────────────────┘
        │
        ▼
    完成
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Timer System                 │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Scheduler │  │Executor  │  │Storage ││
│  └──────────┘  └──────────┘  └────────┘│
└─────────────────────────────────────────┘
                  │
                  │ timer_triggered 事件
                  ▼
┌─────────────────────────────────────────┐
│            Event Loop                   │
│  ┌──────────┐  ┌──────────┐           │
│  │Queue     │  │Dispatcher│           │
│  └──────────┘  └──────────┘           │
└─────────────────────────────────────────┘
                  │
                  │ 分发事件
                  ▼
┌─────────────────────────────────────────┐
│         ┌─────────────┐                 │
│         │ Event       │                 │
│         │ Listeners   │                 │
│         └─────────────┘                 │
└─────────────────────────────────────────┘
        │                     │
        ▼                     ▼
┌──────────────┐      ┌──────────────┐
│Hook Engine   │      │Skill Engine  │
└──────────────┘      └──────────────┘
```

### 定时器事件流

```
┌─────────────────────────────────────────────────────────────┐
│                    Timer System (独立运行)                   │
│                                                             │
│  1. 调度循环检测定时器到期                                   │
│  2. 定时器到期时生成事件                                     │
│     - event_type: "timer_triggered"                         │
│     - timer_id: "xxx"                                       │
│     - callback_config: {...}                                │
│  3. 发送事件到 Event Loop                                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ emit(timer_triggered_event)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Event Loop                               │
│                                                             │
│  1. 接收 timer_triggered 事件                               │
│  2. 匹配注册的事件监听器                                     │
│  3. 并发执行监听器回调                                       │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 分发执行
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                      Event Listeners                        │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │Hook Listener     │  │Skill Listener    │               │
│  │- 触发配置的 Hook  │  │- 执行配置的 Skill│               │
│  └──────────────────┘  └──────────────────┘               │
│                                                             │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │Callback Listener │  │Webhook Listener  │               │
│  │- 执行回调函数    │  │- 发送 Webhook    │               │
│  └──────────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────────────┘
```

### 自然语言定时流程

```
用户输入 (CLI/Web UI)
> "每天早上8点发送AI新闻简报"
        │
        ▼
┌─────────────────────────────────────────┐
│         Agent Runtime + LLM             │
│  - 解析自然语言意图                     │
│  - 识别定时器类型                       │
│  - 提取时间参数和回调配置               │
│  - 决定调用哪个工具                     │
└─────────────────────────────────────────┘
        │
        │ LLM 输出: tool_calls
        │ [{
        │   "tool": "timer.create_interval",
        │   "params": {
        │     "name": "AI新闻简报",
        │     "interval_ms": 86400000,
        │     "callback": {"skill": "news-digester"}
        │   }
        │ }]
        ▼
┌─────────────────────────────────────────┐
│         Tool System                     │
│  - 验证工具调用                         │
│  - 路由到 Timer System                  │
└─────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────┐
│         Timer System                    │
│  create_interval(params)                │
│  - 创建定时器                           │
│  - 返回 timer_id                        │
└─────────────────────────────────────────┘
        │
        ▼
✅ 定时任务已创建: task-001
```

### 事件数据结构

```yaml
# 定时器触发事件
TimerTriggeredEvent:
  id:
    type: string
  type:
    type: string
    value: "timer_triggered"
  source:
    type: string
    value: "timer_system"
  timestamp:
    type: datetime
  data:
    type: object
    properties:
      timer_id:
        type: string
      timer_type:
        type: TimerType
      timer_name:
        type: string
      callback:
        type: TimerCallback
      execution_count:
        type: integer
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/timer-system.yaml
timer_system:
  # 调度器配置
  scheduler:
    workers: 4
    queue_size: 10000
    resolution_ms: 10

  # 持久化配置
  persistence:
    enabled: true
    sync_interval_ms: 60000
    storage_path: "./data/timers"
    backup_count: 5

  # 执行配置
  execution:
    timeout_ms: 300000
    retry_on_failure: false
    max_retries: 3
    retry_delay_ms: 1000

  # 监控配置
  monitoring:
    metrics_enabled: true
    execution_history_limit: 1000
    execution_history_ttl: 7d
    description: "执行历史保留7天后自动清理"
    log_executions: true
    alert_on_failure: true

  # Cron 配置
  cron:
    timezone: UTC
    enable_seconds: false
```

### 环境变量

```bash
# 调度器配置
export KNIGHT_TIMER_WORKERS=4
export KNIGHT_TIMER_QUEUE_SIZE=10000
export KNIGHT_TIMER_RESOLUTION_MS=10

# 持久化配置
export KNIGHT_TIMER_PERSISTENCE_ENABLED=true
export KNIGHT_TIMER_STORAGE_PATH="./data/timers"

# 执行配置
export KNIGHT_TIMER_TIMEOUT_MS=300000
export KNIGHT_TIMER_MAX_RETRIES=3

# 监控配置
export KNIGHT_TIMER_METRICS_ENABLED=true
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 调度精度 | ±10ms | 定时器触发时间误差 |
| 吞吐量 | > 1000 timers/s | 定时器创建速率 |
| 并发定时器 | > 10000 | 同时运行的定时器数 |
| 内存占用 | < 50MB | 每个定时器平均占用 |

### 错误处理

```yaml
error_codes:
  INVALID_DELAY:
    code: 400
    message: "延迟时间无效"
    action: "delay_ms 必须大于 0"

  INVALID_CRON:
    code: 400
    message: "Cron 表达式无效"
    action: "检查 Cron 表达式语法"

  TIMER_NOT_FOUND:
    code: 404
    message: "定时器不存在"
    action: "检查 timer_id"

  TIMER_EXPIRED:
    code: 410
    message: "定时器已过期"
    action: "定时器已完成或已取消"

  EXECUTION_TIMEOUT:
    code: 504
    message: "定时器执行超时"
    action: "增加超时时间或优化回调"

  CALLBACK_FAILED:
    code: 500
    message: "回调执行失败"
    action: "检查回调函数日志"

  PERSISTENCE_FAILED:
    code: 500
    message: "持久化失败"
    action: "检查存储服务"
```

### Cron 表达式示例

```yaml
# 标准 Cron 格式 (分钟 小时 日期 月份 星期)
cron_examples:
  # 每天凌晨 2 点
  daily_backup:
    expression: "0 2 * * *"
    description: "每天 02:00 执行"

  # 每周一早上 9 点
  weekly_report:
    expression: "0 9 * * 1"
    description: "每周一 09:00 执行"

  # 每月 1 号凌晨
  monthly_cleanup:
    expression: "0 0 1 * *"
    description: "每月 1 号 00:00 执行"

  # 每 5 分钟
  frequent_check:
    expression: "*/5 * * * *"
    description: "每 5 分钟执行一次"

  # 工作日每小时
  business_hour:
    expression: "0 9-17 * * 1-5"
    description: "工作日 9-17 点每小时执行"

# 启用秒级精度 (6 位格式)
cron_with_seconds:
  # 每分钟的第 10 秒
    expression: "10 * * * * *"
    description: "每分钟的第 10 秒执行"
```

### 使用场景

```yaml
use_cases:
  # 自动保存
  auto_save:
    type: interval
    interval_ms: 300000
    description: "每 5 分钟自动保存会话"

  # 会话超时
  session_timeout:
    type: oneshot
    delay_ms: 1800000
    description: "30 分钟无操作后超时"

  # 定期清理
  periodic_cleanup:
    type: cron
    expression: "0 3 * * *"
    description: "每天凌晨 3 点清理临时文件"

  # 提醒通知
  reminder:
    type: oneshot
    delay_ms: 3600000
    description: "1 小时后发送提醒"

  # 健康检查
  health_check:
    type: interval
    interval_ms: 60000
    description: "每分钟检查系统健康状态"
```

### 测试策略

```yaml
testing:
  unit_tests:
    - 定时器创建和配置
    - 调度逻辑验证
    - 回调执行测试
    - 状态管理测试
    - Cron 表达式解析

  integration_tests:
    - 与 Hook Engine 集成
    - 与 Skill Engine 集成
    - 持久化恢复测试
    - 并发执行测试

  performance_tests:
    - 大量定时器并发测试
    - 调度精度测试
    - 长时间运行稳定性测试
    - 内存占用测试
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-04-01 | 初始版本 |
