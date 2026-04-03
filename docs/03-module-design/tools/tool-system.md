# Tool System (工具框架)

## 概述

### 职责描述

Tool System 提供统一的工具调用框架，包括：

- 工具注册和发现
- 参数验证和类型检查
- 权限检查和沙箱执行
- 内置工具实现（Read/Write/Edit/Grep/Bash 等）
- MCP 工具适配

### 设计目标

1. **统一接口**: 所有工具使用相同的调用方式
2. **安全优先**: 严格的权限检查和沙箱隔离
3. **可扩展**: 支持自定义工具和 MCP 工具
4. **类型安全**: 参数验证和类型检查

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 路径权限检查。见 [Session Manager 接口](../core/session-manager.md) |
| Security Manager | 依赖 | 沙箱执行、权限验证。见 [Security Manager 接口](../security/security-manager.md) |
| MCP Client | 依赖 | MCP 工具集成。见 [MCP Client 接口](../services/mcp-client.md) |
| Hook Engine | 依赖 | 工具调用钩子。见 [Hook Engine 接口](../core/hook-engine.md) |

---

## 接口定义

### 对外接口

```yaml
# Tool System 接口定义
ToolSystem:
  # ========== 工具管理 ==========
  register_tool:
    description: 注册自定义工具
    inputs:
      tool:
        type: ToolDefinition
        required: true
    outputs:
      success:
        type: boolean

  unregister_tool:
    description: 注销工具
    inputs:
      name:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_tools:
    description: 列出可用工具
    inputs:
      category:
        type: string
        description: 工具类别过滤（省略时返回所有类别）
        required: false
    outputs:
      tools:
        type: array<ToolInfo>

  get_tool:
    description: 获取工具定义
    inputs:
      name:
        type: string
        required: true
    outputs:
      tool:
        type: ToolDefinition | null

  # ========== 工具执行 ==========
  execute:
    description: 执行工具
    inputs:
      name:
        type: string
        required: true
      args:
        type: object
        required: true
      context:
        type: ToolContext
        required: true
    outputs:
      result:
        type: ToolResult

  validate_args:
    description: 验证工具参数
    inputs:
      name:
        type: string
        required: true
      args:
        type: object
        required: true
    outputs:
      valid:
        type: boolean
      errors:
        type: array<string>

  # ========== 权限检查 ==========
  check_permission:
    description: |
      检查工具调用权限
      内部实现：调用 Security Manager.check_permission() 进行权限验证
      - 将 agent_id 映射到 principal
      - 将 tool_name 映射到 resource
      - 将 args 中的操作映射到 action
    inputs:
      agent_id:
        type: string
        required: true
      tool_name:
        type: string
        required: true
      args:
        type: object
        required: true
    outputs:
      allowed:
        type: boolean
      reason:
        type: string

  # ========== MCP 工具 ==========
  register_mcp_tools:
    description: |
      注册 MCP 服务器暴露的工具
      工具来源：由 MCP Client 通过 discover_tools 发现并调用此接口注册
      内部实现：MCP Client.discover_tools() → ToolSystem.register_mcp_tools()
      inputSchema 格式：遵循 MCP 协议规范，与 JSONSchema 格式兼容
    inputs:
      server_name:
        type: string
        description: MCP 服务器名称
        required: true
      tools:
        type: array<MCPToolDefinition>
        description: MCP 服务器暴露的工具列表
        required: true
    outputs:
      registered:
        type: integer
        description: 已注册的工具数量

  # ========== 工具分类 ==========
  get_categories:
    description: 获取工具类别列表
    outputs:
      categories:
        type: array<string>

  get_tools_by_category:
    description: 按类别获取工具
    inputs:
      category:
        type: string
        required: true
    outputs:
      tools:
        type: array<ToolInfo>
```

### 数据结构

```yaml
# 工具定义
ToolDefinition:
  name:
    type: string
    description: 工具名称（唯一标识）
  display_name:
    type: string
    description: 显示名称
  description:
    type: string
    description: 工具描述（用于 LLM）
  category:
    type: string
    description: 工具类别
  parameters:
    type: JSONSchema
    description: 参数定义（JSON Schema）
  handler:
    type: ToolHandler
    description: 工具处理器
  permissions:
    type: array<string>
    description: 所需权限
  dangerous:
    type: boolean
    description: 是否为危险操作
    default: false

# JSON Schema（简化）
JSONSchema:
  type:
    type: string
    description: object/array/string/number/boolean
  properties:
    type: map<string, JSONSchema>
    description: 属性定义
  required:
    type: array<string>
    description: 必需属性
  additionalProperties:
    type: boolean
    description: 允许额外属性

# 工具处理器
ToolHandler:
  type:
    type: enum
    values: [builtin, command, skill, mcp, wasm]
    description: |
      处理器类型：
      - builtin: 内置处理器（Tool System 直接实现）
      - command: 命令处理器（target 为要执行的命令字符串或脚本路径）
      - skill: Skill 处理器（target 为 skill_id）
      - mcp: MCP 工具（由 MCP Client 提供）
      - wasm: WebAssembly 处理器
  target:
    type: string
    description: |
      处理器目标，根据 type 不同含义不同：
      - builtin: 不使用
      - command: 要执行的命令字符串或脚本路径（支持模板变量，如 {{file_path}}）
      - skill: skill_id
      - mcp: MCP 服务器名称
      - wasm: WASM 模块路径
  timeout:
    type: integer
    description: 超时时间（秒）

# 工具信息
ToolInfo:
  name:
    type: string
  display_name:
    type: string
  description:
    type: string
  category:
    type: string
  parameters:
    type: JSONSchema
  dangerous:
    type: boolean

# 工具上下文
ToolContext:
  session_id:
    type: string
  agent_id:
    type: string
  workspace:
    type: string
  variables:
    type: map<string, any>

# 工具结果
ToolResult:
  success:
    type: boolean
  data:
    type: any
  error:
    type: string | null
  error_code:
    type: string | null
  duration_ms:
    type: integer
  metadata:
    type: map<string, any>

# MCP 工具定义
MCPToolDefinition:
  name:
    type: string
  description:
    type: string
  inputSchema:
    type: object
  server_name:
    type: string
```

### 配置选项

```yaml
# config/tools.yaml
tools:
  # 内置工具
  builtin:
    enabled:
      - read
      - write
      - edit
      - grep
      - glob
      - bash
      - git

  # 自定义工具
  custom:
    enabled: true
    directory: "./tools"

  # MCP 工具
  mcp:
    enabled: true
    auto_discover: true

  # 执行配置
  execution:
    default_timeout: 30
    max_output_size: 10485760    # 10MB

  # 权限配置
  permissions:
    default_deny: false
    # default_deny: false = 默认允许所有工具调用（白名单模式）
    # default_deny: true = 默认拒绝所有工具调用（黑名单模式）
    log_denied: true
    # log_denied: true = 记录被拒绝的工具调用到审计日志
```

**配置说明**:

| 配置项 | 说明 |
|--------|------|
| `builtin.enabled` | 启用的内置工具列表 |
| `custom.enabled` | 是否允许加载自定义工具 |
| `custom.directory` | 自定义工具目录路径 |
| `mcp.enabled` | 是否启用 MCP 工具集成 |
| `mcp.auto_discover` | 是否自动发现 MCP 服务器工具 |
| `execution.default_timeout` | 默认超时时间（秒） |
| `execution.max_output_size` | 最大输出大小（字节） |
| `permissions.default_deny` | true=黑名单模式（默认拒绝），false=白名单模式（默认允许） |
| `permissions.log_denied` | 是否记录拒绝的操作到审计日志 |

---

## 核心流程

### 工具执行流程

```
Agent 请求调用工具
        │
        ▼
┌──────────────────────────────┐
│ 1. 触发 tool_call hook       │
│    - before hook             │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 查找工具定义              │
│    - 工具存在性检查          │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 存在？  │
    └───┬────┘
        │ 否
        ▼
    返回工具不存在
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 参数验证                  │
│    - JSON Schema 验证        │
│    - 类型检查                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 有效？  │
    └───┬────┘
        │ 否
        ▼
    返回参数错误
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 权限检查                  │
│    - Agent 权限              │
│    - 路径权限（如适用）      │
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
│ 5. 执行工具                  │
│    - 调用处理器              │
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
│ 6. 错误处理      │   │ 6. 格式化    │
│    - 记录错误    │   │    结果      │
└──────────────────┘   └──────────────┘
        │                     │
        ▼                     ▼
┌──────────────────┐   ┌──────────────┐
│ 7. 触发          │   │ 7. 触发      │
│ tool_result hook │   │ tool_result  │
│ (error)          │   │ hook         │
└──────────────────┘   └──────────────┘
        │                     │
        ▼                     ▼
    返回错误            返回结果
```

### 参数验证流程

```
接收参数
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查必需参数              │
│    - 遍历 required 列表      │
│    - 确认所有参数存在        │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 完整？  │
    └───┬────┘
        │ 否
        ▼
    返回缺少必需参数
        │ 是
        ▼
┌──────────────────────────────┐
│ 2. 类型检查                  │
│    - 根据 schema 类型检查    │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 匹配？  │
    └───┬────┘
        │ 否
        ▼
    返回类型错误
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 约束检查                  │
│    - 枚举值检查              │
│    - 范围检查                │
│    - 格式检查                │
└──────────────────────────────┘
        │
        ▼
    验证通过
```

### 权限检查流程

```
工具权限请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查 Agent 权限           │
│    - 工具白名单              │
│    - 工具黑名单              │
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
┌──────────────────────────────┐
│ 2. 检查路径权限（如适用）    │
│    - 文件路径                │
│    - 会话 Workspace 检查     │
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
┌──────────────────────────────┐
│ 3. 检查危险操作              │
│    - 是否需要确认            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 危险？  │
    └───┬────┘
        │ 是
        ▼
    请求用户确认
        │ 否
        ▼
    允许执行
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│            Tool System                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Registry  │  │Validator │  │Executor││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Builtin   │  │Custom    │  │MCP       │
│Tools     │  │Tools     │  │Tools     │
└──────────┘  └──────────┘  └──────────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Session   │  │Security  │  │Hook      │
│Manager   │  │Manager   │  │Engine    │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
Agent Runtime
    │
    ▼
┌─────────────────────────────┐
│ Tool System                │
│ - 接收调用请求              │
│ - 查找工具                  │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ Validator       │         │ Permission      │
│ - 参数验证      │         │ Checker         │
└─────────────────┘         └─────────────────┘
        │                             │
        └────────────┬────────────────┘
                     ▼
        ┌─────────────────────────────┐
        │ Tool Executor               │
        │ - 调用具体工具              │
        └─────────────────────────────┘
                     │
                     ▼
              返回执行结果
```

---

## 内置工具

### 文件工具

```yaml
# Read 工具
read:
  name: read
  description: 读取文件内容
  parameters:
    type: object
    properties:
      file_path:
        type: string
        description: 文件路径（绝对路径或相对路径）
      offset:
        type: integer
        description: 起始行号
      limit:
        type: integer
        description: 读取行数
    required: [file_path]
  permissions: [file_read]
  dangerous: false

# Write 工具
write:
  name: write
  description: 写入文件（覆盖）
  parameters:
    type: object
    properties:
      file_path:
        type: string
      content:
        type: string
    required: [file_path, content]
  permissions: [file_write]
  dangerous: true

# Edit 工具
edit:
  name: edit
  description: 编辑文件（替换）
  parameters:
    type: object
    properties:
      file_path:
        type: string
      old_string:
        type: string
      new_string:
        type: string
    required: [file_path, old_string, new_string]
  permissions: [file_write]
  dangerous: true
```

### 搜索工具

```yaml
# Grep 工具
grep:
  name: grep
  description: 搜索文件内容
  parameters:
    type: object
    properties:
      pattern:
        type: string
        description: 搜索模式（正则表达式）
      path:
        type: string
        description: 搜索路径
      glob:
        type: string
        description: 文件模式过滤
    required: [pattern]
  permissions: [file_read]
  dangerous: false

# Glob 工具
glob:
  name: glob
  description: 查找文件
  parameters:
    type: object
    properties:
      pattern:
        type: string
        description: 文件模式（支持 **）
      path:
        type: string
        description: 搜索路径
    required: [pattern]
  permissions: [file_read]
  dangerous: false
```

### 命令工具

```yaml
# Bash 工具
bash:
  name: bash
  description: 执行 shell 命令
  parameters:
    type: object
    properties:
      command:
        type: string
        description: 要执行的命令
      timeout:
        type: integer
        description: 超时时间（秒）
    required: [command]
  permissions: [command_execute]
  dangerous: true
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/tools.yaml
tools:
  # 启用的内置工具
  builtin:
    enabled:
      - read
      - write
      - edit
      - grep
      - glob
      - bash

  # 自定义工具目录
  custom:
    enabled: true
    directory: "./tools"

  # MCP 工具
  mcp:
    enabled: true
    auto_discover: true

  # 执行配置
  execution:
    default_timeout: 30
    max_output_size: 10485760    # 10MB
    max_execution_time: 300     # 5 分钟

  # 权限配置
  permissions:
    default_deny: false
    log_denied: true
    log_all: true

  # 危险操作
  dangerous_operations:
    require_confirmation: true
    auto_confirm_on_ci: false
```

### 环境变量

```bash
# 工具目录
export KNIGHT_TOOLS_DIR="./tools"

# 执行限制
export KNIGHT_TOOL_DEFAULT_TIMEOUT=30
export KNIGHT_TOOL_MAX_OUTPUT_SIZE=10485760

# 权限
export KNIGHT_TOOL_DEFAULT_DENY=false
export KNIGHT_TOOL_LOG_DENIED=true
```

### 部署考虑

1. **安全性**: 默认拒绝策略，明确允许的工具
2. **资源限制**: 限制工具执行时间和输出大小
3. **审计**: 记录所有工具调用日志
4. **危险操作**: 对危险操作要求用户确认

---

## 示例

### 使用场景

#### 场景 1: 调用内置工具

```python
# 伪代码
result = tool_system.execute(
    name="read",
    args={
        "file_path": "/path/to/file.txt",
        "offset": 0,
        "limit": 100
    },
    context={
        "session_id": "abc123",
        "agent_id": "agent1",
        "workspace": "/project"
    }
)

if result.success:
    print(result.data)
else:
    print(f"Error: {result.error}")
```

#### 场景 2: 注册自定义工具

```yaml
# tools/my-tool.yaml
name: custom_search
display_name: 自定义搜索
description: 在项目中搜索代码
category: search
parameters:
  type: object
  properties:
    query:
      type: string
      description: 搜索关键词
    file_type:
      type: string
      description: 文件类型
  required: [query]
handler:
  type: command
  target: "./scripts/search.sh"
permissions: [file_read]
```

### 工具定义示例

#### JavaScript 格式化工具

```yaml
name: format_js
display_name: Format JavaScript
description: 格式化 JavaScript 文件
category: formatter
parameters:
  type: object
  properties:
    file_path:
      type: string
    options:
      type: object
      properties:
        semicolons:
          type: boolean
        quotes:
          type: string
          enum: [single, double]
  required: [file_path]
handler:
  type: command
  target: "prettier --write {{file_path}}"
permissions: [file_write]
dangerous: true
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 工具查找 | < 1ms | 内存查找 |
| 参数验证 | < 5ms | JSON Schema 验证 |
| 文件读取 | < 100ms | 小文件 |
| 命令执行 | < 30s | 默认超时 |

### 错误处理

ToolResult 中的 error_code 字段使用字符串格式的错误代码（如 `TOOL_NOT_FOUND`），与 error_codes 附录中的名称对应。

```yaml
error_codes:
  TOOL_NOT_FOUND:
    code: 404
    string_code: "TOOL_NOT_FOUND"
    message: "工具不存在"
    action: "检查工具名称"

  INVALID_ARGUMENTS:
    code: 400
    string_code: "INVALID_ARGUMENTS"
    message: "参数无效"
    action: "检查参数类型和值"

  PERMISSION_DENIED:
    code: 403
    string_code: "PERMISSION_DENIED"
    message: "权限不足"
    action: "联系管理员或更改权限"

  EXECUTION_TIMEOUT:
    code: 408
    string_code: "EXECUTION_TIMEOUT"
    message: "执行超时"
    action: "增加超时时间或优化操作"

  EXECUTION_FAILED:
    code: 500
    string_code: "EXECUTION_FAILED"
    message: "执行失败"
    action: "查看错误详情"
```

### 测试策略

```yaml
test_plan:
  unit_tests:
    - 工具注册/注销
    - 参数验证
    - 权限检查
    - 结果格式化

  integration_tests:
    - 内置工具执行
    - 自定义工具执行
    - MCP 工具调用
    - 错误处理

  security_tests:
    - 路径遍历攻击
    - 命令注入攻击
    - 权限绕过测试
    - 资源耗尽测试
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |

