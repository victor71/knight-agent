# TUI (终端用户界面)

## 概述

TUI (Terminal User Interface) 模块提供的交互式终端用户界面，作为 REPL 的增强版本。

## 职责描述

TUI 模块负责：

- **终端渲染**: 使用 ratatui 渲染用户界面
- **事件处理**: 处理键盘输入和事件分发
- **状态管理**: 维护 UI 状态和系统状态同步
- **Widget 系统**: Header、Main Output、Input、Status Bar、Popup 等组件
- **Session 管理 UI**: Session 列表和切换界面
- **Task 管理 UI**: Task 列表和运行时长显示

## 架构概述

```
┌─────────────────────────────────────────────────────────────┐
│                        TUI Application                      │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Render Loop (16ms, ~60fps)                  │  │
│  │  - Clone state snapshots                              │  │
│  │  - Draw widgets                                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Input Handler (crossterm)                  │  │
│  │  - Key events → command mode / text editing            │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Status Update Task                       │  │
│  │  - Subscribe to Monitor.watch()                        │  │
│  │  - Subscribe to ConfigLoader.subscribe()               │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                          │
                    Channels (mpsc)
                          │
┌─────────────────────────────────────────────────────────────┐
│              Existing Modules (read-only access)              │
│  - KnightAgentSystem                                     │
│  - MonitorImpl                                             │
│  - OrchestratorImpl                                        │
│  - SessionManagerImpl                                     │
│  - AgentRuntimeImpl                                       │
└─────────────────────────────────────────────────────────────┘
```

## UI 布局

```
┌────────────────────────────────────────────────────────┐
│ 📦 D:/workspace/knight-agent     [main]     14:32:05  │ ← Header (3行)
│ Session: abc123-def4  [+ New]  [- Switch]             │
│ Agents: 5 | Tasks: 3 | Memory: 256MB                   │
├────────────────────────────────────────────────────────┤
│                                                        │
│  [Main Output Area - Rich text, code highlighting]      │
│                                                        │
├────────────────────────────────────────────────────────┤
│ knight> help                              [INSERT]     │ ← Input (2行)
├────────────────────────────────────────────────────────┤
│ 🟢 Running | 🔄 code-reviewer 00:02:34 | Tasks: 2     │ ← Status Bar (2行)
│ Tokens: 12,345/200K (6%) | Context: 12MB/25MB (48%)   │
└────────────────────────────────────────────────────────┘
```

## 模块结构

```
rust/tui/
├── Cargo.toml
├── src/
    ├── lib.rs              # Public API
    ├── app.rs              # AppState, main TUI app struct
    ├── event.rs            # AppEvent enum, EventHandler
    ├── state.rs            # State snapshot types
    ├── layout.rs           # Layout definitions
    ├── renderer.rs         # Terminal wrapper, draw loop
    └── widgets/
        ├── mod.rs
        ├── header.rs       # Header widget (project info)
        ├── main_output.rs  # Main output area (rich text)
        ├── input.rs        # Input line (editable)
        ├── status.rs       # Status bar widget
        ├── session_popup.rs # Session list popup
        └── task_popup.rs   # Task list popup
```

## 核心数据结构

### AppEvent

```rust
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Input events
    Input(KeyEvent),
    Paste(String),

    // System events
    Tick,
    Resize { columns: u16, rows: u16 },

    // Status updates
    SystemStatusUpdate(SystemStatusSnapshot),
    AgentUpdate(Vec<AgentInfo>),
    SessionUpdate(SessionInfo),
    ConfigChange(ConfigChangeEvent),

    // Output events
    OutputLine(OutputLine),
    StreamChunk(String),
    ClearOutput,

    // Session events
    SessionListUpdate(Vec<SessionListItem>),
    SessionSwitch(String),

    // Task events
    TaskListUpdate(Vec<TaskInfo>),
    TaskStart(String),
    TaskComplete(String),
    TaskDurationUpdate(Duration),

    // Session metrics events
    TokenUsageUpdate(SessionTokenUsage),
    ContextCompressionUpdate(ContextCompressionStatus),
}
```

### AppState

```rust
pub struct AppState {
    // UI state
    pub terminal_size: (u16, u16),
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub active_popup: Option<PopupType>,

    // Output state
    pub output_lines: Vec<OutputLine>,
    pub output_scroll: usize,
    pub max_output_lines: usize,

    // System state cache (snapshots for rendering)
    pub system_status: SystemStatusSnapshot,
    pub agents: Vec<AgentInfo>,
    pub session_info: SessionInfo,
    pub project_info: ProjectInfo,

    // Session management
    pub sessions: Vec<SessionListItem>,
    pub selected_session_index: usize,

    // Task management
    pub tasks: Vec<TaskInfo>,
    pub current_task_start: Option<DateTime<Local>>,
    pub current_task_duration: Option<Duration>,

    // Session metrics
    pub session_token_usage: SessionTokenUsage,
    pub context_compression_status: ContextCompressionStatus,

    // Channels
    pub event_tx: mpsc::UnboundedSender<AppEvent>,

    // Time
    pub current_time: DateTime<Local>,
}
```

## 快捷键绑定

| 按键 | 功能 | 模式 |
|------|------|------|
| `Alt+N` | 创建新 Session | Normal |
| `Alt+S` | 打开 Session 切换弹窗 | Normal |
| `Alt+T` | 打开 Task 列表弹窗 | Normal |
| `i`, `a` | 进入 Insert 模式 | Normal |
| `Esc` | 返回 Normal 模式 | Insert |
| `/`, `:` | 进入 Insert 模式 | Normal |
| `Up/Down` | 弹窗内导航 | Popup |
| `Enter` | 确认选择 | Popup |
| `q` + `Ctrl` | 快速退出 | Normal |

## 集成点

| 现有模块 | TUI 集成方式 |
|---------|-------------|
| `MonitorImpl` | `watch()` → 状态更新流 |
| `OrchestratorImpl` | `list_agents()` → Agent 列表 |
| `SessionManagerImpl` | `get_current_session()`, `list_sessions()` → 会话信息 |
| `AgentRuntimeImpl` | `list_agents()`, `get_agent_state()` → Agent 状态 |
| `TaskManagerImpl` | `list_tasks()`, `get_current_task()` → 任务列表 |
| `KnightAgentSystem` | `status()`, `health_check()`, `version()` → 系统状态 |
| `ConfigLoader` | `subscribe()` → 配置变更 |

## 启动方式

```bash
# 默认使用 TUI
knight

# 使用 REPL (fallback)
knight --no-tui
```

## 依赖库

| 库 | 版本 | 用途 |
|----|------|------|
| ratatui | 0.29 | TUI 框架 |
| crossterm | 0.28 | 跨平台终端操作 |
| syntect | 5.2 | 代码语法高亮 |
| chrono | 0.4 | 时间处理 |
| unicode-width | 0.2 | Unicode 字符宽度计算 |

## 性能要求

| 指标 | 目标值 |
|------|--------|
| 帧率 | 60 FPS (~16ms) |
| 响应延迟 | < 100ms |
| 内存占用 | < 50MB (不含数据) |
| 输出缓冲 | 1000 行 (可配置) |

## 测试要点

- [ ] 终端大小调整响应正确
- [ ] 所有快捷键正常工作
- [ ] Session 切换功能正常
- [ ] Task 列表实时更新
- [ ] Token 使用统计准确
- [ ] 上下文压缩状态显示正确
- [ ] 退出时终端状态正确恢复
