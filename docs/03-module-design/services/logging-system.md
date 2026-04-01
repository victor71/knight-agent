# Logging System (日志系统)

## 概述

### 职责描述

Logging System 负责管理 Knight-Agent 的所有日志记录,包括:

- 结构化日志记录(支持多种日志级别)
- 日志输出到多种目标(控制台、文件、远程服务)
- 日志轮转和归档
- 日志过滤和搜索
- 日志级别动态调整
- 性能日志和审计日志
- 日志格式化和着色

### 设计目标

1. **高性能**: 异步日志,不阻塞主流程
2. **结构化**: JSON 格式,便于解析和分析
3. **灵活性**: 支持多种输出目标和格式
4. **可观测**: 完整的日志追踪和调试能力
5. **安全性**: 敏感信息脱敏

### 核心需求

| 需求 | 描述 | 优先级 |
|------|------|--------|
| **结构化日志** | JSON 格式日志,支持字段索引 | P0 |
| **多级别日志** | DEBUG/INFO/WARN/ERROR/FATAL | P0 |
| **异步写入** | 不阻塞主线程 | P1 |
| **日志轮转** | 自动归档和清理 | P1 |
| **多输出目标** | 控制台、文件、远程服务 | P1 |
| **敏感信息脱敏** | 自动过滤敏感字段 | P2 |

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Storage Service | 依赖 | 日志文件持久化 |
| 无 | 被依赖 | 所有模块都可能记录日志 |

---

## 接口定义

### 对外接口

```yaml
# Logging System 接口定义
LoggingSystem:
  # ========== 日志记录 ==========
  log:
    description: 记录日志
    inputs:
      level:
        type: LogLevel
        description: 日志级别
        required: true
      message:
        type: string
        description: 日志消息
        required: true
      context:
        type: LogContext
        description: 日志上下文
        required: false
      module:
        type: string
        description: 模块名称
        required: false
      session_id:
        type: string
        description: 会话ID
        required: false

  debug:
    description: 记录 DEBUG 级别日志
    inputs:
      message:
        type: string
        required: true
      context:
        type: LogContext
        required: false

  info:
    description: 记录 INFO 级别日志
    inputs:
      message:
        type: string
        required: true
      context:
        type: LogContext
        required: false

  warn:
    description: 记录 WARN 级别日志
    inputs:
      message:
        type: string
        required: true
      context:
        type: LogContext
        required: false

  error:
    description: 记录 ERROR 级别日志
    inputs:
      message:
        type: string
        required: true
      context:
        type: LogContext
        required: false
      error:
        type: Error
        description: 错误对象
        required: false

  fatal:
    description: 记录 FATAL 级别日志
    inputs:
      message:
        type: string
        required: true
      context:
        type: LogContext
        required: false
      error:
        type: Error
        description: 错误对象
        required: false

  # ========== 日志配置 ==========
  set_level:
    description: 设置日志级别
    inputs:
      level:
        type: LogLevel
        required: true
      module:
        type: string
        description: 模块名称(全局则为null)
        required: false

  get_level:
    description: 获取当前日志级别
    inputs:
      module:
        type: string
        description: 模块名称
        required: false
    outputs:
      level:
        type: LogLevel

  # ========== 日志查询 ==========
  query:
    description: 查询日志
    inputs:
      filter:
        type: LogFilter
        required: false
    outputs:
      logs:
        type: array<LogEntry>

  search:
    description: 搜索日志
    inputs:
      query:
        type: string
        description: 搜索关键词
        required: true
      start_time:
        type: datetime
        required: false
      end_time:
        type: datetime
        required: false
      level:
        type: LogLevel
        required: false
      limit:
        type: integer
        required: false
        default: 100
    outputs:
      logs:
        type: array<LogEntry>

  # ========== 日志导出 ==========
  export:
    description: 导出日志
    inputs:
      format:
        type: enum
        values: [json, csv, text]
        required: false
        default: json
      filter:
        type: LogFilter
        required: false
    outputs:
      data:
        type: string | bytes

  # ========== 日志管理 ==========
  rotate:
    description: 手动触发日志轮转
    inputs:
      target:
        type: string
        description: 目标名称
        required: false

  clear:
    description: 清空日志
    inputs:
      target:
        type: string
        description: 目标名称
        required: false

  get_stats:
    description: 获取日志统计
    outputs:
      stats:
        type: LogStats
```

### 数据结构

```yaml
# 日志级别
LogLevel:
  type: enum
  values:
    - debug
    - info
    - warn
    - error
    - fatal

# 日志上下文
LogContext:
  type: map<string, any>
  description: 日志附加上下文信息
  example:
    user_id: "user-123"
    request_id: "req-456"
    duration_ms: 123

# 日志条目
LogEntry:
  id:
    type: string
    description: 日志唯一标识
  timestamp:
    type: datetime
    description: 时间戳
  level:
    type: LogLevel
    description: 日志级别
  module:
    type: string
    description: 模块名称
  session_id:
    type: string
    description: 会话ID
  message:
    type: string
    description: 日志消息
  context:
    type: LogContext
    description: 日志上下文
  error:
    type: ErrorInfo
    description: 错误信息(如有)

# 错误信息
ErrorInfo:
  type:
    type: string
    description: 错误类型
  message:
    type: string
    description: 错误消息
  stack_trace:
    type: string
    description: 堆栈跟踪

# 日志过滤条件
LogFilter:
  level:
    type: LogLevel | array<LogLevel>
    description: 日志级别过滤
  module:
    type: string | array<string>
    description: 模块过滤
  session_id:
    type: string
    description: 会话ID过滤
  start_time:
    type: datetime
    description: 开始时间
  end_time:
    type: datetime
    description: 结束时间
  message_pattern:
    type: string
    description: 消息模式匹配

# 日志统计
LogStats:
  total_entries:
    type: integer
    description: 总日志条数
  entries_by_level:
    type: map<LogLevel, integer>
    description: 按级别统计
  entries_by_module:
    type: map<string, integer>
    description: 按模块统计
  oldest_entry:
    type: datetime
    description: 最旧日志时间
  newest_entry:
    type: datetime
    description: 最新日志时间
```

### 配置选项

```yaml
# config/logging.yaml
logging:
  # 全局级别
  level:
    default: info
    modules:
      timer_system: debug
      llm_provider: warn

  # 输出目标
  outputs:
    console:
      enabled: true
      format: colored
      level: info

    file:
      enabled: true
      path: ./logs/knight.log
      format: json
      level: debug
      rotation:
        max_size: 100MB
        max_files: 10
        compress: true

    remote:
      enabled: false
      type: loki | elasticsearch | syslog
      endpoint: https://logs.example.com
      level: warn

  # 日志格式
  format:
    timestamp: iso8601
    include_module: true
    include_session: true
    include_location: false

  # 敏感信息脱敏
  sensitive:
    enabled: true
    patterns:
      - pattern: '(password|token|secret|key)="[^"]*"'
        replacement: '$1="***"'
      - pattern: '\b\d{16}\b'  # 信用卡号
        replacement: '****************'
```

---

## 核心流程

### 日志记录流程

```
调用 log()
    │
    ▼
┌──────────────────────────────┐
│ 1. 格式化日志                │
│    - 添加时间戳              │
│    - 添加模块信息            │
│    - 序列化上下文            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 2. 敏感信息脱敏              │
│    - 匹配敏感模式            │
│    - 替换敏感字段            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 3. 级别过滤                  │
│    - 检查是否应该记录        │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 4. 异步写入队列              │
│    - 不阻塞主线程            │
└──────────────────────────────┘
    │
    ▼
┌──────────────────────────────┐
│ 5. 后台写入                  │
│    - 写入各个输出目标        │
└──────────────────────────────┘
```

### 日志轮转流程

```
检查文件大小
    │
    ▼
┌──────────────────────────────┐
│ 超过 max_size?               │
└──────────────────────────────┘
    │
    ├─ 是 ─────────────────────┐
    │                         │
    │                         ▼
    │  ┌────────────────────────┐
    │  │ 1. 关闭当前文件        │
    │  └────────────────────────┘
    │         │
    │         ▼
    │  ┌────────────────────────┐
    │  │ 2. 重命名为归档文件    │
    │  │    knight.log.1        │
    │  └────────────────────────┘
    │         │
    │         ▼
    │  ┌────────────────────────┐
    │  │ 3. 删除旧归档          │
    │  │    (超过 max_files)    │
    │  └────────────────────────┘
    │         │
    │         ▼
    │  ┌────────────────────────┐
    │  │ 4. 压缩归档文件(可选)  │
    │  └────────────────────────┘
    │         │
    │         ▼
    │  ┌────────────────────────┐
    │  │ 5. 创建新日志文件      │
    │  └────────────────────────┘
    │
    └─ 否 ──────────────────────────────► 等待下次检查
```

---

## 模块交互

### 依赖关系图

```
┌─────────────────────────────────────────┐
│          所有模块                        │
│  - Session Manager                      │
│  - Agent Runtime                        │
│  - Timer System                         │
│  - ...                                  │
└──────────────┬──────────────────────────┘
               │
               │ 调用 log()
               ▼
┌─────────────────────────────────────────┐
│         Logging System                   │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Formatter │  │Filter    │  │Async   ││
│  └──────────┘  └──────────┘  │Queue   ││
│                            └────────┘│
└──────────────┬──────────────────────────┘
               │
               │ 写入日志
               ▼
┌─────────────────────────────────────────┐
│         输出目标                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Console   │  │File      │  │Remote  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────────────────────────────────────────┘
```

---

## 配置与部署

### 配置文件格式

```yaml
# config/logging.yaml
logging:
  # 全局级别
  level:
    default: info
    modules:
      timer_system: debug
      llm_provider: warn

  # 输出目标
  outputs:
    console:
      enabled: true
      format: colored
      level: info

    file:
      enabled: true
      path: ./logs/knight.log
      format: json
      level: debug
      rotation:
        max_size: 100MB
        max_files: 10
        compress: true

    remote:
      enabled: false
      type: loki | elasticsearch | syslog
      endpoint: https://logs.example.com
      level: warn

  # 日志格式
  format:
    timestamp: iso8601
    include_module: true
    include_session: true
    include_location: false

  # 敏感信息脱敏
  sensitive:
    enabled: true
    patterns:
      - pattern: '(password|token|secret|key)="[^"]*"'
        replacement: '$1="***"'
      - pattern: '\b\d{16}\b'  # 信用卡号
        replacement: '****************'
```

### 环境变量

```bash
# 日志级别
export KNIGHT_LOG_LEVEL=info
export KNIGHT_LOG_MODULE_TIMER_SYSTEM=debug

# 文件输出
export KNIGHT_LOG_FILE_ENABLED=true
export KNIGHT_LOG_FILE_PATH="./logs/knight.log"
export KNIGHT_LOG_FILE_MAX_SIZE=100MB
export KNIGHT_LOG_FILE_MAX_FILES=10

# 远程日志
export KNIGHT_LOG_REMOTE_ENABLED=false
export KNIGHT_LOG_REMOTE_TYPE=loki
export KNIGHT_LOG_REMOTE_ENDPOINT="https://logs.example.com"
```

---

## 附录

### 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 异步延迟 | < 1ms | 日志写入队列延迟 |
| 吞吐量 | > 10000 logs/s | 日志记录速率 |
| 内存占用 | < 100MB | 异步队列占用 |

### 错误处理

```yaml
error_codes:
  INVALID_LEVEL:
    code: 400
    message: "无效的日志级别"
    action: "级别必须是 debug/info/warn/error/fatal"

  WRITE_FAILED:
    code: 500
    message: "日志写入失败"
    action: "检查磁盘空间和权限"

  ROTATION_FAILED:
    code: 500
    message: "日志轮转失败"
    action: "检查文件权限"

  QUERY_FAILED:
    code: 500
    message: "日志查询失败"
    action: "检查查询语法"
```

### 使用示例

```yaml
# 基础日志记录
logging:
  debug:
    message: "定时器已创建"
    context:
      timer_id: "timer-001"
      type: "interval"

  info:
    message: "Agent 执行完成"
    context:
      agent_id: "code-reviewer"
      duration_ms: 1234

  warn:
    message: "API 调用接近速率限制"
    context:
      endpoint: "/api/v1/chat"
      remaining: 5

  error:
    message: "工具调用失败"
    error:
      type: "ToolTimeoutError"
      message: "工具执行超过30秒"
    context:
      tool_name: "web-scraper"

# 性能日志
performance:
  info:
    message: "LLM 调用性能"
    context:
      model: "claude-opus-4"
      prompt_tokens: 1000
      completion_tokens: 500
      latency_ms: 2345

# 审计日志
audit:
  info:
    message: "敏感操作"
    context:
      action: "delete_session"
      user_id: "user-123"
      session_id: "session-456"
      result: "success"
```

### 测试策略

```yaml
testing:
  unit_tests:
    - 日志级别过滤
    - 格式化功能
    - 敏感信息脱敏
    - 上下文序列化

  integration_tests:
    - 异步写入测试
    - 文件轮转测试
    - 远程日志发送测试
    - 日志查询测试

  performance_tests:
    - 高并发写入测试
    - 长时间运行稳定性测试
    - 内存泄漏测试
```

### 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-04-01 | 初始版本 |
