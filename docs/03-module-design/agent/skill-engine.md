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
4. **可组合**: 技能可以调用其他技能
5. **可观测**: 完整的执行追踪和日志

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Tool System | 依赖 | 工具调用 |
| LLM Provider | 依赖 | LLM 驱动执行解析 |
| Agent Runtime | 依赖 | AI 处理（agent 步骤直接调用） |
| Event Loop | 依赖 | 事件监听 |

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
    type: array<StepResult>
    description: 执行步骤结果列表

# 步骤执行结果
StepResult:
  step_name:
    type: string
  success:
    type: boolean
  tool_used:
    type: string | null
    description: 使用的工具（如果是工具调用）
  agent_used:
    type: string | null
    description: 使用的 Agent（如果是 Agent 调用）
  output:
    type: any
    description: 步骤输出
  error:
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

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # LLM 驱动配置
  llm:
    model: "claude-sonnet-4-20250514"
    max_tokens: 4096
    temperature: 0.7
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
│ 2. 准备执行上下文            │
│    - 合并变量                │
│    - 解析输入参数            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. LLM 解析执行计划          │
│    - 将自然语言步骤发送给 LLM │
│    - LLM 生成执行计划        │
│    - 决定使用哪些工具/Agent  │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. 执行步骤                  │
│    - 按计划调用工具/Agent    │
│    - 收集执行结果            │
│    - 更新上下文变量          │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 5. 保存输出                  │
│    - 汇总步骤结果            │
│    - 返回最终输出            │
└──────────────────────────────┘
```

### LLM 执行计划生成

LLM 根据自然语言步骤描述生成执行计划：

```yaml
# LLM 生成的执行计划
execution_plan:
  steps:
    - name: "收集文件"
      reasoning: "需要先找到所有 TypeScript 文件"
      action:
        type: tool
        tool: glob
        args:
          pattern: "**/*.ts"
      output_key: files

    - name: "运行 Lint"
      reasoning: "需要先检查代码风格问题"
      depends_on: ["收集文件"]
      action:
        type: tool
        tool: bash
        args:
          command: "npm run lint"
      output_key: lint_result

    - name: "AI 分析代码"
      reasoning: "需要 LLM 分析代码质量和安全问题"
      depends_on: ["收集文件"]
      action:
        type: agent
        agent_id: code-reviewer
        prompt: "分析以下代码的质量、安全性和性能问题..."
      output_key: analysis
```

### 触发器匹配流程

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

首先收集所有需要审查的代码文件。

输入：
- `files`: 要审查的文件（来自上下文或变更检测）

### 步骤 2: 运行 Lint 检查

运行代码 lint 检查，发现潜在的代码风格问题。

输入：
- `command`: "npm run lint"

### 步骤 3: AI 代码分析

使用 AI 分析代码的质量、安全性和性能问题。

输入：
- `task`: "请分析以下代码的质量、安全性和性能问题"
- `context`: 代码文件列表和 lint 结果

### 步骤 4: 生成审查报告

将分析结果整理成审查报告。

输出：
- `report`: Markdown 格式的审查报告
```

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
│ LLM Provider                │
│ - 解析自然语言执行步骤      │
│ - 生成执行计划              │
└─────────────────────────────┘
    │
    ▼
┌─────────────────────────────┐
│ 执行技能步骤                │
│ - 调用工具或 Agent          │
│ - 收集执行结果              │
└─────────────────────────────┘
    │
    ├─────────────────────────────┐
    │                             │
    ▼                             ▼
┌─────────────────┐     ┌─────────────────┐
│ Tool System     │     │ Agent Runtime   │
│ - 执行工具      │     │ - AI 处理       │
└─────────────────┘     └─────────────────┘
    │                             │
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
- `files`: 文件列表

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

### 步骤 2: 构建

如果测试通过，执行构建。

输入：
- `task`: "执行项目构建"
- `condition`: 所有测试必须通过

### 步骤 3: 部署

如果构建成功，执行部署。

输入：
- `task`: "部署到 staging 环境"
- `condition`: 构建必须成功
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

  # 触发器配置
  triggers:
    debounce: 500
    max_queue_size: 1000

  # LLM 驱动配置
  llm:
    model: "claude-sonnet-4-20250514"
    max_tokens: 4096
    temperature: 0.7
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

# LLM 配置
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
