# Tool System Module

统一的工具框架，提供工具注册、验证和执行功能。支持内置工具、自定义工具和 MCP 工具。

Design Reference: `docs/03-module-design/tools/tool-system.md`

## 特性

- 统一的工具调用接口
- 工具注册和发现
- JSON Schema 参数验证
- 内置工具：read, write, edit, grep, glob, bash
- MCP 工具支持结构
- 异步执行

## 依赖

```toml
[dependencies]
tool-system = { path = "./rust/tool-system/tool-system" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## 快速开始

```rust
use tool_system::{
    ToolSystemImpl, ToolSystemTrait, ToolDefinition, ToolHandler, HandlerType,
    ExecuteRequest, ToolContext,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建工具系统
    let tool_system = ToolSystemImpl::new()?;

    // 列出所有工具
    let tools = tool_system.list_tools().await?;
    println!("Available tools: {:?}", tools.len());

    // 执行内置工具
    let context = ToolContext {
        session_id: "test".to_string(),
        agent_id: "test".to_string(),
        workspace: ".".to_string(),
        variables: HashMap::new(),
    };

    let request = ExecuteRequest {
        name: "glob".to_string(),
        args: serde_json::json!({
            "pattern": "**/*.rs",
            "path": "."
        }),
        context,
    };

    let result = tool_system.execute(request).await?;
    println!("Result: {:?}", result);

    Ok(())
}
```

## API 接口

### 创建工具系统

```rust
// 创建包含内置工具的工具系统
let tool_system = ToolSystemImpl::new()?;

// 创建空的工具系统
let tool_system = ToolSystemImpl::empty()?;
```

### 注册工具

```rust
let tool = ToolDefinition {
    name: "my_tool".to_string(),
    display_name: "My Tool".to_string(),
    description: "A custom tool".to_string(),
    category: "custom".to_string(),
    parameters: Default::default(),
    handler: ToolHandler {
        handler_type: HandlerType::Command,
        target: "echo hello".to_string(),
        timeout_secs: 30,
    },
    permissions: vec![],
    dangerous: false,
    is_read_only: true,  // 设置为只读工具，可并行执行
};

tool_system.register_tool(tool).await?;
```

### 执行工具

```rust
let request = ExecuteRequest {
    name: "read".to_string(),
    args: serde_json::json!({
        "file_path": "/path/to/file.txt"
    }),
    context,
};

let result = tool_system.execute(request).await?;
```

### 验证参数

```rust
let args = serde_json::json!({
    "file_path": "/path/to/file.txt"
});

let validation = tool_system.validate_args("read", &args).await?;
if !validation.valid {
    for error in validation.errors {
        println!("Error: {} - {}", error.field, error.message);
    }
}
```

### 工具信息

```rust
// 获取工具信息
let tool_info = tool_system.get_tool("read").await?;

// 列出所有工具
let tools = tool_system.list_tools().await?;

// 按类别列出工具
let file_tools = tool_system.list_tools_by_category("builtin").await?;

// 获取所有类别
let categories = tool_system.get_categories().await?;
```

## 内置工具

### read - 读取文件 (is_read_only: true)

```rust
let args = serde_json::json!({
    "file_path": "/path/to/file.txt",
    "offset": 0,      // 可选：起始行
    "limit": 100      // 可选：读取行数
});
```

### write - 写入文件 (is_read_only: false)

```rust
let args = serde_json::json!({
    "file_path": "/path/to/file.txt",
    "content": "Hello, World!"
});
```

### edit - 编辑文件 (is_read_only: false)

```rust
let args = serde_json::json!({
    "file_path": "/path/to/file.txt",
    "old_string": "Hello",
    "new_string": "Hello, World!"
});
```

### grep - 搜索内容 (is_read_only: true)

```rust
let args = serde_json::json!({
    "pattern": "TODO",
    "path": ".",
    "glob": "*.rs"  // 可选：文件过滤
});
```

### glob - 查找文件 (is_read_only: true)

```rust
let args = serde_json::json!({
    "pattern": "**/*.rs",
    "path": "."
});
```

### bash - 执行命令 (is_read_only: false)

```rust
let args = serde_json::json!({
    "command": "echo hello",
    "timeout": 30  // 可选：超时时间（秒）
});
```

**并行执行优化：** 只读工具（read, grep, glob）可以安全地并行执行，非只读工具（write, edit, bash）需要串行执行以避免冲突。

## 错误类型

```rust
pub enum ToolSystemError {
    NotInitialized,           // 工具系统未初始化
    NotFound(String),        // 工具不存在
    AlreadyExists(String),   // 工具已存在
    InvalidArguments(String), // 参数无效
    PermissionDenied(String), // 权限被拒绝
    ExecutionFailed(String),  // 执行失败
    Timeout(String),         // 执行超时
}
```

## 数据结构

### ToolDefinition

```rust
pub struct ToolDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub parameters: JsonSchema,
    pub handler: ToolHandler,
    pub permissions: Vec<String>,
    pub dangerous: bool,
    pub is_read_only: bool,  // 是否只读（只读工具可并行执行）
}
```

### ToolInfo

```rust
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub parameters: JsonSchema,
    pub dangerous: bool,
    pub is_read_only: bool,  // 是否只读（只读工具可并行执行）
}
```

### ToolContext

```rust
pub struct ToolContext {
    pub session_id: String,
    pub agent_id: String,
    pub workspace: String,
    pub variables: HashMap<String, serde_json::Value>,
}
```

### ToolExecutionResult

```rust
pub struct ToolExecutionResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub duration_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}
```
