# Configuration - Knight Agent Configuration Library

Centralized configuration management for Knight Agent with hot-reload support.

## Features

- **Hot Reload**: Configuration changes are automatically detected and applied
- **User-Friendly JSON**: Main user-facing config uses JSON format
- **System YAML**: System configs use YAML for better readability
- **Environment Variables**: Supports `${VAR}` syntax for API keys
- **Type-Safe**: Rust structs with serde validation
- **Single Source of Truth**: LLM config centralized, referenced by all modules

## Directory Structure

```
~/.knight-agent/
├── knight.json              # Main user-facing config (LLM only)
└── config/                  # System configs (YAML format)
    ├── agent.yaml           # Agent modules (6 modules consolidated)
    ├── core.yaml            # Core modules (8 modules consolidated)
    ├── services.yaml        # Services (3 services consolidated)
    ├── tools.yaml           # Tool system
    ├── infrastructure.yaml  # Infrastructure (IPC)
    ├── storage.yaml         # Storage configuration
    ├── security.yaml        # Security configuration
    ├── logging.yaml         # Logging configuration
    ├── monitoring.yaml      # Monitoring configuration
    └── compressor.yaml      # Context compressor configuration
```

**Configuration consolidation**: Reduced from 26 separate config files to 11 consolidated files for better maintainability.

## Configuration Files

### knight.json (Main User Configuration)

**This file only contains LLM provider configuration** - the only config users typically need to modify.

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
            "pricing": {
              "input": 3.0,
              "output": 15.0,
              "currency": "USD"
            },
            "capabilities": ["chat", "tools"]
          },
          {
            "id": "claude-haiku",
            "contextLength": 200000,
            "pricing": {
              "input": 0.25,
              "output": 1.25,
              "currency": "USD"
            },
            "capabilities": ["chat", "tools"]
          }
        ],
        "defaultModel": "claude-sonnet-4-6"
      },
      "openai": {
        "type": "openai",
        "apiKey": "${OPENAI_API_KEY}",
        "baseUrl": "https://api.openai.com/v1",
        "timeoutSecs": 120,
        "models": [
          {
            "id": "gpt-4o",
            "contextLength": 128000,
            "pricing": {
              "input": 2.50,
              "output": 10.00,
              "currency": "USD"
            },
            "capabilities": ["chat", "tools"]
          },
          {
            "id": "gpt-4o-mini",
            "contextLength": 128000,
            "pricing": {
              "input": 0.15,
              "output": 0.60,
              "currency": "USD"
            },
            "capabilities": ["chat", "tools"]
          }
        ],
        "defaultModel": "gpt-4o"
      }
    }
  }
}
```

### config/agent.yaml

Consolidated configuration for all agent-related modules (agent-runtime, skill-engine, task-manager, workflows-directory).

```yaml
# Agent configuration (consolidated)
# Common settings
defaultVariant: null
maxConcurrentTasks: 10
taskTimeoutSecs: 300

# Agent runtime settings
runtime:
  maxExecutionTime: 300
  maxToolCalls: 50
  maxLlmCalls: 20
  retry:
    maxAttempts: 3
    delay: 1000
    backoff: exponential
    retryableErrors:
      - rate_limit
      - timeout
      - connection_error
  timeout:
    llmCall: 60
    toolCall: 30
  streaming:
    enabled: true
    chunkSize: 100

# Skill engine settings
skill:
  directories:
    - "./skills"
    - "~/.knight-agent/skills"
  execution:
    maxSteps: 100
    timeout: 600
    enforceTimeout: true
    enforceMaxSteps: true
  triggers:
    debounce: 500
    maxQueueSize: 1000
  llmParsing:
    retry: 3
    validationEnabled: true

# Task manager settings
task:
  maxParallel: 10
  defaultTimeout: 300
  checkInterval: 5
  retry:
    maxAttempts: 3
    delay: 1000
    backoff: exponential
    retryableErrors: []
  storage:
    persistResults: true
    retentionDays: 30
  dag:
    maxTasks: 1000
    maxDepth: 50

# Workflow settings
workflow:
  directories:
    - "./workflows"
    - "~/.knight-agent/workflows"
  execution:
    defaultMode: background
    timeout: 604800  # 7 days
  versioning:
    enabled: true
    gitTracking: true
  cache:
    enabled: true
    ttl: 3600
```

### config/core.yaml

Consolidated configuration for all core modules (command, cli, event-loop, hooks, orchestrator, router, session-manager, bootstrap).

```yaml
# Command module settings
command:
  prefix: "/"
  commands:
    help:
      enabled: true
      aliases: ["h", "?"]
    status:
      enabled: true
      aliases: ["s"]

# CLI settings
cli:
  prompt: "knight> "
  multilinePrompt: "... "
  historySize: 1000
  historyFile: null
  autoComplete: true
  syntaxHighlight: true

# Event loop settings
eventLoop:
  tickInterval: 100
  maxEventsPerTick: 100
  queueSize: 10000

# Hooks settings
hooks:
  preCommand: []
  postCommand: []
  preAgent: []
  postAgent: []
  onError: []

# Orchestrator settings
orchestrator:
  maxParallelAgents: 5
  timeoutSecs: 300
  retryAttempts: 3

# Router settings
router:
  defaultVariant: null
  routingRules: []

# Session settings
session:
  defaultSessionId: "default"
  maxSessions: 100
  sessionTimeoutSecs: 3600
  persistSessions: true

# Bootstrap settings
bootstrap:
  parallelInit: true
  initTimeoutSecs: 120
  failFast: false
```

### config/services.yaml

Consolidated configuration for all services (mcp-client, report-skill, timer-system).

```yaml
# MCP client settings
mcp:
  maxConcurrentConnections: 10
  connectionTimeoutSecs: 30
  requestTimeoutSecs: 60
  retryAttempts: 3
  servers: {}

# Report skill settings
report:
  enabled: true
  format: markdown
  outputPath: null
  includeTimestamps: true
  includeMetadata: true

# Timer settings
timer:
  enabled: true
  resolution: 100
  maxTimers: 1000
```

### config/tools.yaml

Tool system configuration.

```yaml
# Tool system settings
builtin:
  enabled: true
  timeout: 30

custom:
  enabled: true
  directories:
    - "./tools"
    - "~/.knight-agent/tools"
  timeout: 60
  sandboxEnabled: true

mcp:
  enabled: true
  timeout: 120
  maxToolCalls: 50
```

### config/infrastructure.yaml

Infrastructure configuration (IPC, etc.).

```yaml
# IPC settings
ipc:
  enabled: true
  mechanism: "memory"  # Options: memory, tcp, unix
  address: null
  timeoutSecs: 30
  bufferSize: 1024
```

### config/storage.yaml

Storage service configuration (system internal).

```yaml
# Storage configuration
databasePath: null
maxDbSizeMb: 1024
```

### config/security.yaml

Security and sandbox configuration (system internal).

```yaml
# Security configuration
sandboxEnabled: true
allowedOperations: []
blockedOperations: []
```

### config/logging.yaml

Logging configuration (system internal).

```yaml
# Logging configuration
level: info
maxFileSizeMb: 10
maxFiles: 5
consoleOutput: true
```

### config/monitoring.yaml

Monitoring configuration (system internal).

```yaml
# Monitoring configuration
enabled: false
metricsIntervalSecs: 60
healthCheckIntervalSecs: 30
```

### config/compressor.yaml

Context compressor configuration (system internal).

```yaml
# Compressor configuration
enabled: true
thresholdTokens: 30000
targetRatio: 0.5
strategy: semantic
```

## Usage

### Basic Usage

```rust
use configuration::ConfigLoader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_dir = dirs::home_dir()
        .unwrap()
        .join(".knight-agent");

    let loader = ConfigLoader::new(config_dir).await?;

    // Get LLM configuration (user-facing)
    let llm_config = loader.get_llm_config();

    // Get system configurations (internal)
    let logging_config = loader.get_logging_config();
    let agent_config = loader.get_agent_config();

    Ok(())
}
```

### Hot Reload

```rust
// Subscribe to configuration changes
let mut rx = loader.subscribe();

tokio::spawn(async move {
    while let Ok(change) = rx.recv().await {
        match change {
            configuration::ConfigChangeEvent::MainConfigChanged(config) => {
                // Handle knight.json change
                println!("LLM config changed: {:?}", config.llm);
            }
            configuration::ConfigChangeEvent::SystemConfigChanged { name, config } => {
                // Handle system config change
                println!("System config '{}' changed", name);
            }
        }
    }
});
```

### Module Integration

All modules should use `knight-config` to access LLM configuration instead of defining their own:

```rust
// In skill-engine, agent-runtime, etc.
use configuration::ConfigLoader;

struct SkillEngine {
    config_loader: Arc<ConfigLoader>,
}

impl SkillEngine {
    fn get_default_llm(&self) -> LLMConfig {
        let llm_config = self.config_loader.get_llm_config();
        let default_provider = llm_config
            .and_then(|cfg| cfg.default_provider)
            .unwrap_or_else(|| "default".to_string());

        // Build LLM config from knight.json
        LLMConfig {
            provider: default_provider,
            // ... other fields from providers[default_provider]
        }
    }
}
```

## API Reference

### ConfigLoader

Main configuration loader with hot-reload support.

#### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `new(config_dir)` | `ConfigLoader` | Create loader, auto-creates default configs |
| `get_main_config()` | `KnightConfig` | Full main configuration (LLM only) |
| `get_llm_config()` | `Option<LlmConfig>` | LLM provider configuration |
| `get_agent_config()` | `AgentConfig` | Agent modules (consolidated) |
| `get_core_config()` | `CoreConfig` | Core modules (consolidated) |
| `get_services_config()` | `ServicesConfig` | Services (consolidated) |
| `get_tools_config()` | `ToolsConfig` | Tool system configuration |
| `get_infrastructure_config()` | `InfrastructureConfig` | Infrastructure config |
| `get_storage_config()` | `StorageConfig` | Storage configuration |
| `get_security_config()` | `SecurityConfig` | Security configuration |
| `get_logging_config()` | `LoggingConfig` | Logging configuration |
| `get_monitoring_config()` | `MonitoringConfig` | Monitoring configuration |
| `get_compressor_config()` | `CompressorConfig` | Compressor configuration |
| `get_system_config(name)` | `Option<SystemConfig>` | Get specific system config |
| `subscribe()` | `Receiver<ConfigChangeEvent>` | Subscribe to config change events |
| `reload_main_config()` | `Result<()>` | Manually reload main configuration |
| `config_dir()` | `&Path` | Get config directory path |

### Configuration Types

#### KnightConfig (knight.json)

Main configuration - only contains user-facing LLM configuration.

| Field | Type | Description |
|-------|------|-------------|
| `llm` | `Option<LlmConfig>` | LLM provider configuration |

#### LlmConfig

| Field | Type | Description |
|-------|------|-------------|
| `default_provider` | `Option<String>` | Default provider name |
| `providers` | `HashMap<String, LlmProviderConfig>` | Available providers |

#### System Configs (config/*.yaml)

| Config | File | Description |
|--------|------|-------------|
| `AgentConfig` | agent.yaml | Agent modules (runtime, skill, task, workflow) |
| `CoreConfig` | core.yaml | Core modules (command, cli, event-loop, hooks, orchestrator, router, session, bootstrap) |
| `ServicesConfig` | services.yaml | Services (mcp, report, timer) |
| `ToolsConfig` | tools.yaml | Tool system (builtin, custom, mcp) |
| `InfrastructureConfig` | infrastructure.yaml | Infrastructure (IPC) |
| `StorageConfig` | storage.yaml | Storage/database settings |
| `SecurityConfig` | security.yaml | Security and sandbox settings |
| `LoggingConfig` | logging.yaml | Logging configuration |
| `MonitoringConfig` | monitoring.yaml | Monitoring settings |
| `CompressorConfig` | compressor.yaml | Context compression settings |

## Environment Variables

The configuration loader supports environment variable substitution using `${VAR}` syntax:

```json
{
  "apiKey": "${ANTHROPIC_API_KEY}"
}
```

If the environment variable is not set, the literal string will be used (causing API calls to fail).

Common environment variables:
- `ANTHROPIC_API_KEY` - Anthropic Claude API key
- `OPENAI_API_KEY` - OpenAI API key
- `CUSTOM_API_KEY` - Custom provider API key

## Configuration Priority

For LLM configuration across modules:

1. **Runtime override** (e.g., `llm_override` parameter)
2. **Context-provided config** (from Agent context)
3. **Module-specific config** (deprecated, should use knight.json)
4. **knight.json** `llm.defaultProvider` ← **Single source of truth**

## Testing

```bash
cargo test -p knight-config
```

## License

MIT
