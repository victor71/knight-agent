# External Agent (外部 Agent)

## 概述

### 职责描述

External Agent 模块负责集成和调用外部 Agent 服务，包括：

- 外部 Agent 生命周期管理（启动、停止、监控）
- 与外部 Agent 的进程间通信
- 输出流处理和结果回传
- 错误处理和超时控制
- 资源管理和清理

### 设计目标

1. **透明性**: 调用方无感知内部实现差异
2. **可靠性**: 完善的进程管理和错误处理
3. **可控性**: 支持超时、中断、资源限制
4. **可观测性**: 完整的执行日志和状态追踪

### 与内置 Agent 的区别

| 维度 | 内置 Agent | 外部 Agent |
|------|------------|------------|
| **实现方式** | LLM Provider 调用 | 子进程执行 |
| **上下文** | Knight 统一管理 | 独立会话 |
| **工具集** | Knight 工具系统 | Agent 自带工具 |
| **通信方式** | 函数调用 | 进程 STDIN/STDOUT |
| **生命周期** | 内存对象 | 进程对象 |
| **适用场景** | 简单任务、快速响应 | 复杂任务、深度交互 |

### 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| Agent Runtime | 依赖 | 基础 Agent 接口 |
| Session Manager | 依赖 | 获取会话上下文 |
| Tool System | 依赖 | 结果回传 |
| Monitor | 依赖 | 执行统计 |

---

## 接口定义

### Agent 类型扩展

在 `AgentDefinition` 中添加外部 Agent 类型：

```yaml
# Agent 类型
AgentType:
  enum: [llm, external, hybrid]
  description: |
    - llm: 内置 LLM Agent
    - external: 外部 Agent（如 Claude Code）
    - hybrid: 混合模式（LLM + 外部能力）

# 外部 Agent 配置
ExternalAgentConfig:
  command:
    type: string
    description: 执行命令 (如 "claude")
  args:
    type: array<string>
    description: 启动参数
  env:
    type: map<string, string>
    description: 环境变量
  working_dir:
    type: string
    description: 工作目录
  timeout:
    type: integer
    description: 超时时间（秒）
  stream_output:
    type: boolean
    description: 是否流式输出
  input_mode:
    type: enum
    values: [interactive, batch, pipe]
    description: 输入模式
```

### 外部 Agent 接口

```yaml
# External Agent 接口定义
ExternalAgent:
  # ========== Agent 发现 ==========
  discover:
    description: 发现可用的外部 Agent
    inputs:
      none
    outputs:
      agents:
        type: array<DiscoveredAgent>

  check_availability:
    description: 检查特定外部 Agent 是否可用
    inputs:
      agent_type:
        type: string
        required: true
        description: Agent 类型 (如 "claude-code")
    outputs:
      available:
        type: boolean
      reason:
        type: string
        description: 不可用时的原因
      version:
        type: string
        description: 已安装版本

  install:
    description: 指导用户安装外部 Agent
    inputs:
      agent_type:
        type: string
        required: true
    outputs:
      instructions:
        type: string
        description: 安装指导

  # ========== 生命周期 ==========
  spawn:
    description: 启动外部 Agent
    inputs:
      config:
        type: ExternalAgentConfig
        required: true
      session_context:
        type: SessionContext
        required: true
      task:
        type: string
        description: 初始任务描述
        required: true
    outputs:
      process_id:
        type: string
      agent_id:
        type: string

  terminate:
    description: 终止外部 Agent
    inputs:
      agent_id:
        type: string
        required: true
      force:
        type: boolean
        default: false
    outputs:
      success:
        type: boolean
      exit_code:
        type: integer

  # ========== 交互 ==========
  send_input:
    description: 向外部 Agent 发送输入
    inputs:
      agent_id:
        type: string
        required: true
      input:
        type: string
        required: true
      is_final:
        type: boolean
        description: 是否为最终输入（如 Ctrl+D）
        default: false
    outputs:
      success:
        type: boolean

  get_output:
    description: 获取输出（非阻塞）
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      output:
        type: string
      is_complete:
        type: boolean

  # ========== 监控 ==========
  get_status:
    description: 获取外部 Agent 状态
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      status:
        type: ExternalAgentStatus

  wait_for_completion:
    description: 等待 Agent 完成
    inputs:
      agent_id:
        type: string
        required: true
      timeout:
        type: integer
        description: 超时时间（秒）
    outputs:
      exit_code:
        type: integer
      final_output:
        type: string

  # ========== 控制 ==========
  interrupt:
    description: 中断外部 Agent（发送 SIGINT）
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  resume:
    description: 恢复外部 Agent
    inputs:
      agent_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean
```

---

## 数据结构

### DiscoveredAgent

```yaml
DiscoveredAgent:
  type:
    type: string
    description: Agent 类型 (如 "claude-code")
  name:
    type: string
    description: 显示名称
  available:
    type: boolean
    description: 是否可用
  installed:
    type: boolean
    description: 是否已安装
  version:
    type: string
    nullable: true
    description: 已安装版本
  path:
    type: string
    nullable: true
    description: 可执行文件路径
  reason:
    type: string
    nullable: true
    description: 不可用原因
  install_url:
    type: string
    nullable: true
    description: 安装链接
```

### ExternalAgentStatus

```yaml
ExternalAgentStatus:
  agent_id:
    type: string
  process_id:
    type: string
  state:
    type: enum
    values: [starting, running, waiting_input, completed, error, killed]
  started_at:
    type: datetime
  last_output_at:
    type: datetime
  exit_code:
    type: integer
    nullable: true
  output_lines:
    type: integer
    description: 已输出行数
  memory_mb:
    type: float
    description: 内存使用
  cpu_percent:
    type: float
```

### ClaudeCodeConfig (外部 Agent 配置示例)

```yaml
# Claude Code 作为外部 Agent
claude-code:
  type: external
  name: Claude Code
  description: Anthropic 的 Claude Code CLI 工具

  config:
    command: claude
    args:
      - --print
      - --agent:code
    working_dir: "{{ workspace }}"
    timeout: 600
    stream_output: true
    input_mode: pipe

  # 环境变量
  env:
    ANTHROPIC_API_KEY: "{{ env.ANTHROPIC_API_KEY }}"
    CLAUDE_MD_API_URL: "{{ env.CLAUDE_MD_API_URL }}"

  # 会话同步
  sync_context:
    enabled: true
    sync_files: true
    sync_instructions: false

  # 权限
  permissions:
    auto_approve: false
    allowed_tools:
      - Read
      - Write
      - Edit
      - Bash
      - Glob
      - Grep
    denied_tools: []
```

---

## 实现逻辑

### 进程管理器

```rust
// 外部 Agent 进程管理器
pub struct ExternalAgentManager {
    processes: RwLock<HashMap<String, ManagedProcess>>,
    config: ExternalAgentConfig,
}

pub struct ManagedProcess {
    child: Child,
    id: String,
    state: ProcessState,
    started_at: Instant,
    config: ExternalAgentConfig,
    output_buffer: RwLock<Vec<String>>,
}

impl ExternalAgentManager {
    /// 启动外部 Agent
    pub async fn spawn(
        &self,
        config: &ExternalAgentConfig,
        task: &str,
    ) -> Result<String> {
        let agent_id = generate_id();

        // 构建命令
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        // 设置工作目录
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        // 设置环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // STDIN: 管道模式
        cmd.stdin(Stdio::piped());

        // STDOUT: PIPE 用于流式读取
        cmd.stdout(Stdio::piped());

        // STDERR: 合并到 STDOUT
        cmd.stderr(Stdio::inherit());

        // 启动进程
        let mut child = cmd.spawn()
            .map_err(|e| Error::ProcessSpawnFailed(e.to_string()))?;

        // 获取 STDIN writer
        let stdin = child.stdin.take()
            .ok_or(Error::StdinNotAvailable)?;

        // 发送初始任务
        write_to_stdin(stdin, task).await?;

        // 保存进程信息
        let managed = ManagedProcess {
            child,
            id: agent_id.clone(),
            state: ProcessState::Running,
            started_at: Instant::now(),
            config: config.clone(),
            output_buffer: RwLock::new(Vec::new()),
        };

        self.processes.write().await
            .insert(agent_id.clone(), managed);

        // 启动输出监听
        self.start_output_listener(agent_id.clone()).await;

        Ok(agent_id)
    }

    /// 发送输入到外部 Agent
    pub async fn send_input(
        &self,
        agent_id: &str,
        input: &str,
        is_final: bool,
    ) -> Result<()> {
        let processes = self.processes.read().await;
        let process = processes.get(agent_id)
            .ok_or(Error::AgentNotFound)?;

        let mut stdin = process.child.stdin.take()
            .ok_or(Error::StdinNotAvailable)?;

        write_to_stdin(stdin, input).await?;

        if is_final {
            drop(stdin);
        } else {
            process.child.stdin = Some(stdin);
        }

        Ok(())
    }

    /// 终止外部 Agent
    pub async fn terminate(&self, agent_id: &str, force: bool) -> Result<i32> {
        let mut processes = self.processes.write().await;
        let process = processes.get_mut(agent_id)
            .ok_or(Error::AgentNotFound)?;

        if force {
            process.child.kill().await
                .map_err(|e| Error::KillFailed(e.to_string()))?;
        } else {
            // 优雅终止：发送 SIGTERM
            process.child.signal(syscall::SIGTERM)?;
        }

        let exit_code = process.child.wait().await?
            .code()
            .unwrap_or(-1);

        process.state = ProcessState::Completed;
        processes.remove(agent_id);

        Ok(exit_code)
    }
}
```

### 输出监听器

```rust
impl ExternalAgentManager {
    /// 启动后台输出监听
    async fn start_output_listener(&self, agent_id: String) {
        let processes = Arc::clone(&self.processes);

        tokio::spawn(async move {
            let mut reader = {
                let procs = processes.read().await;
                let proc = procs.get(&agent_id)?;
                proc.child.stdout.take()
            };

            if let Some(stdout) = reader {
                let mut lines = lines_stream(stdout);
                while let Some(line_result) = lines.next().await {
                    match line_result {
                        Ok(line) => {
                            let mut procs = processes.write().await;
                            if let Some(proc) = procs.get_mut(&agent_id) {
                                proc.output_buffer.write().await
                                    .push(line.clone());

                                // 触发输出回调
                                if let Some(callback) = &proc.output_callback {
                                    callback(&line);
                                }
                            }
                        }
                        Err(e) => {
                            // 读取结束
                            break;
                        }
                    }
                }
            }
            Some(())
        });
    }
}
```

### 与 Claude Code 集成

```rust
/// Claude Code 适配器
pub struct ClaudeCodeAdapter {
    manager: ExternalAgentManager,
    monitor: Arc<Monitor>,
}

impl ClaudeCodeAdapter {
    /// 调用 Claude Code 执行任务
    pub async fn invoke(
        &self,
        task: &str,
        workspace: &Path,
        options: ClaudeCodeOptions,
    ) -> Result<ClaudeCodeResult> {
        let config = ClaudeCodeConfig {
            command: "claude".to_string(),
            args: vec![
                "--print".to_string(),
                "--agent".to_string(),
                options.agent.clone().unwrap_or_else(|| "code".to_string()),
                "--no-input".to_string(),
            ],
            working_dir: workspace.to_string_lossy().to_string(),
            timeout: options.timeout.unwrap_or(600),
            stream_output: true,
            input_mode: InputMode::Pipe,
            env: HashMap::new(),
        };

        // 启动 Claude Code
        let agent_id = self.manager.spawn(&config, task).await?;

        // 收集输出
        let mut output = String::new();
        let start_time = Instant::now();

        loop {
            let status = self.manager.get_status(&agent_id).await?;

            match status.state {
                ProcessState::Completed => {
                    let buffer = self.get_output_buffer(&agent_id).await?;
                    output = buffer.join("\n");
                    break;
                }
                _ => {
                    // 流式输出
                    let new_output = self.get_latest_output(&agent_id).await?;
                    if !new_output.is_empty() {
                        output.push_str(&new_output);
                        output.push('\n');
                    }
                }
            }

            // 超时检查
            if start_time.elapsed().as_secs() > config.timeout as u64 {
                self.manager.terminate(&agent_id, true).await?;
                return Err(Error::Timeout);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 记录统计
        self.monitor.record_external_agent_call(
            "claude-code",
            output.len(),
            start_time.elapsed().as_secs(),
        ).await;

        Ok(ClaudeCodeResult {
            output,
            exit_code: status.exit_code,
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}

/// Claude Code 选项
struct ClaudeCodeOptions {
    agent: Option<String>,
    timeout: Option<u64>,
    workspace: Option<String>,
}

/// Claude Code 结果
struct ClaudeCodeResult {
    output: String,
    exit_code: Option<i32>,
    duration_ms: u64,
}
```

---

## Claude Code 集成方式

### 方式一：命令行模式（推荐）

```bash
# 通过 --print 模式调用，不启动交互式界面
claude --print --agent code --no-input <<EOF
请审查 src 目录的代码，找出潜在的 Bug
EOF
```

**优点**：
- 简单直接，无需交互
- 输出到 STDOUT，便于捕获
- 适合管道集成

**限制**：
- 无实时流式输出
- 每次调用独立会话

### 方式二：MCP 协议模式

```bash
# Claude Code 作为 MCP 服务器
claude --mcp-server
```

通过 MCP 协议与 Claude Code 通信，支持：
- 工具调用
- 流式响应
- 会话保持

### 方式三：进程通信模式

```bash
# 交互模式，通过管道通信
claude --agent code

# 发送命令
{"type": "task", "content": "审查代码"}^D
{"type": "interrupt"}^D
```

---

## Agent 发现机制

### 发现流程

```
外部 Agent 发现
        │
        ▼
┌──────────────────────────────┐
│ 1. 扫描已知 Agent 类型        │
│    - claude-code             │
│    - codex                   │
│    - github-copilot          │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 检查每个 Agent 是否安装    │
│    - 查找 PATH               │
│    - 检查已知安装位置         │
│    - 尝试验证版本            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 返回发现结果              │
│    - installed: true/false   │
│    - version: "x.x.x"        │
│    - path: "/usr/bin/claude" │
│    - install_url: "..."      │
└──────────────────────────────┘
```

### 发现实现

```rust
// 外部 Agent 发现器
pub struct ExternalAgentDiscoverer {
    agent_definitions: HashMap<String, AgentDefinition>,
}

pub struct AgentDefinition {
    agent_type: String,
    name: String,
    command: String,
    version_flags: Vec<String>,
    install_url: String,
    install_instructions: String,
}

impl ExternalAgentDiscoverer {
    /// 发现所有外部 Agent
    pub async fn discover(&self) -> Vec<DiscoveredAgent> {
        let mut results = Vec::new();

        for (_, def) in &self.agent_definitions {
            let discovered = self.check_agent(&def).await;
            results.push(discovered);
        }

        results
    }

    /// 检查单个 Agent 是否可用
    pub async fn check_agent(&self, def: &AgentDefinition) -> DiscoveredAgent {
        let check_result = self.find_executable(&def.command).await;

        match check_result {
            Some(path) => {
                // Agent 已安装，尝试获取版本
                let version = self.get_version(&path, &def.version_flags).await;

                DiscoveredAgent {
                    type: def.agent_type.clone(),
                    name: def.name.clone(),
                    available: true,
                    installed: true,
                    version,
                    path: Some(path),
                    reason: None,
                    install_url: None,
                }
            }
            None => {
                // Agent 未安装
                DiscoveredAgent {
                    type: def.agent_type.clone(),
                    name: def.name.clone(),
                    available: false,
                    installed: false,
                    version: None,
                    path: None,
                    reason: Some("Not found in PATH".to_string()),
                    install_url: Some(def.install_url.clone()),
                }
            }
        }
    }

    /// 查找可执行文件
    async fn find_executable(&self, command: &str) -> Option<String> {
        // 1. 直接尝试执行（检查是否在 PATH 中）
        if Command::new(command).arg("--version").output().await.is_ok() {
            return Some(command.to_string());
        }

        // 2. Windows 特定位置
        #[cfg(windows)]
        {
            let windows_paths = vec![
                format!("C:\\Program Files\\Claude\\bin\\{}.exe", command),
                format!("C:\\Users\\{}\\AppData\\Local\\Programs\\Claude\\{}.exe",
                    std::env::var("USERNAME").unwrap_or_default(), command),
            ];

            for path in windows_paths {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }

        // 3. macOS 特定位置
        #[cfg(target_os = "macos")]
        {
            let macos_paths = vec![
                format!("/Applications/Claude.app/Contents/MacOS/{}", command),
                format!("/usr/local/bin/{}", command),
            ];

            for path in macos_paths {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }

        // 4. Linux 特定位置
        #[cfg(target_os = "linux")]
        {
            let linux_paths = vec![
                format!("/usr/bin/{}", command),
                format!("/usr/local/bin/{}", command),
                format!("~/.local/bin/{}", command),
            ];

            for path in linux_paths {
                let expanded = shellexp::expand(&path).ok()?;
                if std::path::Path::new(&expanded).exists() {
                    return Some(expanded);
                }
            }
        }

        None
    }

    /// 获取版本号
    async fn get_version(&self, path: &str, version_flags: &[String]) -> Option<String> {
        let mut cmd = Command::new(path);

        // 尝试不同的版本标志
        for flag in version_flags {
            cmd.arg(flag);
        }

        // 如果没有定义标志，尝试 --version
        if version_flags.is_empty() {
            cmd.arg("--version");
        }

        let output = cmd.output().await.ok()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 解析版本号
            Self::parse_version(&stdout)
        } else {
            None
        }
    }

    /// 解析版本号
    fn parse_version(output: &str) -> Option<String> {
        // 尝试匹配 "x.y.z" 格式
        let re = Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
        re.captures(output)
            .map(|c| c.get(1).unwrap().as_str().to_string())
    }

    /// 获取安装指导
    pub fn get_install_instructions(&self, agent_type: &str) -> Option<String> {
        self.agent_definitions
            .get(agent_type)
            .map(|def| def.install_instructions.clone())
    }
}
```

### Agent 定义注册表

```rust
impl Default for ExternalAgentDiscoverer {
    fn default() -> Self {
        let mut definitions = HashMap::new();

        // Claude Code
        definitions.insert("claude-code".to_string(), AgentDefinition {
            agent_type: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            command: "claude".to_string(),
            version_flags: vec!["--version".to_string()],
            install_url: "https://docs.anthropic.com/en/docs/claude-code".to_string(),
            install_instructions: r#"
Claude Code 安装指南:

macOS:
  brew install anthropic/claude-code/claude-code

Linux:
  npm install -g @anthropic-ai/claude-code

Windows:
  npm install -g @anthropic-ai/claude-code

安装后验证:
  claude --version
"#.to_string(),
        });

        // GitHub Copilot
        definitions.insert("github-copilot".to_string(), AgentDefinition {
            agent_type: "github-copilot".to_string(),
            name: "GitHub Copilot".to_string(),
            command: "copilot".to_string(),
            version_flags: vec!["--version".to_string()],
            install_url: "https://github.com/features/copilot".to_string(),
            install_instructions: r#"
GitHub Copilot CLI 安装指南:

# 使用 npm 安装
npm install -g @githubnext/copilot-cli

安装后验证:
  copilot --version
"#.to_string(),
        });

        Self {
            agent_definitions: definitions,
        }
    }
}
```

### 调用前检查

```rust
impl ExternalAgentManager {
    /// 确保 Agent 可用（调用前检查）
    pub async fn ensure_available(&self, agent_type: &str) -> Result<()> {
        let discoverer = ExternalAgentDiscoverer::default();
        let check = discoverer.check_agent_by_type(agent_type).await;

        if !check.available {
            match &check.install_url {
                Some(url) => {
                    return Err(Error::AgentNotInstalled {
                        agent_type: agent_type.to_string(),
                        install_url: url.clone(),
                        install_instructions: discoverer
                            .get_install_instructions(agent_type)
                            .unwrap_or_default(),
                    });
                }
                None => {
                    return Err(Error::AgentNotFound {
                        agent_type: agent_type.to_string(),
                        reason: check.reason.unwrap_or_default(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 启动前自动检查
    pub async fn spawn_with_check(
        &self,
        config: &ExternalAgentConfig,
        task: &str,
    ) -> Result<String> {
        // 调用前检查
        self.ensure_available(&config.agent_type).await?;

        // 执行启动
        self.spawn(config, task).await
    }
}
```

---

## 配置

### 外部 Agent 注册

```yaml
# config/external-agents.yaml
external_agents:
  claude-code:
    enabled: true
    command: claude
    args:
      - --print
      - --agent:code
    timeout: 600
    retry: 2

  codex:
    enabled: true
    command: openai.Codex
    # 或通过 API 调用
    api_endpoint: https://api.openai.com/v1/agents/codex
    timeout: 300
```

### Claude Code 权限配置

```yaml
# config/claude-code.yaml
claude_code:
  # 命令配置
  command: claude
  default_agent: code

  # 工作目录
  workspace:
    inherit_from_session: true
    fallback: ~/.knight-agent/workspaces/default

  # 超时配置
  timeout:
    default: 600
    max: 3600

  # 重试策略
  retry:
    enabled: true
    max_attempts: 2
    delay: 5000

  # 输出处理
  output:
    stream_to_console: true
    capture_to_session: true
    format: markdown

  # 权限控制
  permissions:
    auto_approve_dangerous: false
    allowed_tools:
      - Read
      - Write
      - Edit
      - Bash
      - Glob
      - Grep
```

---

## CLI 集成

### /invoke 命令

```bash
# 调用外部 Agent（自动检查安装状态）
knight> /invoke claude-code --task "审查 src 目录代码"

正在启动 Claude Code...
[Claude Code] 开始审查代码...
...

✅ Claude Code 执行完成 (退出码: 0)
执行时间: 45s

# 如果未安装
knight> /invoke claude-code --task "审查代码"
❌ Claude Code 未安装

   安装指南:
   macOS:  brew install anthropic/claude-code/claude-code
   Linux:  npm install -g @anthropic-ai/claude-code
   Windows: npm install -g @anthropic-ai/claude-code

   文档: https://docs.anthropic.com/en/docs/claude-code

# 指定 Agent 类型
knight> /invoke claude-code --agent reviewer --task "代码审查"

# 指定工作目录
knight> /invoke claude-code --task "重构" --workspace ./project
```

### /agents 命令扩展

```bash
# 列出所有 Agent（包括外部，显示安装状态）
knight> /list-agents

内置 Agent:
  - coder         [active]
  - reviewer      [idle]
  - planner       [idle]

外部 Agent:
  ✅ claude-code       (v1.2.3)  [available]
  ❌ codex             [not installed]
  ⚠️  github-copilot   (v0.3.1)  [available]

# 查看 Agent 详情
knight> /agent claude-code --info

Claude Code:
  Type: external
  Command: claude --print --agent code
  Version: 1.2.3
  Path: /usr/local/bin/claude
  Status: available

# 查看未安装 Agent 的安装指导
knight> /agent codex --info

Codex:
  Type: external
  Status: not installed
  Install: npm install -g @openai/codex
  Docs: https://platform.openai.com/docs/codex

# 检查外部 Agent 可用性
knight> /check-external-agents

外部 Agent 检查:
  ✅ claude-code: 1.2.3 (/usr/local/bin/claude)
  ❌ codex: 未安装
  ⚠️  github-copilot: 可用但版本较旧
```

---

## 错误处理

### 错误类型

```yaml
ExternalAgentError:
  ProcessSpawnFailed:
    code: "E001"
    message: "无法启动外部 Agent 进程"
    action: "检查命令是否正确安装"

  ProcessNotFound:
    code: "E002"
    message: "外部 Agent 进程不存在"
    action: "检查 Agent ID 是否正确"

  StdinNotAvailable:
    code: "E003"
    message: "无法写入 STDIN"
    action: "进程可能已关闭"

  ProcessTimeout:
    code: "E004"
    message: "外部 Agent 执行超时"
    action: "增加超时时间或终止任务"

  ProcessCrashed:
    code: "E005"
    message: "外部 Agent 进程崩溃"
    details: exit_code, error_signal
    action: "查看日志获取详细信息"

  PermissionDenied:
    code: "E006"
    message: "外部 Agent 权限不足"
    action: "检查安全配置"

  AgentNotInstalled:
    code: "E007"
    message: "外部 Agent 未安装"
    details: agent_type, install_url, install_instructions
    action: "按照安装指南安装后再使用"

  AgentNotFound:
    code: "E008"
    message: "未知的外部 Agent 类型"
    details: agent_type, reason
    action: "检查 Agent 类型名称是否正确"
```

### 错误恢复策略

```rust
impl ExternalAgentManager {
    /// 错误恢复
    async fn handle_error(&self, agent_id: &str, error: &Error) -> Result<()> {
        match error {
            Error::ProcessTimeout => {
                // 超时：尝试优雅终止，然后强制终止
                self.terminate(agent_id, false).await?;
                tokio::time::sleep(Duration::from_secs(5)).await;
                self.terminate(agent_id, true).await?;
            }
            Error::ProcessCrashed(exit_code) => {
                // 崩溃：记录日志，通知 Monitor
                self.monitor.record_error("external_agent", error).await;
            }
            _ => {
                // 其他错误：直接终止
                self.terminate(agent_id, true).await?;
            }
        }
        Ok(())
    }
}
```

---

## 测试要点

### 单元测试

- [ ] 进程启动和终止
- [ ] STDIN/STDOUT 通信
- [ ] 超时处理
- [ ] 错误状态转换
- [ ] 输出缓冲

### 集成测试

- [ ] 与 Claude Code 实际集成
- [ ] 多进程并发管理
- [ ] 资源清理验证
- [ ] 错误恢复流程

### 测试用例

```rust
#[tokio::test]
async fn test_claude_code_invocation() {
    let adapter = ClaudeCodeAdapter::new();
    let result = adapter.invoke(
        "列出当前目录文件",
        Path::new("."),
        ClaudeCodeOptions::default(),
    ).await;

    assert!(result.is_ok());
    assert!(result.unwrap().output.contains("ls"));
}

#[tokio::test]
async fn test_process_timeout() {
    let manager = ExternalAgentManager::new();

    let config = ExternalAgentConfig {
        command: "sleep".to_string(),
        args: vec!["10".to_string()],
        timeout: 1, // 1 秒超时
        ..Default::default()
    };

    let agent_id = manager.spawn(&config, "").await.unwrap();

    // 等待超时
    tokio::time::sleep(Duration::from_secs(2)).await;

    let status = manager.get_status(&agent_id).await.unwrap();
    assert_eq!(status.state, ProcessState::Error);
}
```

---

## 性能考虑

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 进程启动时间 | < 500ms | Claude Code 启动延迟 |
| 输出延迟 | < 50ms | STDOUT 读取延迟 |
| 内存占用 | < 200MB | 单个外部 Agent |
| 并发数量 | 5+ | 同时运行的外部 Agent |

---

## 未来扩展

- [x] Agent 发现能力 - 检查外部 Agent 是否安装
- [ ] MCP 协议深度集成
- [ ] 远程 Agent 支持（SSH）
- [ ] Agent 池管理
- [ ] 输出解析器
- [ ] 中间结果共享
