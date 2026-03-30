# Knight-Agent

> 可扩展的 Agentic 工具开发框架

## 概述

Knight-Agent 是一个受 Claude Code 和 OpenClaw 启发的 Agentic 框架，支持：

- **自定义 Agent** - 通过 Markdown 定义 Agent 角色和能力
- **自定义 Skill** - 通过 Markdown 定义可复用行为模式
- **多会话并行** - 同时在多个项目中工作，完全隔离
- **多 Agent 协作** - 支持主从、流水线、议题等协作模式
- **MCP 工具集成** - 通过 MCP 协议扩展工具能力
- **Hook 系统** - 事件钩子支持插件扩展
- **7×24 运行** - 事件驱动的长期运行能力

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│  用户接口层 (CLI / Web UI)                                   │
├─────────────────────────────────────────────────────────────┤
│  核心引擎层 (Session Manager / Orchestrator / Event Loop)   │
├─────────────────────────────────────────────────────────────┤
│  Agent 运行层 (Agent / Skill / Tool)                        │
├─────────────────────────────────────────────────────────────┤
│  基础服务层 (LLM / MCP / Storage)                            │
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

| 文档 | 描述 |
|------|------|
| [00-priority-overview](./docs/00-priority-overview.md) | 优先级总览 |
| [01-requirements-analysis](./docs/01-requirements-analysis.md) | 需求分析 |
| [02-system-design](./docs/02-system-design.md) | 系统设计 |
| [04-agent-variants](./docs/04-agent-variants.md) | Agent 变体设计 |
| [05-session-system](./docs/05-session-system.md) | 会话系统设计 |

## 优先级

**P0 - MVP (核心功能)**
- Agent 定义系统
- Skill 定义系统
- 基础工具集 (Read/Write/Edit/Grep/Bash)
- LLM 抽象层
- 多会话并行
- Workspace 隔离
- CLI 交互界面

**P1 - V1.0 (重要功能)**
- Agent 变体支持
- 多 Agent 协作
- 事件监听系统
- 任务管理 (DAG)
- MCP 集成
- 权限控制
- 会话持久化
- 上下文压缩
- 历史搜索

**P2 - V1.x (增强功能)**
- Hook 系统（事件钩子）
- 插件系统
- 7×24 守护进程
- 配置热更新
- Skill 调试
- 成本监控

## 路线图

- [ ] Phase 1: 核心 Agent 框架
- [ ] Phase 2: Skill 系统
- [ ] Phase 3: 协作能力
- [ ] Phase 4: 事件驱动自动化
- [ ] Phase 5: 生态集成 (MCP)
- [ ] Phase 6: Hook/插件系统

## 许可证

MIT License
