# Knight-Agent 模块设计文档

本目录包含 Knight-Agent 各核心模块的详细设计文档。每个模块文档包含接口定义、数据结构、算法流程、配置选项等实现细节。

## 文档结构

```
03-module-design/
├── README.md                    # 本文件 - 模块设计索引
├── core/                        # 核心引擎模块 (8个)
│   ├── session-manager.md       # 会话管理器
│   ├── orchestrator.md          # 编排器
│   ├── event-loop.md            # 事件循环
│   ├── hook-engine.md           # Hook 引擎
│   ├── bootstrap.md             # 系统启动器
│   ├── router.md                # 路由器
│   ├── command.md               # 命令系统
│   └── monitor.md               # 监控模块
├── agent/                       # Agent 运行模块 (4个)
│   ├── agent-runtime.md         # Agent 运行时
│   ├── agent-variants.md        # Agent 变体系统
│   ├── skill-engine.md          # 技能引擎
│   └── task-manager.md          # 任务管理器
├── services/                    # 基础服务模块 (6个)
│   ├── llm-provider.md          # LLM 提供者抽象
│   ├── mcp-client.md            # MCP 客户端
│   ├── storage-service.md       # 存储服务
│   ├── context-compressor.md    # 上下文压缩
│   ├── timer-system.md          # 定时器系统
│   └── logging-system.md        # 日志系统
├── tools/                       # 工具系统 (1个)
│   └── tool-system.md           # 工具框架
└── security/                    # 安全模块 (2个)
    ├── security-manager.md      # 安全管理器
    └── sandbox.md               # 沙箱机制
```

## 模块设计规范

每个模块设计文档遵循以下统一结构：

```markdown
# [模块名称]

## 概述
- 职责描述
- 设计目标
- 依赖模块

## 接口定义
- 对外接口 (YAML/伪代码)
- 数据结构
- 配置选项

## 核心流程
- 主要算法流程图
- 状态机设计
- 关键决策点

## 模块交互
- 与其他模块的交互方式
- 依赖关系图
- 消息流

## 配置与部署
- 配置文件格式
- 环境变量
- 部署考虑

## 示例
- 使用场景
- 配置示例

## 附录
- 性能指标
- 错误处理
- 测试策略
```

## 实现优先级

### P0 - 核心模块 (已完成)
- [x] [session-manager](core/session-manager.md) - 会话管理器
- [x] [agent-runtime](agent/agent-runtime.md) - Agent 运行时
- [x] [llm-provider](services/llm-provider.md) - LLM 提供者抽象
- [x] [tool-system](tools/tool-system.md) - 工具框架

### P1 - 扩展模块 (已完成)
- [x] [orchestrator](core/orchestrator.md) - 编排器
- [x] [skill-engine](agent/skill-engine.md) - 技能引擎
- [x] [event-loop](core/event-loop.md) - 事件循环
- [x] [hook-engine](core/hook-engine.md) - Hook 引擎
- [x] [task-manager](agent/task-manager.md) - 任务管理器
- [x] [mcp-client](services/mcp-client.md) - MCP 客户端
- [x] [context-compressor](services/context-compressor.md) - 上下文压缩
- [x] [storage-service](services/storage-service.md) - 存储服务
- [x] [timer-system](services/timer-system.md) - 定时器系统
- [x] [logging-system](services/logging-system.md) - 日志系统
- [x] [agent-variants](agent/agent-variants.md) - Agent 变体系统
- [x] [router](core/router.md) - 路由器
- [x] [command](core/command.md) - 命令系统
- [x] [monitor](core/monitor.md) - 监控模块
- [x] [bootstrap](core/bootstrap.md) - 系统启动器

### P2 - 安全和运维 (已完成)
- [x] [security-manager](security/security-manager.md) - 安全管理器
- [x] [sandbox](security/sandbox.md) - 沙箱机制

## 相关文档

| 文档 | 内容 |
|------|------|
| [02-system-design.md](../02-system-design.md) | 系统架构高层次设计 |
| [01-requirements-analysis.md](../01-requirements-analysis.md) | 需求分析 |
| [00-priority-overview.md](../00-priority-overview.md) | 功能优先级总览 |
| [04-testing-design.md](../04-testing-design.md) | L0/L1 测试设计文档 |
| [SRS.md](../SRS.md) | 软件需求规格说明书 |

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本，创建目录结构 |
