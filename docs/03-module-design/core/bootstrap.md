# Bootstrap (系统启动器)

## 概述

### 职责描述

Bootstrap 负责 Knight-Agent 系统的启动、初始化和关闭,包括:

- 按依赖顺序初始化所有模块
- 配置文件加载和验证
- 模块间依赖注入和连接
- 健康检查和就绪状态管理
- 优雅关闭和资源清理
- 启动失败恢复机制
- 系统版本和状态信息

### 设计目标

1. **可靠启动**: 确保模块按正确顺序初始化
2. **故障隔离**: 单个模块启动失败不影响诊断
3. **快速启动**: 优化启动时间,支持延迟初始化
4. **优雅关闭**: 确保资源正确释放
5. **可观测**: 完整的启动日志和状态报告

### 核心需求

| 需求 | 描述 | 优先级 |
|------|------|--------|
| **模块初始化** | 按依赖顺序初始化所有模块 | P0 |
| **配置管理** | 加载和验证配置文件 | P0 |
| **健康检查** | 启动后验证系统健康状态 | P0 |
| **优雅关闭** | 处理信号并正确清理资源 | P1 |
| **启动恢复** | 失败重试和降级策略 | P1 |
| **延迟初始化** | 按需加载非核心模块 | P2 |

### 依赖模块

Bootstrap 不依赖其他业务模块,它是系统的第一个加载组件。

---

## 接口定义

### 对外接口

```yaml
# Bootstrap 接口定义
Bootstrap:
  # ========== 系统启动 ==========
  start:
    description: 启动 Knight-Agent 系统
    inputs:
      config:
        type: BootstrapConfig
        description: 启动配置
        required: false
      config_path:
        type: string
        description: 配置文件路径
        required: false
    outputs:
      system:
        type: KnightAgentSystem
        description: 系统实例

  # ========== 系统关闭 ==========
  stop:
    description: 停止系统
    inputs:
      graceful:
        type: boolean
        description: 是否优雅关闭(等待正在执行的任务完成)
        required: false
        default: true
      timeout_ms:
        type: integer
        description: 等待超时时间(毫秒)
        required: false
        default: 30000
    outputs:
      success:
        type: boolean

  # ========== 状态查询 ==========
  get_status:
    description: 获取系统状态
    outputs:
      status:
        type: SystemStatus

  is_ready:
    description: 检查系统是否就绪
    outputs:
      ready:
        type: boolean

  get_module_status:
    description: 获取所有模块状态
    outputs:
      modules:
        type: array<ModuleStatus>

  # ========== 健康检查 ==========
  health_check:
    description: 执行系统健康检查
    inputs:
      detailed:
        type: boolean
        description: 是否返回详细信息
        required: false
        default: false
    outputs:
      health:
        type: HealthCheckResult

  # ========== 重启 ==========
  restart:
    description: 重启系统
    inputs:
      graceful:
        type: boolean
        required: false
        default: true
    outputs:
      success:
        type: boolean

  # ========== 版本信息 ==========
  get_version:
    description: 获取系统版本信息
    outputs:
      version:
        type: VersionInfo
```

### 数据结构

```yaml
# 启动配置
BootstrapConfig:
  # 配置文件路径
  config_path:
    type: string
    description: 主配置文件路径
    default: "~/.knight-agent/config.yaml"

  # 工作目录
  workspace:
    type: string
    description: 默认工作目录
    default: "."

  # 日志配置
  logging:
    type: LoggingConfig
    description: 日志系统配置

  # 模块配置
  modules:
    type: ModuleConfigs
    description: 各模块配置

  # 启动选项
  startup:
    type: StartupOptions
    description: 启动选项

# 启动选项
StartupOptions:
  # 初始化超时
  init_timeout_ms:
    type: integer
    default: 60000
    description: 模块初始化超时时间

  # 并行初始化
  parallel_init:
    type: boolean
    default: false
    description: 是否并行初始化无依赖的模块

  # 失败重试
  retry_on_failure:
    type: boolean
    default: true
    description: 初始化失败是否重试

  # 最大重试次数
  max_retries:
    type: integer
    default: 3

  # 延迟初始化模块
  lazy_modules:
    type: array<string>
    description: 延迟初始化的模块列表
    default: []

# 系统状态
SystemStatus:
  phase:
    type: enum
    values: [initializing, running, stopping, stopped, error]
    description: 系统当前阶段

  uptime_seconds:
    type: integer
    description: 运行时长(秒)

  started_at:
    type: datetime
    description: 启动时间

  version:
    type: string
    description: 系统版本号

# 模块状态
ModuleStatus:
  name:
    type: string
    description: 模块名称

  status:
    type: enum
    values: [not_initialized, initializing, ready, error, disabled]
    description: 模块状态

  init_time_ms:
    type: integer
    description: 初始化耗时(毫秒)

  error:
    type: string
    description: 错误信息(如有)

  dependencies:
    type: array<string>
    description: 依赖的模块列表

# 健康检查结果
HealthCheckResult:
  healthy:
    type: boolean
    description: 系统是否健康

  checks:
    type: array<HealthCheck>
    description: 各项检查结果

# 单项健康检查
HealthCheck:
  name:
    type: string
    description: 检查项名称

  status:
    type: enum
    values: [pass, fail, warn]
    description: 检查状态

  message:
    type: string
    description: 检查结果描述

  duration_ms:
    type: integer
    description: 检查耗时

# 版本信息
VersionInfo:
  version:
    type: string
    description: 版本号

  git_commit:
    type: string
    description: Git 提交哈希

  build_date:
    type: string
    description: 构建日期

  rust_version:
    type: string
    description: Rust 版本

  module_versions:
    type: map<string, string>
    description: 各模块版本
```

### 配置选项

```yaml
# config/bootstrap.yaml
bootstrap:
  # 启动选项
  startup:
    init_timeout_ms: 60000
    parallel_init: false
    retry_on_failure: true
    max_retries: 3
    lazy_modules:
      - mcp_client
      - context_compressor

  # 关闭选项
  shutdown:
    timeout_ms: 30000
    wait_for_tasks: true

  # 健康检查
  health_check:
    enabled: true
    interval_ms: 30000
    timeout_ms: 5000

  # 模块依赖 (共 25 个模块)
  module_order:
    # === 阶段 1: 基础设施 (无依赖) ===
    - logging_system          # 最先初始化,记录所有日志
    - storage_service         # 数据持久化基础

    # === 阶段 2: 基础服务 (依赖基础设施) ===
    - llm_provider            # 依赖 storage (缓存)
    - tool_system             # 无其他依赖
    - security_manager        # 权限检查能力需尽早可用

    # === 阶段 3: 事件系统 (依赖核心服务) ===
    - event_loop              # 先初始化，建立事件接收机制
    - timer_system            # 后初始化，向 event_loop 注册并开始发送事件

    # === 阶段 4: 核心引擎层 ===
    - hook_engine             # 依赖 event_loop
    - session_manager         # 依赖 storage, logging
    - router                  # CLI 路由,依赖 session_manager (Task Manager 运行时依赖)
    - monitor                 # 监控,依赖所有模块 (Agent Runtime 运行时依赖)

    # === 阶段 5: Agent 层 ===
    - agent_variants          # Agent 变体系统
    - agent_runtime           # 依赖 llm_provider, tool_system, hook_engine, agent_variants
    - external_agent          # 外部 Agent 集成,依赖 agent_runtime
    - skill_engine            # 依赖 agent_runtime (循环依赖，运行时解决)
    - orchestrator            # 依赖 agent_runtime, external_agent
    - task_manager            # 依赖 skill_engine, orchestrator
    - command                 # 命令执行,依赖 router 和 task_manager
    - workflows_directory     # 工作流目录,依赖 task_manager

    # === 阶段 6: 报告和监控 ===
    - report_skill            # 报告生成,依赖 timer_system

    # === 阶段 7: 上下文优化 (可选) ===
    - context_compressor      # 上下文压缩,依赖 llm_provider

    # === 阶段 8: 安全层 (最后初始化) ===
    - sandbox                 # 依赖 session_manager, tool_system
    - ipc_contract            # 进程间通信契约

  # 故障恢复
  recovery:
    enabled: true
    backup_config: "./config/backup.yaml"
    min_modules_required: 4

  # 依赖说明
  dependency_notes:
    router_task_manager:
      description: "Router 依赖 Task Manager，但 Router 在 Stage 4 初始化而 Task Manager 在 Stage 5"
      resolution: "Router 使用懒加载获取 Task Manager 引用，仅在处理 /workflow 命令时才调用"

    monitor_agent_runtime:
      description: "Monitor 依赖 Agent Runtime，但 Monitor 在 Stage 4 初始化而 Agent Runtime 在 Stage 5"
      resolution: "Monitor 使用懒加载获取 Agent Runtime 引用，仅在监控 Agent 状态时才调用"

    command_task_manager:
      description: "Command 依赖 Task Manager，Command 在 Stage 5 初始化但 Task Manager 也在 Stage 5"
      resolution: "在同一阶段内按列表顺序初始化，Task Manager 先于 Command 初始化"

    skill_engine_agent_runtime:
      description: "Skill Engine 和 Agent Runtime 存在循环依赖"
      detail: |
        - Skill Engine 的 'agent' 类型步骤需要调用 Agent Runtime
        - Agent Runtime 的技能触发功能需要调用 Skill Engine
      resolution: "两个模块使用懒加载互相引用，在首次使用时才建立完整连接"
```

---

## 核心流程

### 系统启动流程

```
Bootstrap::start()
    │
    ▼
┌──────────────────────────────┐
│ 1. 加载配置                  │
│    - 读取配置文件            │
│    - 验证配置有效性          │
│    - 合并环境变量            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 2. 初始化日志系统            │
│    - 最早初始化              │
│    - 记录启动日志            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 按依赖顺序初始化模块      │
│    for module in module_order:│
│      - 检查依赖是否满足      │
│      - 初始化模块            │
│      - 记录模块状态          │
│      - 失败时重试/跳过       │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. 注册模块间连接            │
│    - Event Loop 事件源注册  │
│    - Hook 系统注册           │
│    - 服务注入                │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 5. 启动核心系统              │
│    - Event Loop::start()     │
│    - Timer System::start()   │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 6. 健康检查                  │
│    - 验证核心模块就绪        │
│    - 检查依赖连接            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 7. 工作流恢复（可选）        │
│    - 恢复未完成的后台工作流  │
│    - if config.task.background. │
│         auto_resume: true      │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 8. 系统就绪                  │
│    - 等待用户输入/请求       │
└──────────────────────────────┘
```

### 模块初始化顺序

```
阶段 1: 基础设施 (无依赖)
├── logging_system      # 最先初始化,记录所有日志
└── storage_service      # 数据持久化基础

阶段 2: 基础服务 (依赖基础设施)
├── llm_provider         # 依赖 storage (缓存)
├── tool_system          # 无其他依赖
└── security_manager     # 权限检查能力需尽早可用

阶段 3: 事件系统 (依赖核心服务)
├── event_loop           # 依赖 logging, tool_system
└── timer_system         # 依赖 event_loop

阶段 4: 核心引擎层 (依赖事件系统)
├── hook_engine          # 依赖 event_loop
├── session_manager      # 依赖 storage, logging
├── router               # CLI 路由,依赖 session_manager (Task Manager 运行时依赖)
└── monitor              # 监控,依赖所有模块 (Agent Runtime 运行时依赖)

阶段 5: Agent 层 (依赖引擎层)
├── agent_variants       # Agent 变体系统
├── agent_runtime        # 依赖 llm_provider, tool_system, hook_engine, agent_variants
├── external_agent       # 外部 Agent 集成,依赖 agent_runtime
├── skill_engine         # 依赖 agent_runtime (循环依赖，运行时解决)
├── orchestrator         # 依赖 agent_runtime, external_agent
├── task_manager         # 依赖 skill_engine, orchestrator
├── command              # 命令执行,依赖 router 和 task_manager
└── workflows_directory  # 工作流目录,依赖 task_manager

阶段 6: 报告和监控
└── report_skill          # 报告生成,依赖 timer_system

阶段 7: 上下文优化 (可选)
└── context_compressor    # 上下文压缩,依赖 llm_provider

阶段 8: 安全层 (最后初始化)
├── sandbox              # 依赖 session_manager, tool_system
└── ipc_contract         # 进程间通信契约
```

**模块统计**: 25 个模块 (8 核心 + 6 Agent + 7 服务 + 1 工具 + 1 基础设施 + 2 安全)

### 优雅关闭流程

```
收到关闭信号 (SIGTERM/SIGINT)
    │
    ▼
┌──────────────────────────────┐
│ 1. 设置停止标志              │
│    system.status = stopping  │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 2. 停止接受新任务            │
│    - Event Loop 停止接受事件 │
│    - Session Manager 拒绝新建│
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 等待进行中任务完成        │
│    - 检查活跃任务数          │
│    - 超时后强制中断          │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. 按逆序关闭模块            │
│    for module in reverse_order:│
│      - graceful_shutdown()   │
│      - 释放资源              │
│      - 保存状态              │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 5. 清理系统资源              │
│    - 关闭文件句柄            │
│    - 释放内存                │
│    - 断开连接                │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 6. 退出                      │
│    system.status = stopped   │
└──────────────────────────────┘
```

### 故障恢复流程

```
模块初始化失败
    │
    ▼
┌──────────────────────────────┐
│ 1. 记录错误日志              │
│    - 失败模块名称            │
│    - 错误详情                │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 是否启用重试?                │
└──────────────────────────────┘
    │
    ├─ 是 ──────────────────┐
    │                       │
    │                       ▼
    │           ┌──────────────────────────┐
    │           │ 重试初始化              │
    │           │ - 最多 max_retries 次   │
    │           │ - 每次增加延迟          │
    │           └──────────────────────────┘
    │                       │
    │                       ▼
    │           ┌──────────────────────────┐
    │           │ 重试成功?               │
    │           └──────────────────────────┘
    │                       │
    │           ├─ 是 ─────────→ 继续启动
    │           │
    │           └─ 否 ─────────→ 检查降级策略
    │
    └─ 否 ──────────────────┐
                            │
                            ▼
                ┌──────────────────────────┐
                │ 模块是必须的吗?          │
                └──────────────────────────┘
                            │
                            ├─ 是 ─────→ 启动失败,退出
                            │
                            └─ 否 ─────→ 标记为 disabled,继续

所有核心模块启动成功
    │
    ▼
系统就绪 (可能有部分模块 disabled)
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Bootstrap                    │
│  ┌───────────────────────────────────┐│
│  │  Module Registry                 ││
│  │  - 管理所有模块实例              ││
│  │  - 处理模块依赖关系              ││
│  │  - 控制初始化顺序                ││
│  └───────────────────────────────────┘│
│  ┌───────────────────────────────────┐│
│  │  Config Manager                  ││
│  │  - 加载配置文件                  ││
│  │  - 验证配置有效性                ││
│  │  - 环境变量合并                  ││
│  └───────────────────────────────────┘│
│  ┌───────────────────────────────────┐│
│  │  Lifecycle Manager               ││
│  │  - 管理启动/关闭流程             ││
│  │  - 处理模块生命周期事件         ││
│  │  - 协调优雅关闭                 ││
│  └───────────────────────────────────┘│
└─────────────────────────────────────────┘
           │ 按顺序初始化
           ▼
┌─────────────────────────────────────────┐
│         各功能模块                       │
│  logging → storage → event_loop → ...  │
└─────────────────────────────────────────┘
```

### 消息流

```
CLI/Web 启动请求
    │
    ▼
┌─────────────────────────────────────────┐
│            Bootstrap                    │
│  1. 加载配置                            │
│  2. 初始化模块                          │
│  3. 启动系统                            │
└─────────────────────────────────────────┘
           │
           ▼ 返回 System 实例
┌─────────────────────────────────────────┐
│         KnightAgentSystem               │
│  - 持有所有模块引用                     │
│  - 提供统一访问接口                     │
│  - 管理系统状态                         │
└─────────────────────────────────────────┘
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/bootstrap.yaml
bootstrap:
  # 启动选项
  startup:
    # 配置文件
    config_path: "~/.knight-agent/config.yaml"

    # 工作目录
    workspace: "."

    # 初始化选项
    init_timeout_ms: 60000
    parallel_init: false
    retry_on_failure: true
    max_retries: 3

    # 延迟初始化模块
    lazy_modules:
      - mcp_client
      - context_compressor

  # 关闭选项
  shutdown:
    timeout_ms: 30000
    wait_for_tasks: true
    force_kill_after_ms: 60000

  # 健康检查
  health_check:
    enabled: true
    interval_ms: 30000
    timeout_ms: 5000
    startup_timeout_ms: 120000

  # 模块配置
  modules:
    logging_system:
      enabled: true
      config: "./config/logging.yaml"

    storage_service:
      enabled: true
      config: "./config/storage.yaml"

    llm_provider:
      enabled: true
      config: "./config/llm.yaml"

    tool_system:
      enabled: true
      config: "./config/tools.yaml"

    event_loop:
      enabled: true
      config: "./config/event-loop.yaml"

    timer_system:
      enabled: true
      config: "./config/timer-system.yaml"

    hook_engine:
      enabled: true
      config: "./config/hooks.yaml"

    session_manager:
      enabled: true
      config: "./config/session-manager.yaml"

    # === 核心引擎层 (新增) ===
    router:
      enabled: true
      config: "./config/router.yaml"

    command:
      enabled: true
      config: "./config/command.yaml"

    monitor:
      enabled: true
      config: "./config/monitor.yaml"

    # === Agent 层 ===
    agent_variants:
      enabled: true
      config: "./config/agent-variants.yaml"

    agent_runtime:
      enabled: true
      config: "./config/agent-runtime.yaml"

    external_agent:
      enabled: true
      config: "./config/external-agent.yaml"

    skill_engine:
      enabled: true
      config: "./config/skill-engine.yaml"

    orchestrator:
      enabled: true
      config: "./config/orchestrator.yaml"

    task_manager:
      enabled: true
      config: "./config/task-manager.yaml"
      # 工作流恢复配置
      background:
        auto_resume: true
        checkpoint_interval: 60

    workflows_directory:
      enabled: true
      config: "./config/workflows.yaml"
      # 工作流目录路径
      directories:
        - "./workflows"
        - "~/.knight-agent/workflows"

    # === 服务层 (新增) ===
    report_skill:
      enabled: true
      config: "./config/report-skill.yaml"

    # === 可选模块 ===
    mcp_client:
      enabled: false
      config: "./config/mcp-client.yaml"

    context_compressor:
      enabled: true
      config: "./config/compressor.yaml"

    # === 安全层 ===
    security_manager:
      enabled: true
      config: "./config/security.yaml"

    sandbox:
      enabled: true
      config: "./config/sandbox.yaml"

    # === 基础设施层 ===
    ipc_contract:
      enabled: true
      config: "./config/ipc-contract.yaml"

  # 故障恢复
  recovery:
    enabled: true
    backup_config: "./config/backup.yaml"
    min_core_modules: 4
    degraded_mode_enabled: false
```

### 环境变量

```bash
# 配置文件路径
export KNIGHT_CONFIG_PATH="~/.knight-agent/config.yaml"

# 工作目录
export KNIGHT_WORKSPACE="."

# 日志级别
export KNIGHT_LOG_LEVEL="info"

# 初始化超时
export KNIGHT_INIT_TIMEOUT_MS="60000"

# 并行初始化
export KNIGHT_PARALLEL_INIT="false"

# 启用/禁用模块
export KNIGHT_MODULE_MCP_CLIENT="false"
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 冷启动时间 | < 5s | 从启动到系统就绪 |
| 热启动时间 | < 2s | 有缓存的情况下 |
| 内存占用(启动后) | < 200MB | 基础内存占用 |
| 关闭时间 | < 10s | 优雅关闭耗时 |

### 错误处理

```yaml
error_codes:
  CONFIG_NOT_FOUND:
    code: 404
    message: "配置文件不存在"
    action: "检查配置文件路径或使用默认配置"

  CONFIG_INVALID:
    code: 400
    message: "配置文件格式错误"
    action: "检查配置文件语法"

  MODULE_INIT_FAILED:
    code: 500
    message: "模块初始化失败"
    action: "查看错误日志,检查模块配置"

  CIRCULAR_DEPENDENCY:
    code: 500
    message: "检测到循环依赖"
    action: "检查模块依赖配置"

  STARTUP_TIMEOUT:
    code: 504
    message: "系统启动超时"
    action: "增加超时时间或检查模块性能"

  MIN_MODULES_NOT_MET:
    code: 503
    message: "核心模块启动失败"
    action: "检查核心模块配置"
```

### 使用示例

```yaml
# 基础启动
startup:
  description: "使用默认配置启动系统"
  code: |
    let system = Bootstrap::start(Default::default()).await?;

    // 系统运行中...

    system.stop().await?;

# 使用自定义配置
custom_config:
  description: "指定配置文件路径"
  code: |
    let config = BootstrapConfig {
        config_path: "./custom/config.yaml".into(),
        ..Default::default()
    };
    let system = Bootstrap::start(config).await?;

# 延迟初始化
lazy_init:
  description: "只初始化核心模块"
  code: |
    let config = BootstrapConfig {
        startup: StartupOptions {
            lazy_modules: vec![
                "mcp_client".into(),
                "context_compressor".into(),
                "orchestrator".into(),
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let system = Bootstrap::start(config).await?;

# 健康检查
health_check:
  description: "检查系统健康状态"
  code: |
    let system = get_system();
    let health = system.health_check(true)?;

    if !health.healthy {
        eprintln!("系统不健康:");
        for check in health.checks {
            if check.status == HealthStatus::Fail {
                eprintln!("  - {}: {}", check.name, check.message);
            }
        }
    }

# 优雅关闭
graceful_shutdown:
  description: "等待任务完成后关闭"
  code: |
    let system = get_system();

    // 设置信号处理
    ctrl_c.set_handler(move || {
        tokio::spawn(async move {
            system.stop(true).await.unwrap();
        });
    });
```

### 测试策略

```yaml
testing:
  unit_tests:
    - 配置加载和验证
    - 模块依赖解析
    - 启动顺序计算
    - 状态管理

  integration_tests:
    - 完整启动流程
    - 模块初始化顺序
    - 故障恢复机制
    - 优雅关闭流程

  edge_cases:
    - 配置文件缺失
    - 模块初始化失败
    - 循环依赖检测
    - 启动超时处理
    - 重复启动/关闭

  performance_tests:
    - 冷启动时间
    - 内存占用测试
    - 并发启动测试
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-04-01 | 初始版本 |
