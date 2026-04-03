# Context Compressor (上下文压缩服务)

## 概述

### 职责描述

Context Compressor 负责会话上下文的智能压缩，包括：

- 上下文分析和压缩决策
- 多种压缩策略（摘要、语义、混合）
- 压缩点管理和存储
- Token 估算和优化
- 压缩历史追踪

### 设计目标

1. **智能压缩**: 保留关键信息，丢弃冗余
2. **Token 优化**: 最大化 Token 节省
3. **可恢复性**: 压缩内容可追溯
4. **可配置**: 支持多种压缩策略

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取会话上下文 |
| LLM Provider | 依赖 | 生成压缩摘要 |
| Storage Service | 依赖 | 存储压缩点 |

---

## 接口定义

### 对外接口

```yaml
# Context Compressor 接口定义
ContextCompressor:
  # ========== 压缩管理 ==========
  should_compress:
    description: 检查是否需要压缩
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      should:
        type: boolean
      reason:
        type: string

  compress:
    description: |
      压缩会话上下文
      失败处理：
      - 重试次数耗尽：返回 COMPRESSION_RETRY_EXHAUSTED，继续使用原始上下文
      - Token 预算超限：返回 COMPRESSION_TOKEN_LIMIT，继续使用原始上下文
      - 不会无限重试，避免 Token 消耗过量
    inputs:
      session_id:
        type: string
        required: true
      strategy:
        type: string
        enum: [summary, semantic, hybrid]
        description: 压缩策略
        required: false
      options:
        type: CompressionOptions
        required: false
    outputs:
      compression_point:
        type: CompressionPoint
    errors:
      - COMPRESSION_RETRY_EXHAUSTED
      - COMPRESSION_TOKEN_LIMIT

  compress_async:
    description: 异步压缩会话上下文
    inputs:
      session_id:
        type: string
        required: true
      strategy:
        type: string
        required: false
    outputs:
      job_id:
        type: string

  # ========== 压缩点管理 ==========
  get_compression_points:
    description: 获取会话的压缩点
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      points:
        type: array<CompressionPoint>

  get_compression_point:
    description: 获取单个压缩点详情
    inputs:
      point_id:
        type: string
        required: true
    outputs:
      point:
        type: CompressionPoint | null

  restore_compression_point:
    description: 恢复压缩点内容（用于调试）
    inputs:
      point_id:
        type: string
        required: true
    outputs:
      messages:
        type: array<Message>

  delete_compression_point:
    description: 删除压缩点
    inputs:
      point_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== Token 管理 ==========
  estimate_tokens:
    description: 估算消息 Token 数量
    inputs:
      messages:
        type: array<Message>
        required: true
      model:
        type: string
        description: 模型名称（影响 Token 计算）
        required: false
    outputs:
      count:
        type: integer

  get_compression_stats:
    description: 获取压缩统计
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      stats:
        type: CompressionStats

  # ========== 压缩配置 ==========
  get_compression_config:
    description: 获取压缩配置
    inputs:
      session_id:
        type: string
        required: false
    outputs:
      config:
        type: CompressionConfig

  update_compression_config:
    description: 更新压缩配置
    inputs:
      config:
        type: CompressionConfig
        required: true
    outputs:
      success:
        type: boolean
```

### 数据结构

```yaml
# 压缩点
# 注意: CompressionPoint 定义在 Session Manager 模块
# 见 [Session Manager - CompressionPoint](../core/session-manager.md#compressionpoint-数据结构)
CompressionPoint:
  $ref: ../core/session-manager.md#CompressionPoint
```

**共享类型引用**：

| 类型 | 定义位置 | 说明 |
|------|---------|------|
| CompressionPoint | [Session Manager](../core/session-manager.md#compressionpoint-数据结构) | 压缩点完整定义 |
| Message | [Session Manager](../core/session-manager.md#message-数据结构) | 消息结构 |
| Session | [Session Manager](../core/session-manager.md#session-数据结构) | 会话结构 |

  # 元数据
  model:
    type: string
    description: 使用的模型
  token_saved:
    type: integer
    compression_ratio:
    type: float
    description: 压缩比例 (0-1)

# 压缩前状态
CompressionBefore:
  message_count:
    type: integer
  token_count:
    type: integer
  message_range:
    type: object
    properties:
      start:
        type: integer
      end:
        type: integer

# 压缩后状态
CompressionAfter:
  message_count:
    type: integer
  token_count:
    type: integer
  messages:
    type: array<Message>

# 压缩选项
CompressionOptions:
  keep_recent:
    type: integer
    description: 保留最近消息数
    default: 20
  keep_system:
    type: boolean
    description: 保留系统消息
    default: true
  preserve_keys:
    type: array<string>
    description: 保留的关键消息 ID
  min_compression_ratio:
    type: float
    description: 最小压缩比例
    default: 0.3

# 压缩配置
CompressionConfig:
  # 触发条件
  trigger:
    type: CompressionTrigger
  # 默认策略
  default_strategy:
    type: string
    enum: [summary, semantic, hybrid]
  # 默认选项
  default_options:
    type: CompressionOptions

# 压缩触发条件
CompressionTrigger:
  message_count:
    type: integer
    description: 消息数量阈值
    default: 50
  token_count:
    type: integer
    description: Token 数量阈值
    default: 100000
  auto_compress:
    type: boolean
    description: 自动压缩
    default: true

# 压缩统计
CompressionStats:
  total_compressions:
    type: integer
  total_tokens_saved:
    type: integer
  avg_compression_ratio:
    type: float
  last_compression:
    type: datetime | null
  compression_history:
    type: array<CompressionHistoryEntry>

# 压缩历史条目
CompressionHistoryEntry:
  point_id:
    type: string
  timestamp:
    type: datetime
  strategy:
    type: string
  token_saved:
    type: integer
  compression_ratio:
    type: float
```

### 配置选项

```yaml
# config/compression.yaml
compression:
  # 触发条件
  trigger:
    message_count: 50
    token_count: 100000
    auto_compress: true

  # 最近消息保持不变（不压缩）
  keep_recent:
    messages: 20              # 最近 20 条消息不压缩
    tokens: 10000             # 最近 10K tokens 不压缩

  # 内容类型压缩策略
  content_rules:
    # 代码文件 - 尽量不压缩
    code:
      compression: preserve   # preserve=保留原文
      priority: highest
      min_length: 50          # 超过 50 行才考虑压缩
      truncate: false         # 不截断

    # 日志文件 - 去除重复和过多信息
    log:
      compression: prune      # prune=修剪
      priority: low
      max_entries: 100       # 最多保留 100 条
      remove_duplicates: true
      remove_debug: true     # 移除 debug 日志

    # 普通文本 - 正常摘要压缩
    text:
      compression: summary    # summary=摘要
      priority: normal
      max_length: 5000       # 摘要最大长度

    # 配置/数据文件 - 选择性保留
    config:
      compression: preserve
      priority: high
      truncate: false

    # 系统消息 - 保留
    system:
      compression: preserve
      priority: highest

  # Token 预算保护
  token_budget:
    max_summary_tokens: 5000     # 单个摘要最大长度
    max_input_tokens_per_call: 30000  # 单次 LLM 调用输入上限
    max_total_cost: 100000       # 会话累计压缩 Token 上限
    stop_on_limit: true          # 达到上限后停止压缩

  # 重试限制
  retry:
    max_attempts: 3
    backoff: exponential
    initial_delay_ms: 1000
    max_delay_ms: 10000
    fail_on_max_retries: true

  # 默认策略
  default_strategy: summary

  # 模型配置
  models:
    summary:
      model: claude-haiku
      temperature: 0.3
    semantic:
      model: claude-sonnet-4-6
      temperature: 0.2
    hybrid:
      summary_model: claude-haiku
      semantic_model: claude-sonnet-4-6
```

**配置说明**:

| 配置路径 | 说明 | 默认值 |
|---------|------|--------|
| `keep_recent.messages` | 最近 N 条消息不压缩 | 20 |
| `keep_recent.tokens` | 最近 N tokens 不压缩 | 10000 |
| `content_rules.{type}.compression` | 压缩模式 | - |
| `token_budget.max_summary_tokens` | 单个摘要最大长度 | 5000 |
| `token_budget.max_input_tokens_per_call` | 单次 LLM 调用输入上限 | 30000 |
| `token_budget.max_total_cost` | 累计压缩 Token 上限 | 100000 |

**压缩模式说明**:

| 模式 | 说明 | 适用场景 |
|------|------|---------|
| `preserve` | 保留原文，不压缩 | 代码、系统消息、高优先级内容 |
| `summary` | 摘要压缩 | 普通文本消息 |
| `prune` | 修剪/精简 | 日志、重复内容 |
| `truncate` | 截断 | 过长且无法有效压缩的内容 |

**内容类型检测**:

消息的内容类型通过以下方式检测：

```
消息内容
    │
    ▼
┌──────────────────────────────┐
│ 1. 检测消息角色              │
│    - system → system         │
│    - tool → 工具输出         │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 2. 分析消息格式              │
│    - 包含代码块 → code        │
│    - 日志格式 → log          │
│    - JSON/配置 → config     │
│    - 普通文本 → text         │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 应用压缩规则              │
│    - code: preserve         │
│    - log: prune            │
│    - text: summary          │
│    - system: preserve       │
└──────────────────────────────┘
```

**代码块检测示例**：

```python
# 检测代码块的特征
if "```" in content or "def " in content or "class " in content:
    return "code"
if "[LOG]" in content or re.match(r"\d{4}-\d{2}-\d{2}.*DEBUG", content):
    return "log"
if content.strip().startswith("{") and content.strip().endswith("}"):
    return "config"
return "text"
```

---

## 核心流程

### 压缩决策流程

```
检查是否需要压缩
        │
        ▼
┌──────────────────────────────┐
│ 1. 获取当前上下文            │
│    - 消息数量                │
│    - Token 数量              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 比较阈值                  │
│    - message_count >= 阈值   │
│    - token_count >= 阈值     │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 超过？  │
    └───┬────┘
        │ 否
        ▼
    不需要压缩
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 评估压缩收益              │
│    - 预估 Token 节省         │
│    - 预估压缩比例            │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 值得？  │
    └───┬────┘
        │ 否
        ▼
    不执行压缩
        │ 是
        ▼
    建议压缩
```

### 摘要压缩流程

```
开始摘要压缩
        │
        ▼
┌──────────────────────────────┐
│ 1. 选择压缩范围              │
│    - 排除最近 N 条消息       │
│    - 保留系统消息            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 提取关键信息              │
│    - 识别用户意图            │
│    - 提取关键决策            │
│    - 记录工具调用结果        │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 调用 LLM 生成摘要          │
│    prompt: "请将以下对话...  │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 创建压缩点                │
│    - 保存摘要                │
│    - 记录元数据              │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 语义压缩流程

```
开始语义压缩
        │
        ▼
┌──────────────────────────────┐
│ 1. 选择压缩范围              │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 消息聚类                  │
│    - 按主题分组              │
│    - 识别对话阶段            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 提取关键消息              │
│    - 每个主题选择代表消息    │
│    - 保留重要决策点          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 创建压缩点                │
│    - 保留关键消息列表        │
│    - 添加主题摘要            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 混合压缩流程

```
开始混合压缩
        │
        ▼
┌──────────────────────────────┐
│ 1. 第一阶段：摘要压缩        │
│    - 生成整体摘要            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 第二阶段：语义分析        │
│    - 提取关键消息            │
│    - 识别主题                │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 第三阶段：智能合并        │
│    - 合并摘要和关键消息      │
│    - 优化 Token 使用         │
└──────────────────────────────┘
        │
        ▼
    完成
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│        Context Compressor               │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Analyzer  │  │Compressor│  │Manager  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────┬──────────────┬──────────────┬─────┘
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│Session   │  │LLM       │  │Storage   │
│Manager   │  │Provider  │  │Service   │
└──────────┘  └──────────┘  └──────────┘
```

### 消息流

```
Session Manager
    │
    ▼
┌─────────────────────────────┐
│ Context Compressor          │
│ - 检查是否需要压缩          │
│ - 执行压缩                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ LLM Provider                │
│ - 生成摘要                  │
│ - 语义分析                  │
└─────────────────────────────┘
        │
        ▼
┌─────────────────────────────┐
│ Storage Service             │
│ - 保存压缩点                │
└─────────────────────────────┘
        │
        ▼
    返回压缩点
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/compression.yaml
compression:
  # 触发条件
  trigger:
    message_count: 50
    token_count: 100000
    auto_compress: true

  # 默认策略
  default_strategy: summary

  # 默认选项
  default_options:
    keep_recent: 20
    keep_system: true
    min_compression_ratio: 0.3

  # 模型配置
  models:
    summary:
      model: claude-haiku
      temperature: 0.3
      max_tokens: 2000
    semantic:
      model: claude-sonnet-4-6
      temperature: 0.2
      max_tokens: 4000
    hybrid:
      summary_model: claude-haiku
      semantic_model: claude-sonnet-4-6
```

### 环境变量

```bash
# 触发条件
export KNIGHT_COMPRESS_MESSAGE_COUNT=50
export KNIGHT_COMPRESS_TOKEN_COUNT=100000

# 默认策略
export KNIGHT_COMPRESS_DEFAULT_STRATEGY="summary"

# 保留选项
export KNIGHT_COMPRESS_KEEP_RECENT=20
```

---

## 示例

### 压缩配置

```yaml
# 开发环境 - 较低阈值便于测试
compression:
  trigger:
    message_count: 20
    token_count: 50000
  default_strategy: hybrid

# 生产环境 - 更高阈值
compression:
  trigger:
    message_count: 50
    token_count: 100000
  default_strategy: summary
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 摘要压缩 | < 5s | 依赖 LLM |
| 语义压缩 | < 10s | 依赖 LLM |
| 混合压缩 | < 15s | 两阶段压缩 |
| Token 节省 | > 50% | 压缩比例 |

### 错误处理

```yaml
error_codes:
  COMPRESSION_FAILED:
    code: 500
    message: "压缩失败"
    action: "检查 LLM 服务"
    retryable: true

  COMPRESSION_RETRY_EXHAUSTED:
    code: 510
    message: "压缩重试次数已用尽"
    action: "跳过压缩，继续使用原始上下文"
    retryable: false

  COMPRESSION_TOKEN_LIMIT:
    code: 511
    message: "压缩 Token 消耗超出预算"
    action: "停止压缩，使用原始上下文"
    retryable: false

  INSUFFICIENT_CONTEXT:
    code: 400
    message: "上下文不足"
    action: "等待更多消息"
    retryable: false

  LOW_COMPRESSION_RATIO:
    code: 200
    message: "压缩比例过低"
    action: "调整压缩选项"
    retryable: true

  SESSION_NOT_FOUND:
    code: 404
    message: "会话不存在"
    action: "检查会话 ID"
    retryable: false
```

### 压缩失败处理流程

```
压缩请求
    │
    ▼
┌──────────────────────────────┐
│ 1. 执行压缩                  │
│    - 调用 LLM 生成摘要        │
└──────────────────────────────┘
    │
    ▼
    ┌───┴────┐
    │ 成功？  │
    └───┬────┘
        │ 否
        ▼
┌──────────────────────────────┐
│ 2. 检查重试次数              │
│    - current_attempt < max   │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 可重试？│
    └───┬────┘
        │ 否
        ▼
┌──────────────────────────────┐
│ 3. 放弃压缩                  │
│    - 返回 COMPRESSION_RETRY_EXHAUSTED │
│    - 继续使用原始上下文      │
└──────────────────────────────┘
        │ 是
        ▼
┌──────────────────────────────┐
│ 4. 检查 Token 预算          │
│    - current_tokens + estimated < limit │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 未超限？│
    └───┬────┘
        │ 超限
        ▼
┌──────────────────────────────┐
│ 5. 停止压缩                  │
│    - 返回 COMPRESSION_TOKEN_LIMIT │
│    - 继续使用原始上下文      │
└──────────────────────────────┘
        │ 是
        ▼
┌──────────────────────────────┐
│ 6. 延迟重试 (指数退避)       │
│    - delay = min(delay * 2, max_delay) │
│    - current_attempt++       │
│    - 等待后重试              │
└──────────────────────────────┘
```

**重要**: 当 `COMPRESSION_RETRY_EXHAUSTED` 或 `COMPRESSION_TOKEN_LIMIT` 发生时，**不会无限重试**，系统会继续使用原始上下文，Agent 可以继续工作（虽然上下文较长）。

### 压缩策略对比

| 策略 | 速度 | Token 节省 | 信息保留 | 适用场景 |
|------|------|-----------|----------|----------|
| summary | 快 | 高 | 中 | 一般对话 |
| semantic | 慢 | 中 | 高 | 技术讨论 |
| hybrid | 最慢 | 最高 | 高 | 长对话 |
