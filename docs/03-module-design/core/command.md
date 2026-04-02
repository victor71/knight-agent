# Command (命令系统)

## 概述

### 职责描述

Command 系统负责用户可定义 CLI 命令的入口管理，采用**松耦合、LLM 驱动**的设计：

- 命令定义解析（Markdown 格式）
- 命令元数据管理（名称、描述、参数说明）
- 参数解析和验证（基础级别）
- **LLM 理解命令意图，动态决定调用目标**（Skill/Agent/工具）

### 设计理念

Command 采用**声明式 + LLM 驱动**的设计，类似 Claude Code：

1. **用户声明**：在 Markdown 中定义命令名称、描述、预期行为
2. **LLM 解析**：当用户调用命令时，LLM 根据描述理解意图
3. **动态执行**：LLM 决定调用哪个 Skill、Agent，或直接使用工具
4. **灵活适应**：同一命令可以根据上下文产生不同行为

### 设计目标

1. **极简定义**：用户只需描述命令用途，不需要复杂的编排配置
2. **智能理解**：LLM 根据描述和上下文智能选择执行方式
3. **松散耦合**：命令不硬编码依赖具体的 Skill 或 Agent
4. **灵活适应**：同一命令可以根据不同情况产生不同行为

### 与 Skill Engine 的区别

| 特性 | Command | Skill Engine |
|------|---------|-------------|
| 用途 | CLI 命令入口 | 可复用行为模式 |
| 定义格式 | Markdown + 参数 | Markdown + 触发条件 + 步骤 |
| 执行方式 | 委托给 Skill/Agent | 独立执行引擎 |
| 步骤编排 | ❌ 不包含 | ✅ 完整的 DAG 编排 |
| 调用方式 | 用户调用 `/command` | 事件触发或手动调用 |

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Skill Engine | 委托 | Command 委托给 Skill 执行步骤 |
| Agent Runtime | 委托 | Command 可直接调用 Agent |
| Session Manager | 依赖 | 会话上下文获取 |
| Task Manager | 委托 | Workflow 命令委托给 Task Manager 执行 |
| LLM Provider | 依赖 | Workflow 命令需要 LLM 解析自然语言定义 |
| Storage Service | 依赖 | 加载 workflows/ 目录中的定义文件 |

---

## 数据模型

### CommandDefinition

```yaml
CommandDefinition:
  # 元数据
  metadata:
    name: string                  # 命令名称（如 review）
    description: string           # 命令描述（LLM 理解意图的关键）
    version: string               # 版本号（可选）
    author: string                # 作者（可选）
    file_path: string             # 定义文件路径
    command_type:
      type: enum
      values: [simple, workflow]   # 命令类型
      default: simple
      description: |
        - simple: 普通命令，直接调用 Skill/Agent/Tool
        - workflow: 工作流命令，通过 Task Manager 执行多 Agent 协同

  # 使用说明（供用户参考）
  usage:
    syntax: string                # 语法: /review [path]
    examples: array               # 示例列表
    expected_behavior: string     # 预期行为描述（帮助 LLM 理解）

  # 参数定义（基础说明，LLM 会智能解析）
  args:
    - name: string                # 参数名
      description: string         # 参数说明
      required: boolean           # 是否必需
      type_hint: string           # 类型提示（可选，如 "file path"）
```

**工作流命令特殊字段**（仅当 `command_type: workflow` 时存在）：
```yaml
  workflow_config:
    workflow_definition_path: string  # 工作流定义文件路径
    dynamic_agent_creation: boolean    # 是否动态创建 Agent
    parallel_execution: boolean       # 是否支持并行执行
```

**关键变化**：
- 移除了 `steps` 字段（不再包含硬编码的执行步骤）
- 新增 `expected_behavior` 字段（帮助 LLM 理解命令意图）
- `type_hint` 替代严格 `type`（仅作为提示，LLM 灵活解析）
- 执行由 LLM 动态决定，而非预定义步骤

### CommandExecutionContext

```yaml
CommandExecutionContext:
  command: CommandDefinition      # 命令定义
  parsed_args: map                # 解析后的参数
  session: Session                # 当前会话
  user_input: string              # 用户原始输入
```

---

## Markdown 定义格式

### 完整示例

```markdown
---
name: review
description: 执行代码审查，支持指定文件或目录。根据用户输入智能选择审查策略（快速/完整/安全）。
version: "1.0.0"
---

# Command: review

## Usage

```
/review [文件路径] [--type <类型>]
```

## Args

- `path` (可选): 要审查的文件或目录路径，默认为当前目录
- `type` (可选): 审查类型 (quick/full/security)，默认为 quick

## Expected Behavior

当用户调用 `/review` 时：
1. 分析用户输入的参数
2. 根据路径收集相关文件
3. 根据 type 参数选择合适的审查深度
4. 智能决定：调用 code-reviewer Skill 或直接分析文件
5. 生成审查报告并输出

## Examples

```bash
# 审查当前目录（LLM 会智能决定具体执行方式）
/review

# 完整审查
/review --type full

# 审理指定文件
/review src/App.tsx
```

### 说明

- Command **不定义**具体的执行步骤
- LLM 根据 `description` 和 `expected_behavior` 智能决定如何执行
- 同一个命令可以根据上下文产生不同行为
- 如果需要复杂流程，应由 Skill Engine 定义 Skill，Command 仅仅调用该 Skill
```

### 说明

- Command **不定义**具体的执行步骤
- LLM 根据 `description` 和 `expected_behavior` 智能决定如何执行
- 同一个命令可以根据上下文产生不同行为
- 如果需要复杂流程，应由 Skill Engine 定义 Skill，Command 仅仅调用该 Skill

### 工作流类型命令

工作流命令是 Command 的特殊类型，用于执行多 Agent 协同任务：

```markdown
---
name: workflow
description: 执行多 Agent 协同工作流，自动创建所需 Agents 并按流程协同工作
---

# Command: workflow

## Usage

```
/workflow <workflow-name> [arguments...]
```

## Args

- `workflow_name` (必需): 工作流名称，如 `software-development`
- `arguments` (可变): 工作流特定参数，会传递给工作流定义

## Examples

```bash
# 软件开发工作流
/workflow software-development docs/requirements.md

# 代码审查工作流
/workflow code-review src/ --type full

# 部署工作流
/workflow deploy --env production --version v1.2.0
```

## Expected Behavior

当用户调用 `/workflow software-development docs/requirements.md` 时：

1. Command 识别为工作流类型命令
2. 加载 `workflows/software-development.md` 工作流定义
3. LLM 解析工作流定义，识别所需的 Agents 和步骤
4. 通过 Task Manager 创建工作流实例
5. Task Manager 通过 Orchestrator 动态创建所需的 Agents
6. 按工作流定义的顺序协同执行
7. 返回最终结果

## Workflow Definition Format

工作流定义文件位于 `workflows/software-development.md`：

```markdown
---
name: software-development
description: 从需求到部署的完整软件开发流程
agents:
  - architect
  - developer
  - tester
---

# Software Development Workflow

## Overview

本工作流通过多 Agent 协作完成软件开发任务：

1. **Architect Agent** - 分析需求，生成设计方案
2. **Developer Agent** - 根据设计方案实现代码
3. **Tester Agent** - 测试实现的功能

## Parameters

| 参数 | 类型 | 说明 |
|------|------|------|
| requirement | string | 需求文档路径 |

## Steps

### Step 1: 需求分析

执行者：`architect` Agent

任务：阅读需求文档，生成技术设计方案

输出：`docs/design.md`

### Step 2: 代码实现

执行者：`developer` Agent

依赖：Step 1 完成

任务：根据设计方案实现功能代码

### Step 3: 功能测试

执行者：`tester` Agent

依赖：Step 2 完成

任务：测试实现的功能
```

**工作流命令与普通命令的区别**：

| 特性 | 普通命令 | 工作流命令 |
|------|----------|------------|
| 执行方式 | 直接调用 Skill/Agent/Tool | 通过 Task Manager 执行工作流 |
| 定义位置 | `commands/` 目录 | `workflows/` 目录 |
| 参数解析 | Command 定义中声明 | 工作流定义中声明 |
| Agent 创建 | 无（使用现有） | 动态创建所需 Agents |
| 复杂度 | 单步骤 | 多步骤 DAG |

```

| 字段 | 必需 | 说明 |
|------|------|------|
| `name` | ✅ | 命令名称，用于 `/command` |
| `description` | ✅ | 命令描述 |
| `version` | ❌ | 版本号 |
| `author` | ❌ | 作者 |

---

## 解析器

### Markdown 解析

```rust
// 命令定义解析器
pub struct CommandParser;

impl CommandParser {
    /// 从 Markdown 文件解析命令定义
    pub async fn parse_file(path: &Path) -> Result<CommandDefinition> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::parse_content(&content, path)
    }

    /// 解析 Markdown 内容
    pub fn parse_content(content: &str, path: &Path) -> Result<CommandDefinition> {
        // 1. 提取 front matter (YAML)
        let metadata = Self::extract_frontmatter(content)?;

        // 2. 解析 Usage 部分
        let usage = Self::extract_usage(content)?;

        // 3. 解析 Args 部分
        let args = Self::extract_args(content)?;

        // 4. 解析 Expected Behavior 部分
        let expected_behavior = Self::extract_expected_behavior(content)?;

        Ok(CommandDefinition {
            metadata,
            usage,
            args,
            expected_behavior,
            file_path: path.to_string_lossy().to_string(),
        })
    }
}
```

### 参数绑定

```rust
// 参数绑定器
pub struct ArgBinder;

impl ArgBinder {
    /// 将命令行参数绑定到命令定义
    pub fn bind_args(
        definition: &CommandDefinition,
        input_args: Vec<String>,
    ) -> Result<HashMap<String, Value>> {
        let mut bound = HashMap::new();

        for arg_def in &definition.args {
            let value = if let Some(input) = input_args.iter().find(|s| s.starts_with(&format!("--{}", arg_def.name))) {
                // --name value 格式
                Self::parse_arg_value(input, &arg_def.ty)?
            } else if arg_def.required {
                return Err(Error::MissingArg(arg_def.name.clone()));
            } else {
                arg_def.default.clone().unwrap_or(Value::Null)
            };

            bound.insert(arg_def.name.clone(), value);
        }

        Ok(bound)
    }
}
```

---

## 执行流程

### LLM 驱动的执行流程

```
用户输入: /review src/App.tsx --type full
    ↓
Command 系统解析命令
    - 找到命令定义
    - 解析参数: {path: "src/App.tsx", type: "full"}
    ↓
构建 LLM 理解 Prompt
    - 包含命令描述和预期行为
    - 包含用户参数
    ↓
LLM 分析意图
    - 理解：需要对 src/App.tsx 进行完整代码审查
    - 决定：调用 code-reviewer:full Skill
    ↓
委托执行
    - 调用 Skill Engine 执行 code-reviewer:full
    - 或者直接调用 Agent Runtime
    - 或者调用 Tool System
    ↓
返回结果
```

### 工作流命令执行流程

```
用户输入: /workflow software-development docs/requirements.md
    ↓
Command 系统解析命令
    - 识别 command_type: workflow
    - 解析参数: {workflow_name: "software-development", requirement: "docs/requirements.md"}
    ↓
加载工作流定义
    - 从 workflows/software-development.md 加载工作流定义
    - 解析工作流所需 Agents 和步骤
    ↓
构建 LLM 理解 Prompt
    - 包含工作流定义和用户参数
    ↓
LLM 解析工作流
    - 识别需要的 Agents: [architect, developer, tester]
    - 解析步骤和依赖关系
    - 构建工作流 DAG
    ↓
委托 Task Manager 执行
    - 调用 Task Manager.create_workflow()
    - 传递工作流 DAG 和参数
    ↓
Task Manager 执行工作流
    - 通过 Orchestrator 动态创建 Agents
    - 按依赖顺序调度任务
    - 每个任务由对应 Agent 执行
    ↓
返回工作流结果
```

### Command 执行器

```rust
// Command 执行器
pub struct CommandExecutor;

impl CommandExecutor {
    pub async fn execute(
        command: &CommandDefinition,
        user_input: &str,
        parsed_args: HashMap<String, Value>,
        session: &Session,
    ) -> Result<String> {
        // 1. 检查命令类型
        if command.metadata.command_type == "workflow" {
            return Self::execute_workflow_command(command, user_input, parsed_args, session).await;
        }

        // 2. 普通 LLM 驱动命令
        let prompt = Self::build_llm_prompt(command, user_input, &parsed_args);

        // 3. LLM 分析意图并决定执行方式
        let agent_runtime = session.agent_runtime();
        let llm_response = agent_runtime.send_message(
            &llm_provider.get_default_model(),
            &prompt
        ).await?;

        // 4. 解析 LLM 响应，提取决策
        let decision = Self::parse_llm_decision(&llm_response)?;

        // 5. 根据决策执行
        match decision.target_type {
            "skill" => {
                // 调用 Skill Engine
                let skill_engine = session.skill_engine();
                skill_engine.execute_skill(&decision.target_name, decision.params).await
            }
            "agent" => {
                // 直接调用 Agent
                let agent_runtime = session.agent_runtime();
                agent_runtime.call_agent(&decision.target_name, &decision.prompt, session).await
            }
            "tools" => {
                // 直接调用工具
                let tool_system = session.tool_system();
                // 执行工具调用序列
                Self::execute_tools(&decision.tool_calls, tool_system).await
            }
            _ => Err(Error::UnknownTargetType(decision.target_type)),
        }
    }

    /// 执行工作流命令
    async fn execute_workflow_command(
        command: &CommandDefinition,
        user_input: &str,
        parsed_args: HashMap<String, Value>,
        session: &Session,
    ) -> Result<String> {
        // 1. 加载工作流定义
        let workflow_name = parsed_args.get("workflow_name")
            .or_else(|| parsed_args.get("0"))
            .and_then(|v| v.as_str())
            .ok_or(Error::MissingWorkflowName)?;

        let workflow_path = format!("workflows/{}.md", workflow_name);
        let workflow_definition = Self::load_workflow_definition(&workflow_path).await?;

        // 2. LLM 解析工作流定义
        let parsed_workflow = Self::parse_workflow_with_llm(
            &workflow_definition,
            &parsed_args,
            session
        ).await?;

        // 3. 构建工作流上下文（类型定义见 Task Manager 模块）
        let context = WorkflowContext {
            source: "command".to_string(),
            command_name: Some(command.metadata.name.clone()),
            command_args: parsed_args.keys().cloned().collect(),
            session_id: session.id().to_string(),
            environment: std::env::vars().collect(),
        };

        // 4. 通过 Task Manager 执行工作流（默认后台执行）
        let task_manager = session.task_manager();
        let result = task_manager.create_workflow_from_parsed(
            parsed_workflow,
            context,
            true,  // background = true
        ).await?;

        // 5. 返回工作流 ID（后台执行，立即返回）
        Ok(format!(
            "工作流 '{}' 已启动（后台执行）\n工作流 ID: {}\n使用 `/workflow status {}` 查看进度",
            workflow_name, result.workflow_id, result.workflow_id
        ))
    }

    fn build_llm_prompt(
        command: &CommandDefinition,
        user_input: &str,
        parsed_args: &HashMap<String, Value>,
    ) -> String {
        format!(
            r#"用户执行命令: {}

命令描述: {}

用户参数: {:#}

请分析用户意图，决定如何执行此命令。可用选项：
1. 调用某个 Skill (返回 skill:skill_name)
2. 调用某个 Agent (返回 agent:agent_name[:variant])
3. 直接使用工具 (返回 tools:tool_calls)

返回 JSON 格式：
{{"target_type": "skill|agent|tools", "target_name": "...", "params": {{...}}, "prompt": "...", "tool_calls": [...]}}"#,
            user_input,
            command.metadata.description,
            parsed_args
        )
    }
}
```

### 变量替换

```rust
// 变量解析器
pub struct VariableResolver;

impl VariableResolver {
    /// 解析变量引用 {{ variable_name }}
    pub fn resolve_variables(
        template: &HashMap<String, Value>,
        context: &CommandExecutionContext,
    ) -> Result<HashMap<String, Value>> {
        let mut resolved = HashMap::new();

        for (key, value) in template {
            resolved.insert(key.clone(), Self::resolve_value(value, context)?);
        }

        Ok(resolved)
    }

    fn resolve_value(value: &Value, context: &CommandExecutionContext) -> Result<Value> {
        match value {
            Value::String(s) => {
                if s.contains("{{") && s.contains("}}") {
                    // 提取变量名
                    let var_name = Self::extract_variable_name(s)?;
                    // 从上下文获取值
                    if let Some(var_value) = context.variables.get(&var_name) {
                        Ok(var_value.clone())
                    } else {
                        Ok(Value::String(s.clone()))
                    }
                } else {
                    Ok(value.clone())
                }
            }
            Value::Object(map) => {
                let mut resolved_map = HashMap::new();
                for (k, v) in map {
                    resolved_map.insert(k.clone(), Self::resolve_value(v, context)?);
                }
                Ok(Value::Object(resolved_map))
            }
            _ => Ok(value.clone()),
        }
    }
}
```

---

## 内置函数

### 可用函数

| 函数 | 说明 | 示例 |
|------|------|------|
| `timestamp` | 当前时间戳 | `{{ timestamp }}` |
| `date` | 当前日期 | `{{ date("%Y-%m-%d") }}` |
| `default` | 默认值 | `{{ type \| default: "quick" }}` |
| `upper` | 大写转换 | `{{ name \| upper }}` |
| `lower` | 小写转换 | `{{ name \| lower }}` |

---

## 错误处理

### 错误类型

```yaml
CommandError:
  ParseError:
    description: 命令定义解析失败
    message: "解析错误: {reason}"

  ArgError:
    description: 参数错误
    message: "参数错误: {reason}"

  ExecutionError:
    description: 命令执行失败
    message: "执行失败: {reason}"

  LLMDecisionError:
    description: LLM 决策解析失败
    message: "无法解析 LLM 决策: {reason}"

  VariableError:
    description: 变量引用错误
    message: "变量 {name} 未定义"
```

### 错误恢复

```yaml
on_error:
  stop:                          # 停止执行
    return_error: true           # 返回错误信息

  retry:                         # 重试
    max_attempts: 3
    backoff: exponential
    retry_on: [ExecutionError, LLMDecisionError]
```

---

## 配置

### 存储结构

```
~/.knight-agent/
└── commands/
    └── user/                    # 用户自定义命令
        ├── review.md
        ├── deploy.md
        ├── test.md
        └── analyze.md
```

### 加载配置

```yaml
# config/command.yaml
command:
  # 命令目录
  user_command_path: "~/.knight-agent/commands/user/"

  # 热加载
  hot_reload: true
  watch_interval: 5               # 监听间隔（秒）

  # 缓存
  cache_enabled: true
  cache_ttl: 300

  # 验证
  validate_on_load: true         # 加载时验证命令定义
```

---

## 测试要点

### 单元测试

- [ ] Markdown 解析正确性
- [ ] 参数绑定正确性
- [ ] 变量替换正确性
- [ ] LLM 决策解析正确性
- [ ] 错误处理正确性

### 集成测试

- [ ] 与 Tool System 集成
- [ ] 与 Agent Runtime 集成
- [ ] 与 Skill Engine 集成
- [ ] 命令热加载

### 测试用例示例

```rust
#[tokio::test]
async fn test_command_parse() {
    let markdown = r#"
---
name: test
---
# Command: test
## Args
- name: path
"#;

    let definition = CommandParser::parse_content(markdown, Path::new("test.md")).unwrap();
    assert_eq!(definition.metadata.name, "test");
    assert_eq!(definition.args.len(), 1);
    assert!(definition.expected_behavior.is_some());
}

#[tokio::test]
async fn test_command_execution() {
    let mut context = create_test_context();
    let definition = load_test_command("review.md");

    let result = CommandExecutor::execute(&definition, vec![], &mut context).await;
    assert!(result.is_ok());
}
```

---

## 性能考虑

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 解析时间 | < 10ms | 单个命令定义 |
| 参数绑定 | < 1ms | 单个命令 |
| 变量替换 | < 1ms | 每个步骤 |
| 命令执行 | 取决于步骤 | 由步骤决定 |

---

## 未来扩展

- [ ] 命令模板（参数化命令）
- [ ] 命令管道（输出传递）
- [ ] 命令组合（多个命令链式执行）
- [ ] 命令版本管理
- [ ] 命令分享市场
