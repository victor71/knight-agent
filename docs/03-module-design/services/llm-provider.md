# LLM Provider (LLM 提供者抽象层)

## 1. 概述

### 1.1 职责描述

LLM Provider 提供统一的 LLM 调用接口，支持多个 LLM 提供商，包括：

- 统一的聊天补全接口
- 流式响应支持
- 多云支持和模型路由
- Token 计数和成本估算
- 错误处理和降级策略

### 1.2 设计目标

1. **统一接口**: 隐藏不同提供商的差异
2. **多云支持**: 支持多个 LLM 提供商
3. **智能路由**: 根据任务需求选择最优模型
4. **成本优化**: Token 计数和成本估算

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Hook Engine | 可选 | prompt 构建钩子 |
| Config Service | 依赖 | 获取提供商配置 |

---

## 2. 接口定义

### 2.1 对外接口

```yaml
# LLM Provider 接口定义
LLMProvider:
  # ========== 聊天补全 ==========
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

  # ========== Token 管理 ==========
  count_tokens:
    description: 计算 Token 数量
    inputs:
      text:
        type: string
        required: true
      model:
        type: string
        description: 模型名称（影响 Token 计算）
        required: false
        default: "claude-sonnet-4-6"
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
        description: 成本估算（USD）

  # ========== 模型管理 ==========
  list_models:
    description: 列出可用模型
    inputs:
      provider:
        type: string
        description: 提供商过滤
        required: false
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

  # ========== 路由和降级 ==========
  route_request:
    description: 智能路由请求到最优模型
    inputs:
      request:
        type: ChatCompletionRequest
        required: true
      strategy:
        type: string
        description: 路由策略 (cost/quality/speed/auto)
        required: false
        default: "auto"
    outputs:
      routed_request:
        type: RoutedRequest
        description: 路由后的请求

  # ========== 提供商管理 ==========
  add_provider:
    description: 添加自定义提供商
    inputs:
      provider:
        type: ProviderConfig
        required: true
    outputs:
      success:
        type: boolean

  remove_provider:
    description: 移除提供商
    inputs:
      name:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 健康检查 ==========
  health_check:
    description: 检查提供商健康状态
    inputs:
      provider:
        type: string
        description: 提供商名称，为空则检查所有
        required: false
    outputs:
      status:
        type: map<string, ProviderStatus>
```

### 2.2 数据结构

```yaml
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
    description: 温度参数
    default: 0.7
  max_tokens:
    type: integer
    description: 最大输出 Token
    default: 4096
  top_p:
    type: float
    description: Top-p 采样
    default: 1.0
  stop:
    type: array<string>
    description: 停止序列
  tools:
    type: array<ToolDefinition>
    description: 可用工具定义
  tool_choice:
    type: string | object
    description: 工具选择策略
  stream:
    type: boolean
    description: 是否使用流式输出
    default: false

# 消息结构
Message:
  role:
    type: enum
    values: [system, user, assistant]
  content:
    type: string | array<ContentBlock>
  tool_calls:
    type: array<ToolCall>
    description: 工具调用（仅 assistant）
  tool_call_id:
    type: string
    description: 工具调用 ID（仅 tool）

# 内容块（多模态）
ContentBlock:
  type:
    type: enum
    values: [text, image]
  text:
    type: string
    description: 文本内容
  image_url:
    type: object
    description: 图片 URL

# 工具定义
ToolDefinition:
  type:
    type: string
    description: 函数类型
  function:
    type: object
    description: 函数定义
    properties:
      name:
        type: string
      description:
        type: string
      parameters:
        type: object
        description: JSON Schema

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
        description: JSON 字符串

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
  created:
    type: integer

# Choice
Choice:
  index:
    type: integer
  message:
    type: Message
  finish_reason:
    type: string
    description: stop/length/tool_calls

# Usage
Usage:
  prompt_tokens:
    type: integer
  completion_tokens:
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
  usage:
    type: Usage | null
  delta:
    type: Delta
    description: 增量内容

# ChoiceChunk
ChoiceChunk:
  index:
    type: integer
  delta:
    type: Delta
  finish_reason:
    type: string | null

# Delta
Delta:
  role:
    type: string | null
  content:
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
    default: "USD"

# 模型信息
ModelInfo:
  id:
    type: string
  name:
    type: string
  provider:
    type: string
  context_length:
    type: integer
    description: 上下文长度
  pricing:
    type: Pricing
  capabilities:
    type: array<string>
    description: 支持的能力

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
    default: "USD"

# 提供商配置
ProviderConfig:
  name:
    type: string
  provider_type:
    type: enum
    values: [anthropic, openai, custom]
  api_key:
    type: string
  base_url:
    type: string
  models:
    type: array<string>
  timeout:
    type: integer
    description: 超时时间（秒）
  max_retries:
    type: integer

# 路由请求
RoutedRequest:
  original_request:
    type: ChatCompletionRequest
  routed_model:
    type: string
  routed_provider:
    type: string
  reason:
    type: string
    description: 路由原因

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

### 2.3 配置选项

```yaml
# config/llm.yaml
llm:
  # 默认配置
  default:
    provider: anthropic
    model: claude-sonnet-4-6
    temperature: 0.7
    max_tokens: 8192

  # 提供商配置
  providers:
    anthropic:
      enabled: true
      api_key: ${ANTHROPIC_API_KEY}
      base_url: https://api.anthropic.com
      timeout: 60
      max_retries: 3

    openai:
      enabled: true
      api_key: ${OPENAI_API_KEY}
      base_url: https://api.openai.com/v1
      timeout: 60
      max_retries: 3

    custom:
      enabled: false
      base_url: ${CUSTOM_LLM_URL}
      api_key: ${CUSTOM_LLM_KEY}
      compatible_with: openai

  # 模型路由
  routing:
    enabled: true
    strategy: auto
    rules:
      - name: cost_optimized
        condition:
          task_complexity: low
        route:
          provider: anthropic
          model: claude-haiku

      - name: quality_first
        condition:
          task_complexity: high
        route:
          provider: anthropic
          model: claude-sonnet-4-6

  # 降级策略
  fallback:
    enabled: true
    chain:
      - provider: anthropic
        model: claude-sonnet-4-6
      - provider: anthropic
        model: claude-haiku
      - provider: openai
        model: gpt-4o-mini
    max_attempts: 3
    retry_delay: 1000

  # 成本追踪
  cost_tracking:
    enabled: true
    budget_limit: 100.0
    alert_threshold: 0.8
```

---

## 3. 核心流程

### 3.1 聊天补全流程

```
接收补全请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 触发 prompt_build hook    │
│    - 允许修改 prompt         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 模型路由（可选）          │
│    - 分析请求复杂度          │
│    - 选择最优模型            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. Token 计数和成本估算      │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 调用提供商 API            │
│    - 构建 HTTP 请求          │
│    - 发送请求                │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 5. 降级处理      │   │ 6. 解析响应  │
│    - 尝试备用    │   │    记录使用  │
│    - 记录错误    │   └──────────────┘
└──────────────────┘          │
        │                     ▼
        ▼              ┌──────────────┐
┌──────────────────┐   │ 7. 触发      │
│ 6. 返回错误      │   │ response hook│
└──────────────────┘   └──────────────┘
        │                     │
        ▼                     ▼
    返回错误          返回响应
```

### 3.2 流式响应处理

```
流式请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 建立连接                  │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 逐块接收数据              │
│    while (stream) {          │
│      chunk = read_chunk()    │
│      yield chunk             │
│    }                         │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 累积 Usage 信息          │
│    - 最后一块包含完整 usage  │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 3.3 模型路由决策

```
分析请求特征
        │
        ▼
┌──────────────────────────────┐
│ 1. 提取特征                  │
│    - 消息数量                │
│    - 总 Token 数             │
│    - 任务类型                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 评估复杂度                │
│    - low: 简单问答           │
│    - medium: 代码分析        │
│    - high: 复杂推理          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 应用路由规则              │
│    - 成本优化规则            │
│    - 质量优先规则            │
│    - 自定义规则              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 选择模型                  │
│    - provider: anthropic     │
│    - model: claude-sonnet-4-6│
└──────────────────────────────┘
        │
        ▼
    返回路由结果
```

### 3.4 降级策略

```
请求失败
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查错误类型              │
│    - 可重试错误              │
│    - 不可重试错误            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 可重试？│
    └───┬────┘
        │ 否                │ 是
        ▼                   ▼
┌──────────────────┐   ┌──────────────┐
│ 立即返回错误     │   │ 尝试下一个   │
└──────────────────┘   │ 提供商/模型  │
                       └──────────────┘
                             │
                             ▼
                       ┌──────────────┐
                       │ 还有更多？   │
                       └──────┬───────┘
                          │ 是      │ 否
                          ▼         ▼
                     ┌──────────┐ ┌────────────┐
                     │ 重试     │ │ 返回最终   │
                     │          │ │ 错误       │
                     └──────────┘ └────────────┘
```

---

## 4. 模块交互

### 4.1 依赖关系图

```
┌─────────────────────────────────────────┐
│           LLM Provider                  │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Router    │  │Fallback  │  │Monitor ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Anthropic │  │OpenAI    │  │Custom    │
│Provider  │  │Provider  │  │Provider  │
└──────────┘  └──────────┘  └──────────┘
      │              │              │
      └──────────────┴──────────────┘
                     │
                     ▼
              ┌──────────────┐
              │ HTTP Client  │
              └──────────────┘
```

### 4.2 消息流

```
Agent Runtime
    │
    ▼
┌─────────────────────────────┐
│ LLM Provider                │
│ - 接收请求                  │
│ - 路由模型                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Specific Provider           │
│ - 构建 API 请求             │
│ - 调用 HTTP                 │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ LLM API                     │
│ - Anthropic / OpenAI        │
└─────────────────────────────┘
        │
        ▼
    返回响应
        │
        ▼
┌─────────────────────────────┐
│ LLM Provider                │
│ - 解析响应                  │
│ - 记录使用                  │
└─────────────────────────────┘
        │
        ▼
    Agent Runtime
```

---

## 5. 配置与部署

### 5.1 配置文件格式

```yaml
# config/llm.yaml
llm:
  default:
    provider: anthropic
    model: claude-sonnet-4-6
    temperature: 0.7
    max_tokens: 8192

  providers:
    anthropic:
      enabled: true
      api_key: ${ANTHROPIC_API_KEY}
      base_url: https://api.anthropic.com
      timeout: 60
      max_retries: 3

    openai:
      enabled: true
      api_key: ${OPENAI_API_KEY}
      base_url: https://api.openai.com/v1
      timeout: 60
      max_retries: 3

  routing:
    enabled: true
    strategy: auto

  fallback:
    enabled: true
    max_attempts: 3

  cost_tracking:
    enabled: true
    budget_limit: 100.0
```

### 5.2 环境变量

```bash
# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_BASE_URL="https://api.anthropic.com"

# OpenAI
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://api.openai.com/v1"

# 自定义提供商
export CUSTOM_LLM_URL="https://..."
export CUSTOM_LLM_KEY="..."

# 路由配置
export KNIGHT_LLM_ROUTING_ENABLED="true"
export KNIGHT_LLM_DEFAULT_PROVIDER="anthropic"
export KNIGHT_LLM_DEFAULT_MODEL="claude-sonnet-4-6"
```

### 5.3 部署考虑

1. **API 密钥管理**: 使用环境变量或密钥管理服务
2. **速率限制**: 监控 API 调用频率，避免超限
3. **成本控制**: 设置预算限制和告警
4. **高可用**: 配置多个提供商作为降级

---

## 6. 示例

### 6.1 使用场景

#### 场景 1: 基础聊天补全

```python
# 伪代码
response = llm_provider.chat_completion(
    request={
        "model": "claude-sonnet-4-6",
        "messages": [
            {"role": "user", "content": "你好"}
        ],
        "max_tokens": 100
    }
)
print(response.choices[0].message.content)
```

#### 场景 2: 流式响应

```python
# 伪代码
async for chunk in llm_provider.stream_completion(
    request={
        "model": "claude-sonnet-4-6",
        "messages": messages
    }
):
    print(chunk.delta.content, end="", flush=True)
```

#### 场景 3: 工具调用

```python
# 伪代码
response = llm_provider.chat_completion(
    request={
        "model": "claude-sonnet-4-6",
        "messages": messages,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "读取文件内容",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        },
                        "required": ["path"]
                    }
                }
            }
        ]
    }
)
```

### 6.2 配置示例

#### 开发环境

```yaml
llm:
  default:
    model: claude-haiku  # 更低成本
  providers:
    anthropic:
      timeout: 120
```

#### 生产环境

```yaml
llm:
  default:
    model: claude-sonnet-4-6
  fallback:
    enabled: true
    chain:
      - provider: anthropic
        model: claude-sonnet-4-6
      - provider: openai
        model: gpt-4o-mini
```

---

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| TTFB | < 2s | 首字节时间 |
| 流式延迟 | < 500ms | 块间延迟 |
| Token 计数 | < 10ms | 单次计数 |
| API 调用成功率 | > 99.9% | 含降级 |

### 7.2 错误处理

```yaml
error_codes:
  PROVIDER_NOT_FOUND:
    code: 404
    message: "提供商不存在"
    action: "检查提供商配置"

  MODEL_NOT_FOUND:
    code: 404
    message: "模型不存在"
    action: "检查模型名称"

  API_KEY_INVALID:
    code: 401
    message: "API 密钥无效"
    action: "检查 API 密钥"

  RATE_LIMIT_EXCEEDED:
    code: 429
    message: "超过速率限制"
    action: "等待后重试"
    retryable: true

  CONTEXT_LENGTH_EXCEEDED:
    code: 400
    message: "超过上下文长度"
    action: "减少输入内容"

  PROVIDER_UNAVAILABLE:
    code: 503
    message: "提供商不可用"
    action: "尝试降级提供商"
    retryable: true
```

### 7.3 支持的模型

#### Anthropic

| 模型 | 上下文 | 输入价格 | 输出价格 |
|------|--------|----------|----------|
| claude-sonnet-4-6 | 200K | $3.00 | $15.00 |
| claude-haiku | 200K | $0.25 | $1.25 |

#### OpenAI

| 模型 | 上下文 | 输入价格 | 输出价格 |
|------|--------|----------|----------|
| gpt-4o | 128K | $2.50 | $10.00 |
| gpt-4o-mini | 128K | $0.15 | $0.60 |

> 价格为每 1M tokens 的美元价格
