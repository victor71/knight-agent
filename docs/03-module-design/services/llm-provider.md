# LLM Provider (LLM 提供者抽象层)

## 概述

### 职责描述

LLM Provider 提供统一的 LLM 调用接口，支持 OpenAI 和 Anthropic 两种协议。设计采用配置驱动方式，通过配置文件定义多模型服务。

### 设计目标

1. **协议抽象**: 统一接口，支持 OpenAI 和 Anthropic 协议
2. **配置驱动**: 多模型配置在配置文件中管理
3. **环境变量作为默认服务**: 配置文件找不到时，使用环境变量作为 fallback
4. **成本优化**: Token 计数和成本估算

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Config Service | 依赖 | 获取提供商配置（可选） |
| Hook Engine | 可选 | prompt 构建钩子 |

---

## 架构设计

### 核心概念

```
┌─────────────────────────────────────────────────────────────┐
│                      LLM Provider                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    LLMRouter                         │   │
│  │  - 路由请求到合适的 Provider 基于模型名称            │   │
│  │  - 持有多个 Provider 实例                           │   │
│  │  - 实现 LLMProvider trait                           │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                               │
│          ┌─────────────────┼─────────────────┐             │
│          ▼                 ▼                 ▼             │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │ Provider A   │  │ Provider B   │  │ Provider C   │   │
│  │ (OpenAI)     │  │ (Anthropic)  │  │ (Custom)     │   │
│  └───────────────┘  └───────────────┘  └───────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              GenericLLMProvider                      │   │
│  │  ┌─────────────────┐  ┌─────────────────────────┐  │   │
│  │  │ ProviderConfig  │  │ LLMProtocol (枚举)      │  │   │
│  │  │  - name         │  │   - OpenAI              │  │   │
│  │  │  - api_key      │  │   - Anthropic          │  │   │
│  │  │  - base_url     │  └─────────────────────────┘  │   │
│  │  │  - protocol     │                               │   │
│  │  │  - models[]     │  ┌─────────────────────────┐  │   │
│  │  │  - default_model│  │ ModelConfig (每模型)     │  │   │
│  │  └─────────────────┘  │  - context_length      │  │   │
│  └─────────────────────────│  - pricing            │  │   │
│                              │  - capabilities      │  │   │
│                              └─────────────────────────┘  │   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Protocol Adapter (协议适配层)                        │   │
│  │  - OpenAI: /v1/chat/completions                   │   │
│  │  - Anthropic: /messages                            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### ProviderConfig 配置结构

```yaml
# 配置文件 config/llm.yaml
providers:
  # OpenAI 协议提供商
  openai_prod:
    name: openai_prod
    type: openai                    # 协议类型
    api_key: ${OPENAI_API_KEY}
    base_url: https://api.openai.com/v1
    timeout_secs: 120
    models:
      - id: gpt-4o
        context_length: 128000
        pricing:
          input: 2.50    # 每 1M tokens
          output: 10.00
        capabilities: [chat, tools]
      - id: gpt-4o-mini
        context_length: 128000
        pricing:
          input: 0.15
          output: 0.60
        capabilities: [chat, tools]
      - id: gpt-4-turbo
        context_length: 128000
        pricing:
          input: 10.00
          output: 30.00
        capabilities: [chat, tools]
    default_model: gpt-4o

  # Anthropic 协议提供商
  anthropic_prod:
    name: anthropic_prod
    type: anthropic                 # 协议类型
    api_key: ${ANTHROPIC_API_KEY}
    base_url: https://api.anthropic.com
    timeout_secs: 120
    models:
      - id: claude-sonnet-4-6
        context_length: 200000
        pricing:
          input: 3.00
          output: 15.00
        capabilities: [chat, tools]
      - id: claude-haiku
        context_length: 200000
        pricing:
          input: 0.25
          output: 1.25
        capabilities: [chat, tools]
    default_model: claude-sonnet-4-6

  # 自定义协议提供商（兼容 OpenAI 协议）
  custom_provider:
    name: custom_provider
    type: openai                    # 可选 openai 或 anthropic
    api_key: ${CUSTOM_API_KEY}
    base_url: https://custom-llm.example.com/v1
    timeout_secs: 120
    models:
      - id: custom-model-1
        context_length: 128000
        pricing:
          input: 1.00
          output: 2.00
        capabilities: [chat]
    default_model: custom-model-1

# 默认提供商
default_provider: anthropic_prod
```

### 环境变量（默认模型服务）

当配置文件中找不到对应的模型配置时，使用环境变量作为 fallback：

```bash
# LLM 默认服务配置（当配置文件缺失时使用）
LLM_API_KEY=sk-...                    # API 密钥
LLM_BASE_URL=https://api.openai.com/v1  # 基础 URL
LLM_PROTOCOL=openai                    # 协议类型: openai 或 anthropic
LLM_MODELS=gpt-4o,gpt-4o-mini        # 模型列表（逗号分隔）
LLM_DEFAULT_MODEL=gpt-4o              # 默认模型
```

**优先级**:
1. 配置文件中的模型配置（高优先级）
2. 环境变量作为 fallback（低优先级）

### LLMRouter 路由机制

LLMRouter 是多提供商路由层，根据模型名称自动选择合适的 Provider：

```rust
// LLMRouter 核心方法
impl LLMRouter {
    /// 根据模型ID获取对应的Provider
    fn get_provider_for_model(&self, model: &str) -> Option<Arc<dyn LLMProvider>>;

    /// 获取所有可用模型列表
    fn models(&self) -> Vec<ModelInfo>;

    /// 添加Provider到路由表
    fn add_provider(&mut self, provider: Arc<dyn LLMProvider>);
}
```

**路由规则**:
1. LLMRouter 维护 `model -> Provider` 的映射表
2. 调用 `chat_completion` 或 `stream_completion` 时，根据请求中的 model 字段选择 Provider
3. 如果模型未注册，返回 `ModelNotFound` 错误
4. Provider 由配置初始化时自动注册

---

## 接口定义

### 对外接口

```yaml
LLMProvider:
  chat_completion:
    description: 非流式聊天补全
    inputs:
      request:
        type: ChatCompletionRequest
        required: true
    outputs:
      response:
        type: ChatCompletionResponse

  stream_completion:
    description: 流式聊天补全
    inputs:
      request:
        type: ChatCompletionRequest
        required: true
    outputs:
      stream:
        type: async_stream<ChatCompletionChunk>

  count_tokens:
    description: 计算 Token 数量
    inputs:
      text:
        type: string
        required: true
      model:
        type: string
        required: false
    outputs:
      count:
        type: integer

  estimate_cost:
    description: 估算调用成本
    inputs:
      request:
        type: ChatCompletionRequest
        required: true
    outputs:
      cost:
        type: CostEstimate

  list_models:
    description: 列出可用模型
    inputs: {}
    outputs:
      models:
        type: array<ModelInfo>

  get_model_info:
    description: 获取模型详细信息
    inputs:
      model:
        type: string
        required: true
    outputs:
      info:
        type: ModelInfo

  health_check:
    description: 检查提供商健康状态
    inputs: {}
    outputs:
      status:
        type: ProviderStatus
```

### 数据结构

```yaml
# LLM 协议类型
LLMProtocol:
  type: enum
  values: [openai, anthropic]
  description: |
    - openai: OpenAI 协议 (/v1/chat/completions)
    - anthropic: Anthropic 协议 (/messages)

# 提供商配置
ProviderConfig:
  name:
    type: string
    description: 提供商名称（唯一标识）
  api_key:
    type: string
    description: API 密钥
  base_url:
    type: string
    description: API 基础 URL
  protocol:
    type: LLMProtocol
    description: 协议类型
  models:
    type: array<ModelConfig>
    description: 支持的模型列表
  default_model:
    type: string
    description: 默认模型 ID
  timeout_secs:
    type: integer
    default: 120
    description: 请求超时（秒）

# 模型配置
ModelConfig:
  id:
    type: string
    description: 模型 ID
  context_length:
    type: integer
    description: 上下文长度（tokens）
  pricing:
    type: Pricing
    description: 定价信息
  capabilities:
    type: array<string>
    description: 支持的能力列表

# 聊天补全请求
ChatCompletionRequest:
  model:
    type: string
    description: 模型名称
  messages:
    type: array<Message>
    description: 消息列表
  temperature:
    type: float
    default: 0.7
  max_tokens:
    type: integer
    default: 4096
  top_p:
    type: float
    default: 1.0
  stop:
    type: array<string>
    description: 停止序列
  tools:
    type: array<ToolDefinition>
    description: 工具定义
  stream:
    type: boolean
    default: false

# 消息结构
Message:
  role:
    type: enum
    values: [system, user, assistant, tool]
  content:
    type: string | array<ContentBlock>
  tool_calls:
    type: array<ToolCall>
  tool_call_id:
    type: string

# 工具定义
ToolDefinition:
  type:
    type: string
    default: function
  function:
    type: object
    properties:
      name:
        type: string
      description:
        type: string
      parameters:
        type: object

# 工具调用
ToolCall:
  id:
    type: string
  type:
    type: string
  function:
    type: object
    properties:
      name:
        type: string
      arguments:
        type: string

# 聊天补全响应
ChatCompletionResponse:
  id:
    type: string
  model:
    type: string
  choices:
    type: array<Choice>
  usage:
    type: Usage

# Choice
Choice:
  index:
    type: integer
  message:
    type: Message
  finish_reason:
    type: string

# Usage
Usage:
  input_tokens:
    type: integer
  output_tokens:
    type: integer
  total_tokens:
    type: integer

# 流式响应块
ChatCompletionChunk:
  id:
    type: string
  model:
    type: string
  choices:
    type: array<ChoiceChunk>

# ChoiceChunk
ChoiceChunk:
  index:
    type: integer
  delta:
    type: Delta
  finish_reason:
    type: string

# Delta
Delta:
  content:
    type: string | null
  role:
    type: string | null
  tool_calls:
    type: array<ToolCall> | null

# 成本估算
CostEstimate:
  input_cost:
    type: float
    description: 输入成本（USD）
  output_cost:
    type: float
    description: 输出成本（USD）
  total_cost:
    type: float
    description: 总成本（USD）
  currency:
    type: string
    default: USD

# 模型信息
ModelInfo:
  id:
    type: string
  name:
    type: string
  provider:
    type: string
    description: 提供商名称
  context_length:
    type: integer
  pricing:
    type: Pricing
  capabilities:
    type: array<string>

# 定价信息
Pricing:
  input:
    type: float
    description: 输入价格（每 1M tokens）
  output:
    type: float
    description: 输出价格（每 1M tokens）
  currency:
    type: string
    default: USD

# 提供商状态
ProviderStatus:
  name:
    type: string
  healthy:
    type: boolean
  latency_ms:
    type: integer
  error_rate:
    type: float
  last_check:
    type: datetime
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/llm.yaml
providers:
  anthropic_prod:
    name: anthropic_prod
    type: anthropic
    api_key: ${ANTHROPIC_API_KEY}
    base_url: https://api.anthropic.com
    timeout_secs: 120
    models:
      - id: claude-sonnet-4-6
        context_length: 200000
        pricing:
          input: 3.00
          output: 15.00
        capabilities: [chat, tools]
      - id: claude-haiku
        context_length: 200000
        pricing:
          input: 0.25
          output: 1.25
        capabilities: [chat, tools]
    default_model: claude-sonnet-4-6

  openai_prod:
    name: openai_prod
    type: openai
    api_key: ${OPENAI_API_KEY}
    base_url: https://api.openai.com/v1
    timeout_secs: 120
    models:
      - id: gpt-4o
        context_length: 128000
        pricing:
          input: 2.50
          output: 10.00
        capabilities: [chat, tools]
      - id: gpt-4o-mini
        context_length: 128000
        pricing:
          input: 0.15
          output: 0.60
        capabilities: [chat, tools]
    default_model: gpt-4o

default_provider: anthropic_prod
```

### 环境变量

```bash
# LLM 默认服务配置（当配置文件缺失时使用）
export LLM_API_KEY="sk-..."                       # API 密钥
export LLM_BASE_URL="https://api.openai.com/v1"   # 基础 URL
export LLM_PROTOCOL="openai"                       # 协议类型: openai 或 anthropic
export LLM_MODELS="gpt-4o,gpt-4o-mini"            # 模型列表
export LLM_DEFAULT_MODEL="gpt-4o"                  # 默认模型
```

**配置优先级**:
1. 配置文件中的模型配置（高优先级）
2. 环境变量作为 fallback（低优先级）

---

## 核心流程

### 聊天补全流程

```
接收补全请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 构建请求                  │
│    - 验证模型是否支持        │
│    - Token 计数              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 调用提供商 API            │
│    - 根据 protocol 构建请求  │
│    - OpenAI: /v1/chat/completions
│    - Anthropic: /messages   │
│    - 发送 HTTP 请求          │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 3. 错误处理      │   │ 4. 解析响应  │
│    - Rate limit  │   │    记录使用  │
│    - Timeout     │   └──────────────┘
│    - 返回错误    │          │
└──────────────────┘          ▼
                       返回响应
```

### 协议适配

```
GenericLLMProvider
        │
        ▼
┌──────────────────────────────┐
│ 根据 protocol 选择适配方式    │
└──────────────────────────────┘
        │
        ├──────────────────┐
        ▼                  ▼
   OpenAI              Anthropic
        │                  │
        ▼                  ▼
┌──────────────┐   ┌──────────────┐
│ Header:       │   │ Header:      │
│ Authorization │   │ x-api-key   │
│ Bearer {key} │   │ {key}        │
├──────────────┤   ├──────────────┤
│ URL:         │   │ URL:         │
│ /chat/completions │ /messages │
├──────────────┤   ├──────────────┤
│ Body:        │   │ Body:        │
│ messages[]   │   │ messages[]   │
│ model        │   │ model        │
│ temperature  │   │ max_tokens   │
│ ...          │   │ temperature  │
└──────────────┘   └──────────────┘
```

---

## 示例

### 使用场景

#### 场景 1: 基础聊天补全

```rust
use llm_provider::{
    LLMProvider, ChatCompletionRequest, Message, MessageRole, Content,
    provider::{GenericLLMProvider, LLMProtocol, ProviderConfig},
};

// 创建 OpenAI 协议提供商
let config = ProviderConfig {
    name: "openai".to_string(),
    api_key: "sk-...".to_string(),
    base_url: "https://api.openai.com/v1".to_string(),
    protocol: LLMProtocol::OpenAI,
    models: vec!["gpt-4o".to_string()],
    default_model: Some("gpt-4o".to_string()),
    timeout_secs: 120,
};
let provider = GenericLLMProvider::new(config)?;

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

let response = provider.chat_completion(request).await?;
println!("Response: {}", response.content.unwrap());
```

#### 场景 2: 流式响应

```rust
use futures::StreamExt;
let mut stream = provider.stream_completion(request).await?;
while let Some(chunk_result) = stream.next().await {
    let chunk = chunk_result?;
    if let Some(content) = chunk.content {
        print!("{}", content);
    }
}
```

### 配置示例

#### 生产环境

```yaml
providers:
  anthropic_prod:
    type: anthropic
    api_key: ${ANTHROPIC_API_KEY}
    models:
      - id: claude-sonnet-4-6
        context_length: 200000
        pricing:
          input: 3.00
          output: 15.00
      - id: claude-haiku
        context_length: 200000
        pricing:
          input: 0.25
          output: 1.25
    default_model: claude-sonnet-4-6

default_provider: anthropic_prod
```

---

## 附录

### 支持的模型

#### OpenAI 协议

| 模型 | 上下文 | 输入价格 | 输出价格 |
|------|--------|----------|----------|
| gpt-4o | 128K | $2.50/M | $10.00/M |
| gpt-4o-mini | 128K | $0.15/M | $0.60/M |
| gpt-4-turbo | 128K | $10.00/M | $30.00/M |

#### Anthropic 协议

| 模型 | 上下文 | 输入价格 | 输出价格 |
|------|--------|----------|----------|
| claude-sonnet-4-6 | 200K | $3.00/M | $15.00/M |
| claude-haiku | 200K | $0.25/M | $1.25/M |

### 错误类型

| 错误类型 | 说明 |
|----------|------|
| NotInitialized | 提供商未初始化 |
| InferenceFailed | 推理失败 |
| ModelNotFound | 模型不存在 |
| RateLimitExceeded | 超过速率限制 |
| ContextLengthExceeded | 超过上下文长度 |
| InvalidRequest | 无效请求 |
| Timeout | 超时 |
| ApiKeyInvalid | API 密钥无效 |

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-30 | 初始版本 |
| 2.0.0 | 2026-04-04 | 改为协议抽象 + 配置驱动设计 |
| 2.1.0 | 2026-04-08 | 添加 LLMRouter 多提供商路由层 |
