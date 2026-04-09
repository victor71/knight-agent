# Knight-Agent

> 可扩展的 Agentic 工具开发框架

## 概述

Knight-Agent 是一个受 Claude Code 和 OpenClaw 启发的 Agentic 框架，支持：

- **自然语言工作流** - 使用自然语言定义多 Agent 协作工作流，支持数天级别的后台执行
- **自定义 Agent** - 通过 Markdown 定义 Agent 角色和能力
- **Agent 变体** - 同一 Agent 的不同配置变体（如 architect、developer、tester）
- **动态 Agent 创建** - 根据工作流需求自动创建指定变体的 Agent
- **自定义 Skill** - 通过 Markdown 定义可复用行为模式
- **Skill Pipeline** - 技能管道组合，支持依赖和条件执行
- **多会话并行** - 同时在多个项目中工作，完全隔离
- **多 Agent 协作** - 支持主从、流水线、投票等协作模式
- **MCP 工具集成** - 通过 MCP 协议扩展工具能力
- **Hook 系统** - 事件钩子支持插件扩展（before/after/replace）
- **事件驱动** - 文件变更、Git 事件、定时任务自动触发
- **定时器系统** - 自然语言创建定时任务，支持一次性/周期性任务
- **7×24 运行** - 事件驱动的长期运行能力

## 项目结构

```
knight-agent/
├── docs/
│   ├── 00-priority-overview.md        # 优先级总览
│   ├── 01-requirements-analysis.md    # 需求分析
│   ├── 02-system-design.md            # 系统架构设计
│   ├── 03-module-design/              # 模块详细设计
│   │   ├── README.md                  # 模块设计索引
│   │   ├── cli/                       # CLI 模块 (1)
│   │   │   └── cli.md                 # 命令行接口
│   │   ├── core/                      # 核心引擎模块 (8)
│   │   │   ├── bootstrap.md           # 系统启动器
│   │   │   ├── session-manager.md     # 会话管理器
│   │   │   ├── orchestrator.md        # 编排器
│   │   │   ├── router.md              # 路由器
│   │   │   ├── command.md             # 命令系统
│   │   │   ├── event-loop.md          # 事件循环
│   │   │   ├── hook-engine.md         # Hook 引擎
│   │   │   └── monitor.md             # 监控模块
│   │   ├── agent/                     # Agent 运行模块 (6)
│   │   │   ├── agent-runtime.md       # Agent 运行时
│   │   │   ├── agent-variants.md      # Agent 变体系统
│   │   │   ├── external-agent.md      # 外部 Agent 集成
│   │   │   ├── skill-engine.md        # 技能引擎
│   │   │   ├── task-manager.md        # 任务管理器
│   │   │   └── workflows-directory.md # 工作流目录
│   │   ├── services/                  # 基础服务模块 (7)
│   │   │   ├── llm-provider.md        # LLM 提供者抽象
│   │   │   ├── mcp-client.md          # MCP 客户端
│   │   │   ├── storage-service.md     # 存储服务
│   │   │   ├── context-compressor.md  # 上下文压缩
│   │   │   ├── timer-system.md        # 定时器系统
│   │   │   ├── logging-system.md      # 日志系统
│   │   │   └── report-skill.md        # 报告技能
│   │   ├── tools/                     # 工具系统 (1)
│   │   │   └── tool-system.md         # 工具框架
│   │   ├── infrastructure/            # 基础设施模块 (1)
│   │   │   └── ipc-contract.md        # 进程间通信契约
│   │   └── security/                  # 安全模块 (2)
│   │       ├── security-manager.md    # 安全管理器
│   │       └── sandbox.md             # 沙箱机制
│   ├── 04-testing-design.md           # L0/L1 测试设计
│   └── 05-technical-baseline-tests.md # 技术基线测试
├── workflows/                         # 工作流定义目录
│   ├── README.md
│   ├── software-development/
│   │   ├── feature-development.md
│   │   ├── bug-fix.md
│   │   └── refactoring.md
│   ├── code-quality/
│   ├── deployment/
│   └── documentation/
├── SRS.md                             # 软件需求规格说明书
├── CLAUDE.md                          # Claude Code 配置
└── README.md                          # 项目说明
```

## 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│  用户接口层 (CLI / Web UI)                                   │
│  /workflow <name> <args>                                     │
├─────────────────────────────────────────────────────────────┤
│  核心引擎层                                                  │
│  ┌─────────┬─────────┬─────────┬─────────┬───────────┐   │
│  │Bootstrap│Orchestr│ Router  │Hook     │Event     │   │
│  │         │ator    │         │Engine   │Loop      │   │
│  ├─────────┼─────────┼─────────┼─────────┼───────────┤   │
│  │Session  │Command  │Timer    │Monitor  │Security  │   │
│  │Manager  │System   │System   │         │Manager   │   │
│  └─────────┴─────────┴─────────┴─────────┴───────────┘   │
├─────────────────────────────────────────────────────────────┤
│  Agent 运行层                                                 │
│  ┌──────────────┬──────────────┬──────────────┬───────────┐ │
│  │Agent         │Skill Engine  │Task Manager  │Agent      │ │
│  │Runtime       │              │              │Variants   │ │
│  └──────────────┴──────────────┴──────────────┴───────────┘ │
├─────────────────────────────────────────────────────────────┤
│  工作流层                                                     │
│  ┌──────────────┬──────────────┬──────────────┬───────────┐ │
│  │Workflows     │Workflow      │Background    │Workflow   │ │
│  │Directory     │Parser (LLM)  │Execution     │Recovery   │ │
│  └──────────────┴──────────────┴──────────────┴───────────┘ │
├─────────────────────────────────────────────────────────────┤
│  基础服务层                                                  │
│  ┌──────────────┬──────────────┬──────────────┬───────────┐ │
│  │LLM Provider  │MCP Client    │Storage       │Context    │ │
│  │              │              │Service      │Compressor │ │
│  └──────────────┴──────────────┴──────────────┴───────────┘ │
├─────────────────────────────────────────────────────────────┤
│  工具层                                                      │
│  Read │ Write │ Edit │ Glob │ Grep │ Bash │ Git │ MCP    │
├─────────────────────────────────────────────────────────────┤
│  安全层 (横切关注点)                                         │
│  ┌──────────────┬──────────────┐                              │
│  │Security      │Sandbox       │                              │
│  │Manager      │              │                              │
│  └──────────────┴──────────────┘                              │
└─────────────────────────────────────────────────────────────┘
```

### 工作流执行流程

```
用户输入: /workflow feature-development docs/requirements.md
        │
        ▼
┌──────────────────────────────────────────┐
│ Command 模块                              │
│ - 识别 workflow 命令                      │
│ - 加载 workflows/feature-development.md   │
└──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────┐
│ LLM 解析工作流定义                        │
│ - 提取任务列表                            │
│ - 解析依赖关系（DAG）                     │
│ - 识别 Agent 变体需求                     │
└──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────┐
│ Task Manager                              │
│ - 后台执行模式                            │
│ - 支持 DAG 调度                           │
└──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────┐
│ Orchestrator                              │
│ - 动态创建所需变体的 Agent                │
│ - architect → 新建                       │
│ - developer → 新建                       │
│ - tester → 新建                          │
└──────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────┐
│ 执行工作流 DAG                            │
│ - 按依赖顺序执行                          │
│ - 定期持久化检查点                        │
│ - 支持断点恢复                            │
└──────────────────────────────────────────┘
```

### 架构说明

- **垂直分层**: 用户接口层 → 核心引擎层 → Agent 运行层 → 基础服务层 → 工具层，自上而下调用
- **工作流层**: 连接 Command、Task Manager、Orchestrator，实现自然语言工作流的解析和执行
- **安全层** (Security Manager + Sandbox) 为 **横切关注点**，与所有层交互

## 技术栈

| 模块 | 技术 |
|------|------|
| 核心引擎 | Rust |
| Web UI | Next.js + TypeScript |
| MCP 适配 | TypeScript |
| 存储 | SQLite + 文件系统 |

## 文档

### 概览文档

| 文档 | 描述 |
|------|------|
| [00-priority-overview](./docs/00-priority-overview.md) | 优先级总览 |
| [01-requirements-analysis](./docs/01-requirements-analysis.md) | 需求分析 |
| [02-system-design](./docs/02-system-design.md) | 系统架构设计 |
| [03-module-design/README](./docs/03-module-design/README.md) | 模块设计索引 |
| [04-testing-design](./docs/04-testing-design.md) | L0/L1 测试设计 |
| [05-technical-baseline-tests](./docs/05-technical-baseline-tests.md) | 技术基线测试 |
| [SRS](./SRS.md) | 软件需求规格说明书 |

### 模块设计文档

| 模块 | 文档 | 状态 |
|------|------|------|
| **CLI** | | |
| 命令行接口 | [cli](./docs/03-module-design/cli/cli.md) | ✅ |
| **核心引擎** | | |
| 系统启动器 | [bootstrap](./docs/03-module-design/core/bootstrap.md) | ✅ |
| 会话管理器 | [session-manager](./docs/03-module-design/core/session-manager.md) | ✅ |
| 编排器 | [orchestrator](./docs/03-module-design/core/orchestrator.md) | ✅ |
| 路由器 | [router](./docs/03-module-design/core/router.md) | ✅ |
| 命令系统 | [command](./docs/03-module-design/core/command.md) | ✅ |
| 事件循环 | [event-loop](./docs/03-module-design/core/event-loop.md) | ✅ |
| Hook 引擎 | [hook-engine](./docs/03-module-design/core/hook-engine.md) | ✅ |
| 监控模块 | [monitor](./docs/03-module-design/core/monitor.md) | ✅ |
| **Agent 运行** | | |
| Agent 运行时 | [agent-runtime](./docs/03-module-design/agent/agent-runtime.md) | ✅ |
| Agent 变体 | [agent-variants](./docs/03-module-design/agent/agent-variants.md) | ✅ |
| 外部 Agent | [external-agent](./docs/03-module-design/agent/external-agent.md) | ✅ |
| Skill 引擎 | [skill-engine](./docs/03-module-design/agent/skill-engine.md) | ✅ |
| 任务管理器 | [task-manager](./docs/03-module-design/agent/task-manager.md) | ✅ |
| 工作流目录 | [workflows-directory](./docs/03-module-design/agent/workflows-directory.md) | ✅ |
| **基础服务** | | |
| LLM 提供者 | [llm-provider](./docs/03-module-design/services/llm-provider.md) | ✅ |
| MCP 客户端 | [mcp-client](./docs/03-module-design/services/mcp-client.md) | ✅ |
| 存储服务 | [storage-service](./docs/03-module-design/services/storage-service.md) | ✅ |
| 上下文压缩 | [context-compressor](./docs/03-module-design/services/context-compressor.md) | ✅ |
| 定时器系统 | [timer-system](./docs/03-module-design/services/timer-system.md) | ✅ |
| 日志系统 | [logging-system](./docs/03-module-design/services/logging-system.md) | ✅ |
| 报告技能 | [report-skill](./docs/03-module-design/services/report-skill.md) | ✅ |
| **工具系统** | | |
| 工具框架 | [tool-system](./docs/03-module-design/tools/tool-system.md) | ✅ |
| **基础设施** | | |
| 进程间通信 | [ipc-contract](./docs/03-module-design/infrastructure/ipc-contract.md) | ✅ |
| **安全模块** | | |
| 安全管理器 | [security-manager](./docs/03-module-design/security/security-manager.md) | ✅ |
| 沙箱机制 | [sandbox](./docs/03-module-design/security/sandbox.md) | ✅ |

## 功能优先级

### P0 - 核心模块 (设计完成)

- ✅ 会话管理器 - 多会话并行、Workspace 隔离
- ✅ Agent 运行时 - Agent 生命周期管理
- ✅ LLM 提供者抽象 - 多模型支持
- ✅ 工具框架 - 统一工具接口

### P1 - 扩展模块 (设计完成)

- ✅ 编排器 - 多 Agent 协作、任务调度、动态 Agent 创建
- ✅ 路由器 - CLI 命令处理、请求分发
- ✅ 命令系统 - 用户可定义命令（Markdown）、工作流命令支持
- ✅ Skill 引擎 - 技能定义、触发、执行、Pipeline 组合
- ✅ 事件循环 - 文件变更、Git 事件、定时任务
- ✅ Hook 引擎 - before/after/replace 事件钩子
- ✅ 任务管理器 - DAG 依赖、并行执行、后台工作流、持久化恢复
- ✅ 工作流目录 - 自然语言工作流定义
- ✅ MCP 客户端 - MCP 协议集成、工具发现
- ✅ 上下文压缩 - 智能摘要、语义压缩
- ✅ 存储服务 - SQLite 持久化、备份恢复
- ✅ 定时器系统 - 一次性/周期性/Cron 定时器
- ✅ 日志系统 - 结构化日志、异步写入、日志轮转
- ✅ 监控模块 - Token 统计、状态监控、资源监控、历史数据持久化
- ✅ 报告技能 - 每日/每周/每月使用报告生成
- ✅ Agent 变体 - 多配置支持（architect, developer, tester 等）
- ✅ 外部 Agent - Claude Code 等外部 Agent 集成

### P2 - 安全和运维 (设计完成)

- ✅ 安全管理器 - 权限控制、审计日志
- ✅ 沙箱机制 - 资源隔离、访问控制

## 路线图

### 设计阶段 ✅

- [x] 需求分析
- [x] 系统架构设计
- [x] 模块详细设计
- [x] 工作流系统设计

### 实现阶段 (规划中)

详细周度规划见 [需求分析文档](./docs/01-requirements-analysis.md#里程碑规划)

| 阶段 | 目标 | 周期 |
|------|------|------|
| Phase 1 | 核心 Agent 框架 | Week 1-4 |
| Phase 2 | Skill 系统 + 会话增强 | Week 5-7 |
| Phase 3 | 协作能力 + 工作流 | Week 8-10 |
| Phase 4 | 自动化 | Week 11-12 |
| Phase 5 | 生态 | Week 13-14 |

## 设计亮点

1. **自然语言工作流** - 用 Markdown + 自然语言定义复杂的多 Agent 协作流程
2. **动态 Agent 创建** - 根据工作流需求自动创建指定变体的 Agent
3. **长时间运行支持** - 工作流可运行数天，支持断点恢复
4. **模块化设计** - 独立模块，职责清晰
5. **灵活扩展** - Agent 变体、Skill Pipeline、Hook 系统支持扩展
6. **事件驱动** - Event Loop + Hook Engine 实现自动化
7. **安全优先** - Security Manager + Sandbox 双重保护
8. **可观测** - 完整的审计日志和状态追踪

## 构建与运行

### 前置要求

- Rust 1.70+
- Cargo

### 构建

```bash
# 构建项目
cargo build -p knight-agent

# Release 模式构建
cargo build -p knight-agent --release
```

### 运行

```bash
# 运行 knight-agent
./target/debug/knight-agent.exe     # Windows
./target/debug/knight-agent         # macOS/Linux

# 或使用 cargo 运行
cargo run -p knight-agent
```

### 配置目录

首次运行时，knight-agent 会在用户主目录创建 `.knight-agent` 配置目录：

| 平台 | 路径 |
|------|------|
| Windows | `C:\Users\<用户名>\.knight-agent\` |
| macOS | `~/.knight-agent/` |
| Linux | `~/.knight-agent/` |

**目录结构：**

```
.knight-agent/
├── knight.json          # LLM 提供者配置（用户常用）
├── config/              # 系统配置（YAML 格式，已合并）
│   ├── agent.yaml       # Agent 模块（已合并 6 个模块）
│   ├── core.yaml        # Core 模块（已合并 8 个模块）
│   ├── services.yaml    # Services（已合并 3 个服务）
│   ├── tools.yaml       # 工具系统
│   ├── infrastructure.yaml # 基础设施（IPC）
│   ├── storage.yaml     # 存储配置
│   ├── security.yaml    # 安全配置
│   ├── logging.yaml     # 日志配置
│   ├── monitoring.yaml  # 监控配置
│   └── compressor.yaml  # 上下文压缩配置
├── sessions/            # 会话数据
├── logs/                # 日志文件
├── skills/              # 自定义技能
├── commands/            # 自定义命令
└── workspace/           # 工作区
```

**配置说明：**
- **knight.json** - 唯一用户需要配置的文件，包含 LLM 提供者设置（JSON 格式）
- **全局配置存储** - 使用 OnceLock 实现线程安全的全局配置单例，所有模块统一读取
- **热重载支持** - 配置变更自动检测，LLM Router 支持运行时重载配置
- **config/agent.yaml** - Agent 相关配置（已合并 agent-runtime、skill-engine、task-manager、workflows-directory、agent-variants、external-agent）
- **config/core.yaml** - Core 相关配置（已合并 command、cli、event-loop、hooks、orchestrator、router、session-manager、bootstrap）
- **config/services.yaml** - Services 相关配置（已合并 mcp-client、report-skill、timer-system）
- **config/*.yaml** - 系统内部配置，使用 YAML 格式，通常不需要手动修改

**配置架构：**
```
系统启动
    ↓
init_global_config()
    ↓
┌─────────────────────────────────────────┐
│  全局配置存储 (OnceLock + Arc)           │
│  - knight.json (LLM Provider)            │
│  - config/*.yaml (系统配置)               │
└─────────────────────────────────────────┘
    ↓
其他模块读取配置
    ↓
┌─────────────────────────────────────────┐
│  LLMRouter::initialize()                │
│    └── get_llm_config() → LlmConfig      │
│                                         │
│  Agent Runtime                         │
│  Logging System                         │
│  Monitor                                │
│  ...                                   │
└─────────────────────────────────────────┘
```

**配置合并说明：** 已从 26 个独立配置文件合并为 11 个配置文件，减少配置复杂度。

### CLI 命令

启动后可在 REPL 中使用以下命令：

| 命令 | 说明 |
|------|------|
| `/help`, `/h` | 显示帮助 |
| `/status` | 显示系统状态 |
| `/sessions` | 列出所有会话 |
| `/sessions new` | 创建新会话 |
| `/sessions switch` | 切换会话 |
| `/agents` | 列出所有 Agent |
| `/quit`, `/exit` | 退出 CLI |

## 许可证

MIT License

---

**设计状态**: 🚧 实现阶段进行中
**最后更新**: 2026-04-05
