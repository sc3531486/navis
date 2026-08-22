# 04 - Storage 持久化存储 详细设计

> 模块编号：04 | 层级：核心服务层
> 依赖：01-Logger, 02-Event+IPC, 03-Config
> 被依赖：08-Session, 12-Gateway, 16-Agent, 20-Knowledge

---

## 一、模块概述

### 1.1 定位

Storage 是本地持久化存储引擎，基于 SQLite，提供 KV 存储、会话存储、记忆存储、缓存管理、加密存储能力。

### 1.2 职责边界

```
负责：
├── KV 存储（通用键值对持久化）
├── 会话存储（对话历史、消息记录）
├── 记忆存储（AI 长期记忆）
├── 缓存管理（LRU 缓存、TTL、自动清理）
├── 数据加密（敏感数据 AES-256 加密）
├── 数据备份/恢复
└── 存储空间管理

不负责：
├── 配置文件管理 → Config
├── 日志文件管理 → Logger
├── 文件系统操作 → File
└── 向量存储 → RAG
```

---

## 二、架构设计

### 2.1 子模块划分

```
storage/
├── mod.rs              # 模块入口
├── db.rs               # SQLite 连接管理
├── kv.rs               # KV 存储
├── session_store.rs    # 会话/消息存储
├── memory_store.rs     # AI 记忆存储
├── cache.rs            # 缓存管理（LRU/TTL）
├── cache_types.rs      # 缓存分类定义
├── cleanup.rs          # 缓存清理策略
├── encryption.rs       # 数据加密
├── backup.rs           # 备份/恢复
└── schema.rs           # 根 Schema 版本标记（不做迁移链）
```

### 2.2 数据库表结构

```sql
-- KV 存储
CREATE TABLE kv_store (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    encrypted BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME  -- TTL，NULL 表示永不过期
);

-- 会话
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    worktree_path TEXT,
    name TEXT,
    status TEXT DEFAULT 'active',       -- active / archived / deleted
    model TEXT,                          -- 会话级模型偏好
    system_prompt TEXT,                  -- 会话级系统提示词
    total_tokens INTEGER DEFAULT 0,     -- 会话累计 Token 数
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    archived BOOLEAN DEFAULT FALSE,
    archived_at DATETIME,               -- 归档时间
    metadata TEXT  -- JSON
);

CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_sessions_worktree ON sessions(worktree_path);

-- 消息
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,  -- user / assistant / system / tool
    content TEXT NOT NULL,
    token_count INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT,  -- JSON（工具调用信息等）
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Agent Timeline Part
-- 负责 Turn Timeline 展示事实：thinking / text / tool / permission / error / summary。
CREATE TABLE agent_timeline_parts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    kind TEXT NOT NULL,
    status TEXT,
    call_id TEXT,
    data TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    metadata TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

-- 会话文件变更事实
-- 负责 Review / Diff / Revert / Share。它不是 SessionCheckpoint：
-- checkpoint 只恢复 Agent 会话状态，session_changes 只记录文件实际变更。
CREATE TABLE session_changes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    agent_timeline_part_id TEXT,
    call_id TEXT,
    tool_name TEXT NOT NULL,
    worktree_path TEXT,
    relative_path TEXT,
    absolute_path TEXT NOT NULL,
    operation TEXT NOT NULL, -- create / update / delete
    before_content TEXT,
    after_content TEXT,
    diff TEXT,
    insertions INTEGER NOT NULL DEFAULT 0,
    deletions INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active', -- active / reverted
    created_at DATETIME NOT NULL,
    reverted_at DATETIME,
    metadata TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_timeline_part_id) REFERENCES agent_timeline_parts(id) ON DELETE SET NULL
);

-- 会话软压缩范围
-- 原始 messages 不删除；Prompt 组装时用 summary_message_id 替换 start/end 覆盖的历史。
CREATE TABLE compacted_ranges (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    start_message_id TEXT NOT NULL,
    end_message_id TEXT NOT NULL,
    tail_start_message_id TEXT,
    summary_message_id TEXT NOT NULL,
    summary_timeline_part_id TEXT,
    token_before INTEGER NOT NULL,
    token_after INTEGER NOT NULL,
    trigger TEXT NOT NULL,  -- manual / auto_overflow
    created_at DATETIME NOT NULL,
    metadata TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (summary_message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_compacted_ranges_session
    ON compacted_ranges(session_id, created_at DESC);

-- AI 记忆
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,  -- user_preference / project_fact / conversation_summary
    key TEXT NOT NULL,
    content TEXT NOT NULL,
    confidence REAL DEFAULT 1.0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME
);

-- 缓存元数据
CREATE TABLE cache_meta (
    cache_type TEXT NOT NULL,
    key TEXT NOT NULL,
    size_bytes INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    accessed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    access_count INTEGER DEFAULT 0,
    expires_at DATETIME,
    PRIMARY KEY (cache_type, key)
);
```

---

## 三、接口定义

### 3.1 Rust API

```rust
// 初始化
Storage::init(db_path: &Path, encryption_key: Option<&[u8]>) -> Result<Storage>

// KV 存储
Storage::kv_get(&self, key: &str) -> Result<Option<Value>>
Storage::kv_set(&self, key: &str, value: &Value, ttl: Option<Duration>) -> Result<()>
Storage::kv_delete(&self, key: &str) -> Result<()>
Storage::kv_list(&self, prefix: &str) -> Result<Vec<(String, Value)>>

// 会话存储
Storage::create_session(&self, project_id: &str, worktree_root: &str, options: CreateSessionOptions) -> Result<Session>
Storage::get_session(&self, id: &str) -> Result<Option<Session>>
Storage::list_sessions(&self, archived: bool, status: Option<&str>) -> Result<Vec<Session>>
Storage::delete_session(&self, id: &str) -> Result<()>
Storage::archive_session(&self, id: &str) -> Result<()>

// 消息存储
Storage::add_message(&self, session_id: &str, message: Message) -> Result<()>
Storage::get_messages(&self, session_id: &str, limit: usize, offset: usize) -> Result<Vec<Message>>
Storage::get_message_count(&self, session_id: &str) -> Result<usize>
Storage::delete_messages_before(&self, session_id: &str, before: DateTime) -> Result<usize>
Storage::save_compacted_range(&self, range: CompactedRange) -> Result<()>
Storage::get_compacted_ranges(&self, session_id: &str) -> Result<Vec<CompactedRange>>
Storage::record_session_change(&self, change: SessionChange) -> Result<SessionChange>
Storage::list_session_changes(&self, session_id: &str, turn_id: Option<&str>) -> Result<Vec<SessionChange>>
Storage::mark_session_change_reverted(&self, session_id: &str, change_id: &str) -> Result<SessionChange>

// 记忆存储
Storage::save_memory(&self, memory: Memory) -> Result<()>
Storage::get_memories(&self, category: &str) -> Result<Vec<Memory>>
Storage::search_memories(&self, query: &str) -> Result<Vec<Memory>>
Storage::delete_memory(&self, id: &str) -> Result<()>

// 缓存
Storage::cache_get(&self, cache_type: &str, key: &str) -> Result<Option<Vec<u8>>>
Storage::cache_set(&self, cache_type: &str, key: &str, data: &[u8], ttl: Option<Duration>) -> Result<()>
Storage::cache_delete(&self, cache_type: &str, key: &str) -> Result<()>
Storage::cache_cleanup(&self, cache_type: &str) -> Result<CacheCleanupResult>
Storage::cache_cleanup_all(&self) -> Result<CacheCleanupResult>  // 全量缓存清理，遍历所有 cache_type 依次清理

// 备份
Storage::backup(&self, path: &Path) -> Result<()>
Storage::restore(&self, path: &Path) -> Result<()>
Storage::restore_encrypted(&self, path: &Path, master_password: &str) -> Result<()>  // 跨设备恢复：用户提供主密码解密数据

// 统计
Storage::stats(&self) -> Result<StorageStats>
```

### 3.2 IPC 命令

```typescript
// 会话管理
storage.createSession(options: { projectId: string; worktreeRoot: string; name?: string; model?: string }): Promise<Session>
storage.getSession(id: string): Promise<Session | null>
storage.listSessions(archived?: boolean, status?: string): Promise<Session[]>
storage.deleteSession(id: string): Promise<void>
storage.archiveSession(id: string): Promise<void>

// 消息管理
storage.getMessages(sessionId: string, limit?: number, offset?: number): Promise<Message[]>
storage.getMessageCount(sessionId: string): Promise<number>

// 记忆管理
storage.saveMemory(memory: Memory): Promise<void>
storage.getMemories(category: string): Promise<Memory[]>
storage.searchMemories(query: string): Promise<Memory[]>
storage.deleteMemory(id: string): Promise<void>

// 缓存管理
storage.getCacheStats(): Promise<Record<string, CacheStats>>
storage.cleanupCache(cacheType?: string): Promise<CacheCleanupResult>
storage.cleanupAllCache(): Promise<CacheCleanupResult>  // 全量缓存清理

// 备份
storage.backup(path?: string): Promise<string>  // 返回备份路径
storage.restore(path: string): Promise<void>

// 统计
storage.stats(): Promise<StorageStats>
```

---

## 四、缓存分类与清理策略

| 缓存类型 | 存储位置 | TTL | 最大大小 | 清理策略 |
|----------|----------|-----|----------|----------|
| worktree_cache | DB | 24h | 500MB | LRU + TTL |
| diff_cache | 内存 | 1h | 50MB | 会话切换清理 |
| index_cache | DB + 文件 | 7d | 1GB | 源文件变更清理 |
| model_cache | 文件 | 30d | 2GB | 手动清理 |
| session_cache | 内存 | 会话生命周期 | 100MB | 会话关闭清理 |
| tool_cache | DB | 10min | 50MB | LRU |
| rag_cache | DB | 10min | 100MB | LRU + 知识源更新清理 |

---

## 五、错误处理

| 场景 | 处理策略 |
|------|----------|
| 数据库文件损坏 | 尝试修复，失败则重建（丢失历史） |
| 磁盘空间不足 | 触发缓存清理，发出告警事件 |
| 加密密钥丢失 | 数据不可恢复，提示用户 |
| 并发写入冲突 | SQLite WAL 模式，自动重试 |
| 根 Schema 不匹配 | 返回具体错误，提示重建开发数据库 |

### 5.1 跨设备备份/恢复说明

备份文件中所有敏感数据（API Key、Git 凭证等）均保持 AES-256 加密状态。当用户在另一台设备上执行恢复操作时，加密数据无法使用源设备的设备指纹进行解密，因此需要用户提供主密码（Master Password）来派生解密密钥。恢复流程：

1. `storage.restore(path)` — 恢复非加密数据（会话、消息、记忆等）
2. `storage.restore_encrypted(path, master_password)` — 使用主密码解密并恢复加密数据
3. 若主密码错误，返回解密失败错误，不影响已恢复的非加密数据

---

## 六、事件定义

```typescript
type StorageEvents = {
  'storage.session.created':   { sessionId: string }
  'storage.session.deleted':   { sessionId: string }
  'storage.session.archived':  { sessionId: string }
  'storage.session.updated':   { sessionId: string; changes: string[] }
  'storage.message.added':     { sessionId: string, messageId: string }
  'storage.message.deleted':   { sessionId: string, messageId: string }
  'storage.memory.saved':      { memoryId: string, category: string }
  'storage.cache.cleanup':     { cacheType: string, freedSize: number }
  'storage.backup.created':    { path: string }
  'storage.backup.restored':   { path: string }
  'storage.disk.warning':      { freeSpace: number, threshold: number }
  'storage.cache.invalidated': { cacheType: string, key?: string; reason: string }  // RAG/File 模块通知缓存失效
  'storage.error':             { operation: string, error: string }
}
```

---

## 七、性能指标

| 指标 | 要求 | 说明 |
|------|------|------|
| KV 读取 | < 1ms | SQLite 查询 |
| KV 写入 | < 5ms | SQLite 写入 |
| 消息查询（100条） | < 10ms | 带分页 |
| 缓存命中 | < 0.5ms | 内存缓存 |
| 数据库启动 | < 50ms | SQLite 初始化 |
| 备份（10MB 数据） | < 1s | 文件复制 |

---

## 八、测试策略

```
单元测试：
├── KV 读写/过期/TTL
├── 会话 CRUD
├── 消息分页查询
├── 记忆搜索
├── 缓存 LRU 淘汰
└── 加密/解密

集成测试：
├── 根 Schema 版本校验
├── 备份/恢复
├── 并发读写
├── 磁盘满降级
└── 缓存清理策略
```
