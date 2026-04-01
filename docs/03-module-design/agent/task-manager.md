# Task Manager (任务管理器)

## 概述

### 职责描述

Task Manager 负责任务的生命周期管理和 DAG 依赖解析，包括：

- 任务创建和状态管理
- 依赖关系解析（DAG）
- 任务调度和执行
- 任务结果存储和查询
- 重试和错误处理

### 设计目标

1. **依赖管理**: 支持复杂的任务依赖关系
2. **并行执行**: 自动识别可并行执行的任务
3. **容错机制**: 自动重试和错误恢复
4. **可追踪**: 完整的任务执行历史

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Orchestrator | 依赖 | 分配任务到 Agent |
| Agent Runtime | 依赖 | 执行任务 |
| Storage Service | 依赖 | 持久化任务状态 |

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

  # DAG 配置
  dag:
    max_tasks: 1000
    max_depth: 50
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
