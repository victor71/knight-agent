# 会话系统设计文档

## 1. 需求概述

### 1.1 核心需求

| 需求 | 描述 | 优先级 |
|------|------|--------|
| **多会话并行** | 同时运行多个独立会话，互不干扰 | P0 |
| **Workspace 隔离** | 不同项目的工作区完全隔离 | P0 |
| **会话历史** | 保存会话记录，支持恢复和查询 | P1 |
| **上下文压缩** | 智能压缩长上下文，保留关键信息 | P1 |
| **会话共享** | 多 Agent 共享同一会话上下文 | P1 |
| **会话模板** | 预定义会话配置，快速启动 | P2 |

### 1.2 使用场景

**场景 1: 多项目并行开发**
```
开发者同时在两个项目中工作:
- 会话 A: ~/project-frontend (前端项目)
- 会话 B: ~/project-backend (后端项目)

两个会话完全隔离，上下文不混淆。
```

**场景 2: 长对话项目**
```
与 Agent 进行长期协作开发:
- Day 1: 讨论架构设计
- Day 2: 实现核心功能
- Day 3: 编写测试
- Day 7: 回顾之前的讨论

需要保留完整历史，但上下文要智能压缩。
```

**场景 3: 多 Agent 协作**
```
一个会话中多个 Agent 协作:
- 用户 → Orchestrator
- Orchestrator → Agent A
- Orchestrator → Agent B
- 所有 Agent 共享会话上下文
```

---

## 2. 会话模型设计

### 2.1 核心概念

```rust
// 会话 ID: 全局唯一标识
pub type SessionId = String;

// 会话状态
pub enum SessionStatus {
    Active,      // 活跃中
    Paused,      // 暂停
    Archived,    // 已归档
}

// 会话配置
pub struct SessionConfig {
    pub name: Option<String>,
    pub workspace: PathBuf,
    pub context_limit: usize,        // 上下文消息限制
    pub compression_threshold: f64,  // 压缩触发阈值 (0-1)
    pub persistence: PersistenceMode,
}

pub enum PersistenceMode {
    Memory,      // 仅内存
    Disk,        // 持久化到磁盘
    Cloud,       // 云端同步 (未来)
}

// 会话元数据
pub struct SessionMetadata {
    pub id: SessionId,
    pub name: String,
    pub status: SessionStatus,
    pub workspace: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub total_tokens: u64,
}

// 完整会话
pub struct Session {
    pub metadata: SessionMetadata,
    pub config: SessionConfig,
    pub context: SessionContext,
    pub history: MessageHistory,
    pub compression: CompressionContext,
}
```

### 2.2 会话上下文

```rust
// 会话上下文 (当前活跃的消息)
pub struct SessionContext {
    pub session_id: SessionId,
    pub workspace: WorkspaceContext,
    pub variables: HashMap<String, serde_json::Value>,
    pub active_agents: HashSet<AgentId>,
    pub shared_files: HashSet<PathBuf>,
    pub temp_files: Vec<TempFile>,
}

// Workspace 上下文
pub struct WorkspaceContext {
    pub root: PathBuf,
    pub project_type: Option<String>,  // "rust", "node", "python"
    pub git_info: Option<GitInfo>,
    pub file_index: FileIndex,
}

pub struct GitInfo {
    pub branch: String,
    pub commit: String,
    pub remote: Option<String>,
    pub status: GitStatus,
}

// 文件索引 (Workspace 文件的快速查找)
pub struct FileIndex {
    pub files: HashMap<PathBuf, FileInfo>,
}

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub language: Option<String>,
    pub hash: String,  // 内容哈希，用于变更检测
}
```

### 2.3 消息历史

```rust
// 消息历史 (完整存储)
pub struct MessageHistory {
    pub session_id: SessionId,
    pub messages: Vec<HistoricalMessage>,
    pub compression_points: Vec<CompressionPoint>,
}

// 历史消息
pub struct HistoricalMessage {
    pub id: MessageId,
    pub timestamp: DateTime<Utc>,
    pub role: MessageRole,
    pub content: String,
    pub metadata: MessageMetadata,
    pub compressed: bool,  // 是否已被压缩
}

pub struct MessageMetadata {
    pub agent: Option<AgentId>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub tokens: Option<TokenUsage>,
    pub files_accessed: Vec<PathBuf>,
}

// 压缩点 (压缩后的摘要)
pub struct CompressionPoint {
    pub before_message_id: MessageId,
    pub after_message_id: MessageId,
    pub summary: String,
    pub key_points: Vec<String>,
    pub decisions: Vec<Decision>,
    pub compressed_at: DateTime<Utc>,
}

pub struct Decision {
    pub topic: String,
    pub decision: String,
    pub rationale: String,
}
```

---

## 3. 会话管理器

### 3.1 核心接口

```rust
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    workspaces: HashMap<PathBuf, WorkspaceContext>,
    config: SessionManagerConfig,
    storage: Box<dyn SessionStorage>,
}

impl SessionManager {
    /// 创建新会话
    pub async fn create_session(
        &mut self,
        config: SessionConfig,
    ) -> Result<Session>;

    /// 获取会话
    pub fn get_session(&self, id: &SessionId) -> Option<&Session>;

    /// 切换当前会话
    pub async fn switch_session(&mut self, id: &SessionId) -> Result<()>;

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<&Session>;

    /// 归档会话
    pub async fn archive_session(&mut self, id: &SessionId) -> Result<()>;

    /// 删除会话
    pub async fn delete_session(&mut self, id: &SessionId) -> Result<()>;

    /// 搜索会话历史
    pub async fn search_history(
        &self,
        query: &str,
        workspace: Option<&PathBuf>,
    ) -> Result<Vec<HistoricalMessage>>;

    /// 恢复会话
    pub async fn restore_session(
        &mut self,
        id: &SessionId,
    ) -> Result<Session>;
}
```

### 3.2 Workspace 隔离

```rust
impl SessionManager {
    /// 确保 Workspace 隔离
    pub fn ensure_workspace_isolation(
        &mut self,
        workspace: &PathBuf,
    ) -> Result<WorkspaceContext> {
        // 检查是否已存在
        if let Some(ctx) = self.workspaces.get(workspace) {
            return Ok(ctx.clone());
        }

        // 创建新的隔离上下文
        let context = WorkspaceContext {
            root: workspace.clone(),
            project_type: detect_project_type(workspace)?,
            git_info: get_git_info(workspace)?,
            file_index: build_file_index(workspace)?,
        };

        self.workspaces.insert(workspace.clone(), context);
        Ok(context)
    }

    /// 检查路径是否在 Workspace 范围内
    pub fn validate_path(
        &self,
        session_id: &SessionId,
        path: &Path,
    ) -> Result<bool> {
        let session = self.get_session(session_id)
            .ok_or_else(|| anyhow!("Session not found"))?;

        let workspace_root = &session.context.workspace.root;
        let canonical_path = path.canonicalize()?;
        let canonical_root = workspace_root.canonicalize()?;

        Ok(canonical_path.starts_with(&canonical_root))
    }
}

// 项目类型检测
fn detect_project_type(path: &Path) -> Result<Option<String>> {
    // 检查标志性文件
    let indicators = vec![
        ("Cargo.toml", "rust"),
        ("package.json", "node"),
        ("requirements.txt", "python"),
        ("pom.xml", "java"),
        ("go.mod", "go"),
    ];

    for (file, lang) in indicators {
        if path.join(file).exists() {
            return Ok(Some(lang.to_string()));
        }
    }

    Ok(None)
}

// 文件索引构建
fn build_file_index(path: &Path) -> Result<FileIndex> {
    let mut files = HashMap::new();

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let meta = std::fs::metadata(path)?;
            let content = std::fs::read(path)?;

            files.insert(path.to_path_buf(), FileInfo {
                path: path.to_path_buf(),
                size: meta.len(),
                modified: meta.modified()?.into(),
                language: detect_language(path),
                hash: format!("{:x}", md5::compute(&content)),
            });
        }
    }

    Ok(FileIndex { files })
}
```

---

## 4. 上下文压缩

### 4.1 压缩策略

```rust
pub struct CompressionEngine {
    llm: Arc<dyn LLMClient>,
    config: CompressionConfig,
}

pub struct CompressionConfig {
    /// 触发压缩的消息数量
    pub trigger_message_count: usize,

    /// 触发压缩的 Token 数量
    pub trigger_token_count: usize,

    /// 压缩比例 (目标: 保留多少)
    pub compression_ratio: f32,  // 0.3 = 压缩到 30%

    /// 压缩方法
    pub method: CompressionMethod,
}

pub enum CompressionMethod {
    /// 摘要压缩
    Summary {
        /// 保留最近 N 条消息
        keep_recent: usize,
    },

    /// 语义压缩 (提取关键信息)
    Semantic {
        /// 保留的关键信息类型
        keep_types: Vec<InfoType>,
    },

    /// 混合压缩
    Hybrid {
        summary_ratio: f32,
        semantic_ratio: f32,
    },
}

pub enum InfoType {
    Decision,      // 决策
    Code,          // 代码
    Requirement,   // 需求
    Error,         // 错误
    FileChange,    // 文件变更
}

impl CompressionEngine {
    /// 检查是否需要压缩
    pub fn should_compress(&self, session: &Session) -> bool {
        let message_count = session.history.messages.len();
        let total_tokens = session.metadata.total_tokens;

        message_count >= self.config.trigger_message_count
            || total_tokens >= self.config.trigger_token_count as u64
    }

    /// 执行压缩
    pub async fn compress(
        &self,
        session: &Session,
    ) -> Result<CompressionPoint> {
        match &self.config.method {
            CompressionMethod::Summary { keep_recent } => {
                self.compress_summary(session, *keep_recent).await
            }
            CompressionMethod::Semantic { keep_types } => {
                self.compress_semantic(session, keep_types).await
            }
            CompressionMethod::Hybrid { .. } => {
                self.compress_hybrid(session).await
            }
        }
    }

    /// 摘要压缩
    async fn compress_summary(
        &self,
        session: &Session,
        keep_recent: usize,
    ) -> Result<CompressionPoint> {
        let messages = &session.history.messages;

        // 分离要压缩和保留的消息
        let (to_compress, to_keep) = if messages.len() > keep_recent {
            messages.split_at(messages.len() - keep_recent)
        } else {
            return Ok(CompressionPoint::empty());
        };

        // 构建摘要提示
        let prompt = self.build_summary_prompt(to_compress)?;

        // 调用 LLM 生成摘要
        let response = self.llm.chat(LLMRequest {
            model: "claude-haiku".to_string(),
            messages: vec![
                LLMMessage {
                    role: LLMRole::System,
                    content: "你是一个专业的对话摘要助手。".to_string(),
                    tool_calls: None,
                    tool_id: None,
                },
                LLMMessage {
                    role: LLMRole::User,
                    content: prompt,
                    tool_calls: None,
                    tool_id: None,
                },
            ],
            ..Default::default()
        }).await?;

        // 解析摘要，提取关键点和决策
        let summary = response.content;
        let key_points = self.extract_key_points(&summary)?;
        let decisions = self.extract_decisions(&summary)?;

        Ok(CompressionPoint {
            before_message_id: to_compress.first().unwrap().id.clone(),
            after_message_id: to_compress.last().unwrap().id.clone(),
            summary,
            key_points,
            decisions,
            compressed_at: Utc::now(),
        })
    }

    /// 语义压缩 (提取结构化信息)
    async fn compress_semantic(
        &self,
        session: &Session,
        keep_types: &[InfoType],
    ) -> Result<CompressionPoint> {
        let mut decisions = Vec::new();
        let mut file_changes = Vec::new();
        let mut errors = Vec::new();

        for msg in &session.history.messages {
            // 提取决策
            if keep_types.contains(&InfoType::Decision) {
                if let Some(msg_decisions) = self.extract_decisions_from_msg(&msg)? {
                    decisions.extend(msg_decisions);
                }
            }

            // 提取文件变更
            if keep_types.contains(&InfoType::FileChange) {
                if let Some(changes) = self.extract_file_changes_from_msg(&msg)? {
                    file_changes.extend(changes);
                }
            }

            // 提取错误
            if keep_types.contains(&InfoType::Error) {
                if let Some(msg_errors) = self.extract_errors_from_msg(&msg)? {
                    errors.extend(msg_errors);
                }
            }
        }

        // 生成结构化摘要
        let summary = self.build_structured_summary(
            &decisions, &file_changes, &errors
        )?;

        Ok(CompressionPoint {
            before_message_id: session.history.messages.first().unwrap().id.clone(),
            after_message_id: session.history.messages.last().unwrap().id.clone(),
            summary,
            key_points: decisions.iter().map(|d| d.decision.clone()).collect(),
            decisions,
            compressed_at: Utc::now(),
        })
    }

    fn build_summary_prompt(&self, messages: &[HistoricalMessage]) -> Result<String> {
        let mut prompt = String::from("请摘要以下对话，重点关注:\n");
        prompt.push_str("1. 重要的决策和原因\n");
        prompt.push_str("2. 代码变更和原因\n");
        prompt.push_str("3. 发现的问题和解决方案\n\n");

        for msg in messages {
            prompt.push_str(&format!("{}\n", msg.content));
        }

        Ok(prompt)
    }

    fn extract_key_points(&self, summary: &str) -> Result<Vec<String>> {
        // 解析摘要，提取要点
        // 可以用 LLM 或规则提取
        Ok(vec![])
    }

    fn extract_decisions(&self, summary: &str) -> Result<Vec<Decision>> {
        // 解析决策
        Ok(vec![])
    }
}
```

### 4.2 压缩配置

```yaml
# config/session.yaml
compression:
  # 触发条件
  trigger:
    message_count: 50        # 超过 50 条消息
    token_count: 100000      # 超过 100k tokens

  # 压缩方法
  method: summary
  summary:
    keep_recent: 20          # 保留最近 20 条消息

  # 或者使用语义压缩
  # method: semantic
  # semantic:
  #   keep_types:
  #     - decision
  #     - code
  #     - requirement

  # 或者混合
  # method: hybrid
  # hybrid:
  #   summary_ratio: 0.7      # 70% 用摘要
  #   semantic_ratio: 0.3     # 30% 用语义
```

### 4.3 压缩点使用

```rust
impl Session {
    /// 获取用于 LLM 的消息 (自动处理压缩)
    pub fn get_llm_messages(&self) -> Vec<LLMMessage> {
        let mut messages = Vec::new();

        // 添加系统消息
        messages.push(LLMMessage {
            role: LLMRole::System,
            content: self.system_prompt.clone(),
            tool_calls: None,
            tool_id: None,
        });

        // 添加压缩摘要 (如果有)
        for point in &self.history.compression_points {
            messages.push(LLMMessage {
                role: LLMRole::System,
                content: format!(
                    "[之前对话摘要]\n{}\n[关键决策]\n{}",
                    point.summary,
                    point.decisions.iter()
                        .map(|d| format!("- {}: {}", d.topic, d.decision))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                tool_calls: None,
                tool_id: None,
            });
        }

        // 添加未压缩的最近消息
        for msg in self.history.messages.iter()
            .filter(|m| !m.compressed)
        {
            messages.push(LLMMessage {
                role: match msg.role {
                    MessageRole::User => LLMRole::User,
                    MessageRole::Assistant => LLMRole::Assistant,
                    MessageRole::System => LLMRole::System,
                    MessageRole::Tool => LLMRole::Tool,
                },
                content: msg.content.clone(),
                tool_calls: None,
                tool_id: None,
            });
        }

        messages
    }
}
```

---

## 5. 会话存储

### 5.1 存储接口

```rust
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// 保存会话
    async fn save_session(&self, session: &Session) -> Result<()>;

    /// 加载会话
    async fn load_session(&self, id: &SessionId) -> Result<Option<Session>>;

    /// 列出会话
    async fn list_sessions(&self, filter: SessionFilter)
        -> Result<Vec<SessionMetadata>>;

    /// 删除会话
    async fn delete_session(&self, id: &SessionId) -> Result<()>;

    /// 搜索消息
    async fn search_messages(
        &self,
        query: &str,
        workspace: Option<&Path>,
        limit: usize,
    ) -> Result<Vec<HistoricalMessage>>;
}

pub struct SessionFilter {
    pub workspace: Option<PathBuf>,
    pub status: Option<SessionStatus>,
    pub date_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}
```

### 5.2 文件系统存储

```rust
pub struct FileSystemSessionStorage {
    base_dir: PathBuf,
}

impl FileSystemSessionStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.base_dir.join(id).join("session.json")
    }

    fn messages_path(&self, id: &SessionId) -> PathBuf {
        self.base_dir.join(id).join("messages.jsonl")
    }
}

#[async_trait]
impl SessionStorage for FileSystemSessionStorage {
    async fn save_session(&self, session: &Session) -> Result<()> {
        let session_dir = self.base_dir.join(&session.metadata.id);
        std::fs::create_dir_all(&session_dir)?;

        // 保存会话元数据
        let session_path = self.session_path(&session.metadata.id);
        let json = serde_json::to_string_pretty(&session)?;
        std::fs::write(session_path, json)?;

        // 保存消息历史 (JSONL 格式)
        let messages_path = self.messages_path(&session.metadata.id);
        let mut file = std::fs::File::create(messages_path)?;

        for msg in &session.history.messages {
            let line = serde_json::to_string(&msg)?;
            writeln!(file, "{}", line)?;
        }

        Ok(())
    }

    async fn load_session(&self, id: &SessionId)
        -> Result<Option<Session>>
    {
        let session_path = self.session_path(id);

        if !session_path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(session_path)?;
        let mut session: Session = serde_json::from_str(&json)?;

        // 加载消息历史
        let messages_path = self.messages_path(id);
        let file = std::fs::File::open(messages_path)?;
        let reader = std::io::BufReader::new(file);

        for line in std::io::BufRead::lines(reader) {
            let msg: HistoricalMessage = serde_json::from_str(&line?)?;
            session.history.messages.push(msg);
        }

        Ok(Some(session))
    }

    async fn list_sessions(&self, filter: SessionFilter)
        -> Result<Vec<SessionMetadata>>
    {
        let mut sessions = Vec::new();

        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let session_path = entry.path().join("session.json");

            if let Ok(json) = std::fs::read_to_string(session_path) {
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    if self.matches_filter(&session.metadata, &filter) {
                        sessions.push(session.metadata);
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        let session_dir = self.base_dir.join(id);
        std::fs::remove_dir_all(session_dir)?;
        Ok(())
    }

    async fn search_messages(
        &self,
        query: &str,
        workspace: Option<&Path>,
        limit: usize,
    ) -> Result<Vec<HistoricalMessage>> {
        // 遍历所有会话的消息文件
        // 使用简单的文本匹配 (后期可以用向量搜索)
        let mut results = Vec::new();

        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let session_id = entry.file_name();
            let messages_path = self.base_dir
                .join(session_id)
                .join("messages.jsonl");

            if let Ok(file) = std::fs::File::open(messages_path) {
                let reader = std::io::BufReader::new(file);

                for line in std::io::BufRead::lines(reader) {
                    if let Ok(msg) = serde_json::from_str::<HistoricalMessage>(&line?) {
                        if msg.content.contains(query) {
                            if let Some(ref ws) = workspace {
                                // TODO: 检查 workspace 匹配
                            }
                            results.push(msg);

                            if results.len() >= limit {
                                return Ok(results);
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl FileSystemSessionStorage {
    fn matches_filter(&self, metadata: &SessionMetadata, filter: &SessionFilter)
        -> bool
    {
        if let Some(ref ws) = filter.workspace {
            if metadata.workspace != *ws {
                return false;
            }
        }

        if let Some(ref status) = filter.status {
            if metadata.status != *status {
                return false;
            }
        }

        if let Some((start, end)) = filter.date_range {
            if metadata.created_at < *start || metadata.created_at > *end {
                return false;
            }
        }

        true
    }
}
```

---

## 6. CLI 交互

### 6.1 会话命令

```bash
# 创建新会话
knight session create --name "前端开发" --workspace ~/project-frontend

# 列出所有会话
knight session list
# 输出:
#   SESSION ID    NAME           WORKSPACE              STATUS    UPDATED
#   abc123        前端开发       ~/project-frontend     Active    2m ago
#   def456        后端开发       ~/project-backend      Paused    1h ago
#   ghi789        代码审查       ~/project-shared       Archived  1d ago

# 切换会话
knight session use abc123

# 显示当前会话信息
knight session info
# 输出:
#   Session: abc123
#   Name: 前端开发
#   Workspace: ~/project-frontend
#   Messages: 45
#   Tokens: 12,345
#   Compressions: 2

# 搜索历史
knight session search "React 组件设计" --workspace ~/project-frontend

# 归档会话
knight session archive abc123

# 删除会话
knight session delete def456

# 导出会话
knight session export abc123 --format markdown --output session.md

# 导入会话
knight session import session.md
```

### 6.2 交互模式中的会话切换

```bash
# 启动时指定会话
knight chat --session abc123

# 运行中切换会话
» # 当前在会话 abc123
» /sessions               # 列出会话
» /use def456             # 切换到会话 def456
» /info                   # 显示当前会话信息
» /history                # 显示会话历史
```

---

## 7. 目录结构

```
~/.knight-agent/
├── config/
│   ├── settings.yaml
│   └── session.yaml         # 会话配置
│
├── sessions/                 # 会话存储
│   ├── abc123/
│   │   ├── session.json     # 会话元数据
│   │   ├── messages.jsonl   # 消息历史
│   │   ├── context.json     # 当前上下文快照
│   │   └── compression/     # 压缩点
│   │       ├── point-001.json
│   │       └── point-002.json
│   │
│   └── def456/
│       └── ...
│
├── workspaces/               # Workspace 缓存
│   ├── project-frontend/
│   │   ├── file-index.json
│   │   └── git-info.json
│   └── project-backend/
│       └── ...
│
└── temp/                     # 临时文件
    └── .gitkeep
```

---

## 8. 优先级与实施计划

### 8.1 优先级

| 阶段 | 功能 | 优先级 |
|------|------|--------|
| **P0 - MVP** |
| ✓ 基础会话管理 | 创建、切换、删除会话 | P0 |
| ✓ Workspace 隔离 | 不同项目独立上下文 | P0 |
| ✓ 简单历史 | 当前会话的消息记录 | P0 |
| **P1 - V1.0** |
| ✓ 会话持久化 | 保存到磁盘，重启恢复 | P1 |
| ✓ 上下文压缩 | 智能压缩长对话 | P1 |
| ✓ 历史搜索 | 跨会话搜索 | P1 |
| ✓ 会话模板 | 快速启动预设会话 | P1 |
| **P2 - V1.x** |
| ○ 云端同步 | 多设备同步 | P2 |
| ○ 会话分享 | 分享会话给团队 | P2 |
| ○ 可视化时间线 | 会话时间线视图 | P2 |

### 8.2 实施计划

**Week 1**: 基础会话管理
- 会话模型定义
- 会话管理器核心接口
- Workspace 隔离机制

**Week 2**: 会话持久化
- 文件系统存储
- 会话保存和加载
- 配置管理

**Week 3**: 上下文压缩
- 压缩引擎框架
- 摘要压缩实现
- 压缩点管理

**Week 4**: CLI 集成
- 会话命令
- 交互模式集成
- 历史搜索

---

## 9. 最佳实践

### 9.1 会话命名

```bash
# 好的命名 (描述性强)
knight session create --name "feat-用户认证-后端开发"
knight session create --name "fix-登录bug-前端修复"
knight session create --name "refactor-支付模块重构"

# 不好的命名 (模糊)
knight session create --name "工作"
knight session create --name "test"
```

### 9.2 会话生命周期

```
创建 → 活跃使用 → 暂停 → 归档 → (必要时) 删除
  ↓       ↓         ↓       ↓
配置   定期压缩  保持状态  长期存储
```

### 9.3 压缩策略建议

| 场景 | 推荐策略 | 配置 |
|------|----------|------|
| 日常开发 | Summary | keep_recent: 20 |
| 架构讨论 | Semantic | keep_types: [decision, requirement] |
| 代码审查 | Hybrid | summary_ratio: 0.7 |
| 调试会话 | Summary | keep_recent: 10 (更少) |

---

## 10. 与其他系统集成

### 10.1 与 Agent 系统

```rust
// Agent 使用会话上下文
impl Agent {
    pub async fn process_with_session(
        &mut self,
        session: &Session,
        message: String,
    ) -> Result<Response> {
        // 从会话获取消息
        let messages = session.get_llm_messages();

        // 添加新消息
        // ...

        // 处理响应
        // ...
    }
}
```

### 10.2 与 Skill 系统

```yaml
# Skill 可以访问会话变量
# skills/code-review/SKILL.md
parameters:
  - name: session_var
    type: string
    description: "从会话变量读取配置"

steps:
  - name: load_config
    agent: self
    prompt: |
      使用会话变量中的配置:
      {{ session.code_review_config }}
```

### 10.3 与 Git 集成

```bash
# 自动创建基于分支的会话
knight session create --from-git-branch

# 会话名包含分支信息
# format: "{branch-name}-{timestamp}"
```
