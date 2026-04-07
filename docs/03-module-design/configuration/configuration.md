# Configuration - 配置管理模块

## 概述

### 职责描述

Configuration 是 Knight Agent 的集中配置管理模块，负责：
- 用户配置（LLM 提供者）以 JSON 格式存储在 `knight.json`
- 系统配置以 YAML 格式存储在 `config/*.yaml`
- 配置热更新：文件变更自动检测并通知订阅者
- 环境变量替换：支持 `${VAR}` 语法

### 设计目标

1. **用户友好**：用户只需配置 `knight.json`（LLM 提供者），其他系统配置自动生成默认值
2. **热更新**：配置变更无需重启即可生效
3. **类型安全**：Rust 结构体 + serde 验证
4. **单点管理**：LLM 配置是单一数据源，所有模块共享

### 依赖模块

- `notify` - 文件系统监控
- `tokio` - 异步文件操作
- `parking_lot` - 高效锁
- `serde` - 序列化/反序列化

### 配置目录结构

```
~/.knight-agent/
├── knight.json              # 主配置（仅 LLM，用户常用）
└── config/                  # 系统配置（YAML 格式）
    ├── agent.yaml           # Agent 模块（6 个模块合并）
    ├── core.yaml            # Core 模块（8 个模块合并）
    ├── services.yaml        # Services（3 个服务合并）
    ├── tools.yaml           # 工具系统
    ├── infrastructure.yaml   # 基础设施（IPC）
    ├── storage.yaml         # 存储配置
    ├── security.yaml        # 安全配置
    ├── logging.yaml         # 日志配置
    ├── monitoring.yaml      # 监控配置
    └── compressor.yaml      # 上下文压缩配置
```

---

## 接口定义

### ConfigLoader

主配置加载器，支持热更新。

```rust
pub struct ConfigLoader {
    config_dir: PathBuf,
    main_config: Arc<RwLock<KnightConfig>>,
    system_configs: Arc<RwLock<HashMap<String, SystemConfig>>>,
    change_tx: broadcast::Sender<ConfigChangeEvent>,
    _watcher: RecommendedWatcher,
}

impl ConfigLoader {
    /// 创建配置加载器（异步）
    pub async fn new(config_dir: PathBuf) -> ConfigResult<Self>;

    /// 获取主配置
    pub fn get_main_config(&self) -> KnightConfig;

    /// 获取 LLM 配置
    pub fn get_llm_config(&self) -> Option<LlmConfig>;

    /// 获取系统配置
    pub fn get_system_config(&self, name: &str) -> Option<SystemConfig>;

    /// 获取 Agent 配置
    pub fn get_agent_config(&self) -> AgentConfig;

    /// 获取 Core 配置
    pub fn get_core_config(&self) -> CoreConfig;

    /// 获取 Services 配置
    pub fn get_services_config(&self) -> ServicesConfig;

    /// 获取 Tools 配置
    pub fn get_tools_config(&self) -> ToolsConfig;

    /// 获取 Infrastructure 配置
    pub fn get_infrastructure_config(&self) -> InfrastructureConfig;

    /// 获取 Logging 配置
    pub fn get_logging_config(&self) -> LoggingConfig;

    /// 获取 Monitoring 配置
    pub fn get_monitoring_config(&self) -> MonitoringConfig;

    /// 获取 Storage 配置
    pub fn get_storage_config(&self) -> StorageConfig;

    /// 获取 Security 配置
    pub fn get_security_config(&self) -> SecurityConfig;

    /// 获取 Compressor 配置
    pub fn get_compressor_config(&self) -> CompressorConfig;

    /// 订阅配置变更
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigChangeEvent>;

    /// 手动重载主配置
    pub async fn reload_main_config(&self) -> ConfigResult<()>;

    /// 获取配置目录路径
    pub fn config_dir(&self) -> &Path;
}
```

### 配置变更事件

```rust
#[derive(Debug, Clone)]
pub enum ConfigChangeEvent {
    /// 主配置（knight.json）变更
    MainConfigChanged(KnightConfig),
    /// 系统配置（config/*.yaml）变更
    SystemConfigChanged { name: String, config: SystemConfig },
}
```

### 系统配置枚举

```rust
#[derive(Debug, Clone)]
pub enum SystemConfig {
    Agent(AgentConfig),
    Core(CoreConfig),
    Services(ServicesConfig),
    Tools(ToolsConfig),
    Infrastructure(InfrastructureConfig),
    Logging(LoggingConfig),
    Monitoring(MonitoringConfig),
    Compressor(CompressorConfig),
    Storage(StorageConfig),
    Security(SecurityConfig),
}
```

---

## 核心数据结构

### KnightConfig (knight.json)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnightConfig {
    /// LLM 提供者配置（用户主要配置项）
    pub llm: Option<LlmConfig>,
}
```

### LlmConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    /// 默认提供者名称
    pub default_provider: Option<String>,
    /// 提供者映射
    pub providers: HashMap<String, LlmProviderConfig>,
}
```

### AgentConfig (config/agent.yaml)

已合并 6 个 Agent 相关模块：
- agent-runtime
- skill-engine
- task-manager
- workflows-directory
- agent-variants
- external-agent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// 默认 Agent 变体
    pub default_variant: Option<String>,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务超时（秒）
    pub task_timeout_secs: u64,
    /// Agent Runtime 配置
    #[serde(default)]
    pub runtime: AgentRuntimeConfig,
    /// Skill Engine 配置
    #[serde(default)]
    pub skill: SkillEngineConfig,
    /// Task Manager 配置
    #[serde(default)]
    pub task: TaskManagerConfig,
    /// Workflow 配置
    #[serde(default)]
    pub workflow: WorkflowConfig,
}
```

### CoreConfig (config/core.yaml)

已合并 8 个 Core 相关模块：
- command
- cli
- event-loop
- hooks
- orchestrator
- router
- session-manager
- bootstrap

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreConfig {
    #[serde(default)]
    pub command: CommandConfig,
    #[serde(default)]
    pub cli: CliConfig,
    #[serde(default)]
    pub event_loop: EventLoopConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub router: RouterConfig,
    #[serde(default)]
    pub session: SessionConfig,
    #[serde(default)]
    pub bootstrap: BootstrapConfig,
}
```

### ServicesConfig (config/services.yaml)

已合并 3 个服务：
- mcp-client
- report-skill
- timer-system

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServicesConfig {
    #[serde(default)]
    pub mcp: McpConfig,
    #[serde(default)]
    pub report: ReportConfig,
    #[serde(default)]
    pub timer: TimerConfig,
}
```

---

## 核心流程

### 初始化流程

```
ConfigLoader::new()
       │
       ▼
┌──────────────────────────────┐
│ 1. 创建配置目录结构           │
│    - ~/.knight-agent/        │
│    - ~/.knight-agent/config/ │
└──────────────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│ 2. 加载/创建主配置            │
│    knight.json               │
│    - 文件存在 → 解析          │
│    - 文件不存在 → 创建默认    │
└──────────────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│ 3. 加载/创建系统配置          │
│    config/*.yaml             │
│    - 文件存在 → 解析          │
│    - 文件不存在 → 创建默认    │
└──────────────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│ 4. 设置文件监控               │
│    notify::Watcher           │
│    监控 knight.json 和       │
│    config/*.yaml             │
└──────────────────────────────┘
       │
       ▼
    ConfigLoader 实例
```

### 热更新流程

```
文件变更
   │
   ▼
notify::Watcher 事件
   │
   ▼
解析变更的配置文件
   │
   ▼
更新 RwLock 中的配置
   │
   ▼
通过 broadcast::Sender 发送变更事件
   │
   ├──► Subscriber 1: 重新加载配置
   ├──► Subscriber 2: 重新加载配置
   └──► Subscriber N: 重新加载配置
```

---

## 模块交互

### 与其他模块的关系

```
┌─────────────────────────────────────────────────────────────┐
│                    Configuration 模块                        │
│  - 配置单一数据源                                           │
│  - 热更新广播                                               │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Skill Engine  │ │  Agent Runtime  │ │  CLI/REPL       │
│   (读取 LLM)    │ │  (读取 LLM)     │ │  (显示状态)     │
└─────────────────┘ └─────────────────┘ └─────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  其他所有模块    │
                    │  (订阅配置变更)  │
                    └─────────────────┘
```

### 配置获取优先级

当模块需要 LLM 配置时，按以下优先级：

1. **运行时覆盖** (参数传入)
2. **上下文提供** (Agent context)
3. **配置文件** (knight.json)
4. **默认值**

---

## 配置与部署

### knight.json 示例

```json
{
  "llm": {
    "defaultProvider": "anthropic",
    "providers": {
      "anthropic": {
        "type": "anthropic",
        "apiKey": "${ANTHROPIC_API_KEY}",
        "baseUrl": "https://api.anthropic.com",
        "timeoutSecs": 120,
        "models": [
          {
            "id": "claude-sonnet-4-6",
            "contextLength": 200000,
            "pricing": {"input": 3.0, "output": 15.0, "currency": "USD"},
            "capabilities": ["chat", "tools"]
          }
        ],
        "defaultModel": "claude-sonnet-4-6"
      }
    }
  }
}
```

### config/agent.yaml 示例

```yaml
defaultVariant: null
maxConcurrentTasks: 10
taskTimeoutSecs: 300

runtime:
  maxExecutionTime: 300
  maxToolCalls: 50
  maxLlmCalls: 20

skill:
  directories:
    - "./skills"
    - "~/.knight-agent/skills"
  execution:
    maxSteps: 100
    timeout: 600

task:
  maxParallel: 10
  defaultTimeout: 300

workflow:
  directories:
    - "./workflows"
    - "~/.knight-agent/workflows"
  execution:
    defaultMode: background
```

### config/core.yaml 示例

```yaml
command:
  prefix: "/"
  commands:
    help: {enabled: true, aliases: ["h", "?"]}
    status: {enabled: true, aliases: ["s"]}

cli:
  prompt: "knight> "
  historySize: 1000
  autoComplete: true

eventLoop:
  tickInterval: 100
  maxEventsPerTick: 100
  queueSize: 10000

session:
  defaultSessionId: "default"
  maxSessions: 100
  sessionTimeoutSecs: 3600

bootstrap:
  parallelInit: true
  initTimeoutSecs: 120
  failFast: false
```

### 环境变量

配置支持 `${VAR}` 语法进行环境变量替换：

```json
{
  "apiKey": "${ANTHROPIC_API_KEY}"
}
```

常用环境变量：
- `ANTHROPIC_API_KEY` - Anthropic API 密钥
- `OPENAI_API_KEY` - OpenAI API 密钥

---

## 错误处理

### ConfigError

```rust
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("配置文件不存在: {0}")]
    FileNotFound(String),

    #[error("配置文件解析失败: {0}")]
    ParseError(String),

    #[error("配置验证失败: {0}")]
    ValidationError(String),

    #[error("配置目录创建失败: {0}")]
    DirectoryCreationError(String),

    #[error("配置监听失败: {0}")]
    WatchError(#[from] notify::Error),
}
```

### 错误恢复策略

| 错误类型 | 处理策略 |
|---------|---------|
| 文件不存在 | 创建默认配置 |
| 解析失败 | 记录警告，使用默认配置 |
| 验证失败 | 记录错误，使用默认配置 |
| 监听失败 | 返回错误，配置不生效 |

---

## 附录

### 配置合并历史

| 时间 | 变更 |
|------|------|
| 2026-04-07 | 从 26 个独立配置文件合并为 11 个 |

合并清单：
- `knight.json` - LLM 配置（用户常用）
- `agent.yaml` - 6 个 Agent 模块
- `core.yaml` - 8 个 Core 模块
- `services.yaml` - 3 个 Services
- `tools.yaml` - 工具系统
- `infrastructure.yaml` - 基础设施
- `storage.yaml` - 存储
- `security.yaml` - 安全
- `logging.yaml` - 日志
- `monitoring.yaml` - 监控
- `compressor.yaml` - 上下文压缩

### 相关文档

| 文档 | 描述 |
|------|------|
| [agent-runtime](../agent/agent-runtime.md) | Agent 运行时，使用配置获取 LLM |
| [skill-engine](../agent/skill-engine.md) | 技能引擎，使用配置获取 LLM |
| [bootstrap](../core/bootstrap.md) | 系统启动，负责调用配置加载 |
| [llm-provider](../services/llm-provider.md) | LLM 提供者，从配置获取提供者信息 |

### 测试策略

```bash
# 运行配置模块测试
cargo test -p configuration

# 测试热更新
# 1. 启动 knight-agent
# 2. 修改 knight.json
# 3. 观察日志中的 "Detected change in knight.json"
```
