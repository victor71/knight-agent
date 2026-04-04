# External Agent Module

Design Reference: `docs/03-module-design/agent/external-agent.md`

## 概述

External Agent 模块负责集成和调用外部 Agent 服务，包括：
- 外部 Agent 生命周期管理（启动、停止、监控）
- 与外部 Agent 的进程间通信
- 输出流处理和结果回传
- 错误处理和超时控制
- 资源管理和清理

## 导入

```rust
use external_agent::{
    ExternalAgentManager, ExternalAgentError, ExternalAgentConfig,
    DiscoveredAgent, ExternalAgentStatus, ProcessState, InputMode,
    AgentDefinition,
};
```

## 核心类型

### ExternalAgentConfig
外部 Agent 配置：
- `agent_type`: Agent 类型标识
- `command`: 执行命令
- `args`: 启动参数
- `env`: 环境变量
- `working_dir`: 工作目录
- `timeout`: 超时时间（秒）
- `stream_output`: 是否流式输出
- `input_mode`: 输入模式 (interactive/batch/pipe)

### DiscoveredAgent
发现的 Agent 信息：
- `agent_type`: Agent 类型
- `name`: 显示名称
- `available`: 是否可用
- `installed`: 是否已安装
- `version`: 版本号
- `path`: 可执行文件路径
- `reason`: 不可用原因
- `install_url`: 安装链接

### ExternalAgentStatus
Agent 状态：
- `agent_id`: Agent ID
- `process_id`: 进程 ID
- `state`: 进程状态
- `started_at`: 启动时间
- `exit_code`: 退出码
- `output_lines`: 输出行数
- `memory_mb`: 内存使用
- `cpu_percent`: CPU 使用率

### ProcessState
进程状态枚举：
- `Starting`: 启动中
- `Running`: 运行中
- `WaitingInput`: 等待输入
- `Completed`: 已完成
- `Error`: 错误
- `Killed`: 已终止

## 对外接口

### 创建管理器

```rust
let manager = ExternalAgentManager::new();
```

### 发现 Agent

```rust
let discovered = manager.discover().await;
// Returns list of DiscoveredAgent
```

### 检查可用性

```rust
let result = manager.check_availability("claude-code").await;
println!("Available: {}", result.available);
```

### 获取安装指导

```rust
if let Some(instructions) = manager.get_install_instructions("claude-code") {
    println!("{}", instructions);
}
```

### 启动外部 Agent

```rust
let config = ExternalAgentConfig {
    agent_type: "claude-code".to_string(),
    command: "claude".to_string(),
    args: vec!["--print".to_string()],
    working_dir: Some("/tmp".to_string()),
    timeout: 300,
    ..Default::default()
};

let agent_id = manager.spawn(&config, "List files").await.unwrap();
```

### 发送输入

```rust
manager.send_input(&agent_id, "continue", false).await.unwrap();
```

### 获取输出

```rust
let (output, is_complete) = manager.get_output(&agent_id).await.unwrap();
println!("Output: {}", output);
```

### 获取状态

```rust
let status = manager.get_status(&agent_id).await.unwrap();
println!("State: {:?}", status.state);
```

### 终止 Agent

```rust
let exit_code = manager.terminate(&agent_id, false).await.unwrap();
```

### 等待完成

```rust
let (exit_code, output) = manager
    .wait_for_completion(&agent_id, 60)
    .await
    .unwrap();
```

### 中断 Agent

```rust
manager.interrupt(&agent_id).await.unwrap();
```

### 验证输入

```rust
match manager.validate_input("normal input") {
    Ok(_) => println!("Input is safe"),
    Err(e) => println!("Dangerous input: {}", e),
}
```

## 完整示例

```rust
use external_agent::{ExternalAgentManager, ExternalAgentConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ExternalAgentManager::new();

    // Check if claude-code is available
    let availability = manager.check_availability("claude-code").await;
    if !availability.available {
        println!("Claude Code not available: {:?}", availability.reason);
        if let Some(url) = &availability.install_url {
            println!("Install from: {}", url);
        }
        return Ok(());
    }

    // Spawn agent
    let config = ExternalAgentConfig {
        agent_type: "claude-code".to_string(),
        command: "echo".to_string(),
        args: vec!["hello".to_string()],
        timeout: 30,
        ..Default::default()
    };

    let agent_id = manager.spawn(&config, "Test task").await?;
    println!("Spawned agent: {}", agent_id);

    // Wait for completion
    match manager.wait_for_completion(&agent_id, 60).await {
        Ok((exit_code, output)) => {
            println!("Agent completed with exit code: {:?}", exit_code);
            println!("Output: {}", output);
        }
        Err(e) => {
            println!("Agent failed: {}", e);
        }
    }

    Ok(())
}
```

## 错误处理

ExternalAgentError 枚举定义：
- `NotInitialized`: 未初始化
- `ConnectionFailed(String)`: 连接失败
- `CommunicationFailed(String)`: 通信失败
- `ProcessSpawnFailed(String)`: 进程启动失败
- `ProcessNotFound(String)`: 进程未找到
- `StdinNotAvailable`: STDIN 不可用
- `ProcessTimeout`: 进程超时
- `ProcessCrashed(String)`: 进程崩溃
- `PermissionDenied(String)`: 权限拒绝
- `AgentNotInstalled(String)`: Agent 未安装
- `AgentNotFound(String)`: Agent 未找到
- `InvalidInput(String)`: 无效输入
- `ResourceLimitExceeded(String)`: 资源限制超出
