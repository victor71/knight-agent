# Router (路由器)

## 概述

### 职责描述

Router 负责 CLI 输入的路由和分发，包括：

- CLI 命令识别和解析
- 内置命令执行
- 用户自定义命令加载和执行
- 非命令输入转发给 Agent

### 设计目标

1. **快速响应**: 系统命令立即响应，无需 LLM 调用
2. **可扩展性**: 支持用户通过 Markdown 定义自定义命令
3. **一致性**: 统一的命令格式和错误处理
4. **优先级**: 内置命令优先，用户命令可覆盖

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取当前会话和 main agent |
| Command Loader | 依赖 | 加载用户自定义命令 |
| Agent Runtime | 依赖 | 转发非命令输入 |

---

## 接口定义

### 对外接口

```yaml
# Router 接口定义
Router:
  # ========== 命令处理 ==========
  handle_input:
    description: 处理用户输入
    inputs:
      input:
        type: string
        required: true
        description: 用户原始输入
      session_id:
        type: string
        required: true
        description: 当前会话 ID
    outputs:
      response:
        type: RouterResponse
        description: 处理结果
      to_agent:
        type: boolean
        description: 是否需要转发给 Agent

  # ========== 命令加载 ==========
  load_user_commands:
    description: 加载用户自定义命令
    inputs:
      path:
        type: string
        required: true
        description: 命令目录路径 (~/.knight-agent/commands/)
    outputs:
      commands:
        type: map<string, CommandDefinition>
        description: 命令名到定义的映射

  # ========== 命令查询 ==========
  list_commands:
    description: 列出所有可用命令
    inputs:
      session_id:
        type: string
        required: true
      filter:
        type: string
        required: false
        description: 过滤条件 (built-in|user|all)
    outputs:
      commands:
        type: array<CommandInfo>
```

### RouterResponse 结构

```yaml
RouterResponse:
  type: object
  properties:
    success:
      type: boolean
      description: 是否成功执行
    message:
      type: string
      description: 返回消息
    data:
      type: any
      description: 附加数据
    error:
      type: string
      description: 错误信息（失败时）
```

---

## 处理流程

### 命令识别流程

```
用户输入
    ↓
检测是否以 / 开头
    ↓
    ├─→ 否 → 返回 to_agent=true → 转发给 main agent
    │
    └─→ 是 → 解析命令
              ↓
         提取命令名和参数
              ↓
         查找命令定义
              ↓
         ├─→ 内置命令 → 执行 → 返回结果
         │                  ↓
         │              to_agent=false
         │
         └─→ 用户命令 → 验证 → 执行步骤 → 返回结果
                              ↓
                          to_agent=false
```

### 命令查找顺序

```
1. 内置命令（硬编码）
   └─→ 找到 → 执行

2. 用户自定义命令 (~/.knight-agent/commands/)
   └─→ 找到 → 执行

3. 未找到 → 返回错误提示
```

---

## 内置命令

### 命令列表

| 命令 | 类型 | 说明 |
|------|------|------|
| `/new-session` | 会话管理 | 创建新会话 |
| `/switch-session` | 会话管理 | 切换会话 |
| `/list-sessions` | 会话管理 | 列出所有会话 |
| `/current-session` | 会话管理 | 显示当前会话 |
| `/delete-session` | 会话管理 | 删除会话 |
| `/list-agents` | Agent 管理 | 列出可用 Agent |
| `/use-agent` | Agent 管理 | 切换 Agent |
| `/current-agent` | Agent 管理 | 显示当前 Agent |
| `/clear` | 上下文控制 | 清空上下文 |
| `/history` | 上下文控制 | 显示历史 |
| `/compress` | 上下文控制 | 压缩上下文 |
| `/status` | 系统控制 | 显示状态 |
| `/help` | 系统控制 | 显示帮助 |
| `/exit` | 系统控制 | 退出 REPL |
| `/quit` | 系统控制 | 退出 REPL (别名) |

### 命令实现

```rust
// Router 内置命令实现
impl Router {
    // 处理内置命令
    async fn handle_builtin_command(
        &self,
        command: &str,
        args: Vec<String>,
        session: &Session,
    ) -> Result<RouterResponse> {
        match command {
            "new-session" => self.cmd_new_session(args).await,
            "switch-session" => self.cmd_switch_session(args).await,
            "list-sessions" => self.cmd_list_sessions().await,
            // ... 其他命令
            _ => Err(Error::UnknownCommand),
        }
    }
}
```

---

## 用户自定义命令

### Command 定义格式

```markdown
---
name: review
description: 执行代码审查
---

# Command: review

执行代码审查，支持指定文件或目录。

## Usage

```
/review [文件路径]
```

## Args

- `path` (可选): 要审查的文件或目录路径，默认为当前目录

## Steps

### Step 1: 收集文件
```yaml
tool: glob
args:
  patterns: ["**/*.ts", "**/*.tsx"]
output: files
```

### Step 2: 运行审查
```yaml
agent: code-reviewer
prompt: |
  审查以下文件：
  {{ files }}
```

## Examples

```bash
# 审查当前目录
/review

# 审理指定文件
/review src/App.tsx
```
```

### Command 数据结构

```yaml
CommandDefinition:
  metadata:
    name: string                  # 命令名称
    description: string           # 命令描述
    file_path: string             # 定义文件路径
    version: string               # 版本（可选）

  usage:
    syntax: string                # 使用语法
    examples: array               # 示例列表

  args:
    - name: string
      type: string
      required: boolean
      description: string
      default: any

  steps:
    - name: string
      tool: string                # 工具/agent/skill
      agent: string               # 如果是 agent 类型
      args: map                   # 参数
      output: string              # 输出变量名
```

### Command 执行

```rust
// 用户命令执行
impl Router {
    async fn execute_user_command(
        &self,
        definition: &CommandDefinition,
        args: Vec<String>,
        session: &Session,
    ) -> Result<RouterResponse> {
        let mut context = HashMap::new();

        // 解析参数
        let parsed_args = self.parse_command_args(&definition.args, args)?;

        // 执行步骤
        for step in &definition.steps {
            let result = match step.tool.as_str() {
                "glob" => self.execute_tool_glob(&step.args, &session).await?,
                "agent" => self.execute_agent_call(&step.agent, &step.args, &session).await?,
                _ => return Err(Error::UnknownTool(step.tool.clone())),
            };

            context.insert(step.output.clone(), result);
        }

        Ok(RouterResponse {
            success: true,
            message: "Command executed".to_string(),
            data: Some(context),
            error: None,
        })
    }
}
```

---

## 配置

### 目录结构

```
~/.knight-agent/
└── commands/
    ├── core/                    # 保留，内置命令暂不存储为文件
    └── user/                    # 用户自定义命令
        ├── review.md
        ├── deploy.md
        ├── test.md
        └── analyze.md
```

### 加载配置

```yaml
# config/router.yaml
router:
  # 命令加载
  command_paths:
    user_commands: "~/.knight-agent/commands/user/"

  # 命令缓存
  cache_enabled: true
  cache_ttl: 300                 # 缓存时间（秒）

  # 命令别名
  aliases:
    quit: exit                   # /quit → /exit
    ls: list-sessions            # /ls → /list-sessions
```

---

## 错误处理

### 错误类型

```yaml
RouterError:
  UnknownCommand:
    description: 命令不存在
    message: "未知命令: {command}，输入 /help 查看帮助"

  InvalidArgs:
    description: 参数错误
    message: "参数错误: {reason}"

  CommandFailed:
    description: 命令执行失败
    message: "命令执行失败: {reason}"

  PermissionDenied:
    description: 权限不足
    message: "权限不足: {operation}"
```

---

## 测试要点

### 单元测试

- [ ] 命令识别正确性（/ 开头检测）
- [ ] 参数解析正确性
- [ ] 内置命令执行
- [ ] 用户命令加载和执行
- [ ] 错误处理

### 集成测试

- [ ] 与 Session Manager 集成
- [ ] 与 Agent Runtime 集成
- [ ] 命令热加载

### 测试用例示例

```rust
#[tokio::test]
async fn test_builtin_command() {
    let router = Router::new();
    let session = create_test_session();

    let result = router.handle_input("/status", &session).await;

    assert!(result.to_agent == false);
    assert!(result.response.success == true);
}

#[tokio::test]
async fn test_non_command_input() {
    let router = Router::new();
    let session = create_test_session();

    let result = router.handle_input("帮我审查代码", &session).await;

    assert!(result.to_agent == true);
}
```

---

## 性能考虑

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 命令识别延迟 | < 1ms | 字符串匹配 |
| 命令加载时间 | < 100ms | 首次加载 |
| 内置命令执行 | < 10ms | 简单命令 |
| 用户命令执行 | 取决于步骤 | 由步骤决定 |

---

## 未来扩展

- [ ] 命令自动补全
- [ ] 命令建议（基于历史）
- [ ] 命令模板
- [ ] 命令组合（管道）
