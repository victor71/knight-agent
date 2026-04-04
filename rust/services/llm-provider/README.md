# LLM Provider Module

LLM 提供商抽象层，支持 OpenAI 和 Anthropic 协议。配置驱动设计允许灵活设置提供商。

Design Reference: `docs/03-module-design/services/llm-provider.md`

## 特性

- 统一的大模型调用接口
- OpenAI 和 Anthropic 协议支持
- 配置驱动的提供商设置
- 流式响应支持
- Token 计数和成本估算
- 模型信息和健康检查

## 依赖

```toml
[dependencies]
llm-provider = { path = "./rust/services/llm-provider" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## 快速开始

```rust
use std::collections::HashMap;
use llm_provider::{
    LLMProvider, ChatCompletionRequest, Message, MessageRole, Content,
    provider::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 OpenAI 协议提供商
    let mut model_pricing = HashMap::new();
    model_pricing.insert("gpt-4o".to_string(), ModelPricing::new(2.5, 10.0));

    let openai_config = ProviderConfig {
        name: "openai".to_string(),
        api_key: "your-api-key".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        protocol: LLMProtocol::OpenAI,
        models: vec!["gpt-4o".to_string()],
        default_model: Some("gpt-4o".to_string()),
        timeout_secs: 120,
        model_pricing,
    };
    let provider = GenericLLMProvider::new(openai_config)?;

    // 构建请求
    let request = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: Some(Content::Text("Hello!".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: 0.7,
        max_tokens: 1024,
        ..Default::default()
    };

    // 发送请求
    let response = provider.chat_completion(request).await?;
    println!("Response: {}", response.content.unwrap());

    // 使用实际 usage 计算成本
    let actual_cost = provider.calculate_cost(&response.usage, &response.model).await?;
    println!("Cost: ${:.6}", actual_cost.total_cost);

    Ok(())
}
```

## API 接口

### 提供商创建

```rust
use std::collections::HashMap;
use provider::{GenericLLMProvider, LLMProtocol, ModelPricing, ProviderConfig};

// OpenAI 协议提供商
let mut model_pricing = HashMap::new();
model_pricing.insert("gpt-4o".to_string(), ModelPricing::new(2.5, 10.0));
model_pricing.insert("gpt-4o-mini".to_string(), ModelPricing::new(0.15, 0.6));

let openai_config = ProviderConfig {
    name: "openai".to_string(),
    api_key: "your-api-key".to_string(),
    base_url: "https://api.openai.com/v1".to_string(),
    protocol: LLMProtocol::OpenAI,
    models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
    default_model: Some("gpt-4o".to_string()),
    timeout_secs: 120,
    model_pricing,
};
let provider = GenericLLMProvider::new(openai_config)?;

// Anthropic 协议提供商
let mut model_pricing = HashMap::new();
model_pricing.insert("claude-sonnet-4-6".to_string(), ModelPricing::new(3.0, 15.0));

let anthropic_config = ProviderConfig {
    name: "anthropic".to_string(),
    api_key: "your-api-key".to_string(),
    base_url: "https://api.anthropic.com".to_string(),
    protocol: LLMProtocol::Anthropic,
    models: vec!["claude-sonnet-4-6".to_string()],
    default_model: Some("claude-sonnet-4-6".to_string()),
    timeout_secs: 120,
    model_pricing,
};
let provider = GenericLLMProvider::new(anthropic_config)?;

// 从环境变量创建
let provider = GenericLLMProvider::from_env()?;
```

### ProviderConfig 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | String | 提供商名称 |
| `api_key` | String | API 密钥 |
| `base_url` | String | API 端点基础 URL |
| `protocol` | LLMProtocol | 协议类型 (OpenAI 或 Anthropic) |
| `models` | Vec<String> | 支持的模型列表 |
| `default_model` | Option<String> | 默认模型 |
| `timeout_secs` | u64 | 请求超时时间 (秒) |
| `model_pricing` | HashMap<String, ModelPricing> | 模型定价配置 |

**ModelPricing 结构：**

```rust
pub struct ModelPricing {
    pub input: f64,  // 每 1M 输入 tokens 的价格 (USD)
    pub output: f64, // 每 1M 输出 tokens 的价格 (USD)
}
```

**定价优先级：**
1. 用户在 `model_pricing` 中配置的价格
2. 常见模型的默认定价（gpt-4o, claude-sonnet-4-6 等）
3. 未配置且无默认定价的模型返回 0

### LLMProtocol 枚举

```rust
pub enum LLMProtocol {
    OpenAI,   // OpenAI 协议
    Anthropic, // Anthropic 协议
}
```

### 聊天补全

```rust
// 非流式聊天补全
async fn chat_completion(
    &self,
    request: ChatCompletionRequest,
) -> LLMResult<ChatCompletionResponse>
```

**ChatCompletionRequest 字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `model` | String | 模型名称 |
| `messages` | Vec<Message> | 消息列表 |
| `temperature` | f32 | 温度参数 (默认 0.7) |
| `max_tokens` | u32 | 最大输出 Token (默认 4096) |
| `top_p` | f32 | Top-p 采样 (默认 1.0) |
| `stop` | Option<Vec<String>> | 停止序列 |
| `tools` | Option<Vec<ToolDefinition>> | 工具定义 |
| `stream` | bool | 是否流式输出 |

### 流式聊天补全

```rust
// 流式聊天补全
async fn stream_completion(
    &self,
    request: ChatCompletionRequest,
) -> LLMResult<CompletionStream>

// CompletionStream 是一个异步流
use futures::StreamExt;
let mut stream = provider.stream_completion(request).await?;
while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result?;
    println!("Delta: {:?}", chunk.content);
}
```

### Token 和成本

```rust
// 估算成本（调用 API 前）
async fn estimate_cost(
    &self,
    request: &ChatCompletionRequest,
) -> LLMResult<CostEstimate>

// 根据实际 usage 计算成本（调用 API 后）
async fn calculate_cost(
    &self,
    usage: &Usage,
    model: &str,
) -> LLMResult<CostEstimate>

// CostEstimate
pub struct CostEstimate {
    pub input_cost: f64,      // 输入成本 (USD)
    pub output_cost: f64,     // 输出成本 (USD)
    pub total_cost: f64,      // 总成本
    pub currency: String,     // 货币 (默认 USD)
}

// Usage（API 响应中的实际 token 使用量）
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}
```

**使用示例：**

```rust
// 1. 调用前预估
let estimated = provider.estimate_cost(&request).await?;

// 2. 实际调用 API
let response = provider.chat_completion(request).await?;

// 3. 调用后用实际 usage 计算成本
let actual = provider.calculate_cost(&response.usage, &response.model).await?;
```

### 模型信息

```rust
// 列出可用模型
async fn list_models(&self) -> LLMResult<Vec<String>>

// 获取模型信息
async fn get_model_info(&self, model: &str) -> LLMResult<ModelInfo>

// ModelInfo
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: u32,
    pub pricing: Pricing,
    pub capabilities: Vec<String>,
}
```

### 健康检查

```rust
// 检查提供商健康状态
async fn health_check(&self) -> LLMResult<ProviderStatus>

// ProviderStatus
pub struct ProviderStatus {
    pub name: String,
    pub healthy: bool,
    pub latency_ms: u64,
    pub error_rate: f64,
    pub last_check: String,
}
```

## 消息类型

### MessageRole

```rust
pub enum MessageRole {
    System,    // 系统消息
    User,      // 用户消息
    Assistant, // 助手消息
    Tool,      // 工具消息
}
```

### Content

```rust
// 文本内容
Content::Text("Hello".to_string())

// 多模态内容块
Content::Blocks(vec![
    ContentBlock::Text { text: "...".to_string() },
    ContentBlock::Image { image_url: ImageUrl { url: "...".to_string(), detail: None } },
])
```

## 错误类型

```rust
pub enum LLMError {
    NotInitialized,              // 提供商未初始化
    InferenceFailed(String),     // 推理失败
    ModelNotFound(String),       // 模型不存在
    ProviderNotFound(String),    // 提供商不存在
    RateLimitExceeded,          // 超过速率限制
    ContextLengthExceeded,      // 超过上下文长度
    InvalidRequest(String),      // 无效请求
    Timeout,                    // 超时
    ApiKeyInvalid,              // API 密钥无效
}
```

## 支持的协议

### OpenAI 协议

兼容任何实现 OpenAI Chat Completions API 的服务。

| 配置项 | 值 |
|--------|-----|
| base_url | https://api.openai.com/v1 |
| 协议 | LLMProtocol::OpenAI |

常用模型:

| 模型 | 上下文长度 | 输入价格 | 输出价格 |
|------|-----------|----------|----------|
| gpt-4o | 128K | $2.50/M | $10.00/M |
| gpt-4o-mini | 128K | $0.15/M | $0.60/M |
| gpt-4-turbo | 128K | $10.00/M | $30.00/M |

### Anthropic 协议

| 配置项 | 值 |
|--------|-----|
| base_url | https://api.anthropic.com |
| 协议 | LLMProtocol::Anthropic |

常用模型:

| 模型 | 上下文长度 | 输入价格 | 输出价格 |
|------|-----------|----------|----------|
| claude-sonnet-4-6 | 200K | $3.00/M | $15.00/M |
| claude-haiku | 200K | $0.25/M | $1.25/M |

> 价格为每 1M tokens 的美元价格

## 环境变量

使用 `from_env()` 创建提供商时，从以下环境变量读取配置：

| 环境变量 | 说明 |
|----------|------|
| LLM_API_KEY | API 密钥 |
| LLM_BASE_URL | API 基础 URL (默认 https://api.openai.com/v1) |
| LLM_PROTOCOL | 协议类型 (openai 或 anthropic，默认 openai) |
| LLM_MODELS | 支持的模型列表 (逗号分隔) |
| LLM_DEFAULT_MODEL | 默认模型 |
