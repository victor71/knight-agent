# Agent Variants (Agent 变体系统)

## 1. 概述

### 1.1 职责描述

Agent Variants 系统负责管理同一 Agent 的不同配置版本，包括：

- 变体定义文件的加载和解析
- 变体继承机制
- 变体发现和列表
- 变体配置合并
- CLI 变体语法支持

### 1.2 设计目标

1. **灵活配置**: 支持同一 Agent 的多种场景配置
2. **简单定义**: 使用文件命名约定，无需复杂配置
3. **最小重复**: 通过继承复用基础定义
4. **并行共存**: 多个变体同时可用

### 1.3 变体 vs 版本

| 维度 | 变体 (Variant) | 版本 (Version) |
|------|----------------|----------------|
| **目的** | 不同场景不同配置 | 功能升级演进 |
| **存在方式** | 并行共存 | 先后替换 |
| **命名示例** | `quick`, `full`, `security` | `1.0.0`, `1.1.0`, `2.0.0` |
| **切换方式** | `agent:variant` | `--version x.y.z` |
| **兼容性** | 可能差异很大 | 向后兼容 |
| **适用阶段** | 即时需要 | 成熟后 |
| **优先级** | **P1** | P3 |

### 1.4 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Agent Runtime | 依赖 | 创建 Agent 实例 |
| Storage Service | 依赖 | 读取 Agent 定义文件 |

---

## 2. 接口定义

### 2.1 对外接口

```yaml
# Agent Variants 接口定义
AgentVariants:
  # ========== 变体加载 ==========
  load_agent_definition:
    description: 加载 Agent 定义（支持变体）
    inputs:
      agent_id:
        type: string
        required: true
        description: Agent ID
      variant:
        type: string | null
        required: false
        description: 变体名称，null 表示默认
    outputs:
      definition:
        type: AgentDefinition

  load_variant_file:
    description: 加载指定变体文件
    inputs:
      agent_id:
        type: string
        required: true
      variant:
        type: string
        required: true
    outputs:
      definition:
        type: AgentDefinition

  # ========== 变体发现 ==========
  list_variants:
    description: 列出 Agent 的所有可用变体
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      variants:
        type: array<VariantInfo>

  list_all_agents_with_variants:
    description: 列出所有 Agent 及其变体
    outputs:
      agents:
        type: array<AgentVariantInfo>

  get_variant_info:
    description: 获取变体详细信息
    inputs:
      agent_id:
        type: string
        required: true
      variant:
        type: string
        required: true
    outputs:
      info:
        type: VariantInfo

  # ========== 变体验证 ==========
  validate_variant:
    description: 验证变体定义是否合法
    inputs:
      agent_id:
        type: string
        required: true
      variant:
        type: string
        required: true
    outputs:
      result:
        type: ValidationResult

  validate_all_variants:
    description: 验证 Agent 的所有变体
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      results:
        type: map<string, ValidationResult>

  # ========== 变体管理 ==========
  create_variant:
    description: 创建新变体文件
    inputs:
      agent_id:
        type: string
        required: true
      variant:
        type: string
        required: true
      definition:
        type: AgentVariantDefinition
        required: true
    outputs:
      success:
        type: boolean

  delete_variant:
    description: 删除变体文件
    inputs:
      agent_id:
        type: string
        required: true
      variant:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 变体解析 ==========
  resolve_variant:
    description: 解析变体引用
    inputs:
      agent_ref:
        type: string
        description: Agent 引用 (e.g., "code-reviewer" or "code-reviewer:quick")
        required: true
    outputs:
      resolved:
        type: ResolvedAgentRef
```

### 2.2 数据结构

```yaml
# Agent 定义
AgentDefinition:
  id:
    type: string
    description: Agent 唯一标识
  name:
    type: string
    description: Agent 名称
  version:
    type: string
    description: Agent 版本
  role:
    type: string
    description: Agent 角色描述

  # 模型配置
  model:
    type: ModelConfig
    description: 模型配置

  # 系统指令
  instructions:
    type: string
    description: 系统指令

  # 能力
  tools:
    type: array<string>
    description: 可用工具列表
  skills:
    type: array<string>
    description: 可用技能列表
  capabilities:
    type: array<string>
    description: 能力列表

  # 权限
  permissions:
    type: PermissionConfig

  # 变体相关
  extends:
    type: string | null
    description: 继承的基础定义文件名
  variant:
    type: string | null
    description: 变体名称
  variants:
    type: array<AgentVariant>
    description: 支持的变体列表

# Model 配置
ModelConfig:
  provider:
    type: string
    description: 提供者 (anthropic/openai/custom)
  model:
    type: string
    description: 模型名称
  temperature:
    type: float
    description: 温度参数
    default: 0.7
  max_tokens:
    type: integer
    description: 最大输出 Token
    default: 4096

# Agent 变体定义
AgentVariantDefinition:
  variant:
    type: string
    description: 变体名称
  extends:
    type: string | null
    description: 继承的基础定义
  name:
    type: string | null
    description: 覆盖的显示名称
  role:
    type: string | null
    description: 覆盖的角色描述
  model:
    type: ModelConfig | null
    description: 覆盖的模型配置
  instructions:
    type: string | null
    description: 追加/覆盖的系统指令
  tools:
    type: array<string> | null
    description: 覆盖的工具列表
  skills:
    type: array<string> | null
    description: 覆盖的技能列表
  capabilities:
    type: array<string> | null
    description: 覆盖的能力列表

# Agent 变体信息
AgentVariant:
  name:
    type: string
    description: 变体名称
  model:
    type: ModelConfig | null
    description: 覆盖的模型配置
  instructions:
    type: string | null
    description: 覆盖的系统指令

# 变体信息
VariantInfo:
  variant:
    type: string
    description: 变体名称
  file_path:
    type: string
    description: 变体文件路径
  description:
    type: string | null
    description: 变体描述
  model:
    type: string | null
    description: 使用的模型
  extends:
    type: string | null
    description: 继承的基础定义

# Agent 变体列表信息
AgentVariantInfo:
  agent_id:
    type: string
  agent_name:
    type: string
  variants:
    type: array<VariantInfo>

# 解析的 Agent 引用
ResolvedAgentRef:
  agent_id:
    type: string
  variant:
    type: string | null
  definition:
    type: AgentDefinition

# 验证结果
ValidationResult:
  valid:
    type: boolean
  errors:
    type: array<string>
    description: 错误列表
  warnings:
    type: array<string>
    description: 警告列表
```

### 2.3 配置选项

```yaml
# config/agent-variants.yaml
agent_variants:
  # 文件位置
  agents_dir:
    type: string
    description: Agent 定义目录
    default: "./agents"

  # 文件命名
  file_naming:
    main: "AGENT.md"
    variant_pattern: "AGENT.{variant}.md"
    variant_subdir: "_variants"

  # 继承配置
  inheritance:
    max_depth: 5
    description: 最大继承深度
    allow_circular: false
    description: 是否允许循环继承

  # 缓存配置
  cache:
    enabled: true
    ttl: 300
    description: 缓存时间（秒）
```

---

## 3. 核心流程

### 3.1 变体加载流程

```
请求加载 Agent 变体
        │
        ▼
┌──────────────────────────────┐
│ 1. 确定文件路径              │
│    - 有变体: AGENT.{v}.md    │
│    - 无变体: AGENT.md         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 读取并解析文件            │
│    - 解析 frontmatter        │
│    - 解析内容                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 有     │
    │extends?│
    └───┬────┘
        │ 否
        ▼
    返回定义
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 递归加载基础定义          │
│    - 加载 extends 文件       │
│    - 检查循环依赖            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 合并定义                  │
│    - 变体覆盖基础            │
│    - 追加 instructions       │
│    - 合并 tools/skills       │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 5. 设置变体信息              │
│    - variant 名称            │
│    - 来源文件                │
└──────────────────────────────┘
        │
        ▼
    返回合并后的定义
```

### 3.2 变体发现流程

```
请求列出变体
        │
        ▼
┌──────────────────────────────┐
│ 1. 扫描 Agent 目录           │
│    - agents/{agent_id}/      │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 查找变体文件              │
│    - AGENT.*.md 文件         │
│    - 或 _variants/ 子目录     │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 提取变体名称              │
│    - AGENT.quick.md → quick   │
│    - AGENT.full.md → full     │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 解析变体信息              │
│    - 读取 frontmatter         │
│    - 提取描述/模型            │
└──────────────────────────────┘
        │
        ▼
    返回变体列表
```

### 3.3 定义合并逻辑

```yaml
# 合并规则
merge_rules:
  # 完全覆盖
  override:
    - id                  # 始终使用变体的 ID
    - variant             # 变体名称
    - model               # 模型配置完全覆盖
    - name                # 显示名称覆盖

  # 追加
  append:
    - instructions        # instructions 追加到基础
    - capabilities        # capabilities 合并去重

  # 合并
  merge:
    - tools               # tools 合并去重
    - skills              # skills 合并去重
    - permissions         # permissions 合并

  # 条件覆盖
  conditional_override:
    - role                # 如果变体有则覆盖
    - temperature         # 如果变体有则覆盖
```

---

## 4. 文件格式

### 4.1 目录结构

```
agents/
└── {agent-id}/
    ├── AGENT.md              # 主定义（默认变体）
    ├── AGENT.{variant}.md    # 变体定义
    └── _variants/            # 或者放在子目录
        ├── {variant}.md
        └── ...
```

### 4.2 主定义格式

```markdown
---
id: "code-reviewer"
name: "Code Reviewer"
version: "1.0.0"
---

# Agent: Code Reviewer

## Role
专业的代码审查助手

## Model
- provider: anthropic
- model: claude-sonnet-4-6
- temperature: 0.3

## Instructions
检查代码的：
1. 安全性
2. 性能
3. 可读性
4. 最佳实践

## Capabilities
- read
- grep
- bash (lint)
```

### 4.3 变体定义格式

```markdown
---
extends: AGENT.md         # 继承基础定义
variant: quick            # 声明变体名称
---

## Role
快速代码审查助手

## Model
- model: claude-haiku      # 覆盖：使用更快模型
- temperature: 0.1         # 覆盖：更确定性

## Instructions
仅检查明显问题：
1. 语法错误
2. 命名规范
3. 简单反模式

跳过深度分析和性能评估。
```

### 4.4 多层继承示例

```
AGENT.md (基类)
  └── AGENT.quick.md (继承基类)
      └── AGENT.quick.minimal.md (继承 quick 变体)
```

```markdown
---
extends: AGENT.quick.md
variant: quick.minimal
---

## Instructions
只检查语法错误和命名规范。
```

---

## 5. CLI 交互

### 5.1 基本语法

```bash
# 列出所有 Agent 及其变体
knight agent list
# 输出:
# code-reviewer (default, quick, security, fixer)
# coder (default, lite, pro)

# 使用默认变体
knight ask code-reviewer "审查这段代码"

# 指定变体（语法 1: 冒号）
knight ask code-reviewer:quick "快速检查"

# 指定变体（语法 2: 空格 + --variant）
knight ask code-reviewer --variant quick "快速检查"

# 查看变体信息
knight agent info code-reviewer --variant quick
```

### 5.2 交互模式

```bash
# 启动特定变体
knight chat code-reviewer:quick

# 运行中切换变体
» quick "检查这个"      # 使用 quick 变体
» switch full           # 切换到 full 变体
» "深度审查"            # 使用 full 变体
```

### 5.3 工作流中使用

```yaml
# workflows/pr-check.yaml
name: "PR 检查流程"

steps:
  # 快速检查
  - name: quick_check
    agent: code-reviewer:quick
    inputs:
      files: "{{ changed_files }}"

  # 如果通过，安全检查
  - name: security_check
    agent: code-reviewer:security
    run_if: quick_check.status == "pass"
    inputs:
      files: "{{ changed_files }}"

  # 最终报告
  - name: full_review
    agent: code-reviewer:full
    run_if: security_check.status == "pass"
```

---

## 6. 最佳实践

### 6.1 命名规范

| 变体名 | 用途 | 示例 |
|--------|------|------|
| `quick` | 快速检查 | 使用 Haiku，只检查明显问题 |
| `full` | 完整检查 | 使用 Sonnet，全面分析 |
| `lite` | 轻量版 | 简化能力，节省成本 |
| `pro` | 专业版 | 更多能力，更详细输出 |
| `{专项}` | 专项任务 | `security`, `performance`, `style` |
| `fixer` | 修复型 | 不仅分析，还修复 |

### 6.2 变体设计原则

1. **明确用途** - 每个变体有清晰的使用场景
2. **最小差异** - 只覆盖必要的配置
3. **继承优先** - 复用主定义，减少重复
4. **文档完整** - 说明何时使用哪个变体

### 6.3 何时创建变体

**适合创建变体**:
- 不同复杂度任务（quick vs full）
- 不同模型选择（lite vs pro）
- 不同专业领域（security vs performance）
- 不同行为模式（reviewer vs fixer）

**不适合创建变体**:
- 简单的参数调整 → 用配置文件
- 临时修改 → 用命令行参数
- 版本升级 → 用版本管理

---

## 7. 模块交互

### 7.1 依赖关系图

```
┌─────────────────────────────────────────┐
│           Agent Variants                │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Loader    │  │Merger    │  │Discover ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Agent     │  │Storage   │  │CLI       │
│Runtime   │  │Service   │  │Interface │
└──────────┘  └──────────┘  └──────────┘
```

### 7.2 消息流

```
CLI Request (agent:variant)
    │
    ▼
┌─────────────────────────────┐
│ Agent Variants              │
│ - 解析引用                  │
│ - 加载定义                  │
│ - 处理继承                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Agent Runtime               │
│ - 创建 Agent 实例           │
└─────────────────────────────┘
        │
        ▼
    返回 Agent
```

---

## 8. 配置与部署

### 8.1 配置文件格式

```yaml
# config/agent-variants.yaml
agent_variants:
  # 文件位置
  agents_dir:
    type: string
    default: "./agents"

  # 文件命名
  file_naming:
    main: "AGENT.md"
    variant_pattern: "AGENT.{variant}.md"
    variant_subdir: "_variants"

  # 继承配置
  inheritance:
    max_depth: 5
    allow_circular: false

  # 缓存配置
  cache:
    enabled: true
    ttl: 300
```

### 8.2 环境变量

```bash
# Agent 目录
export KNIGHT_AGENTS_DIR="./agents"

# 文件命名
export KNIGHT_AGENT_MAIN_FILE="AGENT.md"
export KNIGHT_AGENT_VARIANT_PATTERN="AGENT.{variant}.md"

# 继承配置
export KNIGHT_AGENT_INHERITANCE_MAX_DEPTH=5
```

---

## 9. 示例

### 9.1 快速审查变体

```markdown
---
extends: AGENT.md
variant: quick
---

## Role
快速代码检查

## Model
- model: claude-haiku

## Instructions
只检查：
1. 明显错误
2. 命名规范
3. 简单反模式
```

### 9.2 安全专项变体

```markdown
---
extends: AGENT.md
variant: security
---

## Role
安全专项审查

## Model
- temperature: 0.1

## Instructions
专注于安全检查：
1. SQL 注入
2. XSS 漏洞
3. 敏感信息泄露
4. 认证授权问题

## Skills
- security-scan
- secret-detection
```

### 9.3 修复型变体

```markdown
---
extends: AGENT.md
variant: fixer
---

## Role
代码审查并修复

## Instructions
不仅发现问题，还要：
1. 提供修复代码
2. 直接应用修复（经确认）
3. 运行测试验证

## Capabilities
- read
- write
- edit          # 新增：编辑能力
- bash (test)
```

---

## 10. 附录

### 10.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 变体加载 | < 50ms | 含继承 |
| 变体发现 | < 100ms | 扫描目录 |
| 缓存命中 | < 5ms | 内存操作 |

### 10.2 错误处理

```yaml
error_codes:
  AGENT_NOT_FOUND:
    code: 404
    message: "Agent 不存在"
    action: "检查 Agent ID"

  VARIANT_NOT_FOUND:
    code: 404
    message: "变体不存在"
    action: "使用 list 查看可用变体"

  CIRCULAR_INHERITANCE:
    code: 400
    message: "检测到循环继承"
    action: "检查 extends 字段"

  INHERITANCE_TOO_DEEP:
    code: 400
    message: "继承深度超限"
    action: "简化继承层级"

  INVALID_DEFINITION:
    code: 400
    message: "定义文件格式无效"
    action: "检查文件格式"
```

### 10.3 测试策略

```yaml
test_plan:
  unit_tests:
    - 变体加载
    - 继承处理
    - 定义合并
    - 路径解析

  integration_tests:
    - CLI 变体语法
    - 多层继承
    - 缓存失效
    - 错误恢复

  edge_cases:
    - 循环继承
    - 缺失基础文件
    - 无效变体名
    - 空定义文件
```
