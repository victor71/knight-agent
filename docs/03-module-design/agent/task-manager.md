# Task Manager (任务管理器)

## 概述

### 职责描述

Task Manager 负责任务和工作流的生命周期管理及 DAG 依赖解析，包括：

- 任务/工作流创建和状态管理
- DAG 依赖关系解析
- 任务调度和执行协调
- 调用 Orchestrator 分配 Agent
- 任务结果存储和查询
- 重试和错误处理
- **后台工作流执行**（支持数天级别的长时间运行）
- **工作流持久化和恢复**（支持进程重启后恢复）

### 设计目标

1. **依赖管理**: 支持复杂的任务依赖关系（DAG）
2. **并行执行**: 自动识别可并行执行的任务
3. **容错机制**: 自动重试和错误恢复
4. **可追踪**: 完整的任务执行历史
5. **长时间运行**: 支持数天级别的后台工作流执行
6. **可恢复**: 进程重启后可恢复未完成的工作流

### 设计目标

1. **依赖管理**: 支持复杂的任务依赖关系（DAG）
2. **并行执行**: 自动识别可并行执行的任务
3. **容错机制**: 自动重试和错误恢复
4. **可追踪**: 完整的任务执行历史

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Orchestrator | 协作 | 调用 Agent 分配接口 |
| Agent Runtime | 依赖 | 执行任务 |
| Storage Service | 依赖 | 持久化任务状态和检查点 |
| Skill Engine | 依赖 | 技能任务执行 |
| LLM Provider | 协作 | 用于 Command 触发的工作流解析 |
| Command | 被依赖 | Command 模块调用 create_workflow_from_parsed |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| Orchestrator | 被依赖 | 使用 Task Manager 的 Task 数据结构定义 |
| Command | 被依赖 | Command 触发工作流时调用 Task Manager |

**注意**:
- Task Manager 定义系统中所有 Task 相关的数据结构（Task, TaskDefinition, TaskStatus 等），其他模块（如 Orchestrator）引用此定义，不重复定义。
- Command 模块通过 `create_workflow_from_parsed` 接口将解析的工作流提交给 Task Manager执行。

---

## 接口定义

### 对外接口

```yaml
# Task Manager 接口定义
TaskManager:
  # ========== 任务管理 ==========
  create_task:
    description: 创建任务
    inputs:
      task:
        type: TaskDefinition
        required: true
    outputs:
      task_id:
        type: string

  create_workflow:
    description: 创建工作流（多任务 DAG）
    inputs:
      workflow:
        type: WorkflowDefinition
        required: true
    outputs:
      workflow_id:
        type: string
      task_ids:
        type: array<string>

  # ========== Command 触发的工作流 ==========
  create_workflow_from_parsed:
    description: 从 Command 解析的自然语言定义创建工作流
    inputs:
      parsed_workflow:
        type: ParsedWorkflowDefinition
        required: true
        description: 由 Command 通过 LLM 解析得到的工作流定义
      context:
        type: WorkflowContext
        required: false
        description: 工作流上下文（如命令参数、环境变量等）
      background:
        type: boolean
        required: false
        default: true
        description: 是否在后台运行（长时间运行的工作流建议为 true）
    outputs:
      workflow_id:
        type: string
      execution_mode:
        type: string
        description: "foreground | background"

  get_task:
    description: 获取任务详情
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      task:
        type: Task | null

  list_tasks:
    description: 列出任务
    inputs:
      filter:
        type: TaskFilter
        required: false
      limit:
        type: integer
        required: false
    outputs:
      tasks:
        type: array<Task>

  update_task:
    description: 更新任务
    inputs:
      task_id:
        type: string
        required: true
      updates:
        type: TaskUpdate
        required: true
    outputs:
      success:
        type: boolean

  delete_task:
    description: 删除任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 任务执行 ==========
  start_task:
    description: 启动任务执行
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  start_workflow:
    description: 启动工作流执行
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  cancel_task:
    description: 取消任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  retry_task:
    description: 重试失败任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 任务状态 ==========
  get_task_status:
    description: 获取任务状态
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      status:
        type: TaskStatusDetail

  get_workflow_status:
    description: 获取工作流状态
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      status:
        type: WorkflowStatus

  wait_for_task:
    description: 等待任务完成
    inputs:
      task_id:
        type: string
        required: true
      timeout:
        type: integer
        description: 超时时间（秒）
        required: false
    outputs:
      result:
        type: TaskResult

  wait_for_workflow:
    description: 等待工作流完成
    inputs:
      workflow_id:
        type: string
        required: true
      timeout:
        type: integer
        required: false
    outputs:
      result:
        type: WorkflowResult

  # ========== 任务依赖 ==========
  get_dependencies:
    description: 获取任务依赖
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      dependencies:
        type: array<DependencyInfo>

  resolve_dependencies:
    description: 解析 DAG 依赖
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      execution_order:
        type: array<array<string>>
        description: 分层的执行顺序

  # ========== 历史和统计 ==========
  get_task_history:
    description: 获取任务历史
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      history:
        type: array<TaskHistoryEntry>

  get_statistics:
    description: 获取任务统计
    inputs:
      filter:
        type: TaskFilter
        required: false
    outputs:
      stats:
        type: TaskStatistics

  # ========== 后台工作流管理 ==========
  list_background_workflows:
    description: 列出所有后台运行的工作流
    inputs:
      status:
        type: string | array<string> | null
        description: 过滤状态
        required: false
    outputs:
      workflows:
        type: array<BackgroundWorkflowInfo>

  pause_workflow:
    description: 暂停后台工作流
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  resume_workflow:
    description: 恢复暂停的工作流
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  terminate_workflow:
    description: 终止后台工作流
    inputs:
      workflow_id:
        type: string
        required: true
      reason:
        type: string
        required: false
    outputs:
      success:
        type: boolean

  get_workflow_logs:
    description: 获取工作流执行日志
    inputs:
      workflow_id:
        type: string
        required: true
      task_id:
        type: string | null
        description: 可选，获取特定任务的日志
      limit:
        type: integer
        required: false
      offset:
        type: integer
        required: false
    outputs:
      logs:
        type: array<WorkflowLogEntry>

  # ========== 工作流恢复 ==========
  persist_workflow_state:
    description: 持久化工作流状态（用于恢复）
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
      checkpoint_path:
        type: string

  restore_workflow:
    description: 从持久化状态恢复工作流
    inputs:
      workflow_id:
        type: string
        required: true
      checkpoint_path:
        type: string | null
        description: 不指定则从最新检查点恢复
    outputs:
      success:
        type: boolean
```

### 数据结构

```yaml
# 任务定义
TaskDefinition:
  id:
    type: string
  name:
    type: string
  description:
    type: string
  type:
    type: enum
    values: [agent, skill, tool, workflow]
    description: 任务类型

  # 执行配置
  agent:
    type: string | null
    description: 指定 Agent ID
  skill:
    type: string | null
    description: 指定技能 ID
  tool:
    type: string | null
    description: 指定工具名称

  # 参数
  inputs:
    type: map<string, any>
    description: 输入参数
  outputs:
    type: array<string>
    description: 输出变量名

  # 依赖
  depends_on:
    type: array<Dependency>
    description: 任务依赖

  # 条件执行
  run_if:
    type: string | null
    description: 执行条件表达式
  continue_on_error:
    type: boolean
    default: false

  # 重试配置
  retry:
    type: RetryConfig
    description: 重试配置

  # 超时配置
  timeout:
    type: integer | null
    description: 超时时间（秒）

# 依赖关系
Dependency:
  task_id:
    type: string
    description: 依赖的任务 ID
  condition:
    type: string | null
    description: 依赖条件 (success/failed/completed)
    default: "success"

# 重试配置
RetryConfig:
  max_attempts:
    type: integer
    default: 3
  delay:
    type: integer
    description: 重试延迟（毫秒）
    default: 1000
  backoff:
    type: string
    enum: [fixed, exponential]
    default: "fixed"
  retry_on:
    type: array<string>
    description: 可重试的错误码

# 工作流定义
WorkflowDefinition:
  id:
    type: string
  name:
    type: string
  description:
    type: string

  # 工作流变量
  variables:
    type: map<string, any>
    description: 工作流级变量

  # 任务列表
  tasks:
    type: array<TaskDefinition>

  # 全局配置
  retry:
    type: RetryConfig
  timeout:
    type: integer
    description: 全局超时
  max_parallel:
    type: integer
    description: 最大并行任务数

# 任务
Task:
  id:
    type: string
  workflow_id:
    type: string | null
  name:
    type: string
  description:
    type: string
  type:
    type: string

  # 状态
  status:
    type: enum
    values: [pending, ready, running, completed, failed, cancelled, skipped]
  state:
    type: TaskState

  # 执行信息
  assigned_agent:
    type: string | null
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null

  # 结果
  result:
    type: TaskResult | null

  # 重试
  retry_count:
    type: integer
    default: 0

  # 依赖
  depends_on:
    type: array<Dependency>
  dependents:
    type: array<string>
    description: 依赖此任务的任务 ID

# 任务状态
TaskState:
  status:
    type: string
  progress:
    type: float
    description: 进度 0-1
  current_step:
    type: string | null
  error:
    type: ErrorInfo | null

# 任务结果
TaskResult:
  success:
    type: boolean
  outputs:
    type: map<string, any>
  error:
    type: string | null
  error_code:
    type: string | null
  execution_time:
    type: integer
    description: 执行时间（毫秒）

# 工作流状态
WorkflowStatus:
  workflow_id:
    type: string
  status:
    type: enum
    values: [pending, running, completed, failed, cancelled]
  tasks:
    type: map<string, TaskStatusDetail>
  progress:
    type: float
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null

# 任务状态详情
TaskStatusDetail:
  task_id:
    type: string
  status:
    type: string
  progress:
    type: float
  result:
    type: TaskResult | null
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null

# 依赖信息
DependencyInfo:
  task_id:
    type: string
  depends_on:
    type: array<string>
  dependents:
    type: array<string>
  status:
    type: string
  condition:
    type: string

# 任务历史
TaskHistoryEntry:
  timestamp:
    type: datetime
  event:
    type: string
    description: 状态变更事件
  from_status:
    type: string | null
  to_status:
    type: string
  details:
    type: map<string, any>

# 任务统计
TaskStatistics:
  total_tasks:
    type: integer
  by_status:
    type: map<string, integer>
  by_type:
    type: map<string, integer>
  avg_execution_time:
    type: integer
  success_rate:
    type: float

# 任务过滤器
TaskFilter:
  workflow_id:
    type: string | null
  status:
    type: string | array<string> | null
  type:
    type: string | array<string> | null
  agent:
    type: string | null
  created_after:
    type: datetime | null
  created_before:
    type: datetime | null

# 任务更新
TaskUpdate:
  status:
    type: string | null
  progress:
    type: float | null
  result:
    type: TaskResult | null
  error:
    type: ErrorInfo | null

# ========== Command 触发的工作流相关 ==========

# 由 Command 通过 LLM 解析的工作流定义
ParsedWorkflowDefinition:
  workflow_id:
    type: string
    description: 工作流标识符
  name:
    type: string
    description: 工作流名称
  description:
    type: string
    description: 工作流描述

  # 任务定义（从自然语言解析而来）
  tasks:
    type: array<ParsedTaskDefinition>
    description: 任务列表

  # 变量
  variables:
    type: map<string, any>
    description: 工作流变量

  # 依赖关系（DAG）
  dependencies:
    type: array<TaskDependency>
    description: 任务依赖关系

  # 执行配置
  execution_mode:
    type: string
    enum: [foreground, background]
    default: background
  max_parallel:
    type: integer
    description: 最大并行任务数

# 解析的任务定义
ParsedTaskDefinition:
  id:
    type: string
  name:
    type: string
  description:
    type: string

  # Agent 分配要求
  agent_variant:
    type: string | null
    description: Agent 变体（如 architect, developer, tester）
  agent_requirements:
    type: array<string>
    description: 所需能力

  # 任务内容（自然语言）
  prompt:
    type: string
    description: 发送给 Agent 的提示词

  # 输入
  inputs:
    type: map<string, any>
    description: 输入参数

  # 输出
  outputs:
    type: array<string>
    description: 输出变量名

  # 条件执行
  run_if:
    type: string | null
    description: 执行条件
  continue_on_error:
    type: boolean

# 任务依赖关系
TaskDependency:
  from:
    type: string
    description: 源任务 ID
  to:
    type: string
    description: 目标任务 ID
  condition:
    type: string | null
    description: 依赖条件

# 工作流上下文
WorkflowContext:
  source:
    type: string
    enum: [command, api, scheduled]
    description: 触发来源
  command_name:
    type: string | null
    description: 触发的命令名称
  command_args:
    type: array<string>
    description: 命令参数
  session_id:
    type: string
    description: 会话 ID
  environment:
    type: map<string, string>
    description: 环境变量

# ========== 后台工作流相关 ==========

# 后台工作流信息
BackgroundWorkflowInfo:
  workflow_id:
    type: string
  name:
    type: string
  status:
    type: string
    enum: [pending, running, paused, completed, failed, cancelled]
  progress:
    type: float
  started_at:
    type: datetime
  duration:
    type: integer
    description: 已运行时间（秒）
  completed_tasks:
    type: integer
  total_tasks:
    type: integer
  current_tasks:
    type: array<string>
    description: 当前正在执行的任务

# 工作流日志
WorkflowLogEntry:
  timestamp:
    type: datetime
  level:
    type: string
    enum: [debug, info, warn, error]
  workflow_id:
    type: string
  task_id:
    type: string | null
  message:
    type: string
  metadata:
    type: map<string, any>
```

### 配置选项

```yaml
# config/task.yaml
task:
  # 执行配置
  execution:
    max_parallel: 10
    default_timeout: 300
    check_interval: 5

  # 重试配置
  retry:
    max_attempts: 3
    delay: 1000
    backoff: exponential

  # 存储配置
  storage:
    persist_results: true
    retention_days: 30
    checkpoint_interval: 60
    description: 检查点间隔（秒），用于工作流恢复

  # DAG 配置
  dag:
    max_tasks: 1000
    max_depth: 50

  # 后台工作流配置
  background:
    enabled: true
    max_concurrent: 5
    description: 最大并发后台工作流数
    heartbeat_interval: 30
    description: 心跳间隔（秒）
    auto_resume: true
    description: 进程重启后自动恢复未完成工作流
    max_running_time: 604800
    description: 最大运行时间（秒），默认 7 天
    notification:
      on_complete: false
      on_failure: true
      on_timeout: true
```

---

## 核心流程

### 任务执行流程

```
任务启动
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查依赖                  │
│    - 检查前置任务完成        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 满足？  │
    └───┬────┘
        │ 否
        ▼
    等待依赖完成
        │ 是
        ▼
┌──────────────────────────────┐
│ 2. 检查条件                  │
│    - 评估 run_if 表达式      │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 满足？  │
    └───┬────┘
        │ 否
        ▼
    跳过任务
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 分配到 Agent              │
│    - 通过 Orchestrator       │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 执行任务                  │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 5. 错误处理      │   │ 5. 保存结果  │
│    - 检查重试    │   │    更新状态  │
│    - 记录错误    │   └──────────────┘
└──────────────────┘
        │
        ▼
    完成
```

### DAG 依赖解析

```
工作流 DAG
     Task A (design)
         │
         ▼
     Task B (implement) ◄──── Task D (review)
         │                        │
         ▼                        │
     Task C (test) ◄─────────────┘
         │
         ▼
   Task E (deploy)

解析为执行层级：
Layer 1: [Task A]
Layer 2: [Task B, Task D]  # 可并行
Layer 3: [Task C]          # 等待 B 和 D
Layer 4: [Task E]          # 等待 C
```

### 工作流执行流程

```
工作流启动
        │
        ▼
┌──────────────────────────────┐
│ 1. 解析 DAG                  │
│    - 计算执行层级            │
│    - 识别并行机会            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 逐层执行                  │
│    for layer in layers:      │
│      - 并行启动层内任务      │
│      - 等待所有任务完成      │
│      - 检查错误              │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 有错误？│
    └───┬────┘
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 错误处理                  │
│    - 检查 continue_on_error  │
│    - 决定是否继续            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### Command 触发的工作流执行流程

```
用户输入: /workflow software-development docs/requirements.md
        │
        ▼
┌──────────────────────────────┐
│ 1. Command 模块处理          │
│    - 识别 workflow 命令类型  │
│    - 读取工作流 Markdown 文件│
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. LLM 解析工作流定义        │
│    - 解析任务列表            │
│    - 解析依赖关系            │
│    - 生成 ParsedWorkflow     │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. Task Manager 创建工作流   │
│    - create_workflow_from_    │
│      parsed()                │
│    - 生成任务 DAG            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │后台？  │
    └───┬────┘
        │ 是                   │ 否
        ▼                       ▼
┌──────────────────┐   ┌──────────────┐
│ 4a. 后台执行     │   │ 4b. 前台执行 │
│ - 异步运行       │   │ - 同步等待   │
│ - 返回 workflow  │   │ - 返回结果   │
│   ID             │   └──────────────┘
└──────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 5. 动态创建 Agents           │
│    - 根据任务需求            │
│    - 调用 Orchestrator       │
│    - 分配 Agent 变体         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 6. 执行工作流 DAG            │
│    - 按依赖顺序执行          │
│    - 持久化检查点            │
│    - 处理长时间运行          │
└──────────────────────────────┘
```

### 后台工作流执行流程

```
后台工作流启动
        │
        ▼
┌──────────────────────────────┐
│ 1. 创建工作流实例            │
│    - 生成唯一 workflow_id    │
│    - 初始化状态              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 持久化初始状态            │
│    - 保存到 Storage Service  │
│    - 创建检查点              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 后台执行循环              │
│    while not completed:      │
│      - 执行可执行任务        │
│      - 更新进度              │
│      - 定期持久化            │
│      - 检查暂停/终止请求     │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 完成处理                  │
│    - 保存最终结果            │
│    - 清理资源                │
│    - 发送通知（如配置）      │
└──────────────────────────────┘
```

### 工作流恢复流程

```
进程重启 / 恢复请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 扫描未完成工作流          │
│    - 从 Storage Service 加载 │
│    - 识别 running 状态       │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 加载检查点                │
│    - 恢复工作流状态          │
│    - 恢复任务状态            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 继续执行                  │
│    - 从断点恢复              │
│    - 重试失败任务            │
└──────────────────────────────┘
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Task Manager                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │DAG       │  │Scheduler │  │State    ││
│  │Resolver  │  │          │  │Manager  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Agent     │  │Skill     │  │Storage   │
│Runtime   │  │Engine    │  │Service   │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
工作流请求
    │
    ▼
┌─────────────────────────────┐
│ Task Manager                │
│ - 解析 DAG                  │
│ - 调度任务                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Orchestrator                │
│ - 分配任务到 Agent          │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Agent Runtime               │
│ - 执行任务                  │
└─────────────────────────────┘
        │
        ▼
    返回结果
        │
        ▼
┌─────────────────────────────┐
│ Task Manager                │
│ - 更新状态                  │
│ - 触发后续任务              │
└─────────────────────────────┘
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/task.yaml
task:
  # 执行配置
  execution:
    max_parallel: 10
    default_timeout: 300
    check_interval: 5

  # 重试配置
  retry:
    max_attempts: 3
    delay: 1000
    backoff: exponential

  # 存储配置
  storage:
    persist_results: true
    retention_days: 30

  # DAG 配置
  dag:
    max_tasks: 1000
    max_depth: 50
```

### 环境变量

```bash
# 执行配置
export KNIGHT_TASK_MAX_PARALLEL=10
export KNIGHT_TASK_DEFAULT_TIMEOUT=300

# 重试配置
export KNIGHT_TASK_RETRY_MAX_ATTEMPTS=3
export KNIGHT_TASK_RETRY_DELAY=1000
```

---

## 示例

### 工作流定义

```yaml
# workflows/feature-development.yaml
workflow:
  name: "Feature Development"
  description: "从设计到部署的完整流程"

  variables:
    feature_name: string
    target_branch: string

  tasks:
    # 任务 1: 设计
    - name: design
      type: agent
      agent: architect
      inputs:
        requirement: "{{ feature_name }}"
      outputs:
        - design_doc

    # 任务 2: 实现（依赖设计）
    - name: implement
      type: agent
      agent: developer
      depends_on:
        - task_id: design
      inputs:
        design: "{{ design_doc }}"
      outputs:
        - implementation

    # 任务 3: 代码审查（依赖实现）
    - name: review
      type: skill
      skill: code-review
      depends_on:
        - task_id: implement
      inputs:
        code: "{{ implementation }}"
      run_if: "{{ target_branch != 'main' }}"

    # 任务 4: 测试（依赖实现）
    - name: test
      type: skill
      skill: test-runner
      depends_on:
        - task_id: implement
      inputs:
        code: "{{ implementation }}"
      outputs:
        - test_report

    # 任务 5: 部署（依赖审查和测试）
    - name: deploy
      type: agent
      agent: devops
      depends_on:
        - task_id: review
          condition: success
        - task_id: test
          condition: success
      inputs:
        implementation: "{{ implementation }}"
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| DAG 解析 | < 100ms | 1000 任务 |
| 任务调度延迟 | < 10ms | 内存操作 |
| 状态更新 | < 5ms | 数据库写入 |

### 错误处理

```yaml
error_codes:
  TASK_NOT_FOUND:
    code: 404
    message: "任务不存在"
    action: "检查任务 ID"

  DEPENDENCY_FAILED:
    code: 500
    message: "依赖任务失败"
    action: "查看依赖任务状态"

  WORKFLOW_FAILED:
    code: 500
    message: "工作流执行失败"
    action: "查看工作流日志"

  CIRCULAR_DEPENDENCY:
    code: 400
    message: "检测到循环依赖"
    action: "检查工作流定义"
```

### 测试策略

```yaml
test_plan:
  unit_tests:
    - DAG 解析
    - 依赖检查
    - 状态转换
    - 重试逻辑

  integration_tests:
    - 简单工作流
    - 复杂 DAG
    - 并行执行
    - 错误恢复

  edge_cases:
    - 循环依赖
    - 超时处理
    - 资源耗尽
    - 任务取消
