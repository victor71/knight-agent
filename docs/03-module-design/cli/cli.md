# CLI (命令行接口)

## 概述

### 职责描述

CLI 模块是 Knight-Agent 系统的用户入口，负责：

- **System CLI**：`knight` 二进制程序的用户界面，包括守护进程管理和系统命令
- **REPL**：交互式命令行环境，支持 Slash Commands 和自然语言交互
- **IPC 通信**：与 Rust 核心服务通过 IPC 协议通信

### 架构概述

```
┌─────────────────────────────────────────────────────────────┐
│                      knight (二进制)                         │
│  ┌─────────────────┐  ┌─────────────────────────────────┐   │
│  │   System CLI    │  │           REPL                   │   │
│  │                 │  │  ┌─────────────────────────┐    │   │
│  │ daemon start    │  │  │  Slash Commands Parser  │    │   │
│  │ daemon stop     │  │  │  Natural Language Mode │    │   │
│  │ daemon status   │  │  └─────────────────────────┘    │   │
│  │ daemon restart  │  │                                 │   │
│  │ health          │  │  /session /workflow /ask /log   │   │
│  │ version         │  │  /config /schedule /exit       │   │
│  └─────────────────┘  └─────────────────────────────────┘   │
└───────────────────────────┬─────────────────────────────────┘
                            │ IPC (Unix Socket / TCP / stdio)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  knight-agent (守护进程)                     │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │  Session    │  │  Workflow   │  │   Agent     │        │
│  │  Manager    │  │  Manager    │  │  Runtime    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

### 设计目标

1. **简洁交互**：用户友好的命令行界面
2. **可靠通信**：稳定的 IPC 连接和错误处理
3. **快速响应**：即时反馈和状态更新
4. **优雅降级**：无守护进程时也能工作

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| IPC Contract | 依赖 | 与核心服务通信。见 [IPC Contract](../infrastructure/ipc-contract.md) |
| Session Manager | 间接 | REPL 命令委托给 Session Manager |
| Task Manager | 间接 | /workflow 命令委托给 Task Manager |
| LLM Provider | 间接 | 自然语言理解需要 LLM |

### 被依赖模块

| 模块 | 依赖类型 | 说明 |
|------|---------|------|
| 用户 | 入口 | 用户通过 CLI 与系统交互 |

---

## CLI 二进制程序

### 程序入口

```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { action } => handle_daemon(action).await,
        Commands::Health => handle_health().await,
        Commands::Version => handle_version(),
        Commands::Repl { connect } => handle_repl(connect).await,
    }
}
```

### 命令结构

```rust
pub enum Commands {
    /// 守护进程管理
    Daemon {
        action: DaemonAction,
    },
    /// 健康检查
    Health,
    /// 版本信息
    Version,
    /// 启动 REPL
    Repl {
        connect: Option<String>,  // 连接指定地址
    },
}

pub enum DaemonAction {
    Start,
    Stop,
    Status,
    Restart,
}
```

---

## System CLI 命令

### 命令列表

| 命令 | 说明 | 示例 |
|------|------|------|
| `knight` | 启动 REPL（默认） | `knight` |
| `knight daemon start` | 启动守护进程 | `knight daemon start` |
| `knight daemon stop` | 停止守护进程 | `knight daemon stop` |
| `knight daemon status` | 查看守护进程状态 | `knight daemon status` |
| `knight daemon restart` | 重启守护进程 | `knight daemon restart` |
| `knight health` | 健康检查 | `knight health` |
| `knight version` | 显示版本 | `knight version` |

### 实现流程

#### daemon start

```
knight daemon start
    ↓
检查是否已有守护进程运行
    ↓ (无)
启动新的守护进程
    ↓
写入 PID 文件 ~/.knight-agent/knight-agent.pid
    ↓
输出 "Knight-Agent daemon started"
```

#### daemon stop

```
knight daemon stop
    ↓
读取 PID 文件
    ↓
发送 SIGTERM 信号
    ↓
等待进程退出（超时 30s）
    ↓
删除 PID 文件
    ↓
输出 "Knight-Agent daemon stopped"
```

#### daemon status

```
knight daemon status
    ↓
读取 PID 文件
    ↓
检查进程是否存在
    ↓
显示状态信息：
  - Running: PID xxx
  - 或 Not running
```

---

## REPL 交互界面

### 启动流程

```
knight (或 knight repl)
    ↓
检查守护进程是否运行
    ↓ (否)
    ├─ 自动启动守护进程 (daemon start)
    ↓ (是)
连接到守护进程
    ↓
显示欢迎信息
    ↓
进入 REPL 循环
```

### 欢迎界面

```
╔═══════════════════════════════════════════════════════╗
║           Welcome to Knight Agent v1.0.0              ║
║                                                       ║
║  Type /help for available commands                    ║
║  Or just type naturally to interact with your Agent  ║
╚═══════════════════════════════════════════════════════╝

> _
```

### Slash Commands

| 命令 | 说明 | 子命令 |
|------|------|--------|
| `/session` | 会话管理 | create, list, use, delete, info, export, search |
| `/workflow` | 工作流管理 | list, info, exec, status, pause, resume, terminate, logs |
| `/schedule` | 定时任务 | list, info, history |
| `/ask` | Agent 交互 | `<agent>[:<variant>] <message>` |
| `/log` | 日志查看 | view, search |
| `/config` | 配置管理 | get, set, list |
| `/health` | 健康检查 | - |
| `/diagnose` | 诊断信息 | - |
| `/cache` | 缓存管理 | clear |
| `/exit` | 退出 | - |

### REPL 命令解析

```rust
pub enum ReplInput {
    /// Slash 命令
    SlashCommand {
        command: String,
        args: Vec<String>,
    },
    /// 自然语言输入
    NaturalLanguage {
        text: String,
    },
    /// 空行
    Empty,
}

impl ReplInput {
    pub fn parse(line: &str) -> Self {
        let line = line.trim();

        if line.is_empty() {
            return ReplInput::Empty;
        }

        if line.starts_with('/') {
            return Self::parse_slash_command(line);
        }

        ReplInput::NaturalLanguage {
            text: line.to_string(),
        }
    }

    fn parse_slash_command(line: &str) -> Self {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let command = parts[0].trim_start_matches('/').to_string();
        let args = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        ReplInput::SlashCommand { command, args }
    }
}
```

---

## IPC 通信

### 连接管理

```rust
pub struct CliIpcClient {
    connection: IpcConnection,
}

impl CliIpcClient {
    /// 连接到守护进程
    pub async fn connect(addr: &str) -> Result<Self> {
        let connection = IpcConnection::connect(addr).await?;
        Ok(Self { connection })
    }

    /// 发送命令并等待响应
    pub async fn send_command(&self, cmd: IpcCommand) -> Result<IpcResponse> {
        self.connection.send(cmd).await
    }
}
```

### 消息格式

参见 [IPC Contract - 消息格式](../infrastructure/ipc-contract.md#消息格式)

### 请求类型

```rust
pub enum IpcCommand {
    // 系统命令
    DaemonStart,
    DaemonStop,
    DaemonStatus,

    // 会话命令
    SessionCreate(SessionConfig),
    SessionList,
    SessionUse(SessionId),
    SessionDelete(SessionId),

    // 工作流命令
    WorkflowList,
    WorkflowExec(WorkflowName, Vec<String>),
    WorkflowStatus(WorkflowId),

    // Agent 命令
    Ask {
        agent: String,
        variant: Option<String>,
        message: String,
    },

    // 日志命令
    LogView(LogOptions),
    LogSearch(String),

    // 配置命令
    ConfigGet(String),
    ConfigSet(String, Value),
    ConfigList,
}
```

---

## 实现结构

### 文件结构

```
knight-agent/
├── cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # 程序入口
│       ├── commands/         # System CLI 命令
│       │   ├── mod.rs
│       │   ├── daemon.rs     # daemon 命令
│       │   ├── health.rs     # health 命令
│       │   └── version.rs    # version 命令
│       ├── repl/             # REPL 实现
│       │   ├── mod.rs
│       │   ├── parser.rs     # 输入解析
│       │   ├── handler.rs    # 命令处理
│       │   └── renderer.rs   # 输出渲染
│       └── ipc/              # IPC 客户端
│           ├── mod.rs
│           └── client.rs
```

### 核心组件

```rust
// REPL 核心循环
pub struct Repl {
    history: Vec<String>,
    ipc_client: CliIpcClient,
}

impl Repl {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            let line = self.read_line().await?;

            match ReplInput::parse(&line) {
                ReplInput::Empty => continue,
                ReplInput::SlashCommand { command, args } => {
                    self.handle_slash_command(&command, &args).await?;
                }
                ReplInput::NaturalLanguage { text } => {
                    self.handle_natural_language(&text).await?;
                }
            }

            if self.should_exit() {
                break;
            }
        }

        Ok(())
    }
}
```

---

## 配置文件

### CLI 配置

```yaml
# ~/.knight-agent/config/cli.yaml
cli:
  # 默认 REPL 提示符
  prompt: "> "

  # 历史记录
  history_size: 1000
  history_file: "~/.knight-agent/.cli_history"

  # 连接配置
  connection:
    # 自动启动守护进程
    auto_start_daemon: true
    # 连接超时（秒）
    connect_timeout: 30
    # 重试次数
    retry_count: 3

  # 输出格式
  output:
    # 彩色输出
    color: true
    # 显示时间戳
    timestamp: false
    # 表格对齐
    table_align: true
```

---

## 错误处理

### 错误类型

| 错误 | 说明 | 处理方式 |
|------|------|----------|
| `DaemonNotRunning` | 守护进程未运行 | 自动启动或提示用户 |
| `ConnectionFailed` | 连接失败 | 重试并显示错误 |
| `CommandNotFound` | 未知命令 | 显示帮助信息 |
| `SessionNotFound` | 会话不存在 | 列出可用会话 |
| `Timeout` | 操作超时 | 显示超时错误 |

### 错误恢复

```rust
async fn handle_connection_error(err: IpcError) -> Result<()> {
    match err {
        IpcError::DaemonNotRunning => {
            println!("Daemon is not running. Starting...");
            let start_result = daemon::start().await;
            match start_result {
                Ok(_) => {
                    // 重试连接
                    reconnect().await
                }
                Err(e) => {
                    eprintln!("Failed to start daemon: {}", e);
                    Err(e)
                }
            }
        }
        IpcError::ConnectionFailed { retry: n } if n < 3 => {
            println!("Connection failed. Retrying ({}/3)...", n + 1);
            tokio::time::sleep(Duration::from_secs(1)).await;
            reconnect().await
        }
        other => Err(other.into()),
    }
}
```

---

## 测试要点

### 单元测试

- [ ] 命令解析正确性
- [ ] 参数验证
- [ ] IPC 消息序列化

### 集成测试

- [ ] 与守护进程通信
- [ ] REPL 循环正确性
- [ ] 错误恢复机制

### 测试用例

```rust
#[test]
fn test_parse_slash_command() {
    let input = "/session create --name test";
    let parsed = ReplInput::parse(input);

    match parsed {
        ReplInput::SlashCommand { command, args } => {
            assert_eq!(command, "session");
            assert!(args.contains("create"));
            assert!(args.contains("test"));
        }
        _ => panic!("Expected SlashCommand"),
    }
}

#[test]
fn test_parse_natural_language() {
    let input = "帮我审查这段代码";
    let parsed = ReplInput::parse(input);

    match parsed {
        ReplInput::NaturalLanguage { text } => {
            assert_eq!(text, "帮我审查这段代码");
        }
        _ => panic!("Expected NaturalLanguage"),
    }
}
```

---

## 未来扩展

- [ ] 支持配置文件热重载
- [ ] 支持命令别名
- [ ] 支持键盘快捷键
- [ ] 支持输出分页
- [ ] 支持交互式参数输入

---

## TUI 模块

Knight Agent 还提供 TUI (Terminal User Interface) 模式，作为 REPL 的增强版本。

详见: [TUI 模块设计](./tui.md)
