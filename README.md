# Knight-Agent

> 可扩展的 Agentic 工具开发框架

## 概述

Knight-Agent 是一个受 Claude Code 和 OpenClaw 启发的 Agentic 框架，支持：

- **自定义 Agent** - 通过 Markdown 定义 Agent 角色和能力
- **Agent 变体** - 同一 Agent 的不同配置变体（如 agent:reviewer-strict）
- **自定义 Skill** - 通过 Markdown 定义可复用行为模式
- **Skill Pipeline** - 技能管道组合，支持依赖和条件执行
- **多会话并行** - 同时在多个项目中工作，完全隔离
- **多 Agent 协作** - 支持主从、流水线、投票等协作模式
- **MCP 工具集成** - 通过 MCP 协议扩展工具能力
- **Hook 系统** - 事件钩子支持插件扩展（before/after/replace）
- **事件驱动** - 文件变更、Git 事件、定时任务自动触发
- **定时器系统** - 自然语言创建定时任务，支持一次性/周期性任务
- **7×24 运行** - 事件驱动的长期运行能力

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│  用户接口层 (CLI / Web UI)                                   │
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
│  基础服务层                                                  │
│  ┌──────────────┬──────────────┬──────────────┬───────────┐ │
│  │LLM Provider  │MCP Client    │Storage       │Context    │ │
│  │              │              │Service      │Compressor │ │
│  └──────────────┴──────────────┴──────────────┴───────────┘ │
├─────────────────────────────────────────────────────────────┤
│  工具层                                                      │
│  Read │ Write │ Edit │ Glob │ Grep │ Bash │ Git │ MCP    │
├─────────────────────────────────────────────────────────────┤
│  安全层                                                      │
│  ┌──────────────┬──────────────┐                              │
│  │Security      │Sandbox       │                              │
│  │Manager      │              │                              │
│  └──────────────┴──────────────┘                              │
└─────────────────────────────────────────────────────────────┘
```

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
| [03-module-design/README](./docs/03-module-design/README.md) | 模块设计索引（21个模块） |
| [04-testing-design](./docs/04-testing-design.md) | L0/L1 测试设计 |
| [05-technical-baseline-tests](./docs/05-technical-baseline-tests.md) | 技术基线测试 |
| [SRS](../SRS.md) | 软件需求规格说明书 |

### 模块设计文档

| 模块 | 文档 | 状态 |
|------|------|------|
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
| Skill 引擎 | [skill-engine](./docs/03-module-design/agent/skill-engine.md) | ✅ |
| 任务管理器 | [task-manager](./docs/03-module-design/agent/task-manager.md) | ✅ |
| **基础服务** | | |
| LLM 提供者 | [llm-provider](./docs/03-module-design/services/llm-provider.md) | ✅ |
| MCP 客户端 | [mcp-client](./docs/03-module-design/services/mcp-client.md) | ✅ |
| 存储服务 | [storage-service](./docs/03-module-design/services/storage-service.md) | ✅ |
| 上下文压缩 | [context-compressor](./docs/03-module-design/services/context-compressor.md) | ✅ |
| 定时器系统 | [timer-system](./docs/03-module-design/services/timer-system.md) | ✅ |
| 日志系统 | [logging-system](./docs/03-module-design/services/logging-system.md) | ✅ |
| **工具系统** | | |
| 工具框架 | [tool-system](./docs/03-module-design/tools/tool-system.md) | ✅ |
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

- ✅ 编排器 - 多 Agent 协作、任务调度
- ✅ 路由器 - CLI 命令处理、请求分发
- ✅ 命令系统 - 用户可定义命令（Markdown）
- ✅ Skill 引擎 - 技能定义、触发、执行、Pipeline 组合
- ✅ 事件循环 - 文件变更、Git 事件、定时任务
- ✅ Hook 引擎 - before/after/replace 事件钩子
- ✅ 任务管理器 - DAG 依赖、并行执行
- ✅ MCP 客户端 - MCP 协议集成、工具发现
- ✅ 上下文压缩 - 智能摘要、语义压缩
- ✅ 存储服务 - SQLite 持久化、备份恢复
- ✅ 定时器系统 - 一次性/周期性/Cron 定时器
- ✅ 日志系统 - 结构化日志、异步写入、日志轮转
- ✅ 监控模块 - Token 统计、状态监控、资源监控
- ✅ Agent 变体 - 多配置支持

### P2 - 安全和运维 (设计完成)

- ✅ 安全管理器 - 权限控制、审计日志
- ✅ 沙箱机制 - 资源隔离、访问控制

## 核心概念

### Agent 变体

同一 Agent 可以有多个配置变体，通过 `agent:variant` 语法指定：

```
agents/
├── CODE-REVIEWER.md           # 基础 Agent 定义
├── CODE-REVIEWER.strict.md     # strict 变体
└── CODE-REVIEWER.lenient.md    # lenient 变体

# 使用
/agent:code-reviewer        # 默认配置
/agent:code-reviewer:strict # strict 变体
```

### Skill Pipeline

技能管道支持技能组合和依赖管理：

```yaml
pipeline:
  skills:
    - skill_id: test-unit
      depends_on: []

    - skill_id: test-integration
      depends_on: ["test-unit"]

    - skill_id: deploy
      depends_on: ["test-integration"]
      condition: "{{ all_passed }}"
```

### Hook 系统

事件钩子支持在关键点插入自定义逻辑：

```yaml
hooks:
  before_command:
    - hook: validate-input
  after_command:
    - hook: log-result
  replace_command:
    - hook: custom-implementation
```

### 定时器系统

使用自然语言创建定时任务，支持一次性、周期性和递归任务：

```bash
# 自然语言创建
> 每天早上8点给我发送AI新闻简报
✅ 定时任务已创建: task-001

# CLI 管理
knight schedule list
knight schedule pause <task-id>
knight schedule cancel <task-id>
```

## 路线图

### 设计阶段 ✅

- [x] 需求分析
- [x] 系统架构设计
- [x] 模块详细设计 (21 个模块)

### 实现阶段 (规划中)

详细周度规划见 [需求分析文档](./docs/01-requirements-analysis.md#里程碑规划)

| 阶段 | 目标 | 周期 |
|------|------|------|
| Phase 1 | 核心 Agent 框架 | Week 1-4 |
| Phase 2 | Skill 系统 + 会话增强 | Week 5-7 |
| Phase 3 | 协作能力 | Week 8-10 |
| Phase 4 | 自动化 | Week 11-12 |
| Phase 5 | 生态 | Week 13-14 |

## 项目结构

```
knight-agent/
├── docs/
│   ├── 00-priority-overview.md    # 优先级总览
│   ├── 01-requirements-analysis.md # 需求分析
│   ├── 02-system-design.md         # 系统架构设计
│   ├── 03-module-design/          # 模块详细设计 (21个模块)
│   │   ├── README.md              # 模块设计索引
│   │   ├── core/                  # 核心引擎模块 (8个)
│   │   │   ├── bootstrap.md
│   │   │   ├── session-manager.md
│   │   │   ├── orchestrator.md
│   │   │   ├── router.md
│   │   │   ├── command.md
│   │   │   ├── event-loop.md
│   │   │   ├── hook-engine.md
│   │   │   └── monitor.md
│   │   ├── agent/                 # Agent 运行模块 (4个)
│   │   │   ├── agent-runtime.md
│   │   │   ├── agent-variants.md
│   │   │   ├── skill-engine.md
│   │   │   └── task-manager.md
│   │   ├── services/              # 基础服务模块 (6个)
│   │   │   ├── llm-provider.md
│   │   │   ├── mcp-client.md
│   │   │   ├── storage-service.md
│   │   │   ├── context-compressor.md
│   │   │   ├── timer-system.md
│   │   │   └── logging-system.md
│   │   ├── tools/                 # 工具系统 (1个)
│   │   │   └── tool-system.md
│   │   └── security/              # 安全模块 (2个)
│   │       ├── security-manager.md
│   │       └── sandbox.md
│   ├── 04-testing-design.md       # L0/L1 测试设计
│   └── 05-technical-baseline-tests.md # 技术基线测试
├── SRS.md                          # 软件需求规格说明书
├── CLAUDE.md                       # Claude Code 配置
└── README.md                       # 项目说明
```

## 设计亮点

1. **模块化设计** - 21 个独立模块，职责清晰
2. **灵活扩展** - Agent 变体、Skill Pipeline、Hook 系统支持扩展
3. **事件驱动** - Event Loop + Hook Engine 实现自动化
4. **安全优先** - Security Manager + Sandbox 双重保护
5. **可观测** - 完整的审计日志和状态追踪

## 许可证

MIT License

---

**设计状态**: 📐 设计阶段已完成，等待实现阶段
**最后更新**: 2026-04-02
