# Orchestrator Module

Design Reference: `docs/03-module-design/core/orchestrator.md`

## 概述

Orchestrator 负责多 Agent 编排和资源管理：
- Agent 资源分配和负载均衡
- Agent 消息路由和广播
- Agent 间协作协调
- Agent 池管理和容量控制
- **为 Task Manager 提供 Agent 分配接口**

## 导入

```rust
use orchestrator::{
    OrchestratorImpl, OrchestratorError, AgentInfo, AgentStatus, AgentFilter,
    AgentMessage, TopicMessage, CollaborationMode, TaskRequirements, ResourceUsage,
    Collaboration, SendResult, TopicSubscription, OrchestratorConfig, SchedulingStrategy,
};
```

## 核心类型

### OrchestratorError
编排器错误枚举：
- `NotInitialized`: 未初始化
- `AgentNotFound(String)`: Agent 未找到
- `AgentNotAvailable(String)`: 没有可用 Agent
- `AllocationFailed(String)`: 分配失败
- `RegistrationFailed(String)`: 注册失败
- `CollaborationNotFound(String)`: 协作组未找到
- `MessageDeliveryFailed(String)`: 消息投递失败
- `ResourceLimitExceeded(String)`: 资源限制超出
- `InvalidRequest(String)`: 无效请求
- `TopicNotFound(String)`: 主题未找到

### AgentInfo
Agent 信息：
- `id`: Agent ID
- `name`: Agent 名称
- `definition_id`: Agent 定义 ID
- `session_id`: 所属会话 ID
- `variant`: Agent 变体（可选）
- `status`: Agent 状态 (Idle, Busy, Paused, Error, Stopped)
- `current_task`: 当前任务 ID（可选）
- `capabilities`: 能力列表
- `statistics`: 统计信息

### AgentStatus
Agent 状态枚举：
- `Idle`: 空闲
- `Busy`: 忙碌
- `Paused`: 暂停
- `Error`: 错误
- `Stopped`: 已停止

### CollaborationMode
协作模式：
- `MasterWorker`: 主从模式
- `Pipeline`: 流水线模式
- `Voting`: 投票模式

## 对外接口

### 创建编排器

```rust
let orch = OrchestratorImpl::new();
```

### 注册 Agent

```rust
let agent = AgentInfo::new(
    "agent-1".to_string(),
    "Agent One".to_string(),
    "claude".to_string(),
    "session-1".to_string(),
)
.with_variant("developer")
.with_capabilities(vec!["coding".to_string(), "review".to_string()]);

orch.register_agent(agent).await.unwrap();
```

### 注销 Agent

```rust
orch.unregister_agent("agent-1").await.unwrap();
```

### 列出 Agents

```rust
// 列出所有
let agents = orch.list_agents(None).await;

// 按条件过滤
let filter = AgentFilter {
    status: Some(AgentStatus::Idle),
    variant: Some("developer".to_string()),
    ..Default::default()
};
let filtered = orch.list_agents(Some(filter)).await;
```

### 获取 Agent 信息

```rust
let info = orch.get_agent_info("agent-1").await.unwrap();
println!("Status: {:?}", info.status);
```

### 更新 Agent 状态

```rust
orch.update_agent_status("agent-1", AgentStatus::Busy).await.unwrap();
```

### 分配 Agent

```rust
let requirements = TaskRequirements::default();
let agent_id = orch.allocate_agent(&requirements).await.unwrap();
```

### 发送消息

```rust
let msg = AgentMessage::new("from-agent", "to-agent", serde_json::json!("hello"));
orch.send_message("to-agent", msg).await.unwrap();
```

### 广播消息

```rust
let msg = AgentMessage::new("sender", "broadcast", serde_json::json!("hello all"));
let results = orch.broadcast(&["agent-1".to_string(), "agent-2".to_string()], msg).await;
```

### 发布/订阅

```rust
// 订阅主题
orch.subscribe("agent-1", "code-changes").await.unwrap();

// 发布到主题
let topic_msg = TopicMessage::new("code-changes", "agent-2", serde_json::json!({"file": "main.rs"}));
let count = orch.publish("code-changes", topic_msg).await.unwrap();

// 取消订阅
orch.unsubscribe("agent-1", "code-changes").await.unwrap();
```

### 创建协作组

```rust
let collab_id = orch
    .create_collaboration(
        "code-review",
        vec!["agent-1".to_string(), "agent-2".to_string()],
        CollaborationMode::MasterWorker,
    )
    .await
    .unwrap();
```

### 解散协作组

```rust
orch.dissolve_collaboration(&collab_id).await.unwrap();
```

### 获取资源使用情况

```rust
let usage = orch.get_resource_usage().await;
println!("Total agents: {}", usage.total_agents);
println!("Active agents: {}", usage.active_agents);
```

### 设置资源限制

```rust
orch.set_resource_limit("max_agents", 100).await.unwrap();
```

### 获取待处理消息

```rust
let messages = orch.get_messages("agent-1").await;
```

### 记录任务完成

```rust
orch.record_task_completion("agent-1", 1000).await.unwrap();
```

### 检查 Agent 是否存在

```rust
if orch.has_agent("agent-1").await {
    println!("Agent exists!");
}
```

### 获取 Agent 数量

```rust
let count = orch.agent_count().await;
```

## 完整示例

```rust
use orchestrator::{OrchestratorImpl, AgentInfo, TaskRequirements, AgentStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orch = OrchestratorImpl::new();

    // Register agents
    let agent1 = AgentInfo::new(
        "agent-1".to_string(),
        "Developer".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    )
    .with_variant("developer")
    .with_capabilities(vec!["coding".to_string()]);

    let agent2 = AgentInfo::new(
        "agent-2".to_string(),
        "Reviewer".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    )
    .with_variant("reviewer")
    .with_capabilities(vec!["review".to_string()]);

    orch.register_agent(agent1).await?;
    orch.register_agent(agent2).await?;

    // List idle agents
    let filter = orchestrator::AgentFilter {
        status: Some(AgentStatus::Idle),
        ..Default::default()
    };
    let idle_agents = orch.list_agents(Some(filter)).await;
    println!("Idle agents: {}", idle_agents.len());

    // Allocate agent for task
    let requirements = TaskRequirements {
        agent_variant: Some("developer".to_string()),
        ..Default::default()
    };
    let agent_id = orch.allocate_agent(&requirements).await?;
    println!("Allocated agent: {}", agent_id);

    // Get resource usage
    let usage = orch.get_resource_usage().await;
    println!("Total agents: {}", usage.total_agents);
    println!("Active agents: {}", usage.active_agents);

    Ok(())
}
```

## 错误处理

所有操作都返回 `OrchestratorResult<T>` 类型，使用 `?` 操作符进行错误传播：

```rust
match orch.register_agent(agent_info).await {
    Ok(_) => println!("Agent registered!"),
    Err(OrchestratorError::RegistrationFailed(id)) => {
        println!("Agent {} already exists", id);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```
