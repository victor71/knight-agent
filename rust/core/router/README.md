# Router Module

Design Reference: `docs/03-module-design/core/router.md`

## 概述

Router 负责 CLI 输入的路由和分发，包括：
- CLI 命令识别和解析（`/` 开头）
- 内置命令执行
- 用户自定义命令加载并委托给 Command 模块执行
- 非命令输入转发给 Agent

## 导入

```rust
use router::{
    RouterImpl, RouterError, RouterResult, RouterResponse,
    HandleInputRequest, HandleInputResult,
    CommandType, CommandHandler, CommandHandlerType,
    BuiltinCommand, UserCommand, CommandInfo, CommandVariable,
    ParsedInput, Route,
};
```

## 核心类型

### CommandHandlerType
命令处理器类型枚举：`Builtin`, `Session`, `Agent`, `CommandModule`, `TaskManager`

### CommandType
命令类型枚举：`Builtin`, `User`, `Workflow`

### ParsedInput
解析后的输入结构：
- `is_command`: 是否为命令
- `command_name`: 命令名称（不含 `/`）
- `args`: 命令参数
- `raw_input`: 原始输入

### RouterResponse
路由响应结构：
- `success`: 是否成功
- `message`: 返回消息
- `data`: 附加数据
- `error`: 错误信息
- `to_agent`: 是否转发给 Agent

## 对外接口

### 初始化

```rust
use router::RouterImpl;

let router = RouterImpl::new();
router.initialize().await?;
```

### 处理输入

```rust
use router::HandleInputRequest;

let result = router
    .handle_input(HandleInputRequest {
        input: "/help".to_string(),
        session_id: "test".to_string(),
    })
    .await;

if result.to_agent {
    // 转发给 Agent 处理
} else {
    // 命令已处理
    println!("{}", result.response.message);
}
```

### 命令判断

```rust
// 判断输入是否为命令
if RouterImpl::is_command(&input) {
    // 处理命令
} else {
    // 转发给 Agent
}
```

### 列出命令

```rust
// 列出所有命令
let all = router.list_commands(None).await;

// 只列出内置命令
let builtin = router.list_commands(Some("builtin")).await;

// 只列出用户命令
let user = router.list_commands(Some("user")).await;
```

### 注册用户命令

```rust
use router::{UserCommand, CommandHandler, CommandHandlerType, CommandVariable};

let user_cmd = UserCommand {
    name: "greet".to_string(),
    description: "Greet someone".to_string(),
    template: "Hello, {{name}}!".to_string(),
    variables: vec![CommandVariable {
        name: "name".to_string(),
        description: "Name to greet".to_string(),
        required: true,
        default: None,
    }],
    handler: CommandHandler {
        handler_type: CommandHandlerType::CommandModule,
        name: "greet".to_string(),
    },
};

router.register_user_command(user_cmd).await?;
```

### 获取命令信息

```rust
if let Some(cmd) = router.get_command("help").await {
    println!("Command: {} - {}", cmd.name, cmd.description);
}
```

## 内置命令

| 命令 | 描述 | 别名 |
|------|------|------|
| `/help` | 显示帮助信息 | `?` |
| `/clear` | 清屏 | `cl` |
| `/exit` | 退出应用 | `quit`, `q` |
| `/status` | 显示当前状态 | `stat`, `st` |
| `/session` | 会话管理 | `sess` |
| `/agent` | Agent 管理 | - |
| `/history` | 命令历史 | `hist` |
| `/command` | 列出用户命令 | `cmd` |

## 完整示例

```rust
use router::{RouterImpl, HandleInputRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = RouterImpl::new();
    router.initialize().await?;

    loop {
        let input = read_line();

        let result = router
            .handle_input(HandleInputRequest {
                input: input.clone(),
                session_id: "main".to_string(),
            })
            .await;

        if result.to_agent {
            println!("Forwarding to agent: {}", input);
        } else {
            if result.response.success {
                println!("{}", result.response.message);
            } else {
                eprintln!("Error: {}", result.response.error.unwrap_or_default());
            }
        }

        if input == "/exit" {
            break;
        }
    }

    Ok(())
}
```

## 错误处理

RouterError 枚举定义：
- `NotInitialized`: 路由器未初始化
- `RouteNotFound(String)`: 路由未找到
- `RoutingFailed(String)`: 路由失败
- `CommandNotFound(String)`: 命令未找到
- `InvalidCommand(String)`: 无效命令
