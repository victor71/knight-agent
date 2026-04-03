# Monitor (监控模块)

## 概述

### 职责描述

Monitor 模块负责系统的实时状态收集和统计，包括：

- Token 使用统计
- 会话状态监控
- Agent 状态跟踪
- 系统资源监控
- 实时指标查询

### 设计目标

1. **实时性**: 低延迟的状态查询
2. **准确性**: 精确的统计数据
3. **低开销**: 最小化对系统性能的影响
4. **可扩展**: 支持自定义指标

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Session Manager | 依赖 | 获取会话状态。见 [Session Manager 接口](./session-manager.md) |
| Agent Runtime | 依赖 | 获取 Agent 状态。见 [Agent Runtime 接口](../agent/agent-runtime.md) |
| LLM Provider | 依赖 | 获取 Token 使用。见 [LLM Provider 接口](./llm-provider.md) |
| Logging System | 依赖 | 记录监控日志。见 [Logging System 接口](./logging-system.md) |

### 与 Logging System 的区别

| 特性 | Monitor | Logging System |
|------|---------|----------------|
| **数据类型** | 实时状态、统计 | 历史事件、日志 |
| **查询方式** | 内存查询 | 文件查询 |
| **保留时间** | 运行时 | 持久化（30天） |
| **典型用途** | 当前状态查询 | 问题排查、审计 |

### 历史数据持久化

Monitor 除了实时内存查询外，还支持将统计数据持久化到数据库用于历史报告生成：

```yaml
# 持久化接口
persist_stats:
  description: 持久化当前统计数据
  inputs:
    period:
      type: string
      description: 统计周期 (hourly/daily)
  outputs:
    success:
      type: boolean

get_historical_stats:
  description: 获取历史统计数据
  inputs:
    start_date:
      type: date
      required: true
    end_date:
      type: date
      required: true
    granularity:
      type: string
      description: 时间粒度 (hourly/daily)
  outputs:
    stats:
      type: array<HistoricalStats>
```

---

## 接口定义

### 对外接口

```yaml
# Monitor 接口定义
Monitor:
  # ========== 统计查询 ==========
  get_stats:
    description: 获取系统统计信息
    inputs:
      scope:
        type: string
        required: false
        description: 统计范围 (all/session/agent)
      id:
        type: string
        required: false
        description: 具体 ID（scope 为 session/agent 时）
    outputs:
      stats:
        type: SystemStats

  # ========== Token 统计 ==========
  get_token_usage:
    description: 获取 Token 使用统计
    inputs:
      session_id:
        type: string
        required: false
        description: 会话 ID（不指定则返回全局统计）
      start_time:
        type: datetime
        required: false
        description: 起始时间
      end_time:
        type: datetime
        required: false
        description: 结束时间
    outputs:
      usage:
        type: TokenUsage

  # ========== 状态查询 ==========
  get_status:
    description: 获取当前状态
    inputs:
      scope:
        type: string
        required: false
        description: 查询范围 (all/session/agent)
      id:
        type: string
        required: false
    outputs:
      status:
        type: SystemStatus

  # ========== 实时监控 ==========
  watch:
    description: 实时监控（流式更新）
    inputs:
      interval:
        type: int
        required: false
        default: 1
        description: 刷新间隔（秒）
      metrics:
        type: array<string>
        required: false
        description: 要监控的指标列表
    outputs:
      stream:
        type: stream<StatusUpdate>
```

### SystemStats 结构

```yaml
SystemStats:
  # Token 统计
  tokens:
    total_used:
      type: int
      description: 总消耗 Token 数
    by_model:
      type: map<string, int>
      description: 各模型消耗统计
    by_type:
      type: map<string, int>
      description: 按类型统计 (input/output)

  # 会话统计
  sessions:
    active_count:
      type: int
      description: 活跃会话数
    total_count:
      type: int
      description: 总会话数
    archived_count:
      type: int
      description: 归档会话数

  # Agent 统计
  agents:
    active_count:
      type: int
      description: 活跃 Agent 数
    total_created:
      type: int
      description: 总创建 Agent 数
    by_state:
      type: map<string, int>
      description: 按状态统计

  # 系统统计
  system:
    uptime_seconds:
      type: int
      description: 运行时长（秒）
    memory_usage:
      type: MemoryUsage
      description: 内存使用情况
    start_time:
      type: datetime
      description: 启动时间
```

### TokenUsage 结构

```yaml
TokenUsage:
  summary:
    total:
      type: int
      description: 总 Token 数
    input:
      type: int
      description: 输入 Token 数
    output:
      type: int
      description: 输出 Token 数

  by_model:
    - model:
      type: string
      description: 模型名称
      total:
        type: int
      input:
        type: int
      output:
        type: int

  by_session:
    - session_id:
      type: string
      total:
        type: int

  cost_estimate:
    type: float
    description: 预估成本（美元）
```

### SystemStatus 结构

```yaml
SystemStatus:
  timestamp:
    type: datetime
    description: 状态时间戳

  sessions:
    - id:
      type: string
      name:
      type: string
      workspace:
      type: string
      status:
      type: string
      agent:
      type: string
      message_count:
      type: int

  agents:
    - id:
      type: string
      name:
      type: string
      state:
      type: string
      session_id:
      type: string

  system:
    uptime:
      type: int
      description: 运行时长（秒）
    memory_mb:
      type: int
      description: 内存使用（MB）
    cpu_percent:
      type: float
      description: CPU 使用率
```

### HistoricalStats 结构

```yaml
HistoricalStats:
  period:
    type: object
    properties:
      start:
        type: datetime
      end:
        type: datetime
    description: 统计时间段

  # Token 统计
  tokens:
    total:
      type: int
    input:
      type: int
    output:
      type: int
    cost_estimate:
      type: float

  # 会话统计
  sessions:
    new_count:
      type: int
      active_count:
      type: int
      total_messages:
      type: int

  # Agent 统计
  agents:
    llm_calls:
      type: int
      active_count:
      type: int

  # 系统统计
  system:
    avg_memory_mb:
      type: float
    peak_memory_mb:
      type: int
```

---

## 数据收集

### Token 收集

```rust
// Token 统计收集器
pub struct TokenCollector {
    session_stats: RwLock<HashMap<String, SessionTokenStats>>,
    model_stats: RwLock<HashMap<String, ModelTokenStats>>,
    total_stats: RwLock<TokenStats>,
}

impl TokenCollector {
    /// 记录 Token 使用
    pub fn record_usage(
        &self,
        session_id: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) {
        let total = input_tokens + output_tokens;

        // 更新会话统计
        {
            let mut stats = self.session_stats.write().unwrap();
            stats.entry(session_id.to_string())
                .or_insert_with(SessionTokenStats::default)
                .add(model, input_tokens, output_tokens);
        }

        // 更新模型统计
        {
            let mut stats = self.model_stats.write().unwrap();
            stats.entry(model.to_string())
                .or_insert_with(ModelTokenStats::default)
                .add(input_tokens, output_tokens);
        }

        // 更新总统计
        {
            let mut stats = self.total_stats.write().unwrap();
            stats.add(input_tokens, output_tokens);
        }
    }

    /// 获取会话统计
    pub fn get_session_stats(&self, session_id: &str) -> Option<SessionTokenStats> {
        self.session_stats.read().unwrap().get(session_id).cloned()
    }

    /// 获取全局统计
    pub fn get_global_stats(&self) -> TokenStats {
        self.total_stats.read().unwrap().clone()
    }
}
```

### 状态收集

```rust
// 状态收集器
pub struct StatusCollector {
    session_manager: Arc<SessionManager>,
    agent_runtime: Arc<AgentRuntime>,
    start_time: Instant,
}

impl StatusCollector {
    /// 收集会话状态
    pub async fn collect_session_status(&self) -> Vec<SessionStatus> {
        let sessions = self.session_manager.list_sessions().await;
        let mut status_list = Vec::new();

        for session in sessions {
            let status = SessionStatus {
                id: session.id.clone(),
                name: session.name.clone(),
                workspace: session.workspace.root.clone(),
                status: format!("{:?}", session.status),
                agent: session.main_agent.clone(),
                message_count: session.context.messages.len(),
            };
            status_list.push(status);
        }

        status_list
    }

    /// 收集 Agent 状态
    pub async fn collect_agent_status(&self) -> Vec<AgentStatus> {
        let agents = self.agent_runtime.list_agents().await;
        let mut status_list = Vec::new();

        for agent in agents {
            let status = AgentStatus {
                id: agent.id.clone(),
                name: agent.name.clone(),
                state: format!("{:?}", agent.state),
                session_id: agent.session_id.clone(),
            };
            status_list.push(status);
        }

        status_list
    }

    /// 收集系统状态
    pub fn collect_system_status(&self) -> SystemStatusInfo {
        SystemStatusInfo {
            uptime: self.start_time.elapsed().as_secs(),
            memory_mb: Self::get_memory_usage(),
            cpu_percent: Self::get_cpu_usage(),
        }
    }

    fn get_memory_usage() -> usize {
        // 使用系统调用获取内存使用
        #[cfg(unix)]
        {
            use libc::{getrusage, RUSAGE_SELF};
            unsafe {
                let usage = std::mem::zeroed();
                getrusage(RUSAGE_SELF, &usage);
                usage.ru_maxrss as usize // KB
            }
        }
        #[cfg(windows)]
        {
            // Windows 实现
            0
        }
    }
}
```

### 历史数据持久化

```rust
// 统计数据持久化器
pub struct StatsPersister {
    storage: Arc<StorageService>,
}

impl StatsPersister {
    /// 持久化当前统计快照
    pub async fn persist_snapshot(
        &self,
        period: StatsPeriod,
    ) -> Result<()> {
        let snapshot = StatsSnapshot {
            period: period.clone(),
            timestamp: Utc::now(),
            tokens: self.collect_token_stats(),
            sessions: self.collect_session_stats(),
            agents: self.collect_agent_stats(),
            system: self.collect_system_stats(),
        };

        self.storage.save_stats_snapshot(snapshot).await
    }

    /// 获取历史统计数据
    pub async fn get_historical_stats(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        granularity: Granularity,
    ) -> Result<Vec<HistoricalStats>> {
        self.storage.query_stats_range(start, end, granularity).await
    }
}

/// 统计周期
pub enum StatsPeriod {
    Hourly,
    Daily,
}

/// 时间粒度
pub enum Granularity {
    Hourly,
    Daily,
    Weekly,
}
```

---

## 监控指标

### 核心指标

| 类别 | 指标 | 类型 | 说明 |
|------|------|------|------|
| **Token** | total_used | Counter | 总消耗 Token |
| **Token** | by_model | Map | 各模型消耗 |
| **Token** | input_tokens | Counter | 输入 Token |
| **Token** | output_tokens | Counter | 输出 Token |
| **Session** | active_count | Gauge | 活跃会话数 |
| **Session** | total_count | Counter | 总会话数 |
| **Session** | message_count | Counter | 消息总数 |
| **Agent** | active_count | Gauge | 活跃 Agent |
| **Agent** | state | Map | 各状态 Agent 数 |
| **System** | uptime | Counter | 运行时长 |
| **System** | memory_mb | Gauge | 内存使用 |
| **System** | cpu_percent | Gauge | CPU 使用率 |

### 指标收集策略

```yaml
collection:
  # Token 统计
  tokens:
    trigger: on_llm_call         # LLM 调用时收集
    aggregation: sum             # 聚合方式

  # 会话统计
  sessions:
    trigger: periodic            # 定期收集
    interval: 5s                 # 收集间隔

  # Agent 统计
  agents:
    trigger: on_state_change     # 状态变化时收集

  # 系统统计
  system:
    trigger: periodic            # 定期收集
    interval: 10s                # 收集间隔

# 持久化策略
persistence:
  stats_snapshot:
    interval: hourly             # 每小时快照
    retention: 90d               # 保留90天
  report_data:
    interval: daily              # 每日汇总
    retention: 365d              # 保留365天
```

---

## CLI 集成

### /status 命令

```bash
knight> /status

╭────────────────────────────────────────╮
│  Knight Agent Status                    │
├────────────────────────────────────────┤
│  Uptime: 2h 15m                        │
│  Memory: 245 MB                         │
│  CPU: 3.2%                              │
├────────────────────────────────────────┤
│  Sessions: 2 active, 5 total            │
│  Agents: 3 active                       │
├────────────────────────────────────────┤
│  Token Usage:                          │
│    Total: 12,345                        │
│    Input: 8,901                         │
│    Output: 3,444                        │
│                                          │
│    By Model:                            │
│      claude-sonnet: 10,234              │
│      claude-haiku: 2,111                │
╰────────────────────────────────────────╯
```

### 系统监控命令

```bash
# 查看 Token 使用
knight> /status tokens

Token Usage:
  Total: 12,345
  Input: 8,901
  Output: 3,444

  By Model:
    claude-sonnet-4-6: 10,234
    claude-haiku: 2,111

  By Session:
    abc123: 8,456
    def456: 3,889

# 查看会话状态
knight> /status sessions

Active Sessions:
  abc123 "frontend" - agent: coder - 23 messages
  def456 "backend" - agent: developer - 15 messages

# 查看系统资源
knight> /status system

System Resources:
  Uptime: 2h 15m 32s
  Memory: 245 MB / 2 GB
  CPU: 3.2%
```

---

## 配置

### 监控配置

```yaml
# config/monitor.yaml
monitor:
  # 收集配置
  collection:
    token_stats: true
    session_stats: true
    agent_stats: true
    system_stats: true

  # 更新间隔
  intervals:
    session_update: 5s
    agent_update: 1s
    system_update: 10s

  # 数据保留
  retention:
    session_stats: 7d           # 会话统计保留时间
    token_stats: 30d            # Token 统计保留时间
    system_stats: 1d            # 系统统计保留时间

  # 告警
  alerts:
    token_threshold: 100000      # Token 使用告警阈值
    memory_threshold: 0.8        # 内存使用告警阈值（80%）
```

---

## 测试要点

### 单元测试

- [ ] Token 统计正确性
- [ ] 状态收集正确性
- [ ] 指标聚合正确性
- [ ] 查询接口正确性

### 集成测试

- [ ] 与 Session Manager 集成
- [ ] 与 Agent Runtime 集成
- [ ] 与 LLM Provider 集成
- [ ] 与 CLI 集成

### 测试用例示例

```rust
#[tokio::test]
async fn test_token_collection() {
    let monitor = Monitor::new();

    monitor.record_token_usage(
        "session-1",
        "claude-sonnet-4-6",
        1000,
        500,
    );

    let stats = monitor.get_token_usage(None).await;
    assert_eq!(stats.summary.total, 1500);
}

#[tokio::test]
async fn test_status_query() {
    let monitor = create_test_monitor();
    let status = monitor.get_status("all", None).await;

    assert!(status.sessions.len() > 0);
    assert!(status.agents.len() > 0);
}
```

---

## 性能考虑

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 统计更新延迟 | < 100ms | Token 使用更新 |
| 状态查询延迟 | < 50ms | 单次状态查询 |
| 内存开销 | < 10MB | 统计数据占用 |
| CPU 开销 | < 1% | 后台收集线程 |

---

## 未来扩展

- [x] 历史数据持久化
- [x] 每日/每周/每月报告生成
- [ ] 历史趋势图表
- [ ] 自定义指标
- [ ] 指标导出（Prometheus 格式）
- [ ] 告警通知（邮件/Webhook）
- [ ] 性能分析
- [ ] 异常检测
