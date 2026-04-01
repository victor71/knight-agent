# Knight-Agent 技术基线测试文档

## 测试概述

### 测试目的

验证 MVP 验收标准中定义的性能指标和资源占用目标是否可行，为系统设计提供技术基线数据。

### 测试环境

| 项目 | 要求 |
|------|------|
| 操作系统 | macOS / Linux / Windows |
| 内存 | 至少 8GB |
| CPU | 4 核心以上 |
| 网络 | 稳定的外网连接（访问 LLM API） |
| Rust 版本 | 1.70+ |
| Node.js 版本 | 18+ |

### 测试优先级

| 阶段 | 测试项 | 优先级 | 依赖 |
|------|--------|--------|------|
| 第1阶段 | LLM API 基线测试 | P0 | - |
| 第2阶段 | 本地处理性能测试 | P0 | 第1阶段 |
| 第3阶段 | 资源占用测试 | P0 | 第2阶段 |
| 第4阶段 | 并发能力测试 | P1 | 第3阶段 |
| 第5阶段 | 工具能力验证 | P1 | 第2阶段 |
| 第6阶段 | 隔离机制测试 | P1 | 第4阶段 |

---

## 第1阶段：LLM API 基线测试

### 测试目标

获取兼容 Anthropic API 协议的 LLM 提供商的响应时间分布，验证本地处理时间预算（< 500ms）是否合理。

### 测试提供商

| 提供商 | API 端点 | 测试模型 |
|--------|----------|----------|
| Anthropic 官方 | https://api.anthropic.com | claude-3-5-sonnet-20241022 |
| Amazon Bedrock | （根据配置） | anthropic.claude-3-5-sonnet-20241022-v2:0 |
| 其他兼容提供商 | （根据配置） | （根据配置） |

### 测试方法

#### 非流式响应测试

```bash
#!/bin/bash
# tests/llm/latency-test.sh

API_KEY="${ANTHROPIC_API_KEY}"
API_URL="https://api.anthropic.com/v1/messages"
MODEL="claude-3-5-sonnet-20241022"
OUTPUT_FILE="llm_latency_results.csv"

echo "timestamp,provider,model,response_time_ms,tokens_input,tokens_output" > $OUTPUT_FILE

for i in {1..100}; do
  START_TIME=$(date +%s%3N)

  RESPONSE=$(curl -s -X POST "$API_URL" \
    -H "x-api-key: $API_KEY" \
    -H "anthropic-version: 2023-06-01" \
    -H "content-type: application/json" \
    -d "{
      \"model\": \"$MODEL\",
      \"max_tokens\": 100,
      \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}]
    }")

  END_TIME=$(date +%s%3N)
  DURATION=$((END_TIME - START_TIME))

  echo "$(date -Iseconds),anthropic,$MODEL,$DURATION,10,50" >> $OUTPUT_FILE

  sleep 0.1
done

echo "测试完成，结果保存在: $OUTPUT_FILE"
```

#### 流式响应测试

```bash
#!/bin/bash
# tests/llm/streaming-latency-test.sh

API_KEY="${ANTHROPIC_API_KEY}"
API_URL="https://api.anthropic.com/v1/messages"
MODEL="claude-3-5-sonnet-20241022"
OUTPUT_FILE="llm_streaming_latency_results.csv"

echo "timestamp,provider,model,first_token_time_ms,total_time_ms" > $OUTPUT_FILE

for i in {1..50}; do
  FIRST_TOKEN_TIME=0
  START_TIME=$(date +%s%3N)

  curl -s -X POST "$API_URL" \
    -H "x-api-key: $API_KEY" \
    -H "anthropic-version: 2023-06-01" \
    -H "content-type: application/json" \
    -d "{
      \"model\": \"$MODEL\",
      \"max_tokens\": 100,
      \"stream\": true,
      \"messages\": [{\"role\": \"user\", \"content\": \"Say hello\"}]
    }" | while read -r line; do
    if [ $FIRST_TOKEN_TIME -eq 0 ]; then
      FIRST_TOKEN_TIME=$(date +%s%3N)
    fi
  done

  END_TIME=$(date +%s%3N)
  FIRST_TOKEN_DELAY=$((FIRST_TOKEN_TIME - START_TIME))
  TOTAL_DURATION=$((END_TIME - START_TIME))

  echo "$(date -Iseconds),anthropic,$MODEL,$FIRST_TOKEN_DELAY,$TOTAL_DURATION" >> $OUTPUT_FILE

  sleep 0.5
done
```

### 结果分析

```python
#!/usr/bin/env python3
# tests/llm/analyze-results.py

import pandas as pd
import numpy as np

def analyze_latency(csv_file):
    df = pd.read_csv(csv_file)

    print("=== LLM API 延迟分析 ===")
    print(f"总请求数: {len(df)}")
    print(f"平均响应时间: {df['response_time_ms'].mean():.2f} ms")
    print(f"P50 响应时间: {df['response_time_ms'].quantile(0.5):.2f} ms")
    print(f"P95 响应时间: {df['response_time_ms'].quantile(0.95):.2f} ms")
    print(f"P99 响应时间: {df['response_time_ms'].quantile(0.99):.2f} ms")
    print(f"最小响应时间: {df['response_time_ms'].min():.2f} ms")
    print(f"最大响应时间: {df['response_time_ms'].max():.2f} ms")

    # 本地处理时间预算分析
    total_budget = 500  # ms
    p99_llm_time = df['response_time_ms'].quantile(0.99)
    remaining = total_budget - p99_llm_time

    print(f"\n=== 本地处理时间预算分析 ===")
    print(f"总预算: {total_budget} ms")
    print(f"P99 LLM 时间: {p99_llm_time:.2f} ms")
    print(f"剩余本地处理时间: {remaining:.2f} ms")

    if remaining < 0:
        print("⚠️  警告: LLM 响应时间超过预算，需要调整")
    elif remaining < 100:
        print("⚠️  警告: 本地处理时间紧张，建议优化")
    else:
        print("✓ 本地处理时间充足")

if __name__ == "__main__":
    analyze_latency("llm_latency_results.csv")
```

### 验收标准

| 指标 | 目标值 | 实际值 | 状态 |
|------|--------|--------|------|
| P50 响应时间 | < 1000ms | ___ | 待测试 |
| P95 响应时间 | < 2000ms | ___ | 待测试 |
| P99 响应时间 | < 5000ms | ___ | 待测试 |
| 本地处理时间预算 | > 100ms | ___ | 待测试 |

---

## 第2阶段：本地处理性能测试

### 测试目标

测量本地组件的处理时间，确认各性能目标是否合理。

### 测试项

| 测试项 | 测试方法 | 目标值 |
|--------|----------|--------|
| Markdown 解析 | 解析 100 个 Agent 定义文件 | < 50ms/文件 |
| JSON 序列化 | 序列化/反序列化消息对象 | < 10ms/操作 |
| 工具调用开销 | Read 工具本地处理时间（不含 I/O） | < 100ms |

### 测试方法

#### Markdown 解析测试

```rust
// tests/benchmark/markdown_parsing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use knight_core::agent::AgentDefinition;

fn benchmark_parse_agent(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_parse");

    // 测试不同大小的 Agent 定义
    for size in [100, 500, 1000, 5000].iter() {
        let markdown = generate_agent_markdown(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let def: AgentDefinition = serde_yaml::from_str(black_box(&markdown)).unwrap();
                black_box(def)
            })
        });
    }

    group.finish();
}

criterion_group!(benches, benchmark_parse_agent);
criterion_main!(benches);
```

#### 消息序列化测试

```rust
// tests/benchmark/message_serialization.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use knight_core::message::Message;

fn benchmark_message_serialize(c: &mut Criterion) {
    let message = create_test_message();

    c.bench_function("message_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&message)).unwrap();
            black_box(json)
        })
    });

    c.bench_function("message_deserialize", |b| {
        let json = serde_json::to_string(&message).unwrap();
        b.iter(|| {
            let msg: Message = serde_json::from_str(black_box(&json)).unwrap();
            black_box(msg)
        })
    });
}

criterion_group!(benches, benchmark_message_serialize);
criterion_main!(benches);
```

### 验收标准

| 指标 | 目标值 | 实际值 | 状态 |
|------|--------|--------|------|
| Agent 解析（小文件） | < 10ms | ___ | 待测试 |
| Agent 解析（大文件） | < 50ms | ___ | 待测试 |
| 消息序列化 | < 5ms | ___ | 待测试 |
| 消息反序列化 | < 5ms | ___ | 待测试 |

---

## 第3阶段：资源占用测试

### 测试目标

测量 Agent 和会话进程的内存占用，验证资源目标是否合理。

### 测试方法

#### 单 Agent 内存测试

```bash
#!/bin/bash
# tests/resource/agent-memory-test.sh

SESSION_ID="test-memory-$$"
WORKSPACE=$(mktemp -d)

echo "=== 启动测试会话 ==="
knight session create --name "$SESSION_ID" --workspace "$WORKSPACE"

# 等待会话启动
sleep 2

# 获取会话进程 PID
PID=$(pgrep -f "knight session.*$SESSION_ID" | head -1)

if [ -z "$PID" ]; then
    echo "错误: 无法找到会话进程"
    exit 1
fi

echo "会话进程 PID: $PID"

# 监控内存占用
echo "=== 开始内存监控 ==="
echo "timestamp,rss_mb,vsz_mb" > agent_memory.csv

for i in {1..60}; do
    # 读取内存信息
    STATS=$(ps -p $PID -o rss=,vsz=)
    RSS=$(echo $STATS | awk '{print $1}')
    VSZ=$(echo $STATS | awk '{print $2}')

    # 转换为 MB
    RSS_MB=$((RSS / 1024))
    VSZ_MB=$((VSZ / 1024))

    echo "$(date +%s),$RSS_MB,$VSZ_MB" >> agent_memory.csv
    echo "[$i/60] RSS: ${RSS_MB}MB, VSZ: ${VSZ_MB}MB"

    sleep 1
done

# 分析结果
echo "=== 内存占用分析 ==="
python3 tests/resource/analyze-memory.py agent_memory.csv

# 清理
knight session delete "$SESSION_ID"
```

#### 内存分析脚本

```python
#!/usr/bin/env python3
# tests/resource/analyze-memory.py

import pandas as pd
import sys

def analyze_memory(csv_file):
    df = pd.read_csv(csv_file)

    print("=== 内存占用分析 ===")
    print(f"平均 RSS: {df['rss_mb'].mean():.2f} MB")
    print(f"最大 RSS: {df['rss_mb'].max():.2f} MB")
    print(f"最小 RSS: {df['rss_mb'].min():.2f} MB")
    print(f"RSS 标准差: {df['rss_mb'].std():.2f} MB")

    # 检测内存泄漏
    first_10 = df['rss_mb'].head(10).mean()
    last_10 = df['rss_mb'].tail(10).mean()
    growth = last_10 - first_10

    print(f"\n=== 内存泄漏检测 ===")
    print(f"前10秒平均: {first_10:.2f} MB")
    print(f"后10秒平均: {last_10:.2f} MB")
    print(f"增长: {growth:.2f} MB")

    if growth > 50:
        print("⚠️  警告: 可能存在内存泄漏")
    else:
        print("✓ 内存稳定")

    # 验证目标
    print(f"\n=== 目标验证 ===")
    target_idle = 100
    target_running = 500

    max_rss = df['rss_mb'].max()
    if max_rss < target_idle:
        print(f"✓ 满足空闲目标 (< {target_idle}MB)")
    elif max_rss < target_running:
        print(f"✓ 满足运行时目标 (< {target_running}MB)")
    else:
        print(f"⚠️  超出目标 ({max_rss:.2f}MB > {target_running}MB)")

if __name__ == "__main__":
    analyze_memory(sys.argv[1])
```

### 验收标准

| 指标 | 目标值 | 实际值 | 状态 |
|------|--------|--------|------|
| 单 Agent 空闲内存 | < 100MB | ___ | 待测试 |
| 单 Agent 运行时内存 | < 500MB | ___ | 待测试 |
| 会话进程内存 | < 2GB | ___ | 待测试 |
| 内存泄漏 | 无明显增长 | ___ | 待测试 |

---

## 第4阶段：并发能力测试

### 测试目标

验证系统能否支持 6 个并发会话、单会话 20 个并发 Agent。

### 测试方法

#### 多会话并发测试

```bash
#!/bin/bash
# tests/concurrency/multi-session-test.sh

MAX_SESSIONS=6
WORKSPACE_BASE=$(mktemp -d)
SESSION_PIDS=()

echo "=== 启动 $MAX_SESSIONS 个并发会话 ==="

# 启动多个会话
for i in $(seq 1 $MAX_SESSIONS); do
    WORKSPACE="$WORKSPACE_BASE/session-$i"
    mkdir -p "$WORKSPACE"

    knight session create --name "test-session-$i" --workspace "$WORKSPACE" &
    SESSION_PIDS+=($!)
    echo "会话 $i 启动，PID: ${SESSION_PIDS[-1]}"
done

# 等待所有会话启动
sleep 5

# 验证所有会话都在运行
RUNNING_COUNT=$(knight session list | grep -c "Active")
echo "活跃会话数: $RUNNING_COUNT"

if [ "$RUNNING_COUNT" -eq "$MAX_SESSIONS" ]; then
    echo "✓ 所有会话正常运行"
else
    echo "⚠️  部分会话启动失败"
fi

# 清理
for pid in "${SESSION_PIDS[@]}"; do
    kill $pid 2>/dev/null
done

rm -rf "$WORKSPACE_BASE"
```

#### 单会话多 Agent 并发测试

```bash
#!/bin/bash
# tests/concurrency/multi-agent-test.sh

SESSION_ID="test-multi-agent"
MAX_AGENTS=20

echo "=== 创建测试会话 ==="
knight session create --name "$SESSION_ID" --workspace $(mktemp -d)

echo "=== 启动 $MAX_AGENTS 个并发 Agent ==="

AGENT_IDS=()
for i in $(seq 1 $MAX_AGENTS); do
    AGENT_ID="agent-$i"
    AGENT_IDS+=($AGENT_ID)

    # 后台启动 Agent
    knight agent start "$AGENT_ID" --session "$SESSION_ID" &
done

# 等待所有 Agent 启动
sleep 10

# 验证所有 Agent 都在运行
RUNNING_COUNT=$(knight agent list --session "$SESSION_ID" | grep -c "Running")
echo "运行中 Agent 数: $RUNNING_COUNT"

if [ "$RUNNING_COUNT" -eq "$MAX_AGENTS" ]; then
    echo "✓ 所有 Agent 正常运行"
else
    echo "⚠️  部分 Agent 启动失败"
fi

# 清理
knight session delete "$SESSION_ID"
```

### 验收标准

| 指标 | 目标值 | 实际值 | 状态 |
|------|--------|--------|------|
| 最大并发会话数 | 6 | ___ | 待测试 |
| 单会话最大 Agent 数 | 20 | ___ | 待测试 |

---

## 第5阶段：工具能力验证

### 测试目标

验证工具的能力边界和限制是否按预期工作。

### Read 工具测试

```bash
#!/bin/bash
# tests/tool/read-test.sh

echo "=== Read 工具测试 ==="

# 测试1: 1MB 文件读取
echo "测试1: 1MB 文件读取"
dd if=/dev/zero of=/tmp/test-1mb.dat bs=1M count=1
RESULT=$(knight tool read /tmp/test-1mb.dat)
if [ $? -eq 0 ]; then
    SIZE=$(echo "$RESULT" | wc -c)
    if [ $SIZE -eq $((1024*1024)) ]; then
        echo "✓ 1MB 文件读取成功"
    else
        echo "⚠️  文件大小不匹配"
    fi
else
    echo "⚠️  1MB 文件读取失败"
fi

# 测试2: 2MB 文件拒绝
echo "测试2: 2MB 文件应被拒绝"
dd if=/dev/zero of=/tmp/test-2mb.dat bs=1M count=2
RESULT=$(knight tool read /tmp/test-2mb.dat 2>&1)
if echo "$RESULT" | grep -q "exceeds limit"; then
    echo "✓ 2MB 文件正确拒绝"
else
    echo "⚠️  2MB 文件未被拒绝"
fi

# 清理
rm -f /tmp/test-1mb.dat /tmp/test-2mb.dat
```

### Grep 工具测试

```bash
#!/bin/bash
# tests/tool/grep-test.sh

TEST_DIR=$(mktemp -d)

echo "=== Grep 工具测试 ==="

# 创建测试文件
mkdir -p "$TEST_DIR/subdir"
echo "hello world" > "$TEST_DIR/file1.txt"
echo "hello rust" > "$TEST_DIR/subdir/file2.txt"
echo "goodbye world" > "$TEST_DIR/file3.txt"

# 测试1: 单文件搜索
echo "测试1: 单文件搜索"
RESULT=$(knight tool grep "hello" "$TEST_DIR/file1.txt")
if echo "$RESULT" | grep -q "hello world"; then
    echo "✓ 单文件搜索成功"
else
    echo "⚠️  单文件搜索失败"
fi

# 测试2: 目录搜索
echo "测试2: 目录搜索"
RESULT=$(knight tool grep "hello" "$TEST_DIR")
COUNT=$(echo "$RESULT" | wc -l)
if [ $COUNT -eq 2 ]; then
    echo "✓ 目录搜索成功 (找到 $COUNT 个匹配)"
else
    echo "⚠️  目录搜索失败 (预期 2，实际 $COUNT)"
fi

# 测试3: 递归搜索
echo "测试3: 递归搜索"
RESULT=$(knight tool grep "hello" "$TEST_DIR" --recursive)
COUNT=$(echo "$RESULT" | wc -l)
if [ $COUNT -eq 2 ]; then
    echo "✓ 递归搜索成功"
else
    echo "⚠️  递归搜索失败"
fi

# 清理
rm -rf "$TEST_DIR"
```

### Bash 工具测试

```bash
#!/bin/bash
# tests/tool/bash-test.sh

echo "=== Bash 工具测试 ==="

# 测试1: 白名单内命令
echo "测试1: 白名单内命令应直接执行"
RESULT=$(echo "git status" | knight tool bash --allow)
if [ $? -eq 0 ]; then
    echo "✓ 白名单内命令执行成功"
else
    echo "⚠️  白名单内命令执行失败"
fi

# 测试2: 白名单外命令
echo "测试2: 白名单外命令应被拒绝或确认"
RESULT=$(echo "rm -rf /tmp/test" | knight tool bash 2>&1)
if echo "$RESULT" | grep -q "confirm\|denied"; then
    echo "✓ 白名单外命令正确处理"
else
    echo "⚠️  白名单外命令未被正确拦截"
fi

# 测试3: Skill 中定义的命令
echo "测试3: Skill 命令应免确认"
# 需要先创建测试 Skill
RESULT=$(knight skill exec test-skill --command "ls -la")
if [ $? -eq 0 ]; then
    echo "✓ Skill 命令免确认执行"
else
    echo "⚠️  Skill 命令执行失败"
fi
```

### 验收标准

| 工具 | 测试项 | 目标 | 实际值 | 状态 |
|------|--------|------|--------|------|
| Read | 1MB 文件 | 成功读取 | ___ | 待测试 |
| Read | >1MB 文件 | 返回错误 | ___ | 待测试 |
| Grep | 单文件搜索 | 成功 | ___ | 待测试 |
| Grep | 目录搜索 | 成功 | ___ | 待测试 |
| Grep | 递归搜索 | 成功 | ___ | 待测试 |
| Bash | 白名单内命令 | 免确认/直接执行 | ___ | 待测试 |
| Bash | 白名单外命令 | 拒绝或确认 | ___ | 待测试 |
| Bash | Skill 命令 | 免确认执行 | ___ | 待测试 |

---

## 第6阶段：隔离机制测试

### 测试目标

验证进程级隔离是否有效，跨会话访问是否被正确拒绝。

### 测试方法

```bash
#!/bin/bash
# tests/isolation/process-isolation-test.sh

WORKSPACE_A=$(mktemp -d)
WORKSPACE_B=$(mktemp -d)

echo "=== 进程级隔离测试 ==="

# 创建两个会话
echo "创建会话 A: $WORKSPACE_A"
knight session create --name "session-a" --workspace "$WORKSPACE_A"

echo "创建会话 B: $WORKSPACE_B"
knight session create --name "session-b" --workspace "$WORKSPACE_B"

# 在会话 A 中创建敏感文件
echo "secret data" > "$WORKSPACE_A/config.json"

# 切换到会话 B 并尝试访问会话 A 的文件
echo "从会话 B 尝试访问会话 A 的文件"
knight session use session-b

RESULT=$(knight tool read "$WORKSPACE_A/config.json" 2>&1)

# 验证结果
if echo "$RESULT" | grep -q "permission denied\|access denied\|outside workspace"; then
    echo "✓ 跨会话访问被正确拒绝"
    EXIT_CODE=0
else
    echo "⚠️  跨会话访问未被拒绝！"
    echo "结果: $RESULT"
    EXIT_CODE=1
fi

# 清理
knight session delete session-a
knight session delete session-b
rm -rf "$WORKSPACE_A" "$WORKSPACE_B"

exit $EXIT_CODE
```

### 信号隔离测试

```bash
#!/bin/bash
# tests/isolation/signal-isolation-test.sh

echo "=== 信号隔离测试 ==="

# 创建两个会话
knight session create --name "session-sig-a" --workspace $(mktemp -d)
knight session create --name "session-sig-b" --workspace $(mktemp -d)

# 记录初始状态
STATUS_A=$(knight session info session-sig-a | grep "Status")
STATUS_B=$(knight session info session-sig-b | grep "Status")

echo "会话 A 状态: $STATUS_A"
echo "会话 B 状态: $STATUS_B"

# 向会话 A 发送中断信号
echo "向会话 A 发送 SIGINT"
knight session signal session-sig-a SIGINT

# 等待信号处理
sleep 2

# 检查会话状态
NEW_STATUS_A=$(knight session info session-sig-a | grep "Status")
NEW_STATUS_B=$(knight session info session-sig-b | grep "Status")

echo "信号后会话 A 状态: $NEW_STATUS_A"
echo "信号后会话 B 状态: $NEW_STATUS_B"

# 验证会话 B 不受影响
if echo "$NEW_STATUS_B" | grep -q "Active\|Running"; then
    echo "✓ 会话 B 未受信号影响"
    EXIT_CODE=0
else
    echo "⚠️  会话 B 受到影响"
    EXIT_CODE=1
fi

# 清理
knight session delete session-sig-a
knight session delete session-sig-b

exit $EXIT_CODE
```

### 验收标准

| 测试项 | 目标 | 实际值 | 状态 |
|--------|------|--------|------|
| 跨会话文件访问 | 拒绝率 100% | ___ | 待测试 |
| 信号隔离 | 其他会话不受影响 | ___ | 待测试 |

---

## 测试结果汇总

### 结果模板

```markdown
## Knight-Agent 技术基线测试报告

**测试日期**: YYYY-MM-DD
**测试环境**: [操作系统/CPU/内存]
**测试执行人**: [姓名]

### 测试结果汇总

| 阶段 | 测试项 | 目标值 | 实际值 | 状态 |
|------|--------|--------|--------|------|
| 第1阶段 | LLM P50 响应时间 | < 1000ms | ___ | 待测试 |
| 第1阶段 | 本地处理时间预算 | > 100ms | ___ | 待测试 |
| 第2阶段 | Agent 解析（小文件） | < 10ms | ___ | 待测试 |
| 第2阶段 | 消息序列化 | < 5ms | ___ | 待测试 |
| 第3阶段 | 单 Agent 空闲内存 | < 100MB | ___ | 待测试 |
| 第3阶段 | 单 Agent 运行时内存 | < 500MB | ___ | 待测试 |
| 第4阶段 | 并发会话数 | 6 | ___ | 待测试 |
| 第4阶段 | 单会话 Agent 数 | 20 | ___ | 待测试 |
| 第5阶段 | Read 1MB 文件 | 成功 | ___ | 待测试 |
| 第5阶段 | Read >1MB 文件 | 拒绝 | ___ | 待测试 |
| 第6阶段 | 跨会话访问拒绝率 | 100% | ___ | 待测试 |

### 结论

[总体评估/建议]

### 风险与建议

[发现的风险和改进建议]
```

---

## 附录

### 测试命令速查

```bash
# 运行所有测试
make test-baseline

# 运行单个阶段
make test-stage-1  # LLM API 基线
make test-stage-2  # 本地处理性能
make test-stage-3  # 资源占用
make test-stage-4  # 并发能力
make test-stage-5  # 工具能力
make test-stage-6  # 隔离机制

# 查看测试报告
make test-report
```

### Makefile 模板

```makefile
# tests/Makefile

.PHONY: test-baseline test-stage-1 test-stage-2 test-stage-3 test-stage-4 test-stage-5 test-stage-6 test-report

test-baseline:
	@echo "运行所有技术基线测试..."
	@$(MAKE) test-stage-1
	@$(MAKE) test-stage-2
	@$(MAKE) test-stage-3
	@$(MAKE) test-stage-4
	@$(MAKE) test-stage-5
	@$(MAKE) test-stage-6
	@$(MAKE) test-report

test-stage-1:
	@echo "第1阶段: LLM API 基线测试..."
	@./tests/llm/latency-test.sh

test-stage-2:
	@echo "第2阶段: 本地处理性能测试..."
	@cargo test --release --benches

test-stage-3:
	@echo "第3阶段: 资源占用测试..."
	@./tests/resource/agent-memory-test.sh

test-stage-4:
	@echo "第4阶段: 并发能力测试..."
	@./tests/concurrency/multi-session-test.sh
	@./tests/concurrency/multi-agent-test.sh

test-stage-5:
	@echo "第5阶段: 工具能力验证..."
	@./tests/tool/read-test.sh
	@./tests/tool/grep-test.sh
	@./tests/tool/bash-test.sh

test-stage-6:
	@echo "第6阶段: 隔离机制测试..."
	@./tests/isolation/process-isolation-test.sh
	@./tests/isolation/signal-isolation-test.sh

test-report:
	@echo "生成测试报告..."
	@python3 tests/generate-report.py
```
