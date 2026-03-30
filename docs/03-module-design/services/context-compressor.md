# Context Compressor (上下文压缩服务)

## 1. 概述

### 1.1 职责描述

Context Compressor 负责会话上下文的智能压缩，包括：

- 上下文分析和压缩决策
- 多种压缩策略（摘要、语义、混合）
- 压缩点管理和存储
- Token 估算和优化
- 压缩历史追踪

### 1.2 设计目标

1. **智能压缩**: 保留关键信息，丢弃冗余
2. **Token 优化**: 最大化 Token 节省
3. **可恢复性**: 压缩内容可追溯
4. **可配置**: 支持多种压缩策略

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取会话上下文 |
| LLM Provider | 依赖 | 生成压缩摘要 |
| Storage Service | 依赖 | 存储压缩点 |

---

## 2. 接口定义

### 2.1 对外接口

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
    description: 压缩会话上下文
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

### 2.2 数据结构

```yaml
# 压缩点
CompressionPoint:
  id:
    type: string
  session_id:
    type: string
  created_at:
    type: datetime

  # 压缩前
  before:
    type: CompressionBefore
  # 压缩后
  after:
    type: CompressionAfter

  # 压缩信息
  strategy:
    type: string
    enum: [summary, semantic, hybrid]
  summary:
    type: string
    description: 压缩摘要
  key_points:
    type: array<string>
    description: 关键点列表

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

### 2.3 配置选项

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
    semantic:
      model: claude-sonnet-4-6
      temperature: 0.2
    hybrid:
      summary_model: claude-haiku
      semantic_model: claude-sonnet-4-6
```

---

## 3. 核心流程

### 3.1 压缩决策流程

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

### 3.2 摘要压缩流程

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

### 3.3 语义压缩流程

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

### 3.4 混合压缩流程

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

## 4. 模块交互

### 4.1 依赖关系图

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

### 4.2 消息流

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

## 5. 配置与部署

### 5.1 配置文件格式

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

### 5.2 环境变量

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

## 6. 示例

### 6.1 压缩配置

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

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 摘要压缩 | < 5s | 依赖 LLM |
| 语义压缩 | < 10s | 依赖 LLM |
| 混合压缩 | < 15s | 两阶段压缩 |
| Token 节省 | > 50% | 压缩比例 |

### 7.2 错误处理

```yaml
error_codes:
  COMPRESSION_FAILED:
    code: 500
    message: "压缩失败"
    action: "检查 LLM 服务"

  INSUFFICIENT_CONTEXT:
    code: 400
    message: "上下文不足"
    action: "等待更多消息"

  LOW_COMPRESSION_RATIO:
    code: 200
    message: "压缩比例过低"
    action: "调整压缩选项"

  SESSION_NOT_FOUND:
    code: 404
    message: "会话不存在"
    action: "检查会话 ID"
```

### 7.3 压缩策略对比

| 策略 | 速度 | Token 节省 | 信息保留 | 适用场景 |
|------|------|-----------|----------|----------|
| summary | 快 | 高 | 中 | 一般对话 |
| semantic | 慢 | 中 | 高 | 技术讨论 |
| hybrid | 最慢 | 最高 | 高 | 长对话 |
