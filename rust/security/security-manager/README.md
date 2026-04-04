# Security Manager Module

Design Reference: `docs/03-module-design/security/security-manager.md`

## 概述

全面的安全管理模块，提供权限管理、策略引擎、审计日志和密钥管理功能。

## 导入

```rust
use security_manager::{
    SecurityManager, SecurityManagerImpl,
    Principal, Permission, PermissionGrant,
    SecurityPolicy, SecurityPolicyRule as PolicyRule, PolicyType, PolicyEffect,
    SecurityContext, SecurityEvent, SecurityEventType, EventResult, LogQuery,
    SecretInfo, SecretPolicyConfig,
    SecurityConfig, DefaultPolicy,
    PolicyEngine, AuditLogger, SecretManager,
    ThreatInfo, Activity, TimeRange, LogSummary,
};
```

## 核心类型

### Principal
权限主体枚举：
- `Principal::User(String)` - 用户
- `Principal::Agent(String)` - Agent
- `Principal::Session(String)` - 会话

### PermissionGrant
权限授予请求：
```rust
PermissionGrant {
    principal: Principal::User("admin".to_string()),
    resource: "file:/project/**".to_string(),
    actions: vec!["read".to_string(), "write".to_string()],
    conditions: vec![],
    expires_at: None,
}
```

### SecurityContext
安全上下文（用于权限检查）：
```rust
SecurityContext {
    principal: Principal::User("user1".to_string()),
    session_id: Some("session-abc".to_string()),
    agent_id: None,
    workspace: Some("/project".to_string()),
    ip_address: Some("192.168.1.1".to_string()),
    timestamp: std::time::SystemTime::now(),
    metadata: HashMap::new(),
}
```

### SecurityPolicy
安全策略：
```rust
SecurityPolicy {
    id: "policy-1".to_string(),
    name: "Admin Policy".to_string(),
    description: "Full access for admins".to_string(),
    policy_type: PolicyType::Rbac,
    enabled: true,
    rules: vec![
        PolicyRule {
            name: "allow-admin".to_string(),
            effect: PolicyEffect::Allow,
            principal: Some("user:admin".to_string()),
            resource: Some("*".to_string()),
            action: Some("*".to_string()),
            conditions: vec![],
        }
    ],
    priority: 10,
}
```

## 对外接口

### SecurityManagerImpl 创建与初始化

```rust
use security_manager::{SecurityManagerImpl, SecurityManager};

let security = SecurityManagerImpl::new()?;
security.init().await?;
```

### 权限管理

#### 授予权限

```rust
use security_manager::{PermissionGrant, Principal};

let grant = PermissionGrant {
    principal: Principal::User("user1".to_string()),
    resource: "file:/project/**".to_string(),
    actions: vec!["read".to_string(), "write".to_string()],
    conditions: vec![],
    expires_at: None,
};

security.grant_permission(grant).await?;
```

#### 撤销权限

```rust
security.revoke_permission("user:user1", "file:/project/**", "write").await?;
```

#### 检查权限

```rust
// 基础检查
let (allowed, reason) = security
    .check_permission("user:admin", "file:/project/readme.txt", "read", None)
    .await?;

// 带上下文检查
let context = SecurityContext {
    principal: Principal::User("user1".to_string()),
    session_id: Some("session-123".to_string()),
    workspace: Some("/project".to_string()),
    ..Default::default()
};

let (allowed, reason) = security
    .check_permission("user:user1", "file:/project/secret.txt", "read", Some(context))
    .await?;
```

#### 列出权限

```rust
let permissions = security.list_permissions("user:user1").await?;
for perm in permissions {
    println!("Resource: {}, Actions: {:?}", perm.resource, perm.actions);
}
```

### 策略管理

#### 创建策略

```rust
use security_manager::{SecurityPolicy, PolicyType, PolicyEffect, PolicyRule};

let policy = SecurityPolicy {
    id: "read-only-policy".to_string(),
    name: "Read Only".to_string(),
    description: "Read access to all files".to_string(),
    policy_type: PolicyType::Rbac,
    enabled: true,
    rules: vec![
        PolicyRule {
            name: "allow-read".to_string(),
            effect: PolicyEffect::Allow,
            principal: Some("user:*".to_string()),
            resource: Some("file:/**".to_string()),
            action: Some("read".to_string()),
            conditions: vec![],
        },
        PolicyRule {
            name: "deny-write".to_string(),
            effect: PolicyEffect::Deny,
            principal: Some("*".to_string()),
            resource: Some("file:/private/**".to_string()),
            action: Some("write".to_string()),
            conditions: vec![],
        },
    ],
    priority: 10,
};

security.create_policy(policy).await?;
```

#### 更新策略

```rust
security.update_policy("policy-id", updated_policy).await?;
```

#### 删除策略

```rust
security.delete_policy("policy-id").await?;
```

#### 获取策略

```rust
let policy = security.get_policy("policy-id").await?;
```

#### 列出策略

```rust
// 列出所有策略
let all = security.list_policies(None).await?;

// 按类型过滤
let rbac_policies = security.list_policies(Some(PolicyType::Rbac)).await?;
```

#### 评估策略

```rust
let context = SecurityContext {
    principal: Principal::User("user1".to_string()),
    ..Default::default()
};

let result = security.evaluate_policy("policy-id", context).await?;
println!("Allowed: {}, Reason: {}", result.allowed, result.reason);
```

### 审计日志

#### 记录安全事件

```rust
use security_manager::{SecurityEvent, SecurityEventType, EventResult};

let event = SecurityEvent {
    id: "event-1".to_string(),
    timestamp: std::time::SystemTime::now(),
    event_type: SecurityEventType::AccessRequest,
    principal: Principal::User("user1".to_string()),
    resource: Some("file:/doc.txt".to_string()),
    action: Some("read".to_string()),
    result: EventResult::Allowed,
    reason: None,
    details: HashMap::new(),
};

security.log_event(event).await?;
```

#### 查询日志

```rust
use security_manager::LogQuery;

// 基础查询
let events = security.query_logs(LogQuery::default()).await?;

// 按主体过滤
let query = LogQuery {
    principal: Some(Principal::User("user1".to_string())),
    ..Default::default()
};
let user_events = security.query_logs(query).await?;
```

#### 获取日志摘要

```rust
let summary = security.get_log_summary(None).await?;
println!("Total events: {}", summary.total_events);
```

### 密钥管理

#### 存储密钥

```rust
use std::collections::HashMap;

let mut metadata = HashMap::new();
metadata.insert("env".to_string(), serde_json::json!("production"));

security.store_secret("api-key", "secret-value-123!", Some(metadata)).await?;
```

#### 获取密钥

```rust
let value = security.get_secret("api-key").await?;
match value {
    Some(v) => println!("Secret value: {}", v),
    None => println!("Secret not found"),
}
```

#### 删除密钥

```rust
security.delete_secret("api-key").await?;
```

#### 列出密钥

```rust
let secrets = security.list_secrets().await?;
for info in secrets {
    println!("Key: {}, created: {:?}", info.created_at, info.metadata);
}
```

#### 轮转密钥

```rust
security.rotate_secret("api-key", "new-secret-value!").await?;
```

### 威胁检测

#### 分析威胁

```rust
let threats = security.analyze_threats(None).await?;
for threat in threats {
    println!("Threat: {} (severity: {:?})", threat.threat_type, threat.severity);
}
```

#### 检查可疑活动

```rust
use security_manager::Activity;

let activity = Activity {
    principal: Principal::User("user1".to_string()),
    action: "login".to_string(),
    resource: "auth".to_string(),
    timestamp: std::time::SystemTime::now(),
    result: EventResult::Allowed,
};

let (is_suspicious, confidence, reasons) = security.is_suspicious(activity).await?;
if is_suspicious {
    println!("Suspicious! Confidence: {}, Reasons: {:?}", confidence, reasons);
}
```

### 配置管理

#### 获取配置

```rust
use security_manager::{SecurityConfig, DefaultPolicy};

let config = security.get_security_config().await?;
println!("Default policy: {:?}", config.default_policy);
```

#### 更新配置

```rust
let new_config = SecurityConfig {
    default_policy: DefaultPolicy::Allow,
    enable_audit_logging: true,
    enable_threat_detection: true,
    max_login_attempts: 3,
    lockout_duration_secs: 300,
    session_timeout_secs: 3600,
    require_mfa_for_admin: true,
    allowed_ip_ranges: vec!["192.168.1.0/24".to_string()],
};

security.update_security_config(new_config).await?;
```

## 完整示例

```rust
use security_manager::{
    SecurityManagerImpl, SecurityManager,
    Principal, PermissionGrant, PolicyType, PolicyEffect,
    PolicyRule, SecurityPolicy, SecurityContext,
    SecurityEvent, SecurityEventType, EventResult, LogQuery,
    DefaultPolicy, SecurityConfig,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化安全管理系统
    let security = SecurityManagerImpl::new()?;
    security.init().await?;

    // 授予权限
    let grant = PermissionGrant {
        principal: Principal::User("admin".to_string()),
        resource: "file:/project/**".to_string(),
        actions: vec!["read".to_string(), "write".to_string()],
        conditions: vec![],
        expires_at: None,
    };
    security.grant_permission(grant).await?;

    // 检查权限
    let (allowed, reason) = security
        .check_permission("user:admin", "file:/project/readme.txt", "read", None)
        .await?;

    println!("Permission allowed: {}, reason: {:?}", allowed, reason);

    // 创建安全策略
    let policy = SecurityPolicy {
        id: "admin-policy".to_string(),
        name: "Admin Policy".to_string(),
        description: "Full access for admins".to_string(),
        policy_type: PolicyType::Rbac,
        enabled: true,
        rules: vec![PolicyRule {
            name: "allow-all".to_string(),
            effect: PolicyEffect::Allow,
            principal: Some("user:admin".to_string()),
            resource: Some("*".to_string()),
            action: Some("*".to_string()),
            conditions: vec![],
        }],
        priority: 100,
    };
    security.create_policy(policy).await?;

    // 记录安全事件
    let event = SecurityEvent {
        id: "evt-1".to_string(),
        timestamp: std::time::SystemTime::now(),
        event_type: SecurityEventType::AccessRequest,
        principal: Principal::User("admin".to_string()),
        resource: Some("file:/project".to_string()),
        action: Some("read".to_string()),
        result: EventResult::Allowed,
        reason: None,
        details: HashMap::new(),
    };
    security.log_event(event).await?;

    // 查询审计日志
    let logs = security.query_logs(LogQuery::default()).await?;
    println!("Audit logs: {} entries", logs.len());

    // 存储敏感信息
    security.store_secret("db-password", "supersecret123!@#", None).await?;
    let pwd = security.get_secret("db-password").await?;
    println!("DB password: {:?}", pwd);

    Ok(())
}
```

## 错误处理

```rust
use security_manager::SecurityManagerImpl;

let security = match SecurityManagerImpl::new() {
    Ok(s) => s,
    Err(e) => {
        eprintln!("Failed to create security manager: {}", e);
        return Err(e.into());
    }
};

if let Err(e) = security.init().await {
    eprintln!("Failed to initialize: {}", e);
    return Err(e.into());
}
```
