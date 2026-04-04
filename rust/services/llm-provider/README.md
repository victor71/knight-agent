# LLM Provider Module

LLM 提供商抽象层，提供统一的接口支持 Anthropic Claude 和 OpenAI GPT 系列模型。

Design Reference: `docs/03-module-design/services/llm-provider.md`

## 特性

- 统一的大模型调用接口
- 支持 Anthropic 和 OpenAI 提供商
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
use llm_provider::{
    LLMProvider, ChatCompletionRequest, Message, MessageRole, Content,
    provider::AnthropicProvider,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Anthropic 提供商
    let provider = AnthropicProvider::new("your-api-key")?;

    // 构建请求
    let request = ChatCompletionRequest {
        model: "claude-sonnet-4-6".to_string(),
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

    Ok(())
}
```

## API 接口

### 提供商创建

```rust
// Anthropic 提供商
let provider = AnthropicProvider::new("api-key")?;

// OpenAI 提供商
let provider = OpenAIProvider::new("api-key")?;
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
// 估算成本
async fn estimate_cost(
    &self,
    request: &ChatCompletionRequest,
) -> LLMResult<CostEstimate>

// CostEstimate
pub struct CostEstimate {
    pub input_cost: f64,      // 输入成本 (USD)
    pub output_cost: f64,     // 输出成本 (USD)
    pub total_cost: f64,      // 总成本
    pub currency: String,     // 货币 (默认 USD)
}
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
    ProviderNotFound(String),   // 提供商不存在
    RateLimitExceeded,          // 超过速率限制
    ContextLengthExceeded,      // 超过上下文长度
    InvalidRequest(String),      // 无效请求
    Timeout,                    // 超时
    ApiKeyInvalid,              // API 密钥无效
}
```

## 支持的模型

### Anthropic

| 模型 | 上下文长度 | 输入价格 | 输出价格 |
|------|-----------|----------|----------|
| claude-sonnet-4-6 | 200K | $3.00/M | $15.00/M |
| claude-haiku | 200K | $0.25/M | $1.25/M |

### OpenAI

| 模型 | 上下文长度 | 输入价格 | 输出价格 |
|------|-----------|----------|----------|
| gpt-4o | 128K | $2.50/M | $10.00/M |
| gpt-4o-mini | 128K | $0.15/M | $0.60/M |
| gpt-4-turbo | 128K | $10.00/M | $30.00/M |

> 价格为每 1M tokens 的美元价格
