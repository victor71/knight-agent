# Sandbox (沙箱机制)

## 概述

### 职责描述

Sandbox 负责系统资源的隔离和限制，包括：

- 文件系统访问控制
- 命令执行限制
- 资源使用限制（CPU、内存、网络）
- 进程隔离
- 恶意行为检测

### 设计目标

1. **安全隔离**: Agent 操作限制在指定范围内
2. **资源可控**: 限制 CPU、内存、网络使用
3. **灵活配置**: 支持不同级别的隔离
4. **可观测**: 监控资源使用和违规行为

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取 Workspace 配置 |
| Security Manager | 依赖 | 权限检查、审计日志 |
| Tool System | 依赖 | 工具调用拦截 |

**依赖关系说明**:
- Sandbox 依赖 Security Manager：调用 `SecurityManager.check_permission` 进行权限验证，调用 `SecurityManager.log_event` 记录安全事件
- Security Manager 不依赖 Sandbox：Security Manager 通过 `sandbox:bypass` checkpoint 验证绕过沙箱的权限，但不直接调用 Sandbox 接口

---

## 接口定义

### 对外接口

```yaml
# Sandbox 接口定义
Sandbox:
  # ========== 沙箱管理 ==========
  create_sandbox:
    description: 创建沙箱
    inputs:
      config:
        type: SandboxConfig
        required: true
    outputs:
      sandbox_id:
        type: string

  destroy_sandbox:
    description: 销毁沙箱
    inputs:
      sandbox_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  get_sandbox:
    description: 获取沙箱信息
    inputs:
      sandbox_id:
        type: string
        required: true
    outputs:
      sandbox:
        type: SandboxInfo | null

  list_sandboxes:
    description: 列出沙箱
    inputs:
      status:
        type: string
        description: 状态过滤
        enum: [active, paused, terminated]
        required: false
    outputs:
      sandboxes:
        type: array<SandboxInfo>

  # ========== 访问控制 ==========
  check_file_access:
    description: 检查文件访问权限
    inputs:
      sandbox_id:
        type: string
        required: true
      path:
        type: string
        required: true
      action:
        type: string
        enum: [read, write, delete, execute]
        required: true
    outputs:
      allowed:
        type: boolean
      reason:
        type: string | null

  check_command_access:
    description: 检查命令执行权限
    inputs:
      sandbox_id:
        type: string
        required: true
      command:
        type: string
        required: true
      args:
        type: array<string>
        required: false
    outputs:
      allowed:
        type: boolean
      reason:
        type: string | null

  check_network_access:
    description: 检查网络访问权限
    inputs:
      sandbox_id:
        type: string
        required: true
      host:
        type: string
        required: true
      port:
        type: integer
        required: true
    outputs:
      allowed:
        type: boolean
      reason:
        type: string | null

  # ========== 资源监控 ==========
  get_resource_usage:
    description: 获取资源使用情况
    inputs:
      sandbox_id:
        type: string
        required: true
    outputs:
      usage:
        type: ResourceUsage

  get_resource_limits:
    description: 获取资源限制
    inputs:
      sandbox_id:
        type: string
        required: true
    outputs:
      limits:
        type: ResourceLimits

  set_resource_limits:
    description: 设置资源限制
    inputs:
      sandbox_id:
        type: string
        required: true
      limits:
        type: ResourceLimits
        required: true
    outputs:
      success:
        type: boolean

  # ========== 违规处理 ==========
  get_violations:
    description: 获取违规记录
    inputs:
      sandbox_id:
        type: string
        required: true
      time_range:
        type: TimeRange
        required: false
    outputs:
      violations:
        type: array<Violation>

  report_violation:
    description: |
      报告违规行为
      内部实现：将 Violation 转换为 SecurityEvent 并调用 Security Manager.log_event
      - Violation.violation_type 映射到 SecurityEvent.event_type (见下方映射表)
      - Violation.severity 映射到 SecurityEvent.details.severity
    inputs:
      sandbox_id:
        type: string
        required: true
      violation:
        type: Violation
        required: true
    outputs:
      violation_id:
        type: string

  # ========== 沙箱配置 ==========
  get_sandbox_config:
    description: 获取沙箱配置
    inputs:
      sandbox_id:
        type: string
        required: true
    outputs:
      config:
        type: SandboxConfig

  update_sandbox_config:
    description: 更新沙箱配置
    inputs:
      sandbox_id:
        type: string
        required: true
      config:
        type: SandboxConfig
        required: true
    outputs:
      success:
        type: boolean
```

### 数据结构

```yaml
# 沙箱配置
SandboxConfig:
  # 沙箱类型
  level:
    type: string
    enum: [none, basic, strict, full]
    description: 沙箱隔离级别
    default: basic

  # 工作空间
  workspace:
    type: string
    description: 工作目录根路径

  # 文件访问控制
  filesystem:
    type: FilesystemSandbox

  # 命令执行控制
  command:
    type: CommandSandbox

  # 网络访问控制
  network:
    type: NetworkSandbox

  # 资源限制
  resources:
    type: ResourceLimits

  # 违规处理
  violation_action:
    type: string
    enum: [log, warn, block, terminate]
    description: 违规时的操作
    default: warn

# 文件系统沙箱
FilesystemSandbox:
  allowed_paths:
    type: array<string>
    description: 允许访问的路径模式
  denied_patterns:
    type: array<string>
    description: 拒绝的路径模式
  read_only:
    type: array<string>
    description: 只读路径
  max_file_size:
    type: integer
    description: 最大文件大小（字节）
  max_total_size:
    type: integer
    description: 最大总存储（字节）

# 命令沙箱
CommandSandbox:
  allowed_commands:
    type: array<string>
    description: 允许的命令白名单
  denied_commands:
    type: array<string>
    description: 拒绝的命令黑名单
  max_execution_time:
    type: integer
    description: 最大执行时间（秒）
  max_concurrent:
    type: integer
    description: 最大并发进程数

# 网络沙箱
NetworkSandbox:
  enabled:
    type: boolean
    description: 是否允许网络访问
  allowed_hosts:
    type: array<string>
    description: 允许访问的主机
  denied_hosts:
    type: array<string>
    description: 拒绝访问的主机
  allowed_ports:
    type: array<PortRange>
    description: 允许的端口范围
  max_connections:
    type: integer
    description: 最大并发连接数
  max_bandwidth:
    type: integer
    description: 最大带宽（字节/秒）

# 端口范围
PortRange:
  start:
    type: integer
  end:
    type: integer

# 资源限制
ResourceLimits:
  max_memory_mb:
    type: integer
    description: 最大内存（MB）
  max_cpu_percent:
    type: float
    description: 最大 CPU 使用率
  max_execution_time:
    type: integer
    description: 最大执行时间（秒）
  max_file_handles:
    type: integer
    description: 最大文件句柄数

# 沙箱信息
SandboxInfo:
  id:
    type: string
  level:
    type: string
  status:
    type: string
    enum: [active, paused, terminated]
  created_at:
    type: datetime
  config:
    type: SandboxConfig
  usage:
    type: ResourceUsage
  violation_count:
    type: integer

# 资源使用
ResourceUsage:
  memory_mb:
    type: integer
  cpu_percent:
    type: float
  execution_time:
    type: integer
  file_handles:
    type: integer
  network_connections:
    type: integer
  disk_usage:
    type: integer
    description: 磁盘使用（字节）

# 违规记录
Violation:
  id:
    type: string
  sandbox_id:
    type: string
  timestamp:
    type: datetime
  violation_type:
    type: string
    enum: [file_access_denied, command_denied, network_denied, resource_exceeded, malicious_behavior]
  severity:
    type: string
    enum: [low, medium, high, critical]
  description:
    type: string
  details:
    type: map<string, any>
```

### Violation → SecurityEvent 映射

当 Sandbox 调用 `report_violation` 时，内部实现将 Violation 映射到 Security Manager 的 SecurityEvent：

| Violation.violation_type | SecurityEvent.event_type | 说明 |
|-------------------------|-------------------------|------|
| file_access_denied | policy_violation | 文件访问被拒绝 |
| command_denied | policy_violation | 命令执行被拒绝 |
| network_denied | policy_violation | 网络访问被拒绝 |
| resource_exceeded | policy_violation | 资源使用超限 |
| malicious_behavior | threat_detected | 检测到恶意行为 |

| Violation.severity | SecurityEvent.details.severity | 说明 |
|-------------------|-------------------------------|------|
| low | low | 低风险 |
| medium | medium | 中风险 |
| high | high | 高风险 |
| critical | critical | 严重风险 |

### 配置选项

```yaml
# config/sandbox.yaml
sandbox:
  # 默认沙箱级别
  default_level: basic

  # 文件系统
  filesystem:
    denied_patterns:
      - "**/.git/**"
      - "**/node_modules/**"
      - "**/.env"
    max_file_size: 10485760
    max_total_size: 104857600

  # 命令执行
  command:
    max_execution_time: 300
    max_concurrent: 5

  # 网络
  network:
    enabled: true
    max_connections: 10

  # 资源限制
  resources:
    max_memory_mb: 1024
    max_cpu_percent: 80

  # 违规处理
  violation_action: warn
```

---

## 核心流程

### 文件访问检查

```
文件访问请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 规范化路径                │
│    - 转为绝对路径            │
│    - 解析符号链接            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 检查拒绝模式              │
│    - 遍历 denied_patterns    │
│    - glob 匹配               │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 匹配？  │
    └───┬────┘
        │ 是
        ▼
    拒绝访问
        │ 否
        ▼
┌──────────────────────────────┐
│ 3. 检查允许路径              │
│    - 遍历 allowed_paths      │
│    - 路径前缀匹配            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 匹配？  │
    └───┬────┘
        │ 否
        ▼
    拒绝访问
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 检查操作类型              │
│    - 只读路径检查            │
│    - 写入权限检查            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 允许？  │
    └───┬────┘
        │ 否
        ▼
    拒绝访问
        │ 是
        ▼
    允许访问
```

### 命令执行检查

```
命令执行请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 解析命令                  │
│    - 提取命令名              │
│    - 提取参数                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 检查黑名单                │
│    - 遍历 denied_commands    │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 在黑名单？│
    └───┬────┘
        │ 是
        ▼
    拒绝执行
        │ 否
        ▼
┌──────────────────────────────┐
│ 3. 检查白名单                │
│    - 如果白名单非空          │
│    - 检查是否在白名单        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 允许？  │
    └───┬────┘
        │ 否
        ▼
    拒绝执行
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 检查参数安全              │
│    - 检测危险参数            │
│    - 检测命令注入            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 安全？  │
    └───┬────┘
        │ 否
        ▼
    拒绝执行
        │ 是
        ▼
    允许执行
```

### 资源监控

```
资源使用监控
        │
        ▼
┌──────────────────────────────┐
│ 1. 定期采集资源数据          │
│    - 内存使用                │
│    - CPU 使用                │
│    - 文件句柄                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 比较资源限制              │
│    - 检查是否超限            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 超限？  │
    └───┬────┘
        │ 否
        ▼
    继续监控
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 触发违规处理              │
│    - 记录违规                │
│    - 执行响应动作            │
└──────────────────────────────┘
        │
        ▼
    完成
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│              Sandbox                    │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │File      │  │Command   │  │Network ││
│  │Guard     │  │Guard     │  │Guard   ││
│  └──────────┘  └──────────┘  └────────┘│
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Resource  │  │Violation │  │Monitor ││
│  │Limiter   │  │Handler   │  │        ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Session   │  │Security  │  │Tool      │
│Manager   │  │Manager   │  │System    │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
Tool System / Agent Runtime
    │
    ▼
┌─────────────────────────────┐
│ Sandbox                     │
│ - 访问检查                  │
│ - 资源限制                  │
└─────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 允许？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌─────────────────┐   ┌──────────────┐
│ 记录违规        │   │ 执行操作     │
│ 返回拒绝        │   │ 监控资源     │
└─────────────────┘   └──────────────┘
```

---

## 沙箱级别

### 级别定义

| 级别 | 隔离程度 | 适用场景 |
|------|----------|----------|
| none | 无隔离 | 本地开发 |
| basic | 基础隔离 | 一般使用 |
| strict | 严格隔离 | 敏感操作 |
| full | 完全隔离 | 不可信代码 |

### 级别配置

```yaml
# 无隔离
none:
  filesystem:
    allowed_paths: ["**/*"]
    denied_patterns: []
  command:
    allowed_commands: null  # 无限制
  network:
    enabled: true

# 基础隔离
basic:
  filesystem:
    allowed_paths: ["{{ workspace }}/**"]
    denied_patterns:
      - "**/.git/**"
      - "**/.env"
  command:
    denied_commands:
      - rm -rf /
      - mkfs
      - dd
  network:
    enabled: true
    max_connections: 10

# 严格隔离
strict:
  filesystem:
    allowed_paths: ["{{ workspace }}/**"]
    denied_patterns:
      - "**/.git/**"
      - "**/node_modules/**"
      - "**/.env*"
      - "**/target/**"
    read_only:
      - "{{ workspace }}/.git"
  command:
    allowed_commands:
      - git
      - npm
      - node
      - python
      - cargo
    max_execution_time: 300
  network:
    allowed_hosts:
      - "api.anthropic.com"
      - "api.openai.com"
      - "registry.npmjs.org"
    max_connections: 5

# 完全隔离
full:
  filesystem:
    allowed_paths: ["{{ workspace }}/sandbox/**"]
    max_file_size: 1048576
    max_total_size: 10485760
  command:
    allowed_commands: []
  network:
    enabled: false
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/sandbox.yaml
sandbox:
  # 默认沙箱级别
  default_level: basic

  # 文件系统
  filesystem:
    denied_patterns:
      - "**/.git/**"
      - "**/node_modules/**"
      - "**/.env"
    max_file_size: 10485760
    max_total_size: 104857600

  # 命令执行
  command:
    denied_commands:
      - rm -rf
      - mkfs
      - dd
      - chmod 000
    max_execution_time: 300
    max_concurrent: 5

  # 网络
  network:
    enabled: true
    allowed_hosts:
      - "api.anthropic.com"
      - "api.openai.com"
    max_connections: 10

  # 资源限制
  resources:
    max_memory_mb: 1024
    max_cpu_percent: 80
    max_execution_time: 600

  # 违规处理
  violation_action: warn
  log_violations: true
```

### 环境变量

```bash
# 沙箱配置
export KNIGHT_SANDBOX_DEFAULT_LEVEL="basic"
export KNIGHT_SANDBOX_VIOLATION_ACTION="warn"

# 文件系统
export KNIGHT_SANDBOX_MAX_FILE_SIZE=10485760

# 资源限制
export KNIGHT_SANDBOX_MAX_MEMORY_MB=1024
export KNIGHT_SANDBOX_MAX_CPU_PERCENT=80
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 路径检查 | < 1ms | 内存操作 |
| 命令检查 | < 1ms | 内存操作 |
| 资源采样 | 1s | 采样间隔 |

### 错误处理

```yaml
error_codes:
  ACCESS_DENIED:
    code: 403
    message: "访问被拒绝"
    action: "检查路径/命令权限"

  RESOURCE_EXCEEDED:
    code: 429
    message: "超过资源限制"
    action: "减少资源使用"

  SANDBOX_VIOLATION:
    code: 403
    message: "违反沙箱规则"
    action: "查看违规详情"

  MALICIOUS_BEHAVIOR:
    code: 403
    message: "检测到恶意行为"
    action: "终止操作"
```

### 安全最佳实践

1. **使用严格级别**: 生产环境使用 strict 或 full
2. **限制网络访问**: 仅允许必要的网络访问
3. **限制命令**: 使用白名单而非黑名单
4. **监控资源**: 及时发现资源滥用
5. **审计日志**: 记录所有违规行为

---

## 沙箱检查点

### 检查点调用总览

以下列出了所有模块调用 Sandbox 的检查点：

| 模块 | 检查点 | 触发时机 | 检查内容 |
|------|--------|----------|----------|
| **Session Manager** | `session:create_sandbox` | 创建会话 | 创建会话沙箱 |
| | `session:workspace_access` | 访问 Workspace | 验证工作区访问 |
| **Tool System** | `tool:bash_execute` | 执行 Bash 命令 | 命令白名单/参数检查 |
| | `tool:file_read` | 读取文件 | 文件路径访问检查 |
| | `tool:file_write` | 写入文件 | 文件路径写入检查 |
| | `tool:file_delete` | 删除文件 | 文件删除检查 |
| | `tool:glob` | 文件搜索 | 路径模式检查 |
| **Agent Runtime** | `agent:resource_check` | Agent 操作前 | 资源使用检查 |
| | `agent:network_access` | 网络请求 | 网络访问白名单 |
| **Skill Engine** | `skill:file_access` | 技能访问文件 | 技能文件权限检查 |
| **LLM Provider** | `llm:network_call` | LLM API 调用 | API 端点网络检查 |
| **MCP Client** | `mcp:server_connect` | MCP 连接 | MCP 服务器网络检查 |
| **Orchestrator** | `orchestrate:resource_alloc` | 资源分配 | 资源限制检查 |

### 详细检查点说明

#### Session Manager 检查点

```yaml
session:create_sandbox:
  description: 创建会话时创建关联沙箱
  caller: Session Manager
  sandbox_operation: create_sandbox
  config:
    level: basic  # 从配置读取
    workspace: "{{ session.workspace }}"
  default:
    user: 可配置级别
    agent: 继承会话级别

session:workspace_access:
  description: 访问工作区时验证路径
  caller: Session Manager
  sandbox_operation: check_file_access
  check:
    path: "{{ workspace_path }}"
    action: "read"
  default:
    allowed_paths: ["{{ workspace }}/**"]
    denied_patterns: ["**/.git/**", "**/.env"]
```

#### Tool System 检查点

```yaml
tool:bash_execute:
  description: 执行 Bash 命令前检查
  caller: Tool System
  sandbox_operation: check_command_access
  check:
    command: "{{ command_name }}"
    args: "{{ command_args }}"
  validation:
    - 黑名单检查 (rm -rf, mkfs, dd 等)
    - 白名单检查 (如果配置)
    - 参数安全检查 (命令注入)
  default:
    user: allow (除危险命令)
    agent: deny (危险命令)
    agent: allow (白名单命令)

tool:file_read:
  description: 读取文件前检查
  caller: Tool System
  sandbox_operation: check_file_access
  check:
    path: "{{ file_path }}"
    action: "read"
  validation:
    - 路径规范化
    - 拒绝模式匹配
    - 允许路径检查
  default:
    user: allow (工作区内)
    agent: allow (工作区内，排除敏感)

tool:file_write:
  description: 写入文件前检查
  caller: Tool System
  sandbox_operation: check_file_access
  check:
    path: "{{ file_path }}"
    action: "write"
  validation:
    - 路径规范化
    - 拒绝模式匹配
    - 只读路径检查
    - 文件大小限制
  default:
    user: allow (工作区内)
    agent: allow (工作区内，排除敏感和只读)

tool:file_delete:
  description: 删除文件前检查
  caller: Tool System
  sandbox_operation: check_file_access
  check:
    path: "{{ file_path }}"
    action: "delete"
  validation:
    - 路径规范化
    - 保护文件检查
  default:
    user: allow (工作区内)
    agent: deny (敏感路径)

tool:glob:
  description: 文件搜索前检查
  caller: Tool System
  sandbox_operation: check_file_access
  check:
    pattern: "{{ glob_pattern }}"
    action: "read"
  default:
    user: allow (工作区内)
    agent: allow (工作区内)
```

#### Agent Runtime 检查点

```yaml
agent:resource_check:
  description: Agent 操作前检查资源使用
  caller: Agent Runtime
  sandbox_operation: get_resource_usage
  check:
    memory: "{{ current_memory }}"
    cpu: "{{ current_cpu }}"
    time: "{{ execution_time }}"
  validation:
    - 内存超限检查
    - CPU 使用率检查
    - 执行时间检查
  default:
    max_memory_mb: 1024
    max_cpu_percent: 80
    max_execution_time: 600

agent:network_access:
  description: Agent 网络请求前检查
  caller: Agent Runtime
  sandbox_operation: check_network_access
  check:
    host: "{{ request_host }}"
    port: "{{ request_port }}"
  validation:
    - 主机白名单
    - 端口范围
    - 并发连接数
  default:
    user: allow
    agent: deny (未配置白名单)
```

#### Skill Engine 检查点

```yaml
skill:file_access:
  description: 技能访问文件时检查
  caller: Skill Engine → Tool System
  sandbox_operation: check_file_access
  check:
    path: "{{ file_path }}"
    action: "{{ access_action }}"
  default:
    user: allow (工作区内)
    agent: allow (技能指定路径)
```

#### LLM Provider 检查点

```yaml
llm:network_call:
  description: LLM API 调用时网络检查
  caller: LLM Provider
  sandbox_operation: check_network_access
  check:
    host: "{{ api_endpoint }}"
    port: 443
  validation:
    - API 端点白名单
  default:
    user: allow
    agent: allow (配置的 API)
```

#### MCP Client 检查点

```yaml
mcp:server_connect:
  description: 连接 MCP 服务器时检查
  caller: MCP Client
  sandbox_operation: check_network_access
  check:
    host: "{{ mcp_server_host }}"
    port: "{{ mcp_server_port }}"
  validation:
    - MCP 服务器白名单
  default:
    user: allow
    agent: allow (配置的服务器)
```

### 沙箱级别与权限对应

```yaml
# 无隔离 (none)
none:
  user: 本地开发，完全信任
  agent: 不推荐用于 Agent

# 基础隔离 (basic)
basic:
  user: 一般使用
  agent: 默认级别
  filesystem:
    allowed: ["{{ workspace }}/**"]
    denied: ["**/.git/**", "**/.env"]
  command:
    denied: ["rm -rf /", "mkfs", "dd"]
  network:
    enabled: true

# 严格隔离 (strict)
strict:
  user: 敏感操作
  agent: 处理敏感数据时
  filesystem:
    allowed: ["{{ workspace }}/**"]
    denied: ["**/.git/**", "**/node_modules/**", "**/.env*"]
    read_only: ["{{ workspace }}/.git"]
  command:
    allowed: ["git", "npm", "node", "python", "cargo"]
  network:
    allowed_hosts: ["api.anthropic.com", "api.openai.com"]
    max_connections: 5

# 完全隔离 (full)
full:
  user: 不可信代码测试
  agent: 不推荐（功能受限）
  filesystem:
    allowed: ["{{ workspace }}/sandbox/**"]
  command:
    allowed: []
  network:
    enabled: false
```

### 沙箱检查流程

```
┌─────────────────────────────────────────────────────────────┐
│                      模块调用 Sandbox                       │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. 确定操作类型                                             │
│    - 文件访问: check_file_access                            │
│    - 命令执行: check_command_access                         │
│    - 网络访问: check_network_access                         │
│    - 资源检查: get_resource_usage                           │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. 获取会话沙箱配置                                         │
│    - 沙箱级别: none/basic/strict/full                       │
│    - 工作区路径                                             │
│    - 允许/拒绝规则                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. 执行具体检查                                             │
│    - 文件: 路径匹配 + 操作类型检查                          │
│    - 命令: 白名单/黑名单 + 参数检查                         │
│    - 网络: 主机/端口 + 并发检查                             │
│    - 资源: 当前使用 vs 限制                                 │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
                    ┌─────┴─────┐
                    │ 允许？     │
                    └─────┬─────┘
                          │ 否              │ 是
                          ▼                 ▼
┌──────────────────────────────┐   ┌──────────────────────┐
│ 记录违规                     │   │ 允许操作             │
│ 执行违规动作 (log/warn/block)│   │ 监控资源使用         │
└──────────────────────────────┘   └──────────────────────┘
```

### 违规处理策略

```yaml
违规级别与响应:

  low (低):
    action: log
    description: 记录违规日志，允许操作继续
    适用于: 非关键资源轻微超限

  medium (中):
    action: warn
    description: 记录并警告用户/Agent
    适用于: 资源使用接近上限

  high (高):
    action: block
    description: 阻止操作并记录
    适用于: 访问敏感路径、危险命令

  critical (严重):
    action: terminate
    description: 终止会话/Agent
    适用于: 恶意行为检测、严重违规
```

### 沙箱与 Security Manager 协作

```yaml
# 权限检查 → 沙箱执行 流程

1. Security Manager 检查权限
   ├─ 用户 → 允许
   └─ Agent → 检查授权

2. Sandbox 执行隔离
   ├─ 路径检查
   ├─ 命令检查
   └─ 资源限制

3. 违规处理
   ├─ 记录到 Security Manager
   ├─ 触发威胁检测
   └─ 执行响应动作
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 1.1.0 | 2026-04-01 | 添加沙箱检查点文档，完善沙箱级别说明 |
