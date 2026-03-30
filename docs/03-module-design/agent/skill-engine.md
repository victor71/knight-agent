# Skill Engine (技能引擎)

## 1. 概述

### 1.1 职责描述

Skill Engine 负责管理和执行技能（Skills），包括：

- 技能注册和发现
- 触发条件匹配
- 技能执行编排
- 参数传递和结果处理
- 技能间依赖管理

### 1.2 设计目标

1. **声明式定义**: 通过 YAML/Markdown 定义技能行为
2. **灵活触发**: 支持关键词、文件变更、定时等多种触发方式
3. **可组合**: 技能可以调用其他技能
4. **可观测**: 完整的执行追踪和日志

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Tool System | 依赖 | 工具调用 |
| Agent Runtime | 依赖 | AI 处理 |
| Event Loop | 依赖 | 事件监听 |
| Orchestrator | 依赖 | 任务调度 |

---

## 2. 接口定义

### 2.1 对外接口

```yaml
# Skill Engine 接口定义
SkillEngine:
  # ========== 技能管理 ==========
  register_skill:
    description: 注册技能
    inputs:
      skill:
        type: SkillDefinition
        required: true
    outputs:
      skill_id:
        type: string

  unregister_skill:
    description: 注销技能
    inputs:
      skill_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_skills:
    description: 列出技能
    inputs:
      category:
        type: string
        description: 类别过滤
        required: false
      enabled_only:
        type: boolean
        description: 仅显示已启用
        required: false
    outputs:
      skills:
        type: array<SkillInfo>

  get_skill:
    description: 获取技能详情
    inputs:
      skill_id:
        type: string
        required: true
    outputs:
      skill:
        type: SkillDefinition | null

  enable_skill:
    description: 启用技能
    inputs:
      skill_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  disable_skill:
    description: 禁用技能
    inputs:
      skill_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 技能执行 ==========
  execute_skill:
    description: 直接执行技能
    inputs:
      skill_id:
        type: string
        required: true
      context:
        type: SkillContext
        required: true
    outputs:
      result:
        type: SkillResult

  trigger_skill:
    description: 尝试触发技能（自动匹配）
    inputs:
      event:
        type: Event
        required: true
    outputs:
      triggered_skills:
        type: array<string>

  # ========== 触发器管理 ==========
  register_trigger:
    description: 注册触发器
    inputs:
      skill_id:
        type: string
        required: true
      trigger:
        type: Trigger
        required: true
    outputs:
      trigger_id:
        type: string

  remove_trigger:
    description: 移除触发器
    inputs:
      trigger_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 技能编排 ==========
  create_workflow:
    description: 创建工作流（技能组合）
    inputs:
      name:
        type: string
        required: true
      skills:
        type: array<WorkflowStep>
        required: true
    outputs:
      workflow_id:
        type: string

  execute_workflow:
    description: 执行工作流
    inputs:
      workflow_id:
        type: string
        required: true
      context:
        type: SkillContext
        required: true
    outputs:
      result:
        type: WorkflowResult
```

### 2.2 数据结构

```yaml
# 技能定义
SkillDefinition:
  id:
    type: string
  name:
    type: string
  display_name:
    type: string
  description:
    type: string
  category:
    type: string
    description: 技能类别
  version:
    type: string
  enabled:
    type: boolean
    default: true
  triggers:
    type: array<Trigger>
    description: 触发器列表
  steps:
    type: array<SkillStep>
    description: 执行步骤
  variables:
    type: map<string, any>
    description: 技能变量
  permissions:
    type: array<string>
    description: 所需权限

# 触发器
Trigger:
  id:
    type: string
  type:
    type: enum
    values: [keyword, file_change, schedule, event, manual]
    description: 触发器类型
  enabled:
    type: boolean
    default: true

  # 关键词触发
  keyword:
    type: object
    description: 关键词配置
    properties:
      patterns:
        type: array<string>
        description: 关键词模式列表
      match_type:
        type: string
        enum: [exact, contains, regex]
        description: 匹配方式

  # 文件变更触发
  file_change:
    type: object
    properties:
      patterns:
        type: array<string>
        description: 文件模式（glob）
      events:
        type: array<string>
        enum: [created, modified, deleted]
        description: 监听事件
      debounce:
        type: integer
        description: 防抖延迟（毫秒）

  # 定时触发
  schedule:
    type: object
    properties:
      cron:
        type: string
        description: Cron 表达式
      timezone:
        type: string
        description: 时区

  # 事件触发
  event:
    type: object
    properties:
      event_type:
        type: string
        description: 事件类型
      filter:
        type: map<string, any>
        description: 事件过滤条件

# 技能步骤
SkillStep:
  id:
    type: string
  name:
    type: string
  type:
    type: enum
    values: [tool, agent, skill, condition, parallel, loop]
    description: 步骤类型

  # 工具调用
  tool:
    type: object
    properties:
      name:
        type: string
      args:
        type: object
      output:
        type: string
        description: 输出变量名

  # Agent 调用
  agent:
    type: object
    properties:
      agent_id:
        type: string
      prompt:
        type: string
      output:
        type: string

  # 技能调用
  skill:
    type: object
    properties:
      skill_id:
        type: string
      args:
        type: object

  # 条件分支
  condition:
    type: object
    properties:
      expression:
        type: string
        description: 条件表达式
      then_steps:
        type: array<SkillStep>
      else_steps:
        type: array<SkillStep>

  # 并行执行
  parallel:
    type: array<SkillStep>
    description: 并行执行的步骤

  # 循环
  loop:
    type: object
    properties:
      over:
        type: string
        description: 循环变量
      steps:
        type: array<SkillStep>

  # 错误处理
  on_error:
    type: object
    properties:
      continue:
        type: boolean
      retry:
        type: integer
      fallback:
        type: array<SkillStep>

# 技能上下文
SkillContext:
  session_id:
    type: string
  agent_id:
    type: string
  trigger_event:
    type: Event | null
    description: 触发事件
  variables:
    type: map<string, any>
    description: 上下文变量
  inputs:
    type: map<string, any>
    description: 输入参数

# 技能结果
SkillResult:
  success:
    type: boolean
  outputs:
    type: map<string, any>
    description: 输出变量
  error:
    type: string | null
  error_code:
    type: string | null
  execution_time:
    type: integer
    description: 执行时间（毫秒）
  steps_executed:
    type: integer

# 技能信息
SkillInfo:
  id:
    type: string
  name:
    type: string
  display_name:
    type: string
  description:
    type: string
  category:
    type: string
  enabled:
    type: boolean
  trigger_count:
    type: integer
  execution_count:
    type: integer
  last_executed:
    type: datetime | null

# 工作流步骤
WorkflowStep:
  skill_id:
    type: string
  name:
    type: string
  depends_on:
    type: array<string>
    description: 依赖的前置步骤
  args:
    type: map<string, any>
  condition:
    type: string | null
    description: 执行条件

# 工作流结果
WorkflowResult:
  success:
    type: boolean
  completed_steps:
    type: array<string>
  failed_steps:
    type: array<string>
  outputs:
    type: map<string, any>
  execution_time:
    type: integer

# 事件
Event:
  type:
    type: string
  timestamp:
    type: datetime
  source:
    type: string
  data:
    type: object
```

### 2.3 配置选项

```yaml
# config/skill.yaml
skill:
  # 技能目录
  directories:
    - "./skills"
    - "~/.knight-agent/skills"

  # 执行配置
  execution:
    max_steps: 100
    timeout: 600
    parallel_steps: 5

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # 错误处理
  error_handling:
    max_retries: 3
    retry_delay: 1000
    continue_on_error: false
```

---

## 3. 核心流程

### 3.1 技能执行流程

```
技能触发
        │
        ▼
┌──────────────────────────────┐
│ 1. 加载技能定义              │
│    - 验证技能存在            │
│    - 检查是否启用            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 准备执行上下文            │
│    - 合并变量                │
│    - 解析参数                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 依次执行步骤              │
│    for step in steps:        │
│      - 解析步骤类型          │
│      - 执行步骤              │
│      - 处理结果              │
│      - 更新上下文            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 4. 错误处理      │   │ 4. 保存输出  │
│    - 检查重试    │   │    返回结果  │
│    - 执行回退    │   └──────────────┘
└──────────────────┘
```

### 3.2 触发器匹配流程

```
事件到达
        │
        ▼
┌──────────────────────────────┐
│ 1. 遍历所有触发器            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 检查触发器类型            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────────────────────────┐
    │                            │
    ▼                            ▼
┌──────────────┐        ┌──────────────┐
│ 关键词匹配   │        │ 文件变更     │
│ - 模式匹配   │        │ - glob 匹配  │
└──────────────┘        │ - 事件类型   │
        │               └──────────────┘
        │                        │
        ▼                        ▼
┌──────────────┐        ┌──────────────┐
│ 定时触发     │        │ 事件过滤     │
│ - cron 匹配  │        │ - 条件匹配   │
└──────────────┘        └──────────────┘
        │                        │
        └────────────┬───────────┘
                     ▼
            ┌──────────────┐
            │ 触发技能     │
            └──────────────┘
```

### 3.3 步骤执行类型

#### 工具调用步骤

```yaml
type: tool
tool:
  name: read
  args:
    file_path: "{{ context.input_file }}"
  output: file_content
```

#### Agent 调用步骤

```yaml
type: agent
agent:
  agent_id: code-reviewer
  prompt: |
    请分析以下代码：
    {{ context.file_content }}
  output: review_result
```

#### 条件分支步骤

```yaml
type: condition
condition:
  expression: "{{ context.test_result == 'passed' }}"
  then_steps:
    - type: tool
      tool: {name: deploy}
  else_steps:
    - type: tool
      tool: {name: notify_failure}
```

#### 并行执行步骤

```yaml
type: parallel
parallel:
  - type: skill
    skill: {skill_id: test-unit}
  - type: skill
    skill: {skill_id: test-integration}
  - type: skill
    skill: {skill_id: lint}
```

#### 循环步骤

```yaml
type: loop
loop:
  over: context.files
  steps:
    - type: agent
      agent:
        prompt: "分析 {{ item }}"
```

---

## 4. 模块交互

### 4.1 依赖关系图

```
┌─────────────────────────────────────────┐
│            Skill Engine                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Registry  │  │Trigger   │  │Executor││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Tool      │  │Agent     │  │Event     │
│System    │  │Runtime   │  │Loop      │
└──────────┘  └──────────┘  └──────────┘
```

### 4.2 消息流

```
事件触发
    │
    ▼
┌─────────────────────────────┐
│ Skill Engine                │
│ - 匹配触发器                │
│ - 加载技能                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ 执行技能步骤                │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ Tool System     │         │ Agent Runtime   │
│ - 执行工具      │         │ - AI 处理       │
└─────────────────┘         └─────────────────┘
        │                             │
        └────────────┬────────────────┘
                     ▼
              返回执行结果
```

---

## 5. 技能定义示例

### 5.1 代码审查技能

```yaml
# skills/code-review/SKILL.md
---
name: code-review
category: quality
triggers:
  - type: keyword
    keyword:
      patterns: ["review", "审查"]
      match_type: contains
  - type: file_change
    file_change:
      patterns: ["**/*.ts", "**/*.tsx"]
      events: [modified]
---

## Steps

### Step 1: 收集文件
```yaml
type: tool
tool:
  name: glob
  args:
    pattern: "**/*.ts"
  output: files
```

### Step 2: 运行 Lint
```yaml
type: tool
tool:
  name: bash
  args:
    command: npm run lint
  output: lint_result
  on_error:
    continue: true
```

### Step 3: AI 分析
```yaml
type: agent
agent:
  prompt: |
    分析以下代码的质量：
    Files: {{ context.files }}
    Lint: {{ context.lint_result }}
  output: analysis
```

### Step 4: 生成报告
```yaml
type: tool
tool:
  name: write
  args:
    path: "reports/review-{{ timestamp }}.md"
    content: |
      # Code Review Report
      {{ context.analysis }}
```
```

### 5.2 CI/CD 技能

```yaml
# skills/ci-pipeline/SKILL.md
---
name: ci-pipeline
category: automation
triggers:
  - type: event
    event:
      event_type: git.push
      filter:
        branch: main
---

## Steps

### Step 1: 并行测试
```yaml
type: parallel
parallel:
  - type: skill
    skill: {skill_id: test-unit}
  - type: skill
    skill: {skill_id: test-integration}
  - type: skill
    skill: {skill_id: lint}
```

### Step 2: 条件部署
```yaml
type: condition
condition:
  expression: "{{ context.all_tests_passed == true }}"
  then_steps:
    - type: skill
      skill: {skill_id: deploy}
  else_steps:
    - type: tool
      tool:
        name: notify
        args: {message: "CI Failed"}
```
```

---

## 6. 配置与部署

### 6.1 配置文件格式

```yaml
# config/skill.yaml
skill:
  # 技能目录
  directories:
    - "./skills"
    - "~/.knight-agent/skills"

  # 执行配置
  execution:
    max_steps: 100
    timeout: 600
    parallel_steps: 5

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # 错误处理
  error_handling:
    max_retries: 3
    retry_delay: 1000
    continue_on_error: false
```

### 6.2 环境变量

```bash
# 技能目录
export KNIGHT_SKILL_DIRS="./skills:~/.knight-agent/skills"

# 执行限制
export KNIGHT_SKILL_MAX_STEPS=100
export KNIGHT_SKILL_TIMEOUT=600

# 触发器配置
export KNIGHT_TRIGGER_DEBOUNCE=500
```

---

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 技能加载 | < 50ms | 单个技能 |
| 触发器匹配 | < 1ms | 单次匹配 |
| 步骤执行 | < 5s | 简单步骤 |

### 7.2 错误处理

```yaml
error_codes:
  SKILL_NOT_FOUND:
    code: 404
    message: "技能不存在"
    action: "检查技能 ID"

  STEP_EXECUTION_FAILED:
    code: 500
    message: "步骤执行失败"
    action: "查看步骤详情"

  TRIGGER_INVALID:
    code: 400
    message: "触发器配置无效"
    action: "检查触发器配置"

  SKILL_TIMEOUT:
    code: 408
    message: "技能执行超时"
    action: "增加超时时间或优化技能"
```

### 7.3 内置技能

| 技能 | 类别 | 描述 |
|------|------|------|
| tdd-workflow | development | TDD 开发流程 |
| code-review | quality | 代码审查 |
| security-review | security | 安全检查 |
| ci-pipeline | automation | CI/CD 流水线 |
| deploy | operations | 部署技能 |
| notify | communication | 通知技能 |
