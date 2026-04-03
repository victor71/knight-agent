# Security Manager (安全管理器)

## 概述

### 职责描述

Security Manager 负责系统的安全管理，包括：

- 权限控制和管理
- 访问控制列表 (ACL)
- 审计日志
- 密钥和凭证管理
- 安全策略执行
- 威胁检测和防护

### 设计目标

1. **最小权限**: 默认拒绝，显式允许
2. **审计追踪**: 完整的安全事件日志
3. **灵活策略**: 支持细粒度权限控制
4. **安全存储**: 密钥加密存储

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 会话上下文 |
| Storage Service | 依赖 | 存储审计日志 |

---

## 接口定义

### 对外接口

```yaml
# Security Manager 接口定义
SecurityManager:
  # ========== 权限管理 ==========
  grant_permission:
    description: 授予权限
    inputs:
      grant:
        type: PermissionGrant
        required: true
    outputs:
      success:
        type: boolean

  revoke_permission:
    description: 撤销权限
    inputs:
      principal:
        type: string
        description: 主体（Agent/Session/User）
        required: true
      resource:
        type: string
        description: 资源
        required: true
      action:
        type: string
        description: 操作
        required: true
    outputs:
      success:
        type: boolean

  check_permission:
    description: 检查权限
    inputs:
      principal:
        type: string
        required: true
      resource:
        type: string
        required: true
      action:
        type: string
        required: true
      context:
        type: SecurityContext
        required: false
    outputs:
      allowed:
        type: boolean
      reason:
        type: string | null

  list_permissions:
    description: 列出权限
    inputs:
      principal:
        type: string
        required: true
    outputs:
      permissions:
        type: array<Permission>

  # ========== 策略管理 ==========
  create_policy:
    description: 创建安全策略
    inputs:
      policy:
        type: SecurityPolicy
        required: true
    outputs:
      policy_id:
        type: string

  update_policy:
    description: 更新安全策略
    inputs:
      policy_id:
        type: string
        required: true
      policy:
        type: SecurityPolicy
        required: true
    outputs:
      success:
        type: boolean

  delete_policy:
    description: 删除安全策略
    inputs:
      policy_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  get_policy:
    description: 获取策略详情
    inputs:
      policy_id:
        type: string
        required: true
    outputs:
      policy:
        type: SecurityPolicy | null

  list_policies:
    description: 列出策略
    inputs:
      type:
        type: string
        description: 策略类型过滤
        required: false
    outputs:
      policies:
        type: array<SecurityPolicy>

  evaluate_policy:
    description: 评估策略
    inputs:
      policy_id:
        type: string
        required: true
      context:
        type: SecurityContext
        required: true
    outputs:
      result:
        type: PolicyEvaluationResult

  # ========== 审计日志 ==========
  log_event:
    description: 记录安全事件
    inputs:
      event:
        type: SecurityEvent
        required: true
    outputs:
      event_id:
        type: string

  query_logs:
    description: 查询审计日志
    inputs:
      query:
        type: LogQuery
        required: true
    outputs:
      events:
        type: array<SecurityEvent>

  get_log_summary:
    description: 获取日志摘要
    inputs:
      time_range:
        type: TimeRange
        required: false
    outputs:
      summary:
        type: LogSummary

  # ========== 密钥管理 ==========
  store_secret:
    description: 存储密钥
    inputs:
      key:
        type: string
        required: true
      value:
        type: string
        required: true
      metadata:
        type: map<string, any>
        required: false
    outputs:
      success:
        type: boolean

  get_secret:
    description: 获取密钥
    inputs:
      key:
        type: string
        required: true
    outputs:
      value:
        type: string | null

  delete_secret:
    description: 删除密钥
    inputs:
      key:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_secrets:
    description: 列出密钥（不含值）
    outputs:
      keys:
        type: array<SecretInfo>

  rotate_secret:
    description: 轮换密钥
    inputs:
      key:
        type: string
        required: true
      new_value:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 威胁检测 ==========
  analyze_threats:
    description: 分析威胁
    inputs:
      time_range:
        type: TimeRange
        required: false
    outputs:
      threats:
        type: array<ThreatInfo>

  is_suspicious:
    description: 检查活动是否可疑
    inputs:
      activity:
        type: Activity
        required: true
    outputs:
      suspicious:
        type: boolean
      confidence:
        type: float
      reasons:
        type: array<string>

  # ========== 安全配置 ==========
  get_security_config:
    description: 获取安全配置
    outputs:
      config:
        type: SecurityConfig

  update_security_config:
    description: 更新安全配置
    inputs:
      config:
        type: SecurityConfig
        required: true
    outputs:
      success:
        type: boolean
```

### 数据结构

```yaml
# 权限授予
PermissionGrant:
  principal:
    type: string
    description: 主体（Agent/Session/User ID）
  resource:
    type: string
    description: 资源（支持通配符）
  actions:
    type: array<string>
    description: 允许的操作
  conditions:
    type: array<Condition>
    description: 权限条件
  expires_at:
    type: datetime | null
    description: 过期时间

# 权限
Permission:
  id:
    type: string
  principal:
    type: string
  resource:
    type: string
  actions:
    type: array<string>
  conditions:
    type: array<Condition>
  granted_at:
    type: datetime
  expires_at:
    type: datetime | null

# 条件
Condition:
  type:
    type: string
    enum: [time, ip, workspace, custom]
  operator:
    type: string
    enum: [equals, contains, matches, in_range]
  value:
    type: any

# 安全策略
SecurityPolicy:
  id:
    type: string
  name:
    type: string
  description:
    type: string
  type:
    type: string
    enum: [rbac, abac, custom]
  enabled:
    type: boolean
  rules:
    type: array<PolicyRule>
  priority:
    type: integer
    description: 策略优先级

# 策略规则
PolicyRule:
  name:
    type: string
  effect:
    type: string
    enum: [allow, deny]
  principal:
    type: string | null
  resource:
    type: string | null
  action:
    type: string | null
  conditions:
    type: array<Condition>

# 安全上下文
SecurityContext:
  principal:
    type: string
  session_id:
    type: string | null
  agent_id:
    type: string | null
  workspace:
    type: string | null
  ip_address:
    type: string | null
  timestamp:
    type: datetime
  metadata:
    type: map<string, any>

# 策略评估结果
PolicyEvaluationResult:
  allowed:
    type: boolean
  matched_policy:
    type: string | null
  matched_rule:
    type: string | null
  reason:
    type: string

# 安全事件
SecurityEvent:
  id:
    type: string
  timestamp:
    type: datetime
  event_type:
    type: string
    enum: [access_request, access_denied, permission_granted, permission_revoked, policy_violation, threat_detected]
  principal:
    type: string
  resource:
    type: string | null
  action:
    type: string | null
  result:
    type: string
    enum: [allowed, denied]
  reason:
    type: string | null
  details:
    type: map<string, any>

# 日志查询
LogQuery:
  time_range:
    type: TimeRange
  event_types:
    type: array<string> | null
  principal:
    type: string | null
  resource:
    type: string | null
  limit:
    type: integer | null
  offset:
    type: integer | null

# 时间范围
TimeRange:
  start:
    type: datetime
  end:
    type: datetime | null

# 日志摘要
LogSummary:
  total_events:
    type: integer
  by_event_type:
    type: map<string, integer>
  by_principal:
    type: map<string, integer>
  denied_count:
    type: integer
  threat_count:
    type: integer

# 密钥信息
SecretInfo:
  key:
    type: string
  created_at:
    type: datetime
  updated_at:
    type: datetime
  metadata:
    type: map<string, any>

# 活动
Activity:
  principal:
    type: string
  action:
    type: string
  resource:
    type: string
  context:
    type: SecurityContext

# 威胁信息
ThreatInfo:
  threat_type:
    type: string
  confidence:
    type: float
  description:
    type: string
  affected_principals:
    type: array<string>
  detected_at:
    type: datetime
  recommendations:
    type: array<string>

# 安全配置
SecurityConfig:
  # 默认策略
  default_policy:
    type: string
    enum: [allow, deny]
    default: deny

  # 审计配置
  audit:
    log_all_events:
      type: boolean
      default: true
    log_denied:
      type: boolean
      default: true
    retention_days:
      type: integer
      default: 90

  # 威胁检测
  threat_detection:
    enabled:
      type: boolean
      default: true
    sensitivity:
      type: string
      enum: [low, medium, high]
      default: medium

  # 密钥策略
  secret_policy:
    rotation_days:
      type: integer
      default: 90
    min_length:
      type: integer
      default: 16
    require_special_chars:
      type: boolean
      default: true
```

### 配置选项

```yaml
# config/security.yaml
security:
  # 默认策略
  default_policy: deny

  # 审计配置
  audit:
    log_all_events: true
    log_denied: true
    retention_days: 90

  # 威胁检测
  threat_detection:
    enabled: true
    sensitivity: medium
    auto_block: false

  # 密钥策略
  secret_policy:
    rotation_days: 90
    min_length: 16
    require_special_chars: true

  # 权限缓存
  cache:
    enabled: true
    ttl: 300
```

---

## 核心流程

### 权限检查流程

```
权限请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 构建安全上下文            │
│    - 主体信息                │
│    - 会话信息                │
│    - 环境信息                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 查询权限策略              │
│    - RBAC 策略               │
│    - ABAC 策略               │
│    - 自定义策略              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 评估策略                  │
│    - 按优先级排序            │
│    - 匹配规则                │
│    - 应用条件                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 允许？  │
    └───┬────┘
        │ 否
        ▼
┌──────────────────────────────┐
│ 4. 拒绝访问                  │
│    - 记录审计日志            │
│    - 返回拒绝原因            │
└──────────────────────────────┘
        │ 是
        ▼
┌──────────────────────────────┐
│ 5. 允许访问                  │
│    - 记录审计日志            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 威胁检测流程

```
活动发生
        │
        ▼
┌──────────────────────────────┐
│ 1. 收集活动特征              │
│    - 行为模式                │
│    - 访问频率                │
│    - 资源访问                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 应用检测规则              │
│    - 异常访问                │
│    - 权限提升                │
│    - 批量操作                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 可疑？  │
    └───┬────┘
        │ 否
        ▼
    正常活动
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 计算威胁分数              │
│    - 综合多个规则            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 触发响应                  │
│    - 记录威胁                │
│    - 发送告警                │
│    - 自动阻止（可选）        │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 审计日志流程

```
安全事件
        │
        ▼
┌──────────────────────────────┐
│ 1. 格式化事件                │
│    - 添加时间戳              │
│    - 添加上下文              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 写入日志                  │
│    - 同步/异步写入           │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 更新索引                  │
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
│          Security Manager               │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Policy    │  │Audit     │  │Threat  ││
│  │Engine    │  │Logger    │  │Detector││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Session   │  │Storage   │  │All       │
│Manager   │  │Service   │  │Modules   │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
各模块
    │
    ▼
┌─────────────────────────────┐
│ Security Manager            │
│ - 权限检查                  │
│ - 策略评估                  │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ Allow/Deny      │         │ Storage Service │
│                 │         │ - 审计日志      │
└─────────────────┘         └─────────────────┘
```

### 安全执行机制

Knight-Agent 使用 **Hook 系统**作为主要的安全执行机制。安全检查通过 `before` Hook 在操作执行前进行拦截和验证。

**Hook 集成架构**：

```
操作请求（如 tool_call, file_access）
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│  Hook Engine - before hooks (priority: 1 → N)              │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐              │
│  │ Hook 1 │→│ Hook 2 │→│ Hook N │→│ 检查阻断│              │
│  └────────┘ └────────┘ └────────┘ │        │              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Security Hooks (priority: 100)                        │ │
│  │ - tool_call_before → SecurityManager.check_permission │ │
│  │ - file_access_before → SecurityManager.check_path      │ │
│  │ - command_execute_before → SecurityManager.check_cmd   │ │
│  └────────────────────────────────────────────────────────┘ │
│                                  └───┬────┘              │
│                                      │                     │
│                           ┌──────────┴──────────┐          │
│                           │ no block          │          │
│                           ▼                   │          │
│                    ┌────────────────┐         │          │
│                    │  执行原始操作   │         │          │
│                    └────────────────┘         │          │
└─────────────────────────────────────────────────────────────┘
```

**关键安全 Hook 事件**：

| Hook 事件 | 触发时机 | 安全检查 |
|-----------|----------|----------|
| `tool_call.before` | 工具调用前 | `SecurityManager.check_permission(tool, action)` |
| `file_access.before` | 文件访问前 | `SecurityManager.check_path(path, operation)` |
| `command_execute.before` | 命令执行前 | `SecurityManager.check_command(command)` |
| `llm_request.before` | LLM 请求前 | `SecurityManager.check_llm_access(model)` |

**Hook 配置示例**：

```yaml
# config/hooks.yaml
hooks:
  - name: security_tool_call
    event: tool_call
    phase: before
    priority: 100
    handler:
      type: rpc
      target: "security_manager"
      method: "check_tool_permission"
    control:
      can_block: true
      continue_on_error: false  # 安全检查失败时阻止操作

  - name: security_file_access
    event: file_access
    phase: before
    priority: 100
    handler:
      type: rpc
      target: "security_manager"
      method: "check_file_permission"
    control:
      can_block: true
      continue_on_error: false
```

**设计原则**：
- **默认拒绝**: 所有操作必须通过安全检查才能执行
- **不可绕过**: 安全 Hook 设置 `continue_on_error: false`，失败时阻止操作
- **审计追踪**: 所有安全检查结果记录到审计日志
- **优先级最高**: 安全 Hook 使用 `priority: 100`，确保最先执行

---

## 配置与部署

### 配置文件格式

```yaml
# config/security.yaml
security:
  # 默认策略
  default_policy: deny

  # 审计配置
  audit:
    log_all_events: true
    log_denied: true
    retention_days: 90

  # 威胁检测
  threat_detection:
    enabled: true
    sensitivity: medium
    auto_block: false

  # 密钥策略
  secret_policy:
    rotation_days: 90
    min_length: 16
    require_special_chars: true

  # 权限缓存
  cache:
    enabled: true
    ttl: 300
```

### 环境变量

```bash
# 安全配置
export KNIGHT_SECURITY_DEFAULT_POLICY="deny"
export KNIGHT_SECURITY_AUDIT_RETENTION_DAYS=90

# 威胁检测
export KNIGHT_SECURITY_THREAT_DETECTION_ENABLED="true"
export KNIGHT_SECURITY_THREAT_SENSITIVITY="medium"
```

---

## 示例

### 权限配置

```yaml
# 允许 Agent 访问特定目录
permissions:
  - principal: "agent:code-reviewer"
    resource: "file:/project/**"
    actions: [read]
    conditions:
      - type: workspace
        operator: equals
        value: "/project"

# 拒绝访问敏感文件
permissions:
  - principal: "*"
    resource: "file:**/.env"
    actions: [read, write]
    effect: deny
```

### 安全策略

```yaml
# 策略: 工作时间访问
policies:
  - name: business-hours-only
    type: abac
    rules:
      - effect: allow
        actions: ["*"]
        conditions:
          - type: time
            operator: in_range
            value: ["09:00", "18:00"]
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 权限检查 | < 1ms | 内存操作 |
| 策略评估 | < 10ms | 复杂策略 |
| 日志写入 | < 5ms | 异步写入 |

### 错误处理

```yaml
error_codes:
  PERMISSION_DENIED:
    code: 403
    message: "权限不足"
    action: "联系管理员"

  POLICY_VIOLATION:
    code: 403
    message: "违反安全策略"
    action: "查看策略详情"

  SECRET_NOT_FOUND:
    code: 404
    message: "密钥不存在"
    action: "检查密钥名称"

  THREAT_DETECTED:
    code: 403
    message: "检测到威胁活动"
    action: "联系安全团队"
```

### 安全最佳实践

1. **最小权限**: 默认拒绝，显式允许
2. **定期审计**: 定期检查权限和日志
3. **密钥轮换**: 定期更换敏感密钥
4. **威胁监控**: 启用威胁检测和告警
5. **日志保留**: 保留足够长时间的审计日志

---

## 权限模型

### 用户中心权限架构

Knight-Agent 运行在用户侧，采用用户中心的权限模型：

```yaml
权限层次:
  用户:
    描述: 系统所有者，拥有最高权限
    权限:
      - 完全访问系统所有资源
      - 配置 Agent 权限
      - 修改安全策略
      - 查看所有审计日志

  Agent:
    描述: 受限的 AI 助手，默认仅有最小权限
    默认策略: deny（默认拒绝）
    权限来源:
      - 用户显式授予
      - 会话级别配置
      - 安全策略允许
    权限范围:
      - 可配置的文件访问
      - 可配置的工具使用
      - 可配置的网络访问
```

### 权限配置示例

```yaml
# 用户给 Agent 配置较大权限
agent_permissions:
  agent: code-reviewer
  grants:
    - resource: "file:/project/**"
      actions: [read, write]
    - resource: "tool:git"
      actions: [execute]
    - resource: "network:api.anthropic.com"
      actions: [access]
  conditions:
    - type: workspace
      operator: equals
      value: "/project"

# 严格限制的 Agent
agent_permissions:
  agent: untrusted-helper
  grants:
    - resource: "file:/project/sandbox/**"
      actions: [read]
  sandbox_level: strict
```

---

## 安全检查点

### 检查点调用总览

以下列出了所有模块调用 Security Manager 的检查点：

| 模块 | 检查点 | 触发时机 | 检查内容 |
|------|--------|----------|----------|
| **Bootstrap** | `bootstrap:start` | 系统启动 | 验证启动权限 |
| **Session Manager** | `session:create` | 创建会话 | 验证会话创建权限 |
| | `session:access` | 访问会话 | 验证会话访问权限 |
| | `session:compress` | 触发压缩 | 验证压缩操作权限 |
| | `session:terminate` | 终止会话 | 验证会话终止权限 |
| **Agent Runtime** | `agent:create` | 创建 Agent | 验证 Agent 创建权限 |
| | `agent:invoke` | 调用 Agent | 验证 Agent 调用权限 |
| | `agent:send_message` | 发送消息到 LLM | 验证 LLM 访问权限 |
| | `agent:use_tool` | Agent 使用工具 | 验证工具使用权限 |
| **Orchestrator** | `orchestrate:task` | 编排任务 | 验证任务编排权限 |
| | `agent:collaborate` | Agent 协作 | 验证跨 Agent 调用权限 |
| **Skill Engine** | `skill:register` | 注册技能 | 验证技能注册权限 |
| | `skill:execute` | 执行技能 | 验证技能执行权限 |
| | `skill:trigger` | 触发技能 | 验证技能触发权限 |
| **Hook Engine** | `hook:register` | 注册 Hook | 验证 Hook 注册权限 |
| | `hook:execute` | 执行 Hook | 验证 Hook 执行权限 |
| | `hook:modify_prompt` | 修改 Prompt | 验证 Prompt 修改权限 |
| **Event Loop** | `event:emit` | 发送事件 | 验证事件发送权限 |
| | `event:subscribe` | 订阅事件 | 验证事件订阅权限 |
| **Task Manager** | `task:create` | 创建任务 | 验证任务创建权限 |
| | `task:execute` | 执行任务 | 验证任务执行权限 |
| | `task:cancel` | 取消任务 | 验证任务取消权限 |
| **Tool System** | `tool:register` | 注册工具 | 验证工具注册权限 |
| | `tool:execute` | 执行工具 | 验证工具执行权限 |
| | `tool:bash` | 执行 Bash | 验证命令执行权限 |
| | `tool:file_read` | 读取文件 | 验证文件读权限 |
| | `tool:file_write` | 写入文件 | 验证文件写权限 |
| **LLM Provider** | `llm:configure` | 配置 LLM | 验证 LLM 配置权限 |
| | `llm:call` | 调用 LLM API | 验证 API 调用权限 |
| | `llm:use_key` | 使用 API Key | 验证密钥使用权限 |
| **MCP Client** | `mcp:connect` | 连接 MCP 服务器 | 验证 MCP 连接权限 |
| | `mcp:use_tool` | 使用 MCP 工具 | 验证 MCP 工具使用权限 |
| **Storage Service** | `storage:write` | 写入存储 | 验证存储写入权限 |
| | `storage:read` | 读取存储 | 验证存储读取权限 |
| | `storage:delete` | 删除数据 | 验证数据删除权限 |
| **Context Compressor** | `compress:context` | 压缩上下文 | 验证上下文压缩权限 |
| **Timer System** | `timer:create` | 创建定时器 | 验证定时器创建权限 |
| | `timer:cancel` | 取消定时器 | 验证定时器取消权限 |
| **Logging System** | `log:write` | 写入日志 | 验证日志写入权限 |
| | `log:read` | 读取日志 | 验证日志读取权限 |
| **Sandbox** | `sandbox:create` | 创建沙箱 | 验证沙箱创建权限 |
| | `sandbox:bypass` | 绕过沙箱 | 验证沙箱绕过权限（高风险操作，仅管理员可执行） |

**注意**: `sandbox:bypass` 权限允许完全绕过沙箱限制，包括文件访问、命令执行、网络访问和资源限制。此权限仅授予需要完全系统访问权限的可信组件（如系统管理员）。

### 详细检查点说明

#### Bootstrap 检查点

```yaml
bootstrap:start:
  description: 系统启动时验证权限
  caller: Bootstrap
  check:
    principal: system
    resource: "system:bootstrap"
    action: "start"
  context:
    - 检查配置文件访问权限
    - 检查必要目录创建权限
    - 验证端口绑定权限
  default:
    user: allow
    agent: deny
```

#### Session Manager 检查点

```yaml
session:create:
  description: 创建新会话时验证权限
  caller: Session Manager
  check:
    resource: "session:create"
    action: "create"
  context:
    - 用户可直接创建
    - Agent 创建需要显式授权
  default:
    user: allow
    agent: deny

session:access:
  description: 访问会话时验证权限
  caller: Session Manager
  check:
    principal: agent_id
    resource: "session:{{session_id}}"
    action: "access"
  context:
    - Agent 只能访问自己的会话
    - 用户可访问所有会话
  default:
    user: allow
    agent: allow (仅自己)

session:compress:
  description: 触发上下文压缩时验证权限
  caller: Session Manager
  check:
    resource: "session:{{session_id}}:compress"
    action: "compress"
  default:
    user: allow
    agent: deny (自动)

session:terminate:
  description: 终止会话时验证权限
  caller: Session Manager
  check:
    resource: "session:{{session_id}}"
    action: "terminate"
  default:
    user: allow
    agent: allow (仅自己的会话)
```

#### Agent Runtime 检查点

```yaml
agent:create:
  description: 创建 Agent 实例时验证权限
  caller: Agent Runtime
  check:
    resource: "agent:{{agent_type}}"
    action: "create"
  default:
    user: allow
    agent: deny

agent:invoke:
  description: 调用 Agent 时验证权限
  caller: Agent Runtime
  check:
    principal: caller_id
    resource: "agent:{{agent_id}}"
    action: "invoke"
  default:
    user: allow
    agent: deny (除非显式授权)

agent:send_message:
  description: 发送消息到 LLM 时验证权限
  caller: Agent Runtime
  check:
    principal: agent_id
    resource: "llm:{{model_id}}"
    action: "call"
  default:
    user: allow
    agent: allow (需要配置)

agent:use_tool:
  description: Agent 使用工具时验证权限
  caller: Agent Runtime → Tool System
  check:
    principal: agent_id
    resource: "tool:{{tool_name}}"
    action: "use"
  default:
    user: allow
    agent: deny (默认，需显式授权)
```

#### Tool System 检查点

```yaml
tool:execute:
  description: 执行任何工具前验证权限
  caller: Tool System
  check:
    principal: caller_id
    resource: "tool:{{tool_name}}"
    action: "execute"
  default:
    user: allow
    agent: deny (默认)

tool:bash:
  description: 执行 Bash 命令时验证权限
  caller: Tool System
  check:
    principal: caller_id
    resource: "tool:bash:{{command}}"
    action: "execute"
  context:
    - 命令白名单检查
    - 危险命令拦截
  default:
    user: allow
    agent: deny (危险命令)

tool:file_read:
  description: 读取文件时验证权限
  caller: Tool System
  check:
    principal: caller_id
    resource: "file:{{path}}"
    action: "read"
  default:
    user: allow
    agent: deny (敏感路径)

tool:file_write:
  description: 写入文件时验证权限
  caller: Tool System
  check:
    principal: caller_id
    resource: "file:{{path}}"
    action: "write"
  default:
    user: allow
    agent: deny (敏感路径)
```

#### Skill Engine 检查点

```yaml
skill:execute:
  description: 执行技能时验证权限
  caller: Skill Engine
  check:
    principal: caller_id
    resource: "skill:{{skill_id}}"
    action: "execute"
  default:
    user: allow
    agent: allow (如果技能允许)

skill:trigger:
  description: 技能被触发时验证权限
  caller: Skill Engine
  check:
    resource: "skill:{{skill_id}}"
    action: "trigger"
  default:
    user: allow
    agent: allow (自动触发)
```

#### LLM Provider 检查点

```yaml
llm:call:
  description: 调用 LLM API 时验证权限
  caller: LLM Provider
  check:
    principal: caller_id
    resource: "llm:{{provider}}:{{model}}"
    action: "call"
  default:
    user: allow
    agent: allow (需配置 API key)

llm:use_key:
  description: 使用 API Key 时验证权限
  caller: LLM Provider
  check:
    principal: caller_id
    resource: "secret:llm:key:{{key_id}}"
    action: "use"
  default:
    user: allow
    agent: deny
```

### 权限检查流程

```
┌─────────────────────────────────────────────────────────────┐
│                      模块调用 Security Manager              │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. 确定调用主体                                             │
│    - 主体类型: user / agent / session                       │
│    - 主体 ID: user_id / agent_id / session_id               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. 查询权限策略                                             │
│    - 检查用户是否为系统所有者 (user → allow)                │
│    - 检查 Agent 是否有显式授权                              │
│    - 检查会话级别权限配置                                   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
                    ┌─────┴─────┐
                    │ 有授权？   │
                    └─────┬─────┘
                          │ 否              │ 是
                          ▼                 ▼
┌──────────────────────────────┐   ┌──────────────────────┐
│ 记录拒绝日志                 │   │ 记录允许日志         │
│ 返回 denied + reason         │   │ 返回 allowed         │
└──────────────────────────────┘   └──────────────────────┘
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 1.1.0 | 2026-04-01 | 添加安全检查点文档，更新为用户中心权限模型 |
