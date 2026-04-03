# Skill Engine (技能引擎)

## 概述

### 职责描述

Skill Engine 负责管理和执行技能（Skills），包括：

- 技能注册和发现
- 触发条件匹配（基于自然语言）
- 技能执行编排（LLM 驱动）
- 技能管道（Pipeline）组合与执行
- 参数传递和结果处理
- 技能间依赖管理

### 设计目标

1. **自然语言优先**: 通过 Markdown + 自然语言描述技能行为
2. **LLM 驱动执行**: 技能步骤由 LLM 根据自然语言描述决定执行方式
3. **灵活触发**: 支持关键词、文件变更、定时等多种触发方式
4. **可组合**: 技能可以调用其他技能（Skill calling Skill）
5. **可观测**: 完整的执行追踪和日志
6. **LLM 一致性**: Agent 调用 Skill 时使用与 Agent 一致的 LLM Provider

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Tool System | 依赖 | 工具调用 |
| LLM Provider | 依赖 | LLM 驱动执行解析 |
| Agent Runtime | 依赖 | AI 处理（agent 步骤直接调用） |
| Event Loop | 依赖 | 事件监听。Event 类型复用 [Event Loop 接口](../core/event-loop.md#事件)。订阅/发布接口见 [Event Loop 接口](../core/event-loop.md#监听器管理)。 |

---

## 接口定义

### 对外接口

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

  # ========== 技能执行 (LLM 驱动) ==========
  execute_skill:
    description: 执行技能（LLM 驱动：解析自然语言步骤 → 生成执行计划 → 执行）
    inputs:
      skill_id:
        type: string
        required: true
      context:
        type: SkillContext
        required: true
      llm_override:
        type: LLMConfig | null
        required: false
        description: |
          LLM 配置覆盖。
          - 直接执行时：使用 default_llm 或 null（使用默认 LLM Provider）
          - Agent 调用时：应传入 Agent 的 LLM Config 以保持一致性
    outputs:
      result:
        type: SkillResult
    notes:
      - "此方法涉及 LLM 解析，会先将自然语言步骤发送给 LLM 生成执行计划"
      - "LLM 选择规则：优先使用 llm_override，否则使用 context 中的 llm_config"
      - "Skill calling Skill 时，继承调用者的 LLM 配置"

  parse_skill_to_plan:
    description: LLM 解析技能步骤为执行计划（不执行）
    inputs:
      skill_content:
        type: string
        description: Markdown 格式技能定义
        required: true
      context:
        type: SkillContext
        required: true
      llm_config:
        type: LLMConfig
        required: true
    outputs:
      plan:
        type: ExecutionPlan
      confidence:
        type: float
        description: LLM 解析置信度 0-1

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
  create_pipeline:
    description: 创建技能管道（技能组合）
    inputs:
      name:
        type: string
        required: true
      skills:
        type: array<PipelineStep>
        required: true
    outputs:
      pipeline_id:
        type: string

  execute_pipeline:
    description: 执行技能管道
    inputs:
      pipeline_id:
        type: string
        required: true
      context:
        type: SkillContext
        required: true
    outputs:
      result:
        type: PipelineResult
```

### 数据结构

```yaml
# ========== LLM 配置 ==========

# LLM 配置
LLMConfig:
  provider:
    type: string
    description: LLM Provider 名称，如 "anthropic"、"openai"
    default: "default"
  model:
    type: string
    description: 模型名称，如 "claude-sonnet-4-20250514"
  max_tokens:
    type: integer
    description: 最大 token 数
    default: 4096
  temperature:
    type: float
    description: 温度参数
    default: 0.7

# ========== 核心数据结构 ==========

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
  content:
    type: string
    description: Markdown 格式的自然语言技能定义
  default_llm:
    type: LLMConfig | null
    description: 技能的默认 LLM 配置（可选）

# ========== LLM 驱动执行计划 ==========

# 执行计划（LLM 从自然语言步骤解析生成）
ExecutionPlan:
  skill_id:
    type: string
    description: 技能 ID
  generated_at:
    type: datetime
    description: 计划生成时间
  confidence:
    type: float
    description: 解析置信度 0-1
  llm_config_used:
    type: LLMConfig
    description: 生成此计划使用的 LLM 配置
  steps:
    type: array<PlanStep>
    description: 执行步骤列表
  validation:
    type: ValidationResult
    description: 计划验证结果
  original_content:
    type: string
    description: 原始自然语言内容（用于调试）

# 计划步骤
PlanStep:
  id:
    type: string
    description: 步骤 ID
  name:
    type: string
    description: 步骤名称
  reasoning:
    type: string
    description: LLM 对此步骤的推理说明
  action:
    type: Action
    description: 要执行的动作
  output_key:
    type: string
    description: 输出变量名，供后续步骤引用
  depends_on:
    type: array<string>
    description: 依赖的前置步骤 ID
  condition:
    type: Condition
    description: 执行条件
  parallel:
    type: array<PlanStep>
    description: 并行执行的子步骤（与 action 二选一）

# 动作类型（LLM 决定使用哪种）
Action:
  type:
    type: enum
    values: [tool, agent, skill, condition, loop]
    description: |
      动作类型：
      - tool: 调用工具
      - agent: 调用 Agent
      - skill: 调用其他技能（Skill calling Skill）
      - condition: 条件分支
      - loop: 循环执行
  tool:
    type: string
    description: 工具名称（type=tool 时）
  agent_id:
    type: string
    description: Agent ID 或变体（type=agent 时）
  skill_id:
    type: string
    description: 技能 ID（type=skill 时）
  prompt:
    type: string
    description: Agent prompt（type=agent 时）
  args:
    type: map<string, any>
    description: 工具/技能参数
  condition:
    type: Condition
    description: 条件表达式（type=condition 时）
  loop:
    type: LoopSpec
    description: 循环配置（type=loop 时）

# 条件定义
Condition:
  type:
    type: enum
    values: [always, success, failure, expression]
    description: |
      条件类型：
      - always: 无条件执行
      - success: 前置步骤成功时执行
      - failure: 前置步骤失败时执行
      - expression: 表达式条件
  expression:
    type: string
    description: 条件表达式（type=expression 时），如 "steps.lint.success == true"

# 循环配置
LoopSpec:
  over:
    type: string
    description: 循环变量，如 "context.files"
  max_iterations:
    type: integer
    description: 最大迭代次数
  continue_on_error:
    type: boolean
    default: false

# 计划验证结果
ValidationResult:
  valid:
    type: boolean
  errors:
    type: array<string>
    description: 验证错误列表
  warnings:
    type: array<string>
    description: 验证警告列表

# ========== 技能上下文 ==========

# 技能上下文
SkillContext:
  session_id:
    type: string
  agent_id:
    type: string | null
    description: 调用者 Agent ID（Agent 调用 Skill 时）
  trigger_event:
    type: Event | null
    description: 触发事件
  variables:
    type: map<string, any>
    description: 上下文变量
  inputs:
    type: map<string, any>
    description: 输入参数
  available_tools:
    type: array<string>
    description: 可用工具列表（用于 LLM 决策）
  available_agents:
    type: array<string>
    description: 可用 Agent 列表
  available_skills:
    type: array<string>
    description: 可用技能列表（用于 Skill calling Skill）
  llm_config:
    type: LLMConfig
    description: |
      当前使用的 LLM 配置。
      - 直接执行时：使用技能的默认 LLM 或系统默认
      - Agent 调用时：继承调用者的 LLM 配置

# ========== 执行结果 ==========

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
    description: 总执行时间（毫秒）
  steps_executed:
    type: array<StepResult>
    description: 执行步骤结果列表
  execution_plan:
    type: ExecutionPlan
    description: 使用的执行计划（用于调试）
  llm_config_used:
    type: LLMConfig
    description: 执行时使用的 LLM 配置

# 步骤执行结果
StepResult:
  step_id:
    type: string
    description: 步骤 ID
  step_name:
    type: string
    description: 步骤名称
  success:
    type: boolean
  started_at:
    type: datetime
    description: 开始时间
  duration_ms:
    type: integer
    description: 执行时长（毫秒）
  action_type:
    type: enum
    values: [tool, agent, skill, condition, loop, parallel]
    description: 执行的动作类型
  tool_name:
    type: string | null
    description: 使用的工具名称
  agent_id:
    type: string | null
    description: 使用的 Agent ID
  skill_id:
    type: string | null
    description: 调用的技能 ID
  output:
    type: any
    description: 步骤输出
  error:
    type: string | null
  error_code:
    type: string | null

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

# 管道步骤
PipelineStep:
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

# 管道结果
PipelineResult:
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

# ========== 触发器 ==========

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

# 事件
Event:
  $ref: ../core/event-loop.md#Event
  description: 复用 Event Loop 模块定义的 Event 类型。
```

### 配置选项

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
    enforce_timeout: true
    enforce_max_steps: true

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # 默认 LLM 配置
  default_llm:
    provider: "anthropic"
    model: "claude-sonnet-4-20250514"
    max_tokens: 4096
    temperature: 0.7

  # LLM 解析配置
  llm_parsing:
    retry: 3
    validation_enabled: true
```

---

## 核心流程

### LLM 驱动的技能执行流程

```
技能触发
    │
    ▼
┌──────────────────────────────┐
│ 1. 加载技能定义              │
│    - 读取 Markdown 内容      │
│    - 解析触发条件            │
│    - 提取执行步骤描述        │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 2. 确定 LLM 配置              │
│    - 优先使用 llm_override    │
│    - 否则使用 context.llm_config│
│    - Skill calling Skill 时   │
│      继承调用者的 LLM         │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 准备执行上下文            │
│    - 合并变量                │
│    - 解析输入参数            │
│    - 构建可用工具/Agent列表   │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. LLM 解析执行计划          │
│    - 使用确定的 LLM 配置     │
│    - 发送自然语言步骤给 LLM   │
│    - LLM 生成执行计划        │
│    - 验证计划有效性          │
│    - 检查 max_steps, timeout  │
└──────────────────────────────┘
    │
    ├─── 验证失败？ ───→ 返回错误
    │
    ▼
┌──────────────────────────────┐
│ 5. 执行计划                  │
│    - 按 DAG 顺序执行步骤     │
│    - 支持并行步骤            │
│    - 支持条件/循环           │
│    - 支持 Skill calling Skill│
│      （继承 LLM 配置）       │
│    - 收集步骤结果            │
│    - 强制超时和最大步骤数    │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 6. 保存输出                  │
│    - 汇总步骤结果            │
│    - 返回最终输出            │
└──────────────────────────────┘
```

### LLM 配置传递规则

```
┌─────────────────────────────────────────────────────────┐
│ LLM 配置优先级                                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ 1. llm_override（最高优先级）                          │
│    - Agent 调用 Skill 时传入                             │
│    - 用于保持 LLM 一致性                                 │
│                                                         │
│ 2. context.llm_config                                   │
│    - Skill 直接执行时使用                                │
│    - Skill calling Skill 时继承调用者                    │
│                                                         │
│ 3. SkillDefinition.default_llm                          │
│    - 技能自己定义的默认 LLM                             │
│                                                         │
│ 4. 系统 default_llm（最低优先级）                       │
│    - config/skill.yaml 中的配置                          │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### LLM 解析流程

```
输入：Markdown 技能定义
    │
    ▼
┌──────────────────────────────────────────┐
│ 确定 LLM 配置                            │
│ - llm_override > context.llm_config >    │
│   default_llm > 系统默认                  │
└──────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────┐
│ 构建 LLM Prompt                           │
│ - 系统提示：角色定义                      │
│ - 可用工具列表                            │
│ - 可用 Agent 列表                         │
│ - 可用 Skill 列表（用于 Skill calling）   │
│ - 自然语言执行步骤                        │
│ - 输出格式要求（JSON Schema）             │
└──────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────┐
│ 调用 LLM chat_completion                  │
│ - 使用确定的 LLM 配置                     │
└──────────────────────────────────────────┘
    │
    ├─── 解析失败？ ───→ 重试（最多3次）
    │
    ▼
┌──────────────────────────────────────────┐
│ 验证执行计划                              │
│ - 检查步骤完整性                          │
│ - 检查依赖循环                            │
│ - 检查工具/Agent/Skill 存在性             │
│ - 检查条件表达式语法                      │
└──────────────────────────────────────────┘
    │
    ├─── 验证失败？ ───→ 返回 INVALID_PLAN
    │
    ▼
输出：ExecutionPlan（含使用的 LLM 配置）
```

### 计划执行流程（DAG 执行）

```
ExecutionPlan.steps（DAG）
    │
    ▼
┌──────────────────────────────┐
│ 拓扑排序 + 并行识别          │
│ - 计算步骤依赖图              │
│ - 识别可并行步骤              │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 执行就绪步骤                  │
│ (所有依赖已完成且条件满足)    │
│ 继承当前 LLM 配置用于解析    │
└──────────────────────────────┘
    │
    ├─── 并行步骤？ ───→ 并行执行
    │
    ▼
┌──────────────────────────────┐
│ 执行单个步骤                  │
│    │                         │
│    ├─→ type=tool → 调用工具  │
│    ├─→ type=agent → 调用Agent│
│    ├─→ type=skill → 递归执行  │
│    │   （继承 LLM 配置）      │
│    ├─→ type=condition → 条件  │
│    └─→ type=loop → 循环      │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 更新上下文 + 记录结果         │
│ - 保存输出到 variables        │
│ - 记录 StepResult            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 检查是否全部完成              │
│    │                         │
│    ├─── 是 → 返回结果         │
│    └─── 否 → 继续执行就绪步骤 │
└──────────────────────────────┘
```

---

## 技能定义格式

### Markdown 自然语言格式

每个技能是一个 Markdown 文件，包含以下部分：

```markdown
---
name: code-review
category: quality
tags: [code, review, quality]
description: 代码审查技能，自动分析代码质量和安全性
author: knight-agent
version: 1.0.0
llm:                              # 可选：指定技能的默认 LLM
  provider: anthropic
  model: claude-sonnet-4-20250514
---

# Code Review Skill

## 概述

当检测到代码文件变更时，自动执行代码审查。分析代码的质量、安全性、性能和可维护性。

## 触发条件

当以下条件满足时触发：
- 关键词匹配：`review`、`审查`、`代码审查`
- 文件变更：`**/*.ts`, `**/*.tsx`, `**/*.js`, `**/*.jsx`

## 输入参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| files | array | 否 | 要审查的文件列表，默认为变更的文件 |

## 执行步骤

### 步骤 1: 收集文件

首先收集所有需要审查的代码文件。使用 glob 工具收集 TypeScript 和 JavaScript 文件。

输入：
- `files`: 要审查的文件（来自上下文或变更检测）

输出：
- `collected_files`: 收集到的文件列表

### 步骤 2: 运行 Lint 检查

运行代码 lint 检查，发现潜在的代码风格问题。

输入：
- `command`: "npm run lint"

输出：
- `lint_result`: Lint 检查结果

### 步骤 3: AI 代码分析

使用 AI 分析代码的质量、安全性和性能问题。

输入：
- `task`: "请分析以下代码的质量、安全性和性能问题"
- `files`: 来自步骤 1 的文件列表
- `lint_result`: 来自步骤 2 的结果

输出：
- `analysis`: AI 分析结果

### 步骤 4: 生成审查报告

将分析结果整理成审查报告并保存。

输入：
- `analysis`: 来自步骤 3 的分析结果

输出：
- `report`: Markdown 格式的审查报告
```

---

## Skill vs Workflow 边界

### 设计原则

| 特性 | Skill | Workflow |
|------|-------|----------|
| **定义格式** | Markdown 自然语言 | Markdown 自然语言 |
| **执行驱动** | LLM 驱动 | LLM 驱动 |
| **触发方式** | 关键词/事件/定时 | CLI 命令 `/workflow` |
| **执行时长** | 秒级~分钟级 | 分钟级~天级 |
| **状态持久化** | 不持久化 | 持久化，支持断点恢复 |
| **LLM 一致性** | Agent 调用时继承 Agent LLM | 独立 LLM 配置 |
| **使用场景** | 自动化任务、可复用行为 | 复杂多阶段流程 |
| **Agent 引用** | `agent_id: xxx` | `Agent xxx (variant)` |

### 关系

- **Skill 可以调用 Workflow**：通过 `type: skill` 动作，传入 Workflow ID
- **Workflow 可以包含 Skill**：Workflow 的步骤可以是 Skill 调用
- **共同点**：都使用 Markdown 自然语言格式，LLM 驱动执行

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Skill Engine                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Registry  │  │Trigger   │  │Executor ││
│  │          │  │Matcher   │  │(LLM)   ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Tool      │  │Event     │  │LLM       │
│System    │  │Loop      │  │Provider  │
└──────────┘  └──────────┘  └──────────┘
                     │
                     ▼
              ┌──────────┐
              │Agent     │
              │Runtime   │
              └──────────┘
```

### 消息流

```
事件触发
    │
    ▼
┌─────────────────────────────┐
│ Skill Engine                │
│ - 匹配触发器                │
│ - 加载技能（Markdown）      │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ 确定 LLM 配置                │
│ - llm_override >            │
│   context.llm_config         │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ LLM Provider                │
│ - 解析自然语言执行步骤      │
│ - 生成执行计划              │
│ - 验证计划有效性            │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ 执行技能步骤                │
│ - 调用工具或 Agent          │
│ - 调用其他 Skill（递归）    │
│   继承 LLM 配置             │
│ - 收集执行结果              │
└─────────────────────────────┘
    │
    ├─────────────────────────────┐
    │                             │
    ▼                             ▼
┌─────────────────┐     ┌─────────────────┐
│ Tool System     │     │ Agent Runtime    │
│ - 执行工具      │     │ - AI 处理       │
└─────────────────┘     └─────────────────┘
    │                             │
    │                             ▼
    │                     ┌─────────────────┐
    │                     │ Skill Engine    │
    │                     │ (递归调用)      │
    │                     │ 继承 LLM 配置   │
    │                     └─────────────────┘
    └────────────┬────────────────┘
                 ▼
          返回执行结果
```

---

## 技能定义示例

### 代码审查技能

```markdown
---
name: code-review
category: quality
description: 代码审查技能，分析代码质量和安全性
---

# Code Review Skill

## 概述

自动分析代码的质量、安全性和性能。适用于代码提交审查或手动触发的代码检查。

## 触发条件

- 关键词：`review`、`审查`、`代码审查`
- 文件变更：`**/*.ts`, `**/*.tsx`, `**/*.js`

## 输入参数

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| files | array | 否 | 文件列表，默认全部 |

## 执行步骤

### 步骤 1: 收集文件

使用 glob 工具收集所有 TypeScript/JavaScript 文件。

输入：
- `pattern`: "**/*.{ts,tsx,js,jsx}"

输出：
- `collected_files`: 文件列表

### 步骤 2: 运行 Lint

执行 npm run lint 检查代码风格问题。

输入：
- `command`: "npm run lint"

输出：
- `lint_result`: Lint 检查结果

### 步骤 3: AI 分析

使用 AI 分析代码的质量、安全性和性能。

输入：
- `files`: 来自步骤 1 的文件列表
- `lint_result`: 来自步骤 2 的结果
- `task`: "分析这些代码的质量、安全性和性能问题，给出具体的改进建议"

输出：
- `analysis`: AI 分析结果

### 步骤 4: 生成报告

将分析结果保存为 Markdown 报告。

输入：
- `analysis`: 来自步骤 3 的分析结果
- `filename`: "reports/code-review-{{ timestamp }}.md"
```

### CI/CD 技能

```markdown
---
name: ci-pipeline
category: automation
description: CI/CD 流水线技能
---

# CI/CD Pipeline Skill

## 概述

当检测到 Git push 事件时，自动执行 CI/CD 流水线。包括测试、构建和部署。

## 触发条件

- 事件类型：`git.push`
- 分支：`main` 或 `release/*`

## 执行步骤

### 步骤 1: 并行测试

同时运行单元测试和集成测试。

输入：
- `task`: "运行所有单元测试和集成测试"

### 步骤 2: 条件构建

如果测试通过，执行构建。

输入：
- `task`: "执行项目构建"
- `condition`: "steps.parallel_tests.success == true"

### 步骤 3: 条件部署

如果构建成功，执行部署。

输入：
- `task`: "部署到 staging 环境"
- `condition`: "steps.build.success == true"
```

### Skill calling Skill 示例

```markdown
---
name: full-code-review
category: quality
description: 完整代码审查，包含代码审查和安全审查
---

# Full Code Review Skill

## 执行步骤

### 步骤 1: 代码审查

调用 code-review 技能进行标准代码审查。

输入：
- `files`: {{ context.files }}

输出：
- `code_review_result`: 代码审查结果

### 步骤 2: 安全审查

调用 security-review 技能进行安全审查。

输入：
- `files`: {{ context.files }}

输出：
- `security_review_result`: 安全审查结果

### 步骤 3: 生成综合报告

汇总两个审查的结果。
```

---

## 配置与部署

### 配置文件格式

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
    enforce_timeout: true
    enforce_max_steps: true

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # 默认 LLM 配置
  default_llm:
    provider: "anthropic"
    model: "claude-sonnet-4-20250514"
    max_tokens: 4096
    temperature: 0.7

  # LLM 解析配置
  llm_parsing:
    retry: 3
    validation_enabled: true
```

### 环境变量

```bash
# 技能目录
export KNIGHT_SKILL_DIRS="./skills:~/.knight-agent/skills"

# 执行限制
export KNIGHT_SKILL_MAX_STEPS=100
export KNIGHT_SKILL_TIMEOUT=600

# 触发器配置
export KNIGHT_TRIGGER_DEBOUNCE=500

# 默认 LLM
export KNIGHT_LLM_MODEL="claude-sonnet-4-20250514"
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 技能加载 | < 50ms | 单个技能 |
| 触发器匹配 | < 1ms | 单次匹配 |
| LLM 解析步骤 | < 2s | 生成执行计划 |
| 步骤执行 | < 5s | 简单步骤 |

### 错误处理

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

  LLM_PARSING_FAILED:
    code: 500
    message: "LLM 解析步骤失败"
    action: "检查自然语言步骤描述是否清晰"

  INVALID_EXECUTION_PLAN:
    code: 400
    message: "LLM 生成执行计划无效"
    action: "检查步骤描述完整性和依赖循环"

  SKILL_RECURSION_DEPTH:
    code: 400
    message: "技能递归调用深度超限"
    action: "检查 Skill calling Skill 是否存在循环"

  SKILL_NOT_AVAILABLE:
    code: 404
    message: "被调用的技能不存在"
    action: "检查 available_skills 列表"
```

### 内置技能

| 技能 | 类别 | 描述 |
|------|------|------|
| tdd-pipeline | development | TDD 开发流程 |
| code-review | quality | 代码审查 |
| security-review | security | 安全检查 |
| ci-pipeline | automation | CI/CD 流水线 |
| deploy | operations | 部署技能 |
| notify | communication | 通知技能 |

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 1.1.0 | 2026-04-01 | 重命名"工作流"为"管道"(Pipeline) |
| 1.2.0 | 2026-04-03 | 改为 LLM 驱动的自然语言格式 |
| 1.3.0 | 2026-04-03 | 完善 LLM 驱动设计：LLM 一致性规则（Agent 调用 Skill 时继承 LLM）；添加 ExecutionPlan、Action、Condition 等完整数据结构；明确 Skill calling Skill；添加 LLM 解析接口；添加执行计划验证 |
