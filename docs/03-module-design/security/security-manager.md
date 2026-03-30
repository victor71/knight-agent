# Security Manager (安全管理器)

## 1. 概述

### 1.1 职责描述

Security Manager 负责系统的安全管理，包括：

- 权限控制和管理
- 访问控制列表 (ACL)
- 审计日志
- 密钥和凭证管理
- 安全策略执行
- 威胁检测和防护

### 1.2 设计目标

1. **最小权限**: 默认拒绝，显式允许
2. **审计追踪**: 完整的安全事件日志
3. **灵活策略**: 支持细粒度权限控制
4. **安全存储**: 密钥加密存储

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 会话上下文 |
| Storage Service | 依赖 | 存储审计日志 |

---

## 2. 接口定义

### 2.1 对外接口

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

### 2.2 数据结构

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

### 2.3 配置选项

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

## 3. 核心流程

### 3.1 权限检查流程

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

### 3.2 威胁检测流程

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

### 3.3 审计日志流程

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

## 4. 模块交互

### 4.1 依赖关系图

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

### 4.2 消息流

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

---

## 5. 配置与部署

### 5.1 配置文件格式

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

### 5.2 环境变量

```bash
# 安全配置
export KNIGHT_SECURITY_DEFAULT_POLICY="deny"
export KNIGHT_SECURITY_AUDIT_RETENTION_DAYS=90

# 威胁检测
export KNIGHT_SECURITY_THREAT_DETECTION_ENABLED="true"
export KNIGHT_SECURITY_THREAT_SENSITIVITY="medium"
```

---

## 6. 示例

### 6.1 权限配置

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

### 6.2 安全策略

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

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 权限检查 | < 1ms | 内存操作 |
| 策略评估 | < 10ms | 复杂策略 |
| 日志写入 | < 5ms | 异步写入 |

### 7.2 错误处理

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

### 7.3 安全最佳实践

1. **最小权限**: 默认拒绝，显式允许
2. **定期审计**: 定期检查权限和日志
3. **密钥轮换**: 定期更换敏感密钥
4. **威胁监控**: 启用威胁检测和告警
5. **日志保留**: 保留足够长时间的审计日志
