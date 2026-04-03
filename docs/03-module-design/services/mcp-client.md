# MCP Client (MCP 客户端)

## 概述

### 职责描述

MCP Client 负责与 MCP (Model Context Protocol) 服务器通信，包括：

- MCP 服务器连接管理
- 工具发现和注册
- 工具调用和结果处理
- 资源访问
- 提示模板管理
- 服务器健康监控

### 设计目标

1. **协议兼容**: 完全兼容 MCP 协议规范
2. **自动发现**: 自动发现 MCP 服务器暴露的工具
3. **安全隔离**: MCP 工具权限可控
4. **高性能**: 连接复用和并发优化

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Tool System | 依赖 | 注册 MCP 工具。见 [Tool System 接口](../tools/tool-system.md) |
| Session Manager | 依赖 | 会话上下文。见 [Session Manager 接口](../core/session-manager.md) |
| Security Manager | 依赖 | 权限检查。见 [Security Manager 接口](../security/security-manager.md) |

---

## 接口定义

### 对外接口

```yaml
# MCP Client 接口定义
MCPClient:
  # ========== 服务器管理 ==========
  connect_server:
    description: 连接到 MCP 服务器
    inputs:
      server:
        type: MCPServerConfig
        required: true
    outputs:
      server_id:
        type: string

  disconnect_server:
    description: 断开服务器连接
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_servers:
    description: 列出所有服务器
    inputs:
      status:
        type: string
        description: 过滤状态 (connected/disconnected/all)
        required: false
        default: "all"
    outputs:
      servers:
        type: array<MCPServerInfo>

  get_server_info:
    description: 获取服务器信息
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      info:
        type: MCPServerInfo

  reconnect_server:
    description: 重连服务器
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 工具管理 ==========
  discover_tools:
    description: 发现服务器工具
    inputs:
      server_id:
        type: string
        required: true
      force_refresh:
        type: boolean
        description: 强制刷新缓存
        required: false
        default: false
    outputs:
      tools:
        type: array<MCPToolDefinition>

  register_tools:
    description: 注册服务器工具到 Tool System
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      registered_count:
        type: integer

  call_tool:
    description: 调用 MCP 工具
    inputs:
      server_id:
        type: string
        required: true
      tool_name:
        type: string
        required: true
      arguments:
        type: object
        required: true
      timeout:
        type: integer
        description: 超时时间（秒）
        required: false
        default: 30
    outputs:
      result:
        type: MCPToolResult

  list_tools:
    description: 列出服务器工具
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      tools:
        type: array<MCPToolInfo>

  # ========== 资源管理 ==========
  list_resources:
    description: 列出服务器资源
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      resources:
        type: array<MCPResource>

  read_resource:
    description: 读取资源内容
    inputs:
      server_id:
        type: string
        required: true
      uri:
        type: string
        required: true
    outputs:
      content:
        type: MCPResourceContent

  subscribe_resource:
    description: 订阅资源变更
    inputs:
      server_id:
        type: string
        required: true
      uri:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  unsubscribe_resource:
    description: 取消订阅资源
    inputs:
      server_id:
        type: string
        required: true
      uri:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 提示模板管理 ==========
  list_prompts:
    description: 列出服务器提示模板
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      prompts:
        type: array<MCPPrompt>

  get_prompt:
    description: 获取提示模板
    inputs:
      server_id:
        type: string
        required: true
      name:
        type: string
        required: true
      arguments:
        type: object
        required: false
    outputs:
      prompt:
        type: MCPPromptContent

  # ========== 权限管理 ==========
  set_tool_permissions:
    description: 设置工具权限
    inputs:
      server_id:
        type: string
        required: true
      permissions:
        type: MCPToolPermissions
        required: true
    outputs:
      success:
        type: boolean

  get_tool_permissions:
    description: 获取工具权限
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      permissions:
        type: MCPToolPermissions

  # ========== 健康检查 ==========
  health_check:
    description: 检查服务器健康状态
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      status:
        type: MCPHealthStatus

  ping:
    description: Ping 服务器
    inputs:
      server_id:
        type: string
        required: true
    outputs:
      latency_ms:
        type: integer
```

### 数据结构

```yaml
# MCP 服务器配置
MCPServerConfig:
  name:
    type: string
    description: 服务器名称
  enabled:
    type: boolean
    default: true

  # 启动配置
  command:
    type: string
    description: 启动命令
  args:
    type: array<string>
    description: 命令参数
  env:
    type: map<string, string>
    description: 环境变量

  # 连接配置
  transport:
    type: string
    enum: [stdio, sse, websocket]
    description: 传输方式
  url:
    type: string | null
    description: 服务器 URL（非 stdio）

  # 连接选项
  timeout:
    type: integer
    description: 连接超时（秒）
    default: 30
  max_retries:
    type: integer
    description: 最大重试次数
    default: 3
  retry_delay:
    type: integer
    description: 重试延迟（毫秒）
    default: 1000

# MCP 服务器信息
MCPServerInfo:
  id:
    type: string
  name:
    type: string
  status:
    type: enum
    values: [connected, disconnected, error]
  protocol_version:
    type: string
  capabilities:
    type: MCPCapabilities
  tools_count:
    type: integer
  resources_count:
    type: integer
  prompts_count:
    type: integer
  connected_at:
    type: datetime | null
  last_error:
    type: string | null

# MCP 能力
MCPCapabilities:
  tools:
    type: boolean | object
  resources:
    type: boolean | object
  prompts:
    type: boolean | object

# MCP 工具定义
MCPToolDefinition:
  name:
    type: string
  description:
    type: string
  inputSchema:
    type: object
    description: JSON Schema 参数定义
  server_id:
    type: string

# MCP 工具信息
MCPToolInfo:
  name:
    type: string
  description:
    type: string
  server_id:
    type: string
  registered:
    type: boolean
    call_count:
    type: integer
  last_called:
    type: datetime | null

# MCP 工具结果
MCPToolResult:
  success:
    type: boolean
  content:
    type: array<MCPContentBlock>
  error:
    type: MCPError | null
  isError:
    type: boolean
  duration_ms:
    type: integer

# MCP 内容块
MCPContentBlock:
  type:
    type: string
    enum: [text, image, resource]
  text:
    type: string | null
  data:
    type: string | null
    description: Base64 编码数据
  mime_type:
    type: string | null

# MCP 错误
MCPError:
  code:
    type: integer
  message:
    type: string
  details:
    type: object | null

# MCP 资源
MCPResource:
  uri:
    type: string
  name:
    type: string
  description:
    type: string
  mimeType:
    type: string | null

# MCP 资源内容
MCPResourceContent:
  contents:
    type: array<MCPResourceContentItem>
  mime_type:
    type: string

MCPResourceContentItem:
  uri:
    type: string
  text:
    type: string | null
  blob:
    type: string | null

# MCP 提示
MCPPrompt:
  name:
    type: string
  description:
    type: string
  arguments:
    type: array<MCPArgument>

# MCP 参数
MCPArgument:
  name:
    type: string
  description:
    type: string
  required:
    type: boolean

# MCP 提示内容
MCPPromptContent:
  messages:
    type: array<MCPMessage>
  description:
    type: string | null

MCPMessage:
  role:
    type: string
  content:
    type: MCPContentBlock

# MCP 工具权限
MCPToolPermissions:
  allow_all:
    type: boolean
    default: false
  allowed_tools:
    type: array<string>
    description: 允许的工具列表
  denied_tools:
    type: array<string>
    description: 拒绝的工具列表

# MCP 健康状态
MCPHealthStatus:
  healthy:
    type: boolean
  latency_ms:
    type: integer
  last_check:
    type: datetime
  error:
    type: string | null
```

### 配置选项

```yaml
# config/mcp.yaml
mcp:
  # 服务器配置
  servers:
    - name: filesystem
      enabled: true
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
      transport: stdio

    - name: brave-search
      enabled: true
      command: npx
      args: ["-y", "@modelcontextprotocol/server-brave-search"]
      transport: stdio

    - name: github
      enabled: false
      command: npx
      args: ["-y", "@modelcontextprotocol/server-github"]
      transport: stdio

  # 连接配置
  connection:
    timeout: 30
    max_retries: 3
    retry_delay: 1000

  # 工具发现
  discovery:
    auto_discover: true
    cache_ttl: 300
    refresh_interval: 600

  # 权限配置
  permissions:
    default_policy: allow
    log_denied: true
```

---

## 核心流程

### 服务器连接流程

```
连接请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 启动 MCP 服务器进程       │
│    - 执行启动命令            │
│    - 设置环境变量            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 建立通信通道              │
│    - stdio: stdin/stdout     │
│    - sse: HTTP 连接          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 发送初始化请求            │
│    - 协议版本协商            │
│    - 能力交换                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否
        ▼
    记录错误，重试或放弃
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 标记为已连接              │
│    - 启动心跳检测            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 工具调用流程

```
工具调用请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 权限检查                  │
│    - 检查工具是否允许        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 允许？  │
    └───┬────┘
        │ 否
        ▼
    返回权限拒绝
        │ 是
        ▼
┌──────────────────────────────┐
│ 2. 验证参数                  │
│    - JSON Schema 验证        │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 发送工具调用请求          │
│    - 构建 MCP 请求           │
│    - 发送到服务器            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 等待响应                  │
│    - 超时控制                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 5. 错误处理      │   │ 5. 解析结果  │
│    - 记录错误    │   │    转换格式  │
└──────────────────┘   └──────────────┘
        │                     │
        ▼                     ▼
    返回错误            返回结果
```

### 工具发现和注册

```
服务器连接成功
        │
        ▼
┌──────────────────────────────┐
│ 1. 请求工具列表              │
│    - tools/list 请求         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 解析工具定义              │
│    - 提取工具名              │
│    - 提取参数 Schema         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 转换为内部格式            │
│    - MCP 工具 → 内部工具     │
│    - 添加处理器              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 注册到 Tool System        │
│    - 前缀: server_name.tool  │
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
│             MCP Client                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Connector │  │Discovery │  │Adapter  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Tool      │  │Session   │  │Security  │
│System    │  │Manager   │  │Manager   │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
Tool System
    │
    ▼
┌─────────────────────────────┐
│ MCP Client                  │
│ - 接收工具调用              │
│ - 转换为 MCP 请求           │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ MCP Server                  │
│ - 处理请求                  │
│ - 返回结果                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ MCP Client                  │
│ - 解析响应                  │
│ - 转换为内部格式            │
└─────────────────────────────┘
        │
        ▼
    Tool System
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/mcp.yaml
mcp:
  # 服务器配置
  servers:
    - name: filesystem
      enabled: true
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
      transport: stdio

    - name: brave-search
      enabled: true
      command: npx
      args: ["-y", "@modelcontextprotocol/server-brave-search"]
      transport: stdio

    - name: github
      enabled: false
      command: npx
      args: ["-y", "@modelcontextprotocol/server-github"]
      transport: stdio
      env:
        GITHUB_TOKEN: ${GITHUB_TOKEN}

  # 连接配置
  connection:
    timeout: 30
    max_retries: 3
    retry_delay: 1000

  # 工具发现
  discovery:
    auto_discover: true
    cache_ttl: 300
    refresh_interval: 600

  # 权限配置
  permissions:
    default_policy: allow
    log_denied: true
```

### 环境变量

```bash
# MCP 服务器
export MCP_FILESYSTEM_PATH="."
export MCP_BRAVE_SEARCH_API_KEY="your-key"
export MCP_GITHUB_TOKEN="your-token"

# 连接配置
export KNIGHT_MCP_TIMEOUT=30
export KNIGHT_MCP_MAX_RETRIES=3
```

---

## 示例

### 工具调用示例

```python
# 调用 filesystem 服务器的 read 工具
result = mcp_client.call_tool(
    server_id="filesystem",
    tool_name="read",
    arguments={
        "path": "/path/to/file.txt"
    }
)

# 调用 brave-search 服务器的搜索工具
result = mcp_client.call_tool(
    server_id="brave-search",
    tool_name="brave_web_search",
    arguments={
        "query": "Rust programming",
        "count": 5
    }
)
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 服务器连接 | < 2s | 启动+初始化 |
| 工具调用 | < 1s | 简单工具 |
| 工具发现 | < 500ms | 单次请求 |

### 错误处理

```yaml
error_codes:
  SERVER_NOT_FOUND:
    code: 404
    message: "MCP 服务器不存在"
    action: "检查服务器配置"

  CONNECTION_FAILED:
    code: 503
    message: "连接服务器失败"
    action: "检查服务器是否运行"

  TOOL_NOT_FOUND:
    code: 404
    message: "工具不存在"
    action: "检查工具名称"

  CALL_TIMEOUT:
    code: 408
    message: "工具调用超时"
    action: "增加超时时间"

  PERMISSION_DENIED:
    code: 403
    message: "工具调用被拒绝"
    action: "检查权限配置"
```

### 支持的 MCP 服务器

| 服务器 | 工具 | 描述 |
|--------|------|------|
| @modelcontextprotocol/server-filesystem | read, write, search | 文件系统访问 |
| @modelcontextprotocol/server-brave-search | brave_web_search | 网页搜索 |
| @modelcontextprotocol/server-github | create_issue, search_issues | GitHub 操作 |
| @modelcontextprotocol/server-postgres | query, execute | PostgreSQL 数据库 |
| @modelcontextprotocol/server-puppeteer | navigate, screenshot, click | 浏览器自动化 |
