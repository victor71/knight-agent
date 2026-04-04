# Agent Variants Module

Design Reference: `docs/03-module-design/agent/agent-variants.md`

## 概述

Agent Variants 模块负责管理同一 Agent 的不同配置版本，包括：
- 变体定义文件的加载和解析
- 变体继承机制
- 变体发现和列表
- 变体配置合并
- CLI 变体语法支持

## 导入

```rust
use agent_variants::{
    AgentVariantRegistryImpl, AgentVariantError,
    AgentDefinition, AgentVariant, VariantOverrides,
    ModelConfig, PermissionConfig, ValidationResult,
    ResolvedAgentRef, VariantInfo, AgentVariantInfo,
};
```

## 核心类型

### AgentVariantRegistryImpl
Agent 变体注册表实现，负责管理所有 Agent 定义和变体。

### AgentDefinition
Agent 定义，包含：
- `id`: Agent 唯一标识
- `name`: Agent 显示名称
- `version`: Agent 版本
- `role`: Agent 角色描述
- `model`: 模型配置
- `instructions`: 系统指令
- `tools`: 工具列表
- `skills`: 技能列表
- `capabilities`: 能力列表
- `permissions`: 权限配置
- `variant`: 默认变体名称
- `variants`: 支持的变体列表

### AgentVariant
Agent 变体定义，包含：
- `name`: 变体名称
- `description`: 变体描述
- `extends`: 继承的基础定义
- `overrides`: 覆盖配置

### VariantOverrides
变体覆盖配置：
- `model`: 覆盖的模型配置
- `instructions`: 覆盖的系统指令
- `tools`: 覆盖的工具列表
- `skills`: 覆盖的技能列表
- `capabilities`: 覆盖的能力列表
- `permissions`: 覆盖的权限配置

### ResolvedAgentRef
解析后的 Agent 引用：
- `agent_id`: Agent ID
- `variant`: 变体名称（可选）

## 对外接口

### 创建注册表

```rust
let registry = AgentVariantRegistryImpl::new();
```

### 注册 Agent

```rust
let def = AgentDefinition::new(
    "code-reviewer".to_string(),
    "Code Reviewer".to_string(),
    "reviewing code".to_string(),
);
registry.register_agent(def).await.unwrap();
```

### 获取 Agent

```rust
let agent = registry.get_agent("code-reviewer").await.unwrap();
```

### 创建变体

```rust
let variant = AgentVariant {
    name: "quick".to_string(),
    description: "Quick code review".to_string(),
    extends: None,
    overrides: VariantOverrides::default(),
};
registry.create_variant("code-reviewer", variant).await.unwrap();
```

### 获取变体

```rust
let variant = registry.get_variant("code-reviewer", "quick").await.unwrap();
```

### 列出变体

```rust
let variants = registry.list_variants("code-reviewer").await.unwrap();
```

### 加载 Agent 定义（支持变体解析）

```rust
// 不指定变体
let agent = registry.load_agent_definition("code-reviewer", None).await.unwrap();

// 指定变体
let agent = registry.load_agent_definition("code-reviewer", Some("quick")).await.unwrap();
```

### 验证 Agent

```rust
let result = registry.validate_agent("code-reviewer").await.unwrap();
if result.valid {
    println!("Agent is valid");
}
```

### 解析 Agent 引用

```rust
let ref = ResolvedAgentRef::parse("code-reviewer:quick").unwrap();
assert_eq!(ref.agent_id, "code-reviewer");
assert_eq!(ref.variant, Some("quick".to_string()));
```

### 列出所有 Agent

```rust
let agents = registry.list_all_agents().await.unwrap();
```

### 删除变体

```rust
registry.delete_variant("code-reviewer", "quick").await.unwrap();
```

## 完整示例

```rust
use agent_variants::{AgentVariantRegistryImpl, AgentDefinition, AgentVariant, VariantOverrides, ModelConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = AgentVariantRegistryImpl::new();

    // Register base agent
    let mut base = AgentDefinition::new(
        "code-reviewer".to_string(),
        "Code Reviewer".to_string(),
        "reviewing code".to_string(),
    );
    base.model = ModelConfig {
        provider: "anthropic".to_string(),
        model: "claude-sonnet".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
    };
    base.instructions = "Review code for bugs and security issues".to_string();

    registry.register_agent(base).await?;

    // Register quick variant with overrides
    let quick_variant = AgentVariant {
        name: "quick".to_string(),
        description: "Quick code review".to_string(),
        extends: None,
        overrides: VariantOverrides {
            model: Some(ModelConfig {
                provider: "anthropic".to_string(),
                model: "claude-haiku".to_string(),
                temperature: 0.1,
                max_tokens: 2048,
            }),
            instructions: Some("Quick review only".to_string()),
            tools: Some(vec!["read".to_string()]),
            skills: None,
            capabilities: None,
            permissions: None,
        },
    };

    registry.create_variant("code-reviewer", quick_variant).await?;

    // Load agent with variant
    let resolved = registry.load_agent_definition("code-reviewer", Some("quick")).await?;
    println!("Loaded variant: {} using model: {}", resolved.variant.unwrap(), resolved.model.model);

    Ok(())
}
```

## 错误处理

AgentVariantError 枚举定义：
- `NotFound(String)`: 变体未找到
- `RegistrationFailed(String)`: 注册失败
- `ValidationFailed(String)`: 验证失败
- `AgentNotFound(String)`: Agent 未找到
- `InvalidReference(String)`: 无效的引用
