# Router (路由器)

## 概述

### 职责描述

Router 负责 CLI 输入的路由和分发，包括：

- CLI 命令识别和解析
- 内置命令执行
- 命令定义加载并委托给 Command 模块执行
- 非命令输入转发给 Agent

### 设计理念

Router 采用**识别与执行分离**的设计：

1. **Router 负责**：
   - 检测输入是否为命令（`/` 开头）
   - 识别命令类型（内置/用户/workflow）
   - 加载命令定义
   - 委托给对应模块执行

2. **Command 模块负责**：
   - 用户自定义命令的执行（LLM 驱动）
   - 工作流命令的执行

3. **其他模块负责**：
   - Session Manager 执行会话管理命令
   - Agent Runtime 执行 Agent 管理命令

### 设计目标

1. **快速响应**: 系统命令立即响应，无需 LLM 调用
2. **可扩展性**: 支持用户通过 Markdown 定义自定义命令
3. **一致性**: 统一的命令格式和错误处理
4. **委托执行**: Router 识别命令，委托给专门模块执行

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Command 模块 | 委托 | 用户自定义命令和工作流命令执行 |
| Session Manager | 委托 | 会话管理命令执行 |
| Agent Runtime | 委托 | Agent 管理命令执行 |
| Task Manager | 委托 | 工作流状态查询命令执行 |

---

## 接口定义

### 对外接口

```yaml
# Router 接口定义
Router:
  # ========== 输入处理 ==========
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
        description: 过滤条件 (built-in|user|workflow|all)
    outputs:
      commands:
        type: array<CommandInfo>

  # ========== 命令注册 ==========
  register_command:
    description: 注册内置命令处理器
    inputs:
      command:
        type: string
        required: true
        description: 命令名称（不含 /）
      handler:
        type: CommandHandler
        required: true
        description: 命令处理器
    outputs:
      success:
        type: boolean
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

### CommandHandler 结构

```yaml
CommandHandler:
  type: object
  description: 命令处理器
  properties:
    type:
      type: enum
      values: [builtin, session, agent, command_module, task_manager]
      description: 处理器类型
    handler:
      type: function
      description: 处理函数
```

---

## 处理流程

### 命令识别与路由流程

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
         查找命令处理器
              ↓
         ├─→ 内置系统命令 → 执行 → 返回结果
         │                        ↓
         │                    to_agent=false
         │
         ├─→ 会话管理命令 → 委托 Session Manager → 返回结果
         │                        ↓
         │                    to_agent=false
         │
         ├─→ Agent 管理命令 → 委托 Agent Runtime → 返回结果
         │                        ↓
         │                    to_agent=false
         │
         ├─→ 用户命令 (/command) → 委托 Command 模块 → 返回结果
         │                              ↓
         │                          to_agent=false
         │
         └─→ 工作流命令 (/workflow) → 委托 Command 模块 → 返回结果
                                    ↓
                                to_agent=false
```

### 命令查找与处理顺序

```
1. 内置系统命令（Router 内置）
   └─→ 找到 → 直接执行

2. 会话管理命令
   └─→ 找到 → 委托 Session Manager

3. Agent 管理命令
   └─→ 找到 → 委托 Agent Runtime

4. 工作流命令 (/workflow)
   └─→ 匹配 → 委托 Command 模块

5. 用户自定义命令 (~/.knight-agent/commands/)
   └─→ 找到 → 委托 Command 模块

6. 未找到 → 返回错误提示
```

---

## 内置命令

### 命令列表

| 命令 | 类型 | 处理器 | 说明 |
|------|------|--------|------|
| `/help` | 系统 | 内置 | 显示帮助 |
| `/status` | 系统 | 内置 | 显示状态 |
| `/exit` | 系统 | 内置 | 退出 REPL |
| `/quit` | 系统 | 内置 | 退出 REPL (别名) |
| `/new-session` | 会话 | Session Manager | 创建新会话 |
| `/switch-session` | 会话 | Session Manager | 切换会话 |
| `/list-sessions` | 会话 | Session Manager | 列出所有会话 |
| `/current-session` | 会话 | Session Manager | 显示当前会话 |
| `/delete-session` | 会话 | Session Manager | 删除会话 |
| `/list-agents` | Agent | Agent Runtime | 列出可用 Agent |
| `/use-agent` | Agent | Agent Runtime | 切换 Agent |
| `/current-agent` | Agent | Agent Runtime | 显示当前 Agent |
| `/clear` | 上下文 | Agent Runtime | 清空上下文 |
| `/history` | 上下文 | Agent Runtime | 显示历史 |
| `/compress` | 上下文 | Agent Runtime | 压缩上下文 |
| `/workflow` | 工作流 | Command 模块 | 工作流命令 |
| `/invoke` | 外部 Agent | Command 模块 | 调用外部 Agent |

### 命令注册

```rust
// Router 命令注册
impl Router {
    fn register_builtin_handlers(&mut self) {
        // 内置系统命令
        self.register_handler("help", CommandHandler {
            type: HandlerType::Builtin,
            handler: |args, session| self.cmd_help(args, session).await,
        });

        self.register_handler("status", CommandHandler {
            type: HandlerType::Builtin,
            handler: |args, session| self.cmd_status(args, session).await,
        });

        // 会话管理命令 - 委托给 Session Manager
        self.register_handler("new-session", CommandHandler {
            type: HandlerType::Session,
            handler: |args, session| {
                session.create_session(args).await
            },
        });

        // Agent 管理命令 - 委托给 Agent Runtime
        self.register_handler("list-agents", CommandHandler {
            type: HandlerType::Agent,
            handler: |args, session| {
                session.list_agents(args).await
            },
        });

        // 工作流命令 - 委托给 Command 模块
        self.register_handler("workflow", CommandHandler {
            type: HandlerType::CommandModule,
            handler: |args, session| {
                self.command_module.execute_workflow(args, session).await
            },
        });
    }
}
```

---

## 用户自定义命令

### 命令定义格式

Router 负责检测用户命令并加载定义，但执行委托给 Command 模块。

```markdown
---
name: review
description: 执行代码审查
command_type: simple
---

# Command: review

执行代码审查，支持指定文件或目录。

## Usage

```
/review [文件路径]
```

## Args

- `path` (可选): 要审查的文件或目录路径，默认为当前目录

## Expected Behavior

审查代码的质量、安全性、性能。使用 glob 工具收集文件，然后调用 code-reviewer agent 进行分析。

## Examples

```bash
# 审查当前目录
/review

# 审理指定文件
/review src/App.tsx
```
```

### 命令数据结构（引用自 Command 模块）

完整的 `CommandDefinition` 结构定义见 `command.md` 模块，包含：

- `metadata`: 名称、描述、版本
- `command_type`: simple 或 workflow
- `usage`: 语法、示例、预期行为
- `args`: 参数定义

### 命令委托执行

```rust
// Router 委托用户命令执行
impl Router {
    async fn execute_user_command(
        &self,
        command_name: &str,
        args: Vec<String>,
        session: &Session,
    ) -> Result<RouterResponse> {
        // 加载命令定义
        let definition = self.load_command_definition(command_name).await?;

        // 委托给 Command 模块执行
        match definition.metadata.command_type {
            CommandType::Simple => {
                self.command_module
                    .execute_simple_command(&definition, args, session)
                    .await
            }
            CommandType::Workflow => {
                self.command_module
                    .execute_workflow_command(&definition, args, session)
                    .await
            }
        }
    }
}
```

---

## 工作流命令支持

### 工作流命令列表

Router 识别以下工作流相关命令并委托给 Command 模块：

| 命令 | 说明 |
|------|------|
| `/workflow list` | 列出所有工作流 |
| `/workflow info <name>` | 查看工作流详情 |
| `/workflow <name> [args...]` | 执行工作流（后台） |
| `/workflow exec <name> [args...]` | 执行工作流 |
| `/workflow exec --foreground <name> [args...]` | 前台执行工作流 |
| `/workflow status <workflow-id>` | 查询工作流状态 |
| `/workflow pause <workflow-id>` | 暂停工作流 |
| `/workflow resume <workflow-id>` | 恢复工作流 |
| `/workflow terminate <workflow-id>` | 终止工作流 |
| `/workflow logs <workflow-id>` | 查看工作流日志 |

### 工作流命令路由

```rust
impl Router {
    async fn route_workflow_command(
        &self,
        subcommand: &str,
        args: Vec<String>,
        session: &Session,
    ) -> Result<RouterResponse> {
        match subcommand {
            "list" => {
                self.command_module
                    .list_workflows(session)
                    .await
            }
            "info" => {
                let name = args.get(0).ok_or(Error::MissingWorkflowName)?;
                self.command_module
                    .get_workflow_info(name, session)
                    .await
            }
            "status" | "pause" | "resume" | "terminate" | "logs" => {
                let workflow_id = args.get(0).ok_or(Error::MissingWorkflowId)?;
                self.command_module
                    .handle_workflow_control(subcommand, workflow_id, session)
                    .await
            }
            "exec" => {
                self.command_module
                    .execute_workflow_command_with_args(args, session)
                    .await
            }
            _ => {
                // 默认为执行工作流
                self.command_module
                    .execute_workflow_command_with_args(args, session)
                    .await
            }
        }
    }
}
```

---

## 配置

### 目录结构

```
~/.knight-agent/
└── commands/
    ├── core/                    # 内置命令（暂不存储为文件）
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
    workflow_definitions: "~/.knight-agent/workflows/"

  # 命令缓存
  cache_enabled: true
  cache_ttl: 300                 # 缓存时间（秒）

  # 命令别名
  aliases:
    quit: exit                   # /quit → /exit
    ls: list-sessions            # /ls → /list-sessions
    ws: workflow                # /ws → /workflow

  # 命令优先级
  priority:
    - builtin                   # 内置命令最高优先级
    - session                   # 会话管理命令
    - agent                     # Agent 管理命令
    - workflow                  # 工作流命令
    - user                      # 用户命令
```

---

## 模块交互

### 与 Command 模块的关系

```
┌─────────────────────────────────────────────────────────────┐
│  Router                                                   │
│  - 检测 / 命令                                           │
│  - 识别命令类型                                         │
│  - 加载命令定义                                         │
└─────────────────────────────┬───────────────────────────────┘
                              │ 委托
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Command 模块                                             │
│  - LLM 解析命令意图                                      │
│  - 决定执行方式 (Skill/Agent/Tool)                       │
│  - 执行工作流                                           │
└─────────────────────────────────────────────────────────────┘
```

### 与其他模块的交互

```
┌─────────────────────────────────────────────────────────────┐
│  Router                                                   │
│  - 输入: "/new-session"    ──→ Session Manager          │
│  - 输入: "/list-agents"    ──→ Agent Runtime             │
│  - 输入: "/workflow ..."   ──→ Command 模块              │
│  - 输入: "普通文本"        ──→ Agent (to_agent=true)      │
└─────────────────────────────────────────────────────────────┘
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

  WorkflowNotFound:
    description: 工作流不存在
    message: "工作流不存在: {name}"

  WorkflowCommandError:
    description: 工作流命令错误
    message: "工作流命令错误: {reason}"
```

---

## 测试要点

### 单元测试

- [ ] 命令识别正确性（/ 开头检测）
- [ ] 参数解析正确性
- [ ] 内置命令执行
- [ ] 命令委托到 Command 模块
- [ ] 工作流命令路由
- [ ] 错误处理

### 集成测试

- [ ] 与 Command 模块集成
- [ ] 与 Session Manager 集成
- [ ] 与 Agent Runtime 集成
- [ ] 与 Task Manager 集成（工作流状态查询）

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
async fn test_user_command_delegation() {
    let router = Router::new();
    let session = create_test_session();

    // 假设已加载 review 命令
    let result = router.handle_input("/review src/App.tsx", &session).await;

    assert!(result.to_agent == false);
    // 验证调用了 Command 模块
}

#[tokio::test]
async fn test_workflow_command() {
    let router = Router::new();
    let session = create_test_session();

    let result = router.handle_input("/workflow list", &session).await;

    assert!(result.to_agent == false);
    // 验证调用了 Command 模块的工作流方法
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
| 命令定义加载 | < 100ms | 首次加载 |
| 内置命令执行 | < 10ms | 简单命令 |
| 用户命令执行 | 取决于 Command 模块 | LLM 调用 |

---

## 未来扩展

- [ ] 命令自动补全
- [ ] 命令建议（基于历史）
- [ ] 命令模板
- [ ] 命令组合（管道）
