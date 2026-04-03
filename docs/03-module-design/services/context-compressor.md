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
    errors:
      - COMPRESSION_CONFLICT

  get_compression_job_status:
    description: 查询异步压缩任务状态
    inputs:
      job_id:
        type: string
        required: true
    outputs:
      status:
        type: string
        enum: [pending, running, completed, failed]
      progress:
        type: float
        description: 进度 0-1
      result:
        type: CompressionPoint | null
      error:
        type: string | null

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
    type: object
    description: 最近消息保持不变
    properties:
      messages:
        type: integer
        description: 最近 N 条消息不压缩
        default: 20
      tokens:
        type: integer
        description: 最近 N tokens 不压缩
        default: 10000
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

  # Token 预算追踪
  tokens_used:
    type: integer
    description: 当前会话累计消耗的压缩 Token 数
  budget_limit:
    type: integer
    description: Token 预算上限 (max_total_cost)
  budget_remaining:
    type: integer
    description: 剩余预算 (budget_limit - tokens_used)
  compression_paused_until:
    type: datetime | null
    description: 压缩暂停截止时间（预算耗尽时设置）
  last_budget_reset:
    type: datetime | null
    description: 上次预算重置时间

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

# 压缩任务状态
CompressionJob:
  job_id:
    type: string
  session_id:
    type: string
  status:
    type: string
    enum: [pending, running, completed, failed]
  progress:
    type: float
  created_at:
    type: datetime
  started_at:
    type: datetime | null
  completed_at:
    type: datetime | null
  error:
    type: string | null

# 依赖模块接口要求
#
# 注意: 各模块必须实现的接口定义在对应模块文档中
# - LLM Provider 接口: 见 [LLM Provider - Context Compressor 接口](../services/llm-provider.md#context-compressor-接口)
# - Storage Service 接口: 见 [Storage Service - Context Compressor 接口](../services/storage-service.md#context-compressor-接口)
# - Session Manager 接口: 见 [Session Manager - Context Compressor 接口](../core/session-manager.md#context-compressor-接口)
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
    max_summary_tokens: 5000     # 单个摘要最大长度（与模型 max_tokens 对齐）
    max_input_tokens_per_call: 30000  # 单次 LLM 调用输入上限
    max_total_cost: 100000       # 会话累计压缩 Token 上限
    stop_on_limit: true          # 达到上限后停止压缩

    # 预算重置策略
    reset_strategy:
      enabled: true              # 启用预算重置
      mode: periodic             # periodic | session | manual
      reset_interval_hours: 24   # 周期性重置间隔（小时）
      reset_on_session_start: false  # 会话开始时重置

    # 暂停机制
    pause:
      pause_on_limit: true       # 预算耗尽时暂停压缩
      pause_duration_hours: 1    # 暂停持续时间（达到限制后）
      auto_resume: true          # 暂停后自动恢复

  # 重试限制
  retry:
    max_attempts: 3
    backoff: exponential
    initial_delay_ms: 1000
    max_delay_ms: 10000
    max_total_duration_ms: 40000  # 最大重试总时间
    fail_on_max_retries: true

  # 并发控制
  concurrency:
    enabled: true
    lock_timeout_ms: 30000        # 等待锁超时
    max_concurrent_per_session: 1 # 每会话最多一个压缩任务

  # 历史记录管理
  history:
    max_entries: 100              # 最大历史条目数
    ttl_days: 30                   # 保留天数
    cleanup_on_startup: true      # 启动时清理过期记录

  # 默认策略
  default_strategy: summary

  # 模型配置
  models:
    summary:
      model: claude-haiku
      temperature: 0.3
      max_tokens: 5000        # 与 max_summary_tokens 对齐
    semantic:
      model: claude-sonnet-4-6
      temperature: 0.2
      max_tokens: 8000
    hybrid:
      summary_model: claude-haiku
      summary_max_tokens: 5000
      semantic_model: claude-sonnet-4-6
      semantic_max_tokens: 8000
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
| `retry.max_attempts` | 最大重试次数 | 3 |
| `retry.max_total_duration_ms` | 最大重试总时间 | 40000 |
| `concurrency.enabled` | 启用并发控制 | true |
| `concurrency.lock_timeout_ms` | 等待锁超时 | 30000 |
| `concurrency.max_concurrent_per_session` | 每会话最大并发数 | 1 |
| `history.max_entries` | 最大历史条目数 | 100 |
| `history.ttl_days` | 历史记录保留天数 | 30 |

**压缩模式说明**:

| 模式 | 说明 | 适用场景 |
|------|------|---------|
| `preserve` | 保留原文，不压缩 | 代码、系统消息、高优先级内容 |
| `summary` | 摘要压缩 | 普通文本消息 |
| `prune` | 修剪/精简 | 日志、重复内容 |
| `truncate` | 截断 | 过长且无法有效压缩的内容 |

**内容类型检测**:

消息的内容类型通过以下优先级检测：

```
消息内容
    │
    ▼
┌──────────────────────────────┐
│ 1. 检查 metadata.content_type│
│    - 如果工具已设置          │
│    - 直接使用该值            │
└──────────────────────────────┘
    │
    ▼ 未设置
┌──────────────────────────────┐
│ 2. 检测消息角色              │
│    - system → system         │
│    - tool → 工具输出         │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 分析消息格式（启发式）    │
│    - 包含代码块 → code        │
│    - 日志格式 → log          │
│    - JSON/配置 → config     │
│    - 普通文本 → text         │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. 应用压缩规则              │
│    - code: preserve         │
│    - log: prune            │
│    - text: summary          │
│    - system: preserve       │
└──────────────────────────────┘
```

**工具应设置 content_type**:

工具在生成输出时应设置 `metadata.content_type`，避免依赖启发式检测：

```yaml
# 工具输出示例
tool_result:
  content: "..."
  metadata:
    content_type: "code"     # 明确标记为代码
    file_path: "src/main.rs"
```
```

**代码块检测示例**（启发式方法，仅当未设置 content_type 时使用）：

```python
# 优先检查工具设置的 content_type
if message.metadata.get("content_type"):
    return message.metadata["content_type"]

# 回退到启发式检测
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
│ 2. 语义分块 (可选)           │
│    - 如果内容超过阈值        │
│    - 按主题/话题分组         │
│    - 保持对话连贯性          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 提取关键信息              │
│    - 识别用户意图            │
│    - 提取关键决策            │
│    - 记录工具调用结果        │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 调用 LLM 生成摘要          │
│    prompt: "请将以下对话...  │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 5. 创建压缩点                │
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

### 并发控制流程

```
压缩请求
        │
        ▼
┌──────────────────────────────┐
│ 1. 检查会话锁                │
│    - 尝试获取会话锁          │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 获取成功？│
    └───┬────┘
        │ 否
        ▼
┌──────────────────────────────┐
│ 2. 等待锁或返回冲突          │
│    - lock_timeout_ms 内等待 │
│    - 超时返回 COMPRESSION_CONFLICT │
└──────────────────────────────┘
        │ 是
        ▼
┌──────────────────────────────┐
│ 3. 执行压缩                  │
│    - 记录压缩开始            │
│    - 执行压缩逻辑            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 4. 释放会话锁                │
│    - 无论成功或失败         │
│    - 允许后续压缩请求        │
└──────────────────────────────┘
```

---

## 语义分块 (Semantic Chunking)

### 为什么需要语义分块

当对话历史超过 `max_input_tokens_per_call` (30,000 tokens) 时，如果简单地按 Token 数量切分，可能会在话题中间切断对话，导致生成的摘要失去上下文连贯性。

**问题示例**：
```
[消息 1-15] 讨论数据库设计方案
[消息 16-20] 讨论 API 接口设计  ← 在这里切分
[消息 21-35] 继续讨论 API 接口设计
[消息 36-50] 讨论前端实现
```

如果按 Token 数量切分，第二个 chunk 可能从消息 16 开始，导致 API 设计的讨论被分割到两个不相关的摘要中。

### 语义分块策略

```yaml
semantic_chunking:
  # 启用语义分块
  enabled: true

  # 分块策略
  strategy: theme_based      # theme_based | conversation_turn | hybrid

  # 主题边界检测
  theme_detection:
    method: llm              # llm | embedding | heuristic
    min_messages_per_chunk: 10
    max_messages_per_chunk: 50
    max_tokens_per_chunk: 30000

  # 对话回合检测
  conversation_turn:
    detect_by: user_message  # user_message | tool_call | time_gap
    max_turns_per_chunk: 10
```

### 主题分块流程

```
选择待压缩的消息范围
        │
        ▼
┌──────────────────────────────┐
│ 1. 检测是否需要分块          │
│    - token_count > 25000    │
│    - message_count > 30     │
└──────────────────────────────┘
        │
        ▼
    ┌───┴────┐
    │ 需要分块？│
    └───┬────┘
        │ 否                    │ 是
        ▼                       ▼
    直接压缩              ┌──────────────────────┐
                          │ 2. 识别主题边界      │
                          │ - 使用 LLM 分析话题  │
                          │ - 检测话题转换点    │
                          └──────────────────────┘
                                   │
                                   ▼
                          ┌──────────────────────┐
                          │ 3. 按边界分组消息    │
                          │ - 每组保持话题完整   │
                          │ - 每组 < 30K tokens  │
                          └──────────────────────┘
                                   │
                                   ▼
                          ┌──────────────────────┐
                          │ 4. 为每块生成摘要    │
                          │ - 并行/串行处理      │
                          │ - 保留块间关联      │
                          └──────────────────────┘
                                   │
                                   ▼
                          ┌──────────────────────┐
                          │ 5. 合并摘要          │
                          │ - 添加时间戳        │
                          │ - 添加主题标签      │
                          └──────────────────────┘
                                   │
                                   ▼
                          创建压缩点
```

### 主题检测 Prompt

```
你是一个对话分析专家。请分析以下对话历史，识别主题转换点。

对话消息:
{{ messages }}

请输出 JSON 格式:
{
  "themes": [
    {
      "name": "主题名称",
      "start_index": 0,
      "end_index": 15,
      "description": "简短描述",
      "token_count": 12000
    }
  ],
  "transitions": [5, 15, 28]
}

要求:
1. 每个主题内的消息应该围绕同一话题
2. 主题大小不超过 30000 tokens
3. 在话题自然转换处分割
```

### 分块策略对比

| 策略 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **theme_based** | 保持话题完整性 | 需要 LLM 调用 | 长对话、多主题 |
| **conversation_turn** | 简单快速 | 可能跨主题 | 短对话、单任务 |
| **hybrid** | 平衡效果和成本 | 实现复杂 | 通用场景 |

### 配置示例

```yaml
# config/compression.yaml
compression:
  chunking:
    # 启用语义分块
    semantic_chunking: true

    # 分块阈值
    chunk_threshold:
      min_tokens: 25000      # 超过此值启用分块
      min_messages: 30       # 超过此值启用分块

    # 主题检测配置
    theme_detection:
      model: claude-haiku    # 使用轻量模型检测
      temperature: 0.1
      max_themes: 10         # 最多识别 10 个主题

    # 分块后处理
    post_chunking:
      merge_small_chunks: true      # 合并小块
      min_chunk_tokens: 5000        # 小块阈值
      add_theme_labels: true        # 添加主题标签
      add_timestamps: true          # 添加时间戳
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
      max_tokens: 5000
    semantic:
      model: claude-sonnet-4-6
      temperature: 0.2
      max_tokens: 8000
    hybrid:
      summary_model: claude-haiku
      semantic_model: claude-sonnet-4-6
```

### 环境变量

```bash
# 触发条件
export KNIGHT_COMPRESS_MESSAGE_COUNT=50
export KNIGHT_COMPRESS_TOKEN_COUNT=100000
export KNIGHT_COMPRESS_AUTO=true

# 最近消息保持
export KNIGHT_COMPRESS_KEEP_RECENT_MESSAGES=20
export KNIGHT_COMPRESS_KEEP_RECENT_TOKENS=10000

# Token 预算
export KNIGHT_COMPRESS_MAX_SUMMARY_TOKENS=5000
export KNIGHT_COMPRESS_MAX_INPUT_TOKENS=30000
export KNIGHT_COMPRESS_MAX_TOTAL_COST=100000

# 重试配置
export KNIGHT_COMPRESS_MAX_RETRIES=3
export KNIGHT_COMPRESS_MAX_TOTAL_DURATION_MS=40000

# 并发控制
export KNIGHT_COMPRESS_CONCURRENCY_ENABLED=true
export KNIGHT_COMPRESS_LOCK_TIMEOUT_MS=30000

# 历史记录
export KNIGHT_COMPRESS_HISTORY_MAX_ENTRIES=100
export KNIGHT_COMPRESS_HISTORY_TTL_DAYS=30

# 默认策略
export KNIGHT_COMPRESS_DEFAULT_STRATEGY="summary"
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

  LLM_UNAVAILABLE:
    code: 512
    message: "LLM 服务不可用"
    action: "检查 LLM Provider 配置和网络连接"
    retryable: true

  STORAGE_WRITE_FAILED:
    code: 513
    message: "存储写入失败"
    action: "检查 Storage Service 可用性"
    retryable: true

  COMPRESSION_CONFLICT:
    code: 514
    message: "压缩冲突"
    action: "等待其他压缩任务完成"
    retryable: true

  COMPRESSION_INEFFECTIVE:
    code: 515
    message: "压缩无效"
    description: "压缩后消息反而增多或压缩比过低"
    action: "跳过压缩，使用原始上下文"
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

### 预算重置机制

当预算耗尽后，系统自动重置预算以避免干扰用户：

**自动重置条件**（满足任一即触发）:
1. **时间间隔**: 距离上次压缩超过 `reset_interval_minutes`
2. **会话重启**: 新会话创建时

```yaml
# 配置示例
token_budget:
  reset_strategy:
    auto_reset: true
    reset_interval_minutes: 60   # 60 分钟无压缩活动后自动重置
    reset_on_new_session: false  # 新会话是否重置
```

### 可观测性指标

当压缩失败时（达到重试限制或 Token 预算），系统发出以下指标供监控：

```yaml
# 压缩失败指标
compression_failure:
  # 重试耗尽
  retry_exhausted:
    metric: "compression_retry_exhausted_total"
    type: counter
    labels:
      - session_id
      - strategy          # summary/semantic/hybrid
      - attempts          # 尝试次数
    description: "压缩重试次数耗尽"

  # Token 预算超限
  token_limit_exceeded:
    metric: "compression_token_limit_exceeded_total"
    type: counter
    labels:
      - session_id
      - budget_limit      # 预算上限
      - tokens_used       # 已使用 Token
    description: "压缩 Token 预算超限"

  # 每次压缩成本
  compression_cost:
    metric: "compression_compression_cost_tokens"
    type: histogram
    labels:
      - session_id
      - strategy
      - success           # true/false
    buckets: [1000, 5000, 10000, 20000, 50000]
    description: "每次压缩消耗的 Token 数"

  # 压缩执行时间
  compression_duration:
    metric: "compression_duration_seconds"
    type: histogram
    labels:
      - strategy
      - success
    buckets: [1, 5, 10, 30, 60]
    description: "压缩执行耗时"
```

### 压缩策略对比

| 策略 | 速度 | Token 节省 | 信息保留 | 适用场景 |
|------|------|-----------|----------|----------|
| summary | 快 | 高 | 中 | 一般对话 |
| semantic | 慢 | 中 | 高 | 技术讨论 |
| hybrid | 最慢 | 最高 | 高 | 长对话 |
