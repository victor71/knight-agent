# Agent Runtime Module

Design Reference: `docs/03-module-design/agent/agent-runtime.md`

## 概述

Agent Runtime 负责单个 Agent 的执行逻辑，包括：
- Agent 生命周期管理（初始化、执行、停止）
- LLM 调用和响应处理
- Tool/Skill 调用执行
- 上下文管理和状态更新
- 错误处理和重试逻辑

## 导入

```rust
use agent_runtime::{
    AgentRuntimeImpl, AgentRuntimeError,
    AgentStatus, AgentState, AgentStatistics, Agent,
    ErrorInfo, AwaitInfo, Message, ToolResult, UserResponse,
    RuntimeConfig, RuntimeResult,
};
```

## 核心类型

### AgentStatus
Agent 状态枚举：
- `Idle`: 空闲，等待消息
- `Thinking`: 思考中，正在处理消息
- `Acting`: 执行中，正在执行工具
- `Paused`: 已暂停
- `AwaitingUser`: 等待用户响应
- `Error`: 错误状态
- `Stopped`: 已停止

### AgentState
Agent 运行时状态：
- `status`: 当前状态
- `current_action`: 当前执行的动作
- `error`: 错误信息
- `statistics`: 统计信息
- `await_info`: 等待用户响应信息

### AgentStatistics
执行统计：
- `messages_sent`: 发送消息数
- `messages_received`: 接收消息数
- `tools_called`: 工具调用次数
- `llm_calls`: LLM 调用次数
- `total_tokens`: 总 Token 数
- `errors`: 错误数

### RuntimeConfig
运行时配置：
- `max_execution_time`: 最大执行时间（秒）
- `max_tool_calls`: 最大工具调用次数
- `max_llm_calls`: 最大 LLM 调用次数
- `max_retry_attempts`: 最大重试次数
- `retry_delay_ms`: 重试延迟（毫秒）
- `llm_timeout_secs`: LLM 调用超时（秒）
- `tool_timeout_secs`: 工具调用超时（秒）

## 对外接口

### 创建运行时

```rust
let runtime = AgentRuntimeImpl::new();
let runtime = AgentRuntimeImpl::with_config(config);
```

### 初始化

```rust
let mut runtime = AgentRuntimeImpl::new();
runtime.initialize().await.unwrap();
```

### 创建 Agent

```rust
let agent = runtime
    .create_agent(
        "code-reviewer".to_string(),
        "session-1".to_string(),
        Some("quick".to_string()),
    )
    .await
    .unwrap();
```

### 启动/停止 Agent

```rust
runtime.start_agent(&agent.id).await.unwrap();
runtime.stop_agent(&agent.id, false).await.unwrap();
```

### 暂停/恢复 Agent

```rust
runtime.pause_agent(&agent.id).await.unwrap();
runtime.resume_agent(&agent.id).await.unwrap();
```

### 发送消息

```rust
let msg = Message::user("Hello world");
let response = runtime.send_message(&agent.id, msg, false).await.unwrap();
```

### 获取状态和上下文

```rust
let state = runtime.get_agent_state(&agent.id).await.unwrap();
let context = runtime.get_context(&agent.id).await.unwrap();
```

### 更新变量

```rust
let mut vars = serde_json::Map::new();
vars.insert("name".to_string(), serde_json::json!("test"));
runtime.update_variables(&agent.id, vars).await.unwrap();
```

### 调用工具

```rust
let args = serde_json::json!({"file": "test.txt"});
let result = runtime.call_tool(&agent.id, "read", args).await.unwrap();
```

### 等待用户响应

```rust
let await_id = runtime
    .await_user(&agent.id, "confirmation", "Continue?")
    .await
    .unwrap();
```

### 处理用户响应

```rust
let response = UserResponse::new(&await_id, serde_json::json!("yes"), true);
let resumed_state = runtime.handle_user_response(&agent.id, response).await.unwrap();
```

### 取消操作

```rust
let cancelled = runtime.cancel_operation(&agent.id).await.unwrap();
```

### 错误处理

```rust
let error = ErrorInfo::new("TEST_ERROR", "Something went wrong");
runtime.set_error(&agent.id, error).await.unwrap();
```

## 完整示例

```rust
use agent_runtime::{AgentRuntimeImpl, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and initialize runtime
    let mut runtime = AgentRuntimeImpl::new();
    runtime.initialize().await?;

    // Create an agent
    let agent = runtime
        .create_agent(
            "code-reviewer".to_string(),
            "session-1".to_string(),
            Some("quick".to_string()),
        )
        .await?;

    println!("Created agent: {}", agent.id);

    // Start the agent
    runtime.start_agent(&agent.id).await?;

    // Send a message
    let msg = Message::user("Review this code");
    let response = runtime.send_message(&agent.id, msg, false).await?;
    println!("Response: {:?}", response);

    // Complete execution
    runtime.complete(&agent.id).await?;

    // Stop the agent
    runtime.stop_agent(&agent.id, false).await?;

    Ok(())
}
```

## 状态转换

```
Idle --> Thinking: receive message
Thinking --> Acting: tool use detected
Thinking --> Idle: complete
Thinking --> AwaitingUser: user.query()
AwaitingUser --> Thinking: user.response()
Acting --> Thinking: tool result
Idle --> Paused: pause()
Paused --> Idle: resume()
Any --> Stopped: stop()
```

## 错误处理

AgentRuntimeError 枚举定义：
- `NotInitialized`: 运行时未初始化
- `AgentNotFound(String)`: Agent 未找到
- `ExecutionFailed(String)`: 执行失败
- `AlreadyRunning(String)`: Agent 已在运行
- `InvalidStateTransition(String)`: 无效的状态转换
- `ToolExecutionFailed(String)`: 工具执行失败
- `LlmCallFailed(String)`: LLM 调用失败
- `ContextUpdateFailed(String)`: 上下文更新失败
- `OperationCancelled(String)`: 操作被取消
- `Timeout(String)`: 超时
