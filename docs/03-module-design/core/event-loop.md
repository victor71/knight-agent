# Event Loop (事件循环)

## 概述

### 职责描述

Event Loop 负责系统的事件驱动架构，包括：

- 事件源管理（文件、Git、定时器等）
- 事件队列和分发
- 事件监听器注册
- 防抖和节流控制
- 后台任务调度

### 设计目标

1. **高性能**: 支持大量并发事件
2. **可靠传递**: 确保事件不丢失
3. **灵活调度**: 支持优先级和延迟调度
4. **可观测**: 完整的事件追踪

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Hook Engine | 依赖 | 触发 Hook |
| Skill Engine | 依赖 | 触发技能 |
| Orchestrator | 依赖 | 分发到 Agent |

---

## 接口定义

### 对外接口

```yaml
# Event Loop 接口定义
EventLoop:
  # ========== 循环控制 ==========
  start:
    description: 启动事件循环
    inputs:
      config:
        type: EventLoopConfig
        required: false
    outputs:
      success:
        type: boolean

  stop:
    description: 停止事件循环
    inputs:
      graceful:
        type: boolean
        description: 优雅停止（处理完当前事件）
        required: false
        default: true
    outputs:
      success:
        type: boolean

  get_status:
    description: 获取循环状态
    outputs:
      status:
        type: EventLoopStatus

  # ========== 事件源管理 ==========
  register_source:
    description: 注册事件源
    inputs:
      source:
        type: EventSource
        required: true
    outputs:
      source_id:
        type: string

  unregister_source:
    description: 注销事件源
    inputs:
      source_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  enable_source:
    description: 启用事件源
    inputs:
      source_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  disable_source:
    description: 禁用事件源
    inputs:
      source_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_sources:
    description: 列出事件源
    outputs:
      sources:
        type: array<EventSourceInfo>

  # ========== 监听器管理 ==========
  add_listener:
    description: 添加事件监听器
    inputs:
      listener:
        type: EventListener
        required: true
    outputs:
      listener_id:
        type: string

  remove_listener:
    description: 移除监听器
    inputs:
      listener_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_listeners:
    description: 列出监听器
    inputs:
      event_type:
        type: string
        description: 事件类型过滤
        required: false
    outputs:
      listeners:
        type: array<EventListenerInfo>

  # ========== 事件操作 ==========
  emit:
    description: 手动触发事件
    inputs:
      event:
        type: Event
        required: true
    outputs:
      delivered_count:
        type: integer

  emit_delayed:
    description: 延迟触发事件
    inputs:
      event:
        type: Event
        required: true
      delay_ms:
        type: integer
        required: true
    outputs:
      scheduled:
        type: boolean

  cancel_delayed:
    description: 取消延迟事件
    inputs:
      event_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 调度器 ==========
  schedule:
    description: 调度定时任务
    inputs:
      task:
        type: ScheduledTask
        required: true
    outputs:
      task_id:
        type: string

  unschedule:
    description: 取消定时任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_scheduled:
    description: 列出定时任务
    outputs:
      tasks:
        type: array<ScheduledTaskInfo>

  # ========== 统计和监控 ==========
  get_stats:
    description: 获取事件统计
    outputs:
      stats:
        type: EventStats

  get_queue_info:
    description: 获取队列信息
    outputs:
      info:
        type: QueueInfo
```

### 数据结构

```yaml
# 事件
Event:
  id:
    type: string
  type:
    type: string
    description: 事件类型
  source:
    type: string
    description: 事件源 ID
  timestamp:
    type: datetime
  data:
    type: object
    description: 事件数据
  priority:
    type: integer
    description: 优先级
    default: 100
  metadata:
    type: map<string, any>

# 事件源
EventSource:
  id:
    type: string
  name:
    type: string
  type:
    type: enum
    values: [file_watcher, git_watcher, scheduler, custom]
    description: 事件源类型
  enabled:
    type: boolean
    default: true

  # 文件监控
  file_watcher:
    type: object
    properties:
      paths:
        type: array<string>
        description: 监控路径
      patterns:
        type: array<string>
        description: 文件模式
      events:
        type: array<string>
        enum: [created, modified, deleted]
        description: 监控事件
      debounce:
        type: integer
        description: 防抖延迟（毫秒）

  # Git 监控
  git_watcher:
    type: object
    properties:
      path:
        type: string
        description: 仓库路径
      events:
        type: array<string>
        enum: [commit, push, branch_change]
      poll_interval:
        type: integer
        description: 轮询间隔（秒）

  # 定时器
  scheduler:
    type: object
    properties:
      timezone:
        type: string
      cron:
        type: string

  # 自定义源
  custom:
    type: object
    properties:
      endpoint:
        type: string
      poll_interval:
        type: integer
      headers:
        type: map<string, string>

# 事件监听器
EventListener:
  id:
    type: string
  name:
    type: string
  enabled:
    type: boolean
    default: true

  # 过滤条件
  filter:
    type: EventFilter

  # 处理器
  handler:
    type: EventHandler

  # 错误处理
  error_handling:
    type: ListenerErrorHandling

# 事件过滤器
EventFilter:
  event_type:
    type: string | array<string>
    description: 事件类型过滤
  source:
    type: string | array<string>
    description: 事件源过滤
  conditions:
    type: map<string, any>
    description: 自定义条件

# 事件处理器
EventHandler:
  type:
    type: enum
    values: [skill, hook, webhook, callback]

  # 技能触发
  skill:
    type: object
    properties:
      skill_id:
        type: string
      args:
        type: map<string, any>

  # Hook 触发
  hook:
    type: object
    properties:
      hook_id:
        type: string

  # Webhook
  webhook:
    type: object
    properties:
      url:
        type: string
      method:
        type: string
        enum: [POST, PUT, PATCH]
      headers:
        type: map<string, string>

  # 回调函数
  callback:
    type: function

# 监听器错误处理
ListenerErrorHandling:
  retry:
    type: boolean
  max_retries:
    type: integer
  continue_on_error:
    type: boolean
  log_errors:
    type: boolean

# 定时任务
ScheduledTask:
  id:
    type: string
  name:
    type: string
  schedule:
    type: string
    description: Cron 表达式
  handler:
    type: EventHandler
  enabled:
    type: boolean
  timezone:
    type: string

# 事件循环配置
EventLoopConfig:
  queue_size:
    type: integer
    description: 队列大小
  overflow_policy:
    type: string
    enum: [block, drop_oldest, drop_newest]
    description: 队列溢出策略
  workers:
    type: integer
    description: 工作线程数
  batch_size:
    type: integer
    description: 批处理大小

# 事件循环状态
EventLoopStatus:
  running:
    type: boolean
  uptime_seconds:
    type: integer
  events_processed:
    type: integer
  events_per_second:
    type: float
  active_sources:
    type: integer
  active_listeners:
    type: integer

# 事件统计
EventStats:
  total_events:
    type: integer
  events_by_type:
    type: map<string, integer>
  events_by_source:
    type: map<string, integer>
  processing_time_avg_ms:
    type: float
  error_count:
    type: integer

# 队列信息
QueueInfo:
  size:
    type: integer
  capacity:
    type: integer
  utilization_percent:
    type: float
  oldest_event_age_ms:
    type: integer
```

### 配置选项

```yaml
# config/event-loop.yaml
event_loop:
  # 队列配置
  queue:
    size: 10000
    overflow_policy: block

  # 工作线程
  workers: 4
  batch_size: 10

  # 事件源
  sources:
    file_watcher:
      enabled: true
      debounce: 500
    git_watcher:
      enabled: true
      poll_interval: 30

  # 定时器
  scheduler:
    enabled: true
    timezone: UTC
```

---

## 核心流程

### 事件循环主流程

```
启动事件循环
        │
        ▼
┌──────────────────────────────┐
│ 1. 初始化事件源              │
│    - 启动文件监控            │
│    - 启动 Git 监控           │
│    - 启动定时器              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 事件循环                  │
│    while (running) {         │
│      - 从队列取事件          │
│      - 匹配监听器            │
│      - 执行处理器            │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    停止
```

### 事件分发流程

```
事件到达
        │
        ▼
┌──────────────────────────────┐
│ 1. 事件验证                  │
│    - 检查格式                │
│    - 添加时间戳              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 加入队列                  │
│    - 按优先级排序            │
│    - 检查队列容量            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 匹配监听器                │
│    - 按事件类型过滤          │
│    - 按事件源过滤            │
│    - 按条件过滤              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 并发执行处理器            │
│    - 技能触发                │
│    - Hook 触发               │
│    - Webhook 调用            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 文件监控流程

```
文件变更事件
        │
        ▼
┌──────────────────────────────┐
│ 1. 防抖处理                  │
│    - 合并同一文件的变更      │
│    - 等待防抖延迟            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 模式匹配                  │
│    - glob 匹配               │
│    - 路径过滤                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 生成事件                  │
│    type: file_change         │
│    data: {                   │
│      event: created/modified  │
│      path: /path/to/file     │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    发送到队列
```

### 定时任务流程

```
Cron 触发
        │
        ▼
┌──────────────────────────────┐
│ 1. 解析 Cron 表达式          │
│    - 计算下次执行时间        │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 等待触发时间              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 生成调度事件              │
│    type: schedule            │
│    data: {                   │
│      task_id: xxx            │
│      scheduled_time: xxx     │
│    }                         │
└──────────────────────────────┘
        │
        ▼
    发送到队列
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Event Loop                   │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Queue     │  │Dispatcher│  │Scheduler││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Skill     │  │Hook      │  │External  │
│Engine    │  │Engine    │  │Webhook   │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
事件源
    │
    ▼
┌─────────────────────────────┐
│ Event Loop                  │
│ - 接收事件                  │
│ - 队列管理                  │
│ - 分发到监听器              │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ Skill Engine    │         │ Hook Engine     │
│ - 触发技能      │         │ - 触发 Hook     │
└─────────────────┘         └─────────────────┘
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/event-loop.yaml
event_loop:
  # 队列配置
  queue:
    size: 10000
    overflow_policy: block

  # 工作线程
  workers: 4
  batch_size: 10

  # 事件源
  sources:
    file_watcher:
      enabled: true
      debounce: 500
    git_watcher:
      enabled: true
      poll_interval: 30
    scheduler:
      enabled: true
      timezone: UTC

  # 监控
  monitoring:
    metrics_enabled: true
    log_events: false
```

### 环境变量

```bash
# 队列配置
export KNIGHT_EVENT_QUEUE_SIZE=10000
export KNIGHT_EVENT_OVERFLOW_POLICY="block"

# 工作线程
export KNIGHT_EVENT_WORKERS=4

# 监控
export KNIGHT_EVENT_METRICS_ENABLED=true
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 事件延迟 | < 10ms | 端到端 |
| 吞吐量 | > 10000 events/s | 单机 |
| 队列容量 | 10000 | 最大事件数 |
| 内存占用 | < 100MB | 基础占用 |

### 错误处理

```yaml
error_codes:
  QUEUE_FULL:
    code: 503
    message: "事件队列已满"
    action: "等待或增加队列容量"

  SOURCE_FAILED:
    code: 500
    message: "事件源失败"
    action: "检查事件源配置"

  LISTENER_FAILED:
    code: 500
    message: "监听器执行失败"
    action: "查看监听器日志"

  SCHEDULE_ERROR:
    code: 400
    message: "调度配置错误"
    action: "检查 Cron 表达式"
```

### 内置事件源

| 事件源 | 类型 | 描述 |
|--------|------|------|
| file_watcher | 文件监控 | 监控文件系统变化 |
| git_watcher | Git 监控 | 监控 Git 仓库变化 |
| scheduler | 定时器 | Cron 定时任务 |
| http_source | HTTP 轮询 | HTTP 端点轮询 |
| webhook_source | Webhook | 接收 Webhook |
