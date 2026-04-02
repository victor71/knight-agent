# Command (命令系统)

## 概述

### 职责描述

Command 系统负责用户可定义 CLI 命令的完整生命周期管理，包括：

- 命令定义解析（Markdown 格式）
- 命令元数据管理
- 命令执行引擎
- 命令参数验证和绑定
- 命令步骤编排

### 设计目标

1. **易用性**: 用户通过简单的 Markdown 定义命令
2. **灵活性**: 支持工具调用、Agent 调用、Skill 调用
3. **可组合**: 步骤之间可以传递数据
4. **可扩展**: 支持自定义步骤类型

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Tool System | 依赖 | 工具调用 |
| Agent Runtime | 依赖 | Agent 调用 |
| Skill Engine | 依赖 | Skill 调用 |
| Session Manager | 依赖 | 会话上下文获取 |

---

## 数据模型

### CommandDefinition

```yaml
CommandDefinition:
  # 元数据
  metadata:
    name: string                  # 命令名称（如 review）
    description: string           # 命令描述
    version: string               # 版本号（可选）
    author: string                # 作者（可选）
    file_path: string             # 定义文件路径

  # 使用说明
  usage:
    syntax: string                # 语法: /review [path]
    examples: array               # 示例列表

  # 参数定义
  args:
    - name: string                # 参数名
      type: string                # 类型: string/int/float/boolean/array
      required: boolean           # 是否必需
      description: string         # 描述
      default: any                # 默认值（可选）

  # 执行步骤
  steps:
    - id: string                 # 步骤 ID
      name: string               # 步骤名称
      type: string               # 步骤类型: tool/agent/skill/command
      source: string             # 工具/Agent/Skill 名称
      action: string             # 操作名称（可选）
      args: map                  # 参数映射
      output: string             # 输出变量名
      condition: string          # 执行条件（可选）
      on_error: string           # 错误处理: continue/stop（可选）
```

### CommandExecutionContext

```yaml
CommandExecutionContext:
  command: CommandDefinition      # 命令定义
  parsed_args: map                # 解析后的参数
  variables: map                  # 步骤间变量
  session: Session                # 当前会话
  current_step: int               # 当前步骤索引
  results: array                  # 步骤执行结果
```

---

## Markdown 定义格式

### 完整示例

```markdown
---
name: review
description: 执行代码审查
version: "1.0.0"
---

# Command: review

执行代码审查，支持指定文件或目录。

## Usage

```
/review [文件路径] [--type <类型>]
```

## Args

- `path` (可选): 要审查的文件或目录路径，默认为当前目录
- `type` (可选): 审查类型 (quick/full/security)，默认为 quick

## Steps

### Step 1: 收集文件

```yaml
tool: glob
args:
  patterns: ["**/*.ts", "**/*.tsx"]
output: files
```

### Step 2: 选择 Agent

根据审查类型选择不同的 Agent。

```yaml
agent: code-reviewer:{{ type | default: "quick" }}
prompt: |
  审查以下文件：
  {{ files }}
output: review_result
```

### Step 3: 生成报告

```yaml
tool: write
args:
  path: "reports/review-{{ timestamp }}.md"
  content: |
    # Code Review Report

    {{ review_result }}
```

## Examples

```bash
# 审查当前目录
/review

# 完整审查
/review --type full

# 审理指定文件
/review src/App.tsx
```
```

### Front Matter 字段

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

        // 4. 解析 Steps 部分
        let steps = Self::extract_steps(content)?;

        Ok(CommandDefinition {
            metadata,
            usage,
            args,
            steps,
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

## 执行引擎

### 执行流程

```
命令调用
    ↓
解析参数
    ↓
创建执行上下文
    ↓
遍历步骤
    ↓
    ├─→ 检查执行条件
    │   ↓
    ├─→ 条件满足 → 执行步骤
    │   ↓
    │   ├─→ tool → Tool System
    │   ├─→ agent → Agent Runtime
    │   └─→ skill → Skill Engine
    │   ↓
    │   保存输出到变量
    │   ↓
    └─→ 下一步骤
        ↓
    返回最终结果
```

### 步骤执行器

```rust
// 步骤执行器
pub struct StepExecutor;

impl StepExecutor {
    pub async fn execute_step(
        step: &CommandStep,
        context: &mut CommandExecutionContext,
    ) -> Result<Value> {
        // 检查执行条件
        if let Some(condition) = &step.condition {
            if !Self::evaluate_condition(condition, context)? {
                return Ok(Value::Null);
            }
        }

        // 变量替换
        let resolved_args = Self::resolve_variables(&step.args, context)?;

        // 执行步骤
        let result = match step.ty.as_str() {
            "tool" => Self::execute_tool(&step.source, &resolved_args, context).await?,
            "agent" => Self::execute_agent(&step.source, &resolved_args, context).await?,
            "skill" => Self::execute_skill(&step.source, &resolved_args, context).await?,
            "command" => Self::execute_command(&step.source, &resolved_args, context).await?,
            _ => return Err(Error::UnknownStepType(step.ty.clone())),
        };

        // 保存输出
        if let Some(output_var) = &step.output {
            context.variables.insert(output_var.clone(), result.clone());
        }

        Ok(result)
    }

    async fn execute_tool(
        tool_name: &str,
        args: &HashMap<String, Value>,
        context: &CommandExecutionContext,
    ) -> Result<Value> {
        let tool_system = context.session.tool_system();
        tool_system.call_tool(tool_name, args).await
    }

    async fn execute_agent(
        agent_name: &str,
        args: &HashMap<String, Value>,
        context: &CommandExecutionContext,
    ) -> Result<Value> {
        let prompt = args.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or(Error::MissingPrompt)?;

        let agent_runtime = context.session.agent_runtime();
        agent_runtime.call_agent(agent_name, prompt, context).await
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

  StepError:
    description: 步骤执行失败
    message: "步骤 {step} 执行失败: {reason}"

  VariableError:
    description: 变量引用错误
    message: "变量 {name} 未定义"
```

### 错误恢复

```yaml
on_error:
  continue:                     # 继续执行下一步骤
    log: true                    # 记录错误
    set_var: error_message       # 设置错误变量

  stop:                          # 停止执行
    return_error: true           # 返回错误信息

  retry:                         # 重试
    max_attempts: 3
    backoff: exponential
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
- [ ] 步骤执行正确性
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
