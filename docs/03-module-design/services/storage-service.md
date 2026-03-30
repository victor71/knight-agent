# Storage Service (存储服务)

## 1. 概述

### 1.1 职责描述

Storage Service 提供统一的数据持久化接口，包括：

- 会话数据存储和检索
- 消息历史持久化
- 压缩点存储
- 任务状态管理
- 配置文件管理
- 数据备份和恢复

### 1.2 设计目标

1. **简单可靠**: 基于 SQLite 的零配置存储
2. **高性能**: 批量写入和索引优化
3. **可扩展**: 支持迁移到其他数据库
4. **数据安全**: 自动备份和恢复

### 1.3 依赖模块

| 依赖模块 | 依赖类型 | 说明 |
|---------|---------|------|
| 无 | - | 基础服务模块 |

---

## 2. 接口定义

### 2.1 对外接口

```yaml
# Storage Service 接口定义
StorageService:
  # ========== 会话存储 ==========
  save_session:
    description: 保存会话
    inputs:
      session:
        type: Session
        required: true
    outputs:
      success:
        type: boolean

  load_session:
    description: 加载会话
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      session:
        type: Session | null

  delete_session:
    description: 删除会话
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  list_sessions:
    description: 列出会话
    inputs:
      filter:
        type: SessionFilter
        required: false
      limit:
        type: integer
        required: false
      offset:
        type: integer
        required: false
    outputs:
      sessions:
        type: array<Session>

  # ========== 消息存储 ==========
  append_message:
    description: 追加消息到会话
    inputs:
      session_id:
        type: string
        required: true
      message:
        type: Message
        required: true
    outputs:
      success:
        type: boolean

  get_messages:
    description: 获取会话消息
    inputs:
      session_id:
        type: string
        required: true
      limit:
        type: integer
        required: false
      offset:
        type: integer
        required: false
      after:
        type: string
        description: 获取指定消息之后的消息
        required: false
    outputs:
      messages:
        type: array<Message>

  delete_messages:
    description: 删除消息
    inputs:
      session_id:
        type: string
        required: true
      before:
        type: string
        description: 删除指定消息之前的所有消息
        required: true
    outputs:
      deleted_count:
        type: integer

  # ========== 压缩点存储 ==========
  save_compression_point:
    description: 保存压缩点
    inputs:
      point:
        type: CompressionPoint
        required: true
    outputs:
      success:
        type: boolean

  get_compression_points:
    description: 获取会话压缩点
    inputs:
      session_id:
        type: string
        required: true
    outputs:
      points:
        type: array<CompressionPoint>

  delete_compression_point:
    description: 删除压缩点
    inputs:
      point_id:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 任务存储 ==========
  save_task:
    description: 保存任务
    inputs:
      task:
        type: Task
        required: true
    outputs:
      success:
        type: boolean

  load_task:
    description: 加载任务
    inputs:
      task_id:
        type: string
        required: true
    outputs:
      task:
        type: Task | null

  update_task:
    description: 更新任务状态
    inputs:
      task_id:
        type: string
        required: true
      updates:
        type: TaskUpdate
        required: true
    outputs:
      success:
        type: boolean

  list_tasks:
    description: 列出任务
    inputs:
      filter:
        type: TaskFilter
        required: false
      limit:
        type: integer
        required: false
    outputs:
      tasks:
        type: array<Task>

  # ========== 工作流存储 ==========
  save_workflow:
    description: 保存工作流
    inputs:
      workflow:
        type: WorkflowDefinition
        required: true
    outputs:
      workflow_id:
        type: string

  load_workflow:
    description: 加载工作流
    inputs:
      workflow_id:
        type: string
        required: true
    outputs:
      workflow:
        type: WorkflowDefinition | null

  list_workflows:
    description: 列出工作流
    outputs:
      workflows:
        type: array<WorkflowDefinition>

  # ========== 配置存储 ==========
  save_config:
    description: 保存配置
    inputs:
      key:
        type: string
        required: true
      value:
        type: any
        required: true
    outputs:
      success:
        type: boolean

  load_config:
    description: 加载配置
    inputs:
      key:
        type: string
        required: true
    outputs:
      value:
        type: any

  delete_config:
    description: 删除配置
    inputs:
      key:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 统计查询 ==========
  get_stats:
    description: 获取存储统计
    outputs:
      stats:
        type: StorageStats

  # ========== 备份和恢复 ==========
  backup:
    description: 备份数据库
    inputs:
      path:
        type: string
        description: 备份文件路径
        required: true
    outputs:
      success:
        type: boolean

  restore:
    description: 恢复数据库
    inputs:
      path:
        type: string
        description: 备份文件路径
        required: true
    outputs:
      success:
        type: boolean

  export_data:
    description: 导出数据
    inputs:
      format:
        type: string
        enum: [json, yaml]
        required: true
      output_path:
        type: string
        required: true
    outputs:
      success:
        type: boolean

  # ========== 维护 ==========
  vacuum:
    description: 清理数据库
    outputs:
      space_freed:
        type: integer

  reindex:
    description: 重建索引
    outputs:
      success:
        type: boolean
```

### 2.2 数据结构

```yaml
# 存储统计
StorageStats:
  # 会话统计
  sessions:
    type: SessionStats
  # 消息统计
  messages:
    type: MessageStats
  # 任务统计
  tasks:
    type: TaskStats
  # 数据库大小
  database_size_mb:
    type: float
  # 压缩率
  compression_ratio:
    type: float

# 会话统计
SessionStats:
  total:
    type: integer
  active:
    type: integer
  archived:
    type: integer
  total_messages:
    type: integer

# 消息统计
MessageStats:
  total:
    type: integer
  by_role:
    type: map<string, integer>
  avg_tokens:
    type: float

# 任务统计
TaskStats:
  total:
    type: integer
  by_status:
    type: map<string, integer>
  by_type:
    type: map<string, integer>

# 会话过滤器
SessionFilter:
  status:
    type: string | array<string>
  created_after:
    type: datetime
  created_before:
    type: datetime
  workspace:
    type: string

# 任务过滤器
TaskFilter:
  workflow_id:
    type: string | null
  status:
    type: string | array<string> | null
  type:
    type: string | array<string> | null
  created_after:
    type: datetime | null
  created_before:
    type: datetime | null
```

### 2.3 配置选项

```yaml
# config/storage.yaml
storage:
  # 数据库配置
  database:
    path: "./storage/knight-agent.db"
    wal_enabled: true
    cache_size: 10000
    page_size: 4096

  # 备份配置
  backup:
    enabled: true
    interval: 86400
    retention: 7
    path: "./storage/backups"

  # 维护配置
  maintenance:
    vacuum_interval: 604800
    reindex_interval: 1209600
```

---

## 3. 核心流程

### 3.1 消息存储流程

```
追加消息
        │
        ▼
┌──────────────────────────────┐
│ 1. 序列化消息                │
│    - JSON 格式化             │
│    - 压缩（可选）            │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 2. 追加到文件                │
│    - session/{id}.jsonl      │
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ 3. 更新索引                  │
│    - 更新会话元数据          │
│    - 更新消息计数            │
└──────────────────────────────┘
        │
        ▼
    完成
```

### 3.2 数据库结构

```sql
-- 会话表
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    workspace_root TEXT NOT NULL,
    project_type TEXT,
    created_at INTEGER NOT NULL,
    last_active_at INTEGER NOT NULL,
    metadata TEXT
);

-- 消息表
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    metadata TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- 压缩点表
CREATE TABLE compression_points (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    before_count INTEGER NOT NULL,
    after_count INTEGER NOT NULL,
    summary TEXT NOT NULL,
    token_saved INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- 任务表
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    workflow_id TEXT,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    status TEXT NOT NULL,
    agent_id TEXT,
    inputs TEXT,
    outputs TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    completed_at INTEGER
);

-- 工作流表
CREATE TABLE workflows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    definition TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- 配置表
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 索引
CREATE INDEX idx_messages_session ON messages(session_id, timestamp);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_workflow ON tasks(workflow_id);
```

---

## 4. 模块交互

### 4.1 依赖关系图

```
┌─────────────────────────────────────────┐
│          Storage Service                │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────┐  ┌──────────┐  ┌────────┐│
│  │Database  │  │File      │  │Backup   ││
│  │Layer     │  │Storage   │  │Manager  ││
│  └──────────┘  └──────────┘  └────────┘│
└─────────────────────────────────────────┘
        ▲
        │
        │
┌───────┴────────────────────────────────┐
│ 所有模块                              │
│ - Session Manager                     │
│ - Agent Runtime                       │
│ - Task Manager                        │
│ - Context Compressor                  │
└───────────────────────────────────────┘
```

### 4.2 数据流

```
各模块
    │
    ▼
┌─────────────────────────────┐
│ Storage Service             │
│ - 统一接口                  │
│ - 数据验证                  │
└─────────────────────────────┘
        │
        ├─────────────────────────────┐
        │                             │
        ▼                             ▼
┌─────────────────┐         ┌─────────────────┐
│ SQLite Database │         │ File Storage    │
│ - 结构化数据    │         │ - 大文件存储    │
└─────────────────┘         └─────────────────┘
```

---

## 5. 存储布局

### 5.1 目录结构

```
storage/
├── knight-agent.db            # SQLite 数据库
├── backups/                   # 备份目录
│   ├── backup-2026-03-30.db
│   └── backup-2026-03-29.db
├── sessions/                  # 会话数据（可选）
│   └── {session-id}/
│       ├── messages.jsonl     # 消息历史
│       └── state.json         # 会话状态
├── compression/               # 压缩点缓存
│   └── {session-id}/
│       └── points.jsonl
└── logs/                      # 日志文件
    ├── storage.log
    └── backup.log
```

### 5.2 文件格式

#### messages.jsonl

```jsonl
{"id":"msg1","session_id":"abc123","role":"user","content":"Hello","timestamp":1648690000000}
{"id":"msg2","session_id":"abc123","role":"assistant","content":"Hi!","timestamp":1648690001000}
```

#### state.json

```json
{
  "id": "abc123",
  "name": "frontend-dev",
  "status": "active",
  "workspace": {...},
  "variables": {...}
}
```

---

## 6. 配置与部署

### 6.1 配置文件格式

```yaml
# config/storage.yaml
storage:
  # 数据库配置
  database:
    path: "./storage/knight-agent.db"
    wal_enabled: true
    cache_size: 10000
    page_size: 4096
    connection_pool:
      max_connections: 10
      min_connections: 2

  # 备份配置
  backup:
    enabled: true
    interval: 86400
    retention: 7
    path: "./storage/backups"
    compress: true

  # 维护配置
  maintenance:
    vacuum_interval: 604800
    reindex_interval: 1209600
    auto_vacuum: true

  # 监控
  monitoring:
    log_queries: false
    log_slow_queries: true
    slow_query_threshold: 1000
```

### 6.2 环境变量

```bash
# 数据库路径
export KNIGHT_STORAGE_DB_PATH="./storage/knight-agent.db"

# 备份配置
export KNIGHT_STORAGE_BACKUP_ENABLED=true
export KNIGHT_STORAGE_BACKUP_INTERVAL=86400

# 维护配置
export KNIGHT_STORAGE_AUTO_VACUUM=true
```

---

## 7. 附录

### 7.1 性能指标

| 指标 | 目标值 | 说明 |
|------|--------|------|
| 写入延迟 | < 10ms | 单条消息 |
| 查询延迟 | < 50ms | 索引查询 |
| 批量写入 | > 1000 msg/s | 批量操作 |
| 数据库大小 | < 1GB | 1000 会话 |

### 7.2 错误处理

```yaml
error_codes:
  DATABASE_LOCKED:
    code: 503
    message: "数据库被锁定"
    action: "等待或重试"

  SESSION_NOT_FOUND:
    code: 404
    message: "会话不存在"
    action: "检查会话 ID"

  MESSAGE_APPEND_FAILED:
    code: 500
    message: "消息追加失败"
    action: "检查磁盘空间"

  BACKUP_FAILED:
    code: 500
    message: "备份失败"
    action: "检查备份路径"
```

### 7.3 迁移指南

#### SQLite → PostgreSQL

```yaml
# 修改配置
storage:
  database:
    type: postgresql
    host: localhost
    port: 5432
    database: knight_agent
    user: knight
    password: ${DB_PASSWORD}
```

#### 数据迁移脚本

```bash
# 导出 SQLite 数据
./knight storage export --format json --output backup.json

# 导入 PostgreSQL
./knight storage import --format json --input backup.json
```
