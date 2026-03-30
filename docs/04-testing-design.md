# Knight-Agent 测试设计文档

## 1. 概述

### 1.1 测试范围

本文档定义 Knight-Agent 项目的测试策略，覆盖 **L0 (P0 核心模块)** 和 **L1 (P1 扩展模块)** 的测试方案。

### 1.2 测试层级

```
┌─────────────────────────────────────────────────────────────┐
│                        测试金字塔                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│                    ▲                                        │
│                   /  \                                       │
│                  /    \                                      │
│                 / E2E  \        ← 少量端到端测试            │
│                /--------\                                    │
│               /          \                                   │
│              / Integration \       ← 中量集成测试           │
│             /--------------\                               │
│            /                  \                              │
│           /     Unit Tests     \   ← 大量单元测试           │
│          /----------------------\                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 测试类型定义

| 测试类型 | 覆盖范围 | 目标 | 自动化 |
|---------|---------|------|--------|
| **单元测试** | 单个函数/方法 | 验证代码逻辑正确性 | 100% |
| **集成测试** | 模块间交互 | 验证模块协作正确性 | 90% |
| **端到端测试** | 完整场景 | 验证业务流程正确性 | 70% |
| **性能测试** | 系统性能 | 验证性能指标达标 | 手动 |
| **安全测试** | 安全漏洞 | 验证安全机制有效 | 手动+自动化 |

---

## 2. L0 (P0) 核心模块测试

### 2.1 模块列表

| 模块 | 文档 | 优先级 |
|------|------|--------|
| Session Manager | `core/session-manager.md` | P0 |
| Agent Runtime | `agent/agent-runtime.md` | P0 |
| LLM Provider | `services/llm-provider.md` | P0 |
| Tool System | `tools/tool-system.md` | P0 |

### 2.2 Session Manager 测试

#### 单元测试

```yaml
test_suite: SessionManager Unit Tests
file: tests/unit/session_manager_test.rs

test_cases:
  # 会话创建
  - name: 创建会话成功
    input:
      workspace: "/tmp/test-project"
      name: "test-session"
    expect:
      status: active
      workspace.root: "/tmp/test-project"

  - name: 创建会话时检测项目类型
    input:
      workspace: "/tmp/rust-project"
    setup:
      - create_file: "Cargo.toml"
    expect:
      project_type: "rust"

  # Workspace 隔离
  - name: 路径访问检查 - 允许访问
    input:
      session_id: "test-session"
      path: "/tmp/test-project/src/main.rs"
      action: "read"
    expect:
      allowed: true

  - name: 路径访问检查 - 拒绝访问外部路径
    input:
      session_id: "test-session"
      path: "/etc/passwd"
      action: "read"
    expect:
      allowed: false
      reason: "路径不在 workspace 范围内"

  - name: 路径访问检查 - 拒绝敏感文件
    input:
      session_id: "test-session"
      path: "/tmp/test-project/.env"
      action: "read"
    expect:
      allowed: false
      reason: "匹配拒绝模式"

  # 消息管理
  - name: 添加消息到会话
    input:
      session_id: "test-session"
      message:
        role: "user"
        content: "Hello"
    expect:
      message_count: 1

  - name: 获取会话上下文
    input:
      session_id: "test-session"
    expect:
      messages: []
      compression_points: []

  # 会话状态
  - name: 暂停会话
    input:
      session_id: "test-session"
    expect:
      status: paused

  - name: 恢复会话
    input:
      session_id: "test-session"
    expect:
      status: active

  # 边界条件
  - name: 获取不存在的会话
    input:
      session_id: "non-existent"
    expect:
      error: "SESSION_NOT_FOUND"

  - name: 创建会话时 workspace 不存在
    input:
      workspace: "/non-existent"
    expect:
      error: "WORKSPACE_INVALID"
```

#### 集成测试

```yaml
test_suite: SessionManager Integration Tests
file: tests/integration/session_manager_test.rs

test_cases:
  # 多会话并行
  - name: 两个会话完全隔离
    setup:
      - create_session:
          name: "session-a"
          workspace: "/tmp/project-a"
      - create_session:
          name: "session-b"
          workspace: "/tmp/project-b"
    steps:
      - add_message:
          session: "session-a"
          content: "message for A"
      - add_message:
          session: "session-b"
          content: "message for B"
    verify:
      - session_a.messages: ["message for A"]
      - session_b.messages: ["message for B"]
      - session_a.messages != session_b.messages

  # 会话持久化
  - name: 会话保存和恢复
    setup:
      - create_session:
          name: "persist-test"
          workspace: "/tmp/test"
    steps:
      - add_message:
          content: "test message"
      - save_session:
          id: "persist-test"
      - clear_sessions
      - load_session:
          id: "persist-test"
    verify:
      - session.messages: ["test message"]
      - session.status: active

  # 文件索引
  - name: 文件变更检测
    setup:
      - create_session:
          workspace: "/tmp/test"
      - build_file_index
      - modify_file: "src/main.rs"
    steps:
      - check_file_changes
    verify:
      - changed_files: ["src/main.rs"]

  # 项目类型检测
  - name: 自动检测多种项目类型
    parametric:
      projects:
        - rust: ["Cargo.toml"]
        - node: ["package.json"]
        - python: ["requirements.txt", "pyproject.toml"]
        - go: ["go.mod"]
    verify:
      - detected_type == param.type
```

### 2.3 Agent Runtime 测试

#### 单元测试

```yaml
test_suite: AgentRuntime Unit Tests
file: tests/unit/agent_runtime_test.rs

test_cases:
  # Agent 加载
  - name: 加载 Agent 定义
    input:
      definition: |
        # agents/test/AGENT.md
        ---
        name: test-agent
        role: "You are a test agent"
        ---
    expect:
      agent.name: "test-agent"
      agent.role: "You are a test agent"

  # 变体加载
  - name: 加载 Agent 变体
    setup:
      - create_agent: "agents/test/AGENT.md"
      - create_variant: "agents/test/AGENT.strict.md"
    input:
      agent_id: "test-agent:strict"
    expect:
      agent.variant: "strict"

  # 消息处理
  - name: 处理用户消息
    input:
      agent: "test-agent"
      messages:
        - role: "user"
          content: "Hello"
    expect:
      response.role: "assistant"
      response.content: type(string)

  # 工具调用
  - name: Agent 调用工具
    input:
      agent: "test-agent"
      tools: ["read", "write"]
      messages:
        - role: "user"
          content: "读取 file.txt"
    expect:
      tool_calls: type(array)
      tool_calls[0].name: "read"

  # 错误处理
  - name: 工具调用失败时继续
    input:
      agent: "test-agent"
      messages:
        - role: "user"
          content: "读取不存在的文件"
      tool_response:
        error: "file not found"
    expect:
      response.content: type(string)
      agent.recovered: true

  # 边界条件
  - name: Agent 定义不存在
    input:
      agent_id: "non-existent"
    expect:
      error: "AGENT_NOT_FOUND"

  - name: 空消息处理
    input:
      agent: "test-agent"
      messages: []
    expect:
      error: "EMPTY_MESSAGES"
```

#### 集成测试

```yaml
test_suite: AgentRuntime Integration Tests
file: tests/integration/agent_runtime_test.rs

test_cases:
  # Agent 与 LLM 集成
  - name: 端到端 Agent 执行
    setup:
      - start_mock_llm_server
    steps:
      - create_agent:
          name: "test-agent"
      - send_message:
          content: "Say hello"
      - wait_for_response
    verify:
      - response.content: "Hello"

  # Agent 与 Tool 集成
  - name: Agent 使用工具
    setup:
      - create_agent:
          tools: ["read", "write"]
      - create_test_file: "test.txt"
    steps:
      - send_message:
          content: "读取 test.txt"
      - wait_for_tool_call
      - return_tool_result
      - wait_for_response
    verify:
      - tool_called: "read"
      - response.contains: "test.txt"

  # 多 Agent 协作
  - name: 主从 Agent 协作
    setup:
      - create_agent:
          name: "master"
          role: "协调者"
      - create_agent:
          name: "worker"
          role: "执行者"
    steps:
      - send_message:
          agent: "master"
          content: "让 worker 完成任务"
      - wait_for_delegation
      - execute_worker_task
      - return_result
    verify:
      - master.messages: type(array)
      - worker.executed: true
```

### 2.4 LLM Provider 测试

#### 单元测试

```yaml
test_suite: LLMProvider Unit Tests
file: tests/unit/llm_provider_test.rs

test_cases:
  # 提供者选择
  - name: 选择 Anthropic 提供者
    input:
      provider: "anthropic"
      model: "claude-sonnet-4-20250514"
    expect:
      provider.type: "anthropic"

  - name: 选择 OpenAI 提供者
    input:
      provider: "openai"
      model: "gpt-4"
    expect:
      provider.type: "openai"

  # 消息构建
  - name: 构建 Anthropic API 请求
    input:
      provider: "anthropic"
      messages:
        - role: "user"
          content: "Hello"
      max_tokens: 1000
    expect:
      request.model: "claude-sonnet-4-20250514"
      request.max_tokens: 1000
      request.messages: type(array)

  # 流式响应
  - name: 处理流式响应
    input:
      provider: "anthropic"
      stream: true
    expect:
      response.type: "stream"
      response.chunks: type(array)

  # 重试逻辑
  - name: 网络错误时重试
    input:
      max_retries: 3
      mock_errors: [network_error, network_error]
    expect:
      retry_count: 2
      success: true

  - name: 超过重试次数
    input:
      max_retries: 2
      mock_errors: [network_error, network_error, network_error]
    expect:
      retry_count: 2
      success: false
      error: "MAX_RETRIES_EXCEEDED"

  # Token 计算
  - name: 计算消息 Token 数
    input:
      messages:
        - role: "user"
          content: "Hello, world!"
    expect:
      token_count: type(integer)
      token_count > 0
```

#### 集成测试

```yaml
test_suite: LLMProvider Integration Tests
file: tests/integration/llm_provider_test.rs

test_cases:
  # 真实 API 调用
  - name: 调用 Anthropic API
    tags: [external, slow]
    input:
      provider: "anthropic"
      api_key: env(ANTHROPIC_API_KEY)
      messages:
        - role: "user"
          content: "Say 'API test'"
    expect:
      response.content: "API test"

  - name: 调用 OpenAI API
    tags: [external, slow]
    input:
      provider: "openai"
      api_key: env(OPENAI_API_KEY)
      messages:
        - role: "user"
          content: "Say 'API test'"
    expect:
      response.content: "API test"

  # 流式 API
  - name: Anthropic 流式响应
    tags: [external, slow]
    input:
      provider: "anthropic"
      stream: true
      messages:
        - role: "user"
          content: "Count 1 to 5"
    expect:
      chunks: type(array)
      chunks.length > 1
```

### 2.5 Tool System 测试

#### 单元测试

```yaml
test_suite: ToolSystem Unit Tests
file: tests/unit/tool_system_test.rs

test_cases:
  # 工具注册
  - name: 注册工具
    input:
      tool:
        name: "test-tool"
        description: "A test tool"
        execute: |args| { result: "ok" }
    expect:
      tool.name: "test-tool"
      tools.contains: "test-tool"

  # 工具执行
  - name: 执行工具
    input:
      tool: "read"
      args:
        path: "/tmp/test.txt"
    mock:
      file_system: {"/tmp/test.txt": "content"}
    expect:
      result: "content"

  # 参数验证
  - name: 缺少必需参数
    input:
      tool: "read"
      args: {}
    expect:
      error: "MISSING_REQUIRED_PARAM"
      error.contains: "path"

  - name: 参数类型错误
    input:
      tool: "edit"
      args:
        path: 123
    expect:
      error: "INVALID_PARAM_TYPE"

  # 并发执行
  - name: 多个工具并行执行
    input:
      tools:
        - name: "read"
          args: {path: "a.txt"}
        - name: "read"
          args: {path: "b.txt"}
        - name: "read"
          args: {path: "c.txt"}
    expect:
      results: type(array)
      results.length: 3

  # 错误处理
  - name: 工具执行失败
    input:
      tool: "read"
      args:
        path: "/non-existent"
    expect:
      error: "FILE_NOT_FOUND"

  - name: 工具超时
    input:
      tool: "long-running"
      timeout: 1000
    expect:
      error: "TOOL_TIMEOUT"
```

#### 集成测试

```yaml
test_suite: ToolSystem Integration Tests
file: tests/integration/tool_system_test.rs

test_cases:
  # 基础工具链
  - name: Read → Edit → Write 流程
    setup:
      - create_file: "test.txt"
          content: "Hello World"
    steps:
      - tool: "read"
        args: {path: "test.txt"}
      - tool: "edit"
        args:
          path: "test.txt"
          old_string: "World"
          new_string: "Rust"
      - tool: "write"
        args:
          path: "output.txt"
          content: "{{ read_result }}"
    verify:
      - files_equal: ["test.txt", "output.txt"]

  # 复杂工具链
  - name: 文件搜索与批量处理
    setup:
      - create_files:
          - "src/main.rs"
          - "src/lib.rs"
          - "tests/test.rs"
    steps:
      - tool: "grep"
        args:
          pattern: "fn main"
          path: "src"
      - tool: "edit"
        args:
          apply_to: "{{ grep_result.files }}"
    verify:
      - edited_files: type(array)
      - edited_files.length > 0
```

---

## 3. L1 (P1) 扩展模块测试

### 3.1 模块列表

| 模块 | 文档 | 优先级 |
|------|------|--------|
| Orchestrator | `core/orchestrator.md` | P1 |
| Skill Engine | `agent/skill-engine.md` | P1 |
| Event Loop | `core/event-loop.md` | P1 |
| Hook Engine | `core/hook-engine.md` | P1 |
| Task Manager | `agent/task-manager.md` | P1 |
| MCP Client | `services/mcp-client.md` | P1 |
| Context Compressor | `services/context-compressor.md` | P1 |
| Storage Service | `services/storage-service.md` | P1 |
| Agent Variants | `agent/agent-variants.md` | P1 |

### 3.2 Orchestrator 测试

#### 单元测试

```yaml
test_suite: Orchestrator Unit Tests
file: tests/unit/orchestrator_test.rs

test_cases:
  # 任务分配
  - name: 分配任务给最合适的 Agent
    input:
      task:
        type: "code-review"
        required_skills: ["rust"]
      agents:
        - id: "rust-expert"
          skills: ["rust", "review"]
          workload: 0.3
        - id: "generalist"
          skills: ["review"]
          workload: 0.1
    expect:
      assigned_agent: "rust-expert"

  - name: 负载均衡分配
    input:
      task:
        type: "generic"
      agents:
        - id: "agent-a"
          workload: 0.8
        - id: "agent-b"
          workload: 0.2
    expect:
      assigned_agent: "agent-b"

  # 协作模式
  - name: Master-Worker 模式
    input:
      mode: "master-worker"
      master: "orchestrator"
      workers: ["agent-a", "agent-b"]
      task: "并行处理多个文件"
    expect:
      workers_assigned: ["agent-a", "agent-b"]
      master.coordinating: true

  - name: Pipeline 模式
    input:
      mode: "pipeline"
      stages:
        - agent: "writer"
        - agent: "reviewer"
        - agent: "tester"
      task: "开发新功能"
    expect:
      execution_order: ["writer", "reviewer", "tester"]

  - name: Voting 模式
    input:
      mode: "voting"
      agents: ["agent-a", "agent-b", "agent-c"]
      task: "决策问题"
      threshold: 0.67
    mock:
      votes: [yes, yes, no]
    expect:
      decision: "yes"
      consensus: true
```

#### 集成测试

```yaml
test_suite: Orchestrator Integration Tests
file: tests/integration/orchestrator_test.rs

test_cases:
  # 端到端协作
  - name: 代码审查流程
    setup:
      - create_agents:
          - id: "developer"
          - id: "reviewer"
          - id: "tester"
      - create_code: "src/main.rs"
    steps:
      - assign_task:
          agent: "developer"
          task: "实现功能"
      - wait_for_completion
      - assign_task:
          agent: "reviewer"
          task: "代码审查"
      - wait_for_completion
      - assign_task:
          agent: "tester"
          task: "运行测试"
      - wait_for_completion
    verify:
      - all_tasks.completed: true
      - execution_order: ["developer", "reviewer", "tester"]

  # 错误恢复
  - name: Agent 失败时重新分配
    setup:
      - create_agents:
          - id: "primary"
          - id: "backup"
    steps:
      - assign_task:
          agent: "primary"
          task: "处理任务"
      - simulate_failure: "primary"
      - wait_for_reassignment
    verify:
      - reassigned_to: "backup"
      - task.completed: true
```

### 3.3 Skill Engine 测试

#### 单元测试

```yaml
test_suite: SkillEngine Unit Tests
file: tests/unit/skill_engine_test.rs

test_cases:
  # Skill 加载
  - name: 加载 Skill 定义
    input:
      file: "skills/test/SKILL.md"
      content: |
        ---
        name: test-skill
        triggers:
          - type: keyword
            keyword:
              patterns: ["test"]
        ---
        ## Steps
        ### Step 1
        type: tool
        tool: {name: echo}
    expect:
      skill.name: "test-skill"
      skill.triggers: type(array)

  # 触发器匹配
  - name: 关键词触发器匹配
    input:
      trigger:
        type: "keyword"
        patterns: ["test", "测试"]
        match_type: "contains"
    events:
      - content: "请帮我测试代码"
    expect:
      matched: true

  - name: 关键词触发器不匹配
    input:
      trigger:
        type: "keyword"
        patterns: ["review"]
        match_type: "contains"
    events:
      - content: "请帮我测试代码"
    expect:
      matched: false

  # 文件变更触发
  - name: 文件变更触发器匹配
    input:
      trigger:
        type: "file_change"
        patterns: ["**/*.rs"]
        events: ["modified"]
    events:
      - type: "file_change"
        file: "src/main.rs"
        event: "modified"
    expect:
      matched: true

  # Skill 执行
  - name: 顺序执行步骤
    input:
      skill:
        steps:
          - type: "tool"
            tool: {name: "step1"}
          - type: "tool"
            tool: {name: "step2"}
          - type: "tool"
            tool: {name: "step3"}
    expect:
      executed_steps: ["step1", "step2", "step3"]
      execution_order: sequential

  # 并行执行
  - name: 并行执行步骤
    input:
      skill:
        steps:
          - type: "parallel"
            parallel:
              - type: "tool"
                tool: {name: "task1"}
              - type: "tool"
                tool: {name: "task2"}
              - type: "tool"
                tool: {name: "task3"}
    expect:
      executed_steps: ["task1", "task2", "task3"]
      execution_order: parallel

  # 条件分支
  - name: 条件为真执行 then_steps
    input:
      skill:
        steps:
          - type: "condition"
            condition:
              expression: "{{ value > 5 }}"
            then_steps:
              - type: "tool"
                tool: {name: "then-action"}
            else_steps:
              - type: "tool"
                tool: {name: "else-action"}
      context:
        value: 10
    expect:
      executed: "then-action"

  # 循环执行
  - name: 遍历数组执行
    input:
      skill:
        steps:
          - type: "loop"
            loop:
              over: "{{ files }}"
              steps:
                - type: "tool"
                  tool: {name: "process"}
      context:
        files: ["a.txt", "b.txt", "c.txt"]
    expect:
      execution_count: 3
      processed: ["a.txt", "b.txt", "c.txt"]

  # Skill Pipeline
  - name: 创建并执行 Pipeline
    input:
      pipeline:
        name: "ci-pipeline"
        skills:
          - skill_id: "test"
            depends_on: []
          - skill_id: "build"
            depends_on: ["test"]
          - skill_id: "deploy"
            depends_on: ["build"]
            condition: "{{ build_success }}"
    expect:
      execution_order: ["test", "build", "deploy"]
      dependencies_satisfied: true
```

#### 集成测试

```yaml
test_suite: SkillEngine Integration Tests
file: tests/integration/skill_engine_test.rs

test_cases:
  # 技能嵌套
  - name: Skill 调用另一个 Skill
    setup:
      - register_skill:
          id: "inner"
          steps:
            - type: "tool"
              tool: {name: "inner-action"}
      - register_skill:
          id: "outer"
          steps:
            - type: "skill"
              skill: {skill_id: "inner"}
    execute:
      skill: "outer"
    verify:
      - executed: ["outer", "inner"]
      - inner_action.called: true

  # 端到端 CI Pipeline
  - name: 完整 CI 流程
    setup:
      - create_skills:
          - id: "lint"
          - id: "test-unit"
          - id: "test-integration"
          - id: "build"
          - id: "deploy"
      - create_pipeline:
          name: "ci"
          skills:
            - skill_id: "lint"
            - skill_id: "test-unit"
              depends_on: ["lint"]
            - skill_id: "test-integration"
              depends_on: ["lint"]
            - skill_id: "build"
              depends_on: ["test-unit", "test-integration"]
            - skill_id: "deploy"
              depends_on: ["build"]
    execute:
      pipeline: "ci"
    verify:
      - pipeline.success: true
      - execution_order: ["lint", "test-unit", "test-integration", "build", "deploy"]
```

### 3.4 Event Loop 测试

#### 单元测试

```yaml
test_suite: EventLoop Unit Tests
file: tests/unit/event_loop_test.rs

test_cases:
  # 事件监听
  - name: 注册文件监听器
    input:
      listener:
        type: "file_change"
        pattern: "**/*.rs"
        handler: "on-rust-change"
    expect:
      listener.registered: true

  - name: 移除监听器
    input:
      listener_id: "test-listener"
    expect:
      listener.removed: true

  # 事件触发
  - name: 文件变更触发事件
    input:
      event:
        type: "file_change"
        file: "src/main.rs"
        event: "modified"
      listeners:
        - pattern: "**/*.rs"
    expect:
      triggered_listeners: type(array)
      triggered_listeners.length > 0

  # 防抖处理
  - name: 防抖去重
    input:
      debounce: 100
      events:
        - time: 0
          file: "test.txt"
        - time: 50
          file: "test.txt"
        - time: 150
          file: "test.txt"
    expect:
      triggered_count: 2
      triggered_at: [0, 150]

  # 定时任务
  - name: Cron 定时触发
    input:
      schedule: "0 9 * * *"
      current_time: "2026-03-30 09:00:00"
    expect:
      should_trigger: true

  - name: Cron 时间未到
    input:
      schedule: "0 9 * * *"
      current_time: "2026-03-30 08:59:59"
    expect:
      should_trigger: false
```

#### 集成测试

```yaml
test_suite: EventLoop Integration Tests
file: tests/integration/event_loop_test.rs

test_cases:
  # 文件监控
  - name: 监控文件变更并触发 Skill
    setup:
      - create_skill:
          id: "on-change"
          triggers:
            - type: "file_change"
              patterns: ["**/*.rs"]
      - start_event_loop
    steps:
      - modify_file: "src/main.rs"
      - wait: 500
    verify:
      - skill.triggered: true
      - skill.execution_count: 1

  # Git 集成
  - name: Git push 触发 CI
    setup:
      - create_skill:
          id: "ci-trigger"
          triggers:
            - type: "event"
              event:
                event_type: "git.push"
      - init_git_repo
    steps:
      - git_commit
      - git_push
      - wait: 1000
    verify:
      - skill.triggered: true
```

### 3.5 Hook Engine 测试

#### 单元测试

```yaml
test_suite: HookEngine Unit Tests
file: tests/unit/hook_engine_test.rs

test_cases:
  # Hook 注册
  - name: 注册 before 钩子
    input:
      hook:
        name: "validate-input"
        phase: "before_command"
        priority: 10
        handler: |args| { validate(args) }
    expect:
      hook.registered: true
      hook.phase: "before_command"

  # 钩子执行顺序
  - name: 按优先级执行
    input:
      hooks:
        - name: "low-priority"
          priority: 1
          phase: "before"
        - name: "high-priority"
          priority: 10
          phase: "before"
        - name: "mid-priority"
          priority: 5
          phase: "before"
    execute:
      phase: "before"
    expect:
      execution_order: ["high-priority", "mid-priority", "low-priority"]

  # 钩子拦截
  - name: before 钩子拦截执行
    input:
      hooks:
        - name: "security-check"
          phase: "before_command"
          handler: |args| {
              if dangerous(args)
                return block()
            }
      command: "rm -rf /"
    execute:
      phase: "before_command"
    expect:
      blocked: true
      security_check.called: true

  # 钩子修改
  - name: replace 钩子修改行为
    input:
      hooks:
        - name: "custom-impl"
          phase: "replace_command"
          handler: |args| { custom_impl(args) }
      command: "standard-command"
    execute:
      phase: "replace_command"
    expect:
      executed: "custom-impl"
      standard_implementation: not_called

  # 钩子结果处理
  - name: after 钩子处理结果
    input:
      hooks:
        - name: "log-result"
          phase: "after_command"
          handler: |result| { log(result) }
      command_result:
        success: true
        output: "done"
    execute:
      phase: "after_command"
      context: {result: command_result}
    expect:
      log_result.called: true
      log_contains: "done"
```

### 3.6 Task Manager 测试

#### 单元测试

```yaml
test_suite: TaskManager Unit Tests
file: tests/unit/task_manager_test.rs

test_cases:
  # DAG 解析
  - name: 解析简单 DAG
    input:
      workflow:
        tasks:
          - id: "a"
            depends_on: []
          - id: "b"
            depends_on: ["a"]
          - id: "c"
            depends_on: ["b"]
    expect:
      layers: [["a"], ["b"], ["c"]]

  - name: 解析并行 DAG
    input:
      workflow:
        tasks:
          - id: "a"
            depends_on: []
          - id: "b"
            depends_on: ["a"]
          - id: "c"
            depends_on: ["a"]
          - id: "d"
            depends_on: ["b", "c"]
    expect:
      layers: [["a"], ["b", "c"], ["d"]]

  # 循环依赖检测
  - name: 检测循环依赖
    input:
      workflow:
        tasks:
          - id: "a"
            depends_on: ["b"]
          - id: "b"
            depends_on: ["c"]
          - id: "c"
            depends_on: ["a"]
    expect:
      error: "CIRCULAR_DEPENDENCY"
      cycle: ["a", "b", "c", "a"]

  # 任务状态
  - name: 任务状态转换
    input:
      task:
        id: "test-task"
    transitions:
      - from: "pending"
        to: "ready"
        trigger: "dependencies_met"
      - from: "ready"
        to: "running"
        trigger: "start"
      - from: "running"
        to: "completed"
        trigger: "finish"
    expect:
      final_state: "completed"

  # 条件执行
  - name: 跳过不满足条件的任务
    input:
      tasks:
        - id: "always"
          condition: null
        - id: "conditional"
          condition: "{{ value == true }}"
          depends_on: ["always"]
      context:
        value: false
    expect:
      always.executed: true
      conditional.executed: false
      conditional.status: "skipped"

  # 错误处理
  - name: continue_on_error 时继续
    input:
      tasks:
        - id: "fail-task"
          continue_on_error: true
        - id: "next-task"
          depends_on: ["fail-task"]
      mock:
        fail-task.error: "simulated error"
    expect:
      fail-task.status: "failed"
      next-task.executed: true

  - name: 错误传播
    input:
      tasks:
        - id: "fail-task"
          continue_on_error: false
        - id: "next-task"
          depends_on: ["fail-task"]
      mock:
        fail-task.error: "simulated error"
    expect:
      fail-task.status: "failed"
      next-task.status: "cancelled"
      workflow.status: "failed"
```

### 3.7 MCP Client 测试

#### 单元测试

```yaml
test_suite: MCPClient Unit Tests
file: tests/unit/mcp_client_test.rs

test_cases:
  # 服务器连接
  - name: 连接到 MCP 服务器
    input:
      server:
        name: "test-server"
        command: "npx -y @modelcontextprotocol/server-filesystem"
        args: ["/tmp/test"]
    expect:
      connection.connected: true
      server.capabilities: type(object)

  # 工具发现
  - name: 发现服务器工具
    input:
      server:
        tools:
          - name: "read_file"
          - name: "write_file"
          - name: "list_directory"
    expect:
      discovered_tools: ["read_file", "write_file", "list_directory"]

  # 工具调用
  - name: 调用 MCP 工具
    input:
      server: "filesystem"
      tool: "read_file"
      args:
        path: "/tmp/test.txt"
    mock:
      server_response:
        content: "file content"
    expect:
      result.content: "file content"

  # 资源访问
  - name: 访问 MCP 资源
    input:
      server: "filesystem"
      resource: "file:///tmp/test.txt"
    mock:
      server_response:
        content: "file content"
    expect:
      result.content: "file content"

  # 错误处理
  - name: 服务器不可达
    input:
      server:
        command: "non-existent-server"
    expect:
      error: "SERVER_UNREACHABLE"

  - name: 工具调用超时
    input:
      server: "slow-server"
      tool: "slow_operation"
      timeout: 1000
    mock:
      server_delay: 5000
    expect:
      error: "TIMEOUT"
```

### 3.8 Context Compressor 测试

#### 单元测试

```yaml
test_suite: ContextCompressor Unit Tests
file: tests/unit/context_compressor_test.rs

test_cases:
  # Token 估算
  - name: 估算消息 Token 数
    input:
      messages:
        - role: "user"
          content: "Hello, world!"
        - role: "assistant"
          content: "Hi there!"
    expect:
      token_count: type(integer)
      token_count > 0

  # 压缩触发
  - name: 消息数超限触发压缩
    input:
      messages: generate(60)
      threshold:
        message_count: 50
    expect:
      should_compress: true

  - name: Token 数超限触发压缩
    input:
      messages:
        - content: generate_large(100000)
      threshold:
        token_count: 100000
    expect:
      should_compress: true

  # 摘要压缩
  - name: 生成摘要
    input:
      messages:
        - role: "user"
          content: "我想实现一个用户登录功能"
        - role: "assistant"
          content: "好的，我们可以使用 JWT..."
        - role: "user"
          content: "那数据库表怎么设计？"
        - role: "assistant"
          content: "我们需要 users 表..."
      method: "summary"
      keep_recent: 2
    mock:
      llm_response: |
        用户讨论了登录功能的实现，
        包括 JWT 认证和数据库设计。
    expect:
      compression.summary: type(string)
      compression.token_saved > 0

  # 语义压缩
  - name: 提取关键信息
    input:
      messages: [...]
      method: "semantic"
      keep_types: ["decision", "requirement"]
    expect:
      compression.decisions: type(array)
      compression.requirements: type(array)

  # 压缩点使用
  - name: 获取压缩后的消息
    input:
      messages: [original...]
      compression_points:
        - summary: "之前的对话摘要"
          key_points: ["决策A", "决策B"]
          decisions: [{topic: "架构", decision: "使用 Rust"}]
    expect:
      llm_messages:
        - role: "system"
          content: "之前的对话摘要\n关键决策:\n- 架构: 使用 Rust"
        - role: "user"
          content: "原始消息"
```

### 3.9 Storage Service 测试

#### 单元测试

```yaml
test_suite: StorageService Unit Tests
file: tests/unit/storage_service_test.rs

test_cases:
  # 会话存储
  - name: 保存会话
    input:
      session:
        id: "test-session"
        name: "Test"
        status: "active"
    expect:
      saved: true

  - name: 加载会话
    setup:
      - save_session:
          id: "test-session"
    input:
      session_id: "test-session"
    expect:
      session.id: "test-session"
      session.status: "active"

  - name: 删除会话
    setup:
      - save_session:
          id: "test-session"
    input:
      session_id: "test-session"
    expect:
      deleted: true
      load_session: null

  # 消息存储
  - name: 追加消息
    input:
      session_id: "test-session"
      message:
        role: "user"
        content: "Hello"
    expect:
      appended: true

  - name: 获取消息历史
    setup:
      - add_messages:
          session_id: "test"
          messages: [msg1, msg2, msg3]
    input:
      session_id: "test"
      limit: 10
    expect:
      messages: type(array)
      messages.length: 3

  # 压缩点存储
  - name: 保存压缩点
    input:
      session_id: "test"
      compression:
        summary: "对话摘要"
        token_saved: 1000
    expect:
      saved: true

  # 查询功能
  - name: 列出活跃会话
    setup:
      - create_sessions:
          - id: "a"
            status: "active"
          - id: "b"
            status: "active"
          - id: "c"
            status: "archived"
    input:
      filter:
        status: "active"
    expect:
      sessions: type(array)
      sessions.length: 2

  # 备份恢复
  - name: 备份数据库
    input:
      path: "/backup/backup.db"
    expect:
      backup.created: true

  - name: 从备份恢复
    setup:
      - create_backup
    input:
      path: "/backup/backup.db"
    expect:
      restored: true
```

---

## 4. 测试基础设施

### 4.1 测试框架

```yaml
testing_stack:
  unit_tests:
    framework: "cargo test"
    directory: "tests/unit"
    coverage_target: 80%

  integration_tests:
    framework: "cargo test"
    directory: "tests/integration"
    coverage_target: 60%

  e2e_tests:
    framework: "playwright"
    directory: "tests/e2e"
    coverage_target: 40%
```

### 4.2 测试工具

```yaml
test_tools:
  # 单元测试
  - name: "cargo-test"
    description: "Rust 内置测试框架"

  - name: "mockall"
    description: "Mock 框架"

  # 测试辅助
  - name: "tempfile"
    description: "临时文件管理"

  - name: "assert_fs"
    description: "文件系统断言"

  # 集成测试
  - name: "docker-compose"
    description: "测试环境编排"

  # E2E 测试
  - name: "playwright"
    description: "浏览器自动化"

  # 性能测试
  - name: "criterion"
    description: "Rust 性能测试框架"
```

### 4.3 Mock 服务

```yaml
mock_services:
  llm_mock:
    name: "Mock LLM Server"
    purpose: "模拟 LLM API 响应"
    implementation: "tests/mocks/llm_server.rs"

  mcp_mock:
    name: "Mock MCP Server"
    purpose: "模拟 MCP 协议服务"
    implementation: "tests/mocks/mcp_server.rs"

  git_mock:
    name: "Mock Git Repository"
    purpose: "模拟 Git 仓库"
    implementation: "tests/mocks/git_repo.rs"
```

---

## 5. 测试执行计划

### 5.1 单元测试计划

| 模块 | 测试用例数 | 目标覆盖率 | 状态 |
|------|-----------|-----------|------|
| Session Manager | 15 | 85% | ⏳ |
| Agent Runtime | 12 | 80% | ⏳ |
| LLM Provider | 10 | 75% | ⏳ |
| Tool System | 18 | 85% | ⏳ |
| Orchestrator | 10 | 75% | ⏳ |
| Skill Engine | 20 | 80% | ⏳ |
| Event Loop | 8 | 75% | ⏳ |
| Hook Engine | 12 | 80% | ⏳ |
| Task Manager | 15 | 80% | ⏳ |
| MCP Client | 10 | 75% | ⏳ |
| Context Compressor | 8 | 70% | ⏳ |
| Storage Service | 12 | 80% | ⏳ |

### 5.2 集成测试计划

| 场景 | 测试用例数 | 涉及模块 | 状态 |
|------|-----------|---------|------|
| 多会话并行 | 5 | Session Manager | ⏳ |
| Agent 协作 | 8 | Agent Runtime, Orchestrator | ⏳ |
| Skill Pipeline | 10 | Skill Engine, Task Manager | ⏳ |
| 事件驱动自动化 | 6 | Event Loop, Hook Engine | ⏳ |
| MCP 集成 | 5 | MCP Client, Tool System | ⏳ |
| 会话持久化 | 4 | Session Manager, Storage | ⏳ |
| 完整 CI 流程 | 1 | 全部模块 | ⏳ |

### 5.3 E2E 测试计划

| 场景 | 描述 | 状态 |
|------|------|------|
| 代码开发流程 | 创建 → 编码 → 审查 → 测试 → 部署 | ⏳ |
| Bug 修复流程 | 报告 → 定位 → 修复 → 验证 | ⏳ |
| 代码审查流程 | 触发 → 分析 → 反馈 | ⏳ |

---

## 6. 测试覆盖率目标

### 6.1 整体目标

```
整体覆盖率目标
├── 单元测试: ≥ 80%
├── 集成测试: ≥ 60%
└── E2E 测试: ≥ 40%

模块覆盖率要求
├── P0 核心模块: ≥ 85%
├── P1 扩展模块: ≥ 75%
└── P2 增强模块: ≥ 60%
```

### 6.2 关键路径覆盖

```
关键用户路径
├── Agent 创建与执行: 100%
├── Skill 定义与触发: 90%
├── 多会话管理: 90%
├── 工具调用链: 85%
└── MCP 工具集成: 75%
```

---

## 7. 持续集成配置

### 7.1 CI Pipeline

```yaml
# .github/workflows/test.yml
name: Test Pipeline

on: [push, pull_request]

jobs:
  unit-tests:
    name: Unit Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run unit tests
        run: |
          cargo test --lib --no-fail-fast
          cargo test --lib --coverage
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  integration-tests:
    name: Integration Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        run: cargo test --test integration

  e2e-tests:
    name: E2E Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install dependencies
        run: npm ci
      - name: Run E2E tests
        run: npm run test:e2e

  lint:
    name: Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run clippy
        run: cargo clippy -- -D warnings
```

---

## 8. 测试数据管理

### 8.1 测试夹具

```
tests/fixtures/
├── agents/              # 测试用 Agent 定义
├── skills/              # 测试用 Skill 定义
├── projects/            # 测试用项目
│   ├── rust-project/
│   ├── node-project/
│   └── python-project/
└── data/                # 测试数据
    ├── large_file.txt
    └── sample_code.rs
```

### 8.2 测试数据生成

```rust
// tests/helpers/mod.rs

pub mod generators {
    /// 生成测试消息
    pub fn generate_message(count: usize) -> Vec<Message> {
        (0..count)
            .map(|i| Message {
                id: format!("msg{}", i),
                role: if i % 2 == 0 { "user" } else { "assistant" },
                content: format!("Message {}", i),
                timestamp: Utc::now(),
            })
            .collect()
    }

    /// 生成测试会话
    pub fn generate_session(name: &str) -> Session {
        Session {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            status: SessionStatus::Active,
            workspace: PathBuf::from(format!("/tmp/{}", name)),
            ..Default::default()
        }
    }

    /// 生成测试 Agent
    pub fn generate_agent(name: &str) -> AgentDefinition {
        AgentDefinition {
            id: name.to_string(),
            name: name.to_string(),
            role: format!("You are {}", name),
            ..Default::default()
        }
    }
}
```

---

## 9. 测试最佳实践

### 9.1 命名规范

```rust
// 测试文件命名
// tests/unit/{module}_test.rs

// 测试函数命名
#[test]
fn test_session_manager_create_session_success() {
    // Given-When-Then
}

#[test]
fn test_session_manager_create_session_with_invalid_workspace_fails() {
    // Given-When-Then
}
```

### 9.2 AAA 模式

```rust
#[test]
fn test_scenario() {
    // Arrange - 准备测试数据
    let manager = SessionManager::new();
    let config = SessionConfig {
        workspace: PathBuf::from("/tmp/test"),
        ..Default::default()
    };

    // Act - 执行被测试的操作
    let result = manager.create_session(config);

    // Assert - 验证结果
    assert!(result.is_ok());
    let session = result.unwrap();
    assert_eq!(session.status, SessionStatus::Active);
}
```

### 9.3 参数化测试

```rust
#[rstest]
#[case("rust", "Cargo.toml")]
#[case("node", "package.json")]
#[case("python", "requirements.txt")]
fn test_project_type_detection(
    #[case] expected: &str,
    #[case] marker: &str,
) {
    // 测试不同项目类型的检测
}
```

---

## 10. 附录

### 10.1 测试环境配置

```toml
# .cargo/config.toml
[env]
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER = "docker"

[build]
target-dir = "target"

[target.x86_64-unknown-linux-gnu]
linker = "x86_64-linux-gnu-gcc"
```

### 10.2 Docker 测试环境

```yaml
# docker-compose.test.yml
version: '3.8'
services:
  test-runner:
    build: .
    volumes:
      - ./target:/app/target
      - ./tests:/app/tests
    environment:
      - RUST_BACKTRACE=1
      - TEST_LOG=trace
```

### 10.3 性能基准

```
性能基准目标

Session Manager:
- 会话创建: < 100ms
- 路径检查: < 1ms
- 消息追加: < 5ms

Agent Runtime:
- Agent 加载: < 50ms
- 消息处理: < 500ms (不含 LLM)

LLM Provider:
- 请求构建: < 10ms
- 流式响应首字节: < 1000ms

Tool System:
- 工具调用: < 10ms (不含实际工具执行)
```

---

**文档版本:** 1.0
**创建日期:** 2026-03-30
**覆盖范围:** L0 (P0) + L1 (P1) 模块
