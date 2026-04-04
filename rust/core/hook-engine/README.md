# Hook Engine Module

Design Reference: `docs/03-module-design/core/hook-engine.md`

## 概述

Hook引擎提供事件驱动的钩子机制，支持在特定事件和阶段执行自定义处理逻辑。支持多种处理器类型：Command、Skill、RPC、WASM和Callback。

## 导入

```rust
use hook_engine::{
    HookRegistry, HookExecutor, HookContext, HookPhase, HookHandler,
    HookDefinition, HookFilter, HookControl, HookErrorHandling,
    HookExecutionResult, TriggerResult, HookInfo, EventPoint,
};
```

## 核心类型

### HookPhase
Hook执行阶段枚举：`Before`, `After`, `Replace`

### HookHandler
Hook处理器类型枚举：
- `HookHandler::Command { executable, args, env }`: 命令执行
- `HookHandler::Skill { skill_id, args }`: Skill调用
- `HookHandler::Rpc { endpoint, method, timeout_secs }`: RPC调用
- `HookHandler::Wasm { module, function }`: WASM模块调用
- `HookHandler::Callback { handler }`: 内部回调

### HookDefinition
Hook定义结构：
- `id`: Hook唯一标识
- `name`: Hook名称
- `event`: 监听的事件名
- `phase`: 执行阶段（Before/After/Replace）
- `priority`: 优先级（数值越小优先级越高）
- `enabled`: 是否启用
- `handler`: 处理器
- `filter`: 过滤条件
- `control`: 控制选项
- `error_handling`: 错误处理配置

### HookContext
Hook执行时的运行时上下文：
- `event`: 事件名
- `phase`: 阶段
- `session`: 会话上下文
- `agent`: Agent上下文
- `request`: 请求上下文
- `response`: 响应上下文
- `data`: 自定义数据

## 对外接口

### HookRegistry - Hook注册与管理

```rust
use hook_engine::{HookRegistry, HookPhase, HookHandler, HookDefinition};
use std::collections::HashMap;

// 创建注册表
let registry = HookRegistry::new();

// 创建Hook定义
let handler = HookHandler::Skill {
    skill_id: "my_skill".to_string(),
    args: HashMap::new(),
};
let hook = HookDefinition::new(
    "hook_1".to_string(),
    "tool_call".to_string(),
    HookPhase::Before,
    handler,
);

// 注册Hook
registry.register(hook).await?;

// 获取Hook
let retrieved = registry.get("hook_1").await?;

// 列出所有Hook
let all_hooks = registry.list(None).await;

// 按事件过滤
let filtered = registry.list(Some("tool_call")).await;

// 查找匹配的Hook
let matching = registry.find_matching("tool_call", HookPhase::Before).await;

// 启用/禁用Hook
registry.enable("hook_1").await?;
registry.disable("hook_1").await?;

// 注销Hook
registry.unregister("hook_1").await?;

// 统计与清空
let count = registry.len().await;
registry.clear().await;
```

### HookExecutor - Hook执行

```rust
use hook_engine::{HookExecutor, HookContext, HookPhase};
use std::sync::Arc;

// 创建执行器
let executor = HookExecutor::new(Arc::clone(&registry));

// 创建上下文
let context = HookContext::new("tool_call".to_string(), HookPhase::Before);

// 触发Hook执行
let result = executor.trigger("tool_call", HookPhase::Before, context).await;

// 检查结果
if result.blocked {
    println!("Blocked: {:?}", result.block_reason);
}
if result.modified {
    println!("Modified data: {:?}", result.modified_data);
}
println!("Executed {} hooks, {} failed", result.hooks_executed, result.hooks_failed);
```

### HookContext - 上下文构建器

```rust
use hook_engine::{HookContext, SessionContext, AgentContext};
use std::collections::HashMap;

// 基本上下文
let ctx = HookContext::new("agent_execute".to_string(), HookPhase::Before);

// 添加会话上下文
let session = SessionContext {
    id: "session_123".to_string(),
    workspace: Some("/path/to/workspace".to_string()),
    variables: HashMap::new(),
};
let ctx = ctx.with_session(session);

// 添加Agent上下文
let agent = AgentContext {
    id: "agent_1".to_string(),
    name: Some("MyAgent".to_string()),
    state: Some("running".to_string()),
};
let ctx = ctx.with_agent(agent);

// 添加自定义数据
let ctx = ctx.with_data("key", serde_json::json!("value"));
```

### HookExecutionResult - 执行结果

```rust
use hook_engine::HookExecutionResult;

// 成功的执行结果
let result = HookExecutionResult::success("hook_1".to_string());
assert!(result.success);
assert!(!result.blocked);

// 被阻止的执行结果
let result = HookExecutionResult::blocked("hook_1".to_string(), "Access denied".to_string());
assert!(result.blocked);
assert_eq!(result.block_reason, Some("Access denied".to_string()));
```

### HookInfo - Hook查询信息

```rust
// list() 返回 Vec<HookInfo>，包含运行时统计
let hooks = registry.list(None).await;
for info in hooks {
    println!("Hook: {} - executions: {}", info.id, info.execution_count);
    if let Some(last) = info.last_executed {
        println!("  Last executed: {}", last);
    }
}
```

## 完整示例

```rust
use hook_engine::{
    HookRegistry, HookExecutor, HookContext, HookPhase, HookHandler,
    HookDefinition,
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建注册表和执行器
    let registry = Arc::new(HookRegistry::new());
    let executor = HookExecutor::new(Arc::clone(&registry));

    // 注册一个Before Hook
    let hook1 = HookDefinition::new(
        "before_tool_call".to_string(),
        "tool_call".to_string(),
        HookPhase::Before,
        HookHandler::Skill {
            skill_id: "validate_tool".to_string(),
            args: HashMap::new(),
        },
    );
    registry.register(hook1).await?;

    // 注册一个After Hook
    let hook2 = HookDefinition::new(
        "after_tool_call".to_string(),
        "tool_call".to_string(),
        HookPhase::After,
        HookHandler::Command {
            executable: "/usr/local/bin/hook.sh".to_string(),
            args: vec![],
            env: HashMap::new(),
        },
    );
    registry.register(hook2).await?;

    // 创建上下文并触发
    let context = HookContext::new("tool_call".to_string(), HookPhase::Before)
        .with_data("tool_name", serde_json::json!("read_file"));

    let result = executor.trigger("tool_call", HookPhase::Before, context).await;

    println!("Trigger result: {:?}", result);
    println!("Hooks executed: {}", result.hooks_executed);

    Ok(())
}
```

## 错误处理

HookError 枚举定义：
- `NotInitialized`: Hook引擎未初始化
- `ExecutionFailed(String)`: Hook执行失败
- `NotFound(String)`: Hook不存在
- `AlreadyExists(String)`: Hook已存在
- `Disabled(String)`: Hook已被禁用
- `Blocked(String)`: Hook被阻止
