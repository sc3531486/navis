//! 数据库连接管理模块
//!
//! 负责 SQLite 数据库的连接初始化、WAL 模式配置和根表结构创建。
//! 使用 rusqlite 的 bundled SQLite 特性，无需系统安装 SQLite。
//!
//! # 职责边界（C0-6 收敛）
//! 本模块承诺**落盘原语**：连接管理、WAL/外键配置、`kv_store` 等通用表、
//! 内核审计落盘（`audit_log`）与根 Schema 版本标记（`schema.rs`）。
//! 其余业务表（sessions / messages / api_keys / jobs 等）为历史遗留的根 Schema 宿主，
//! 已在 `create_tables` 中逐表标注"业务 schema（属业务域，渐进迁出）"，
//! **不要在此层新增业务表**；新表应声明在对应业务域的扩展建表路径。

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

use super::schema;

/// SQLite 数据库路径（默认目录名）
pub const DB_DIR_NAME: &str = "data";
/// SQLite 数据库文件名
pub const DB_FILE_NAME: &str = "navis.db";

/// 数据库管理器
///
/// 封装 SQLite 连接的创建、初始化和根 Schema 校验。
/// 使用 WAL 模式提高并发读写性能。
pub struct Database;

impl Database {
    /// 创建并初始化 SQLite 数据库连接
    ///
    /// # Arguments
    /// * `db_path` - 数据库文件路径
    ///
    /// # Returns
    /// 初始化完成的 rusqlite Connection，已启用 WAL 模式和外键约束
    pub fn open(db_path: &Path) -> Result<Connection> {
        tracing::info!(path = %db_path.display(), "Opening SQLite database");
        let database_existed = db_path.exists();

        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("无法创建数据库目录: {}", parent.display()))?;
        }

        let mut conn = Self::open_configured(db_path)?;

        if database_existed {
            if let Some(version) = Self::existing_root_schema_version(&conn)? {
                if version != schema::CURRENT_VERSION {
                    tracing::warn!(
                        path = %db_path.display(),
                        current_version = version,
                        required_version = schema::CURRENT_VERSION,
                        "Development database root schema mismatch; deleting and rebuilding"
                    );
                    drop(conn);
                    Self::delete_development_database(db_path)?;
                    conn = Self::open_configured(db_path)?;
                }
            }
        }

        // 创建表结构
        Self::create_tables(&conn)?;

        tracing::info!(path = %db_path.display(), "SQLite database initialized successfully");
        Ok(conn)
    }

    fn open_configured(db_path: &Path) -> Result<Connection> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("无法打开数据库: {}", db_path.display()))?;
        Self::configure(&conn)?;
        Ok(conn)
    }

    /// 创建内存数据库（用于测试）
    ///
    /// # Returns
    /// 内存数据库连接，已配置并初始化表结构
    pub fn open_memory() -> Result<Connection> {
        tracing::debug!("Opening in-memory SQLite database");
        let conn = Connection::open_in_memory().context("无法创建内存数据库")?;

        Self::configure(&conn)?;
        Self::create_tables(&conn)?;

        tracing::debug!("In-memory SQLite database initialized");
        Ok(conn)
    }

    /// 配置 SQLite 连接参数
    ///
    /// 启用 WAL 模式（提高并发性能）和外键约束。
    fn configure(conn: &Connection) -> Result<()> {
        // 启用 WAL 模式 - 提高并发读写性能
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("无法设置 WAL 模式")?;

        // 启用外键约束
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .context("无法启用外键约束")?;

        // 设置忙等待超时（5 秒）
        conn.execute_batch("PRAGMA busy_timeout=5000;")
            .context("无法设置忙等待超时")?;

        tracing::debug!("SQLite connection configured (WAL mode, foreign keys enabled)");
        Ok(())
    }

    /// Return the stamped root schema version for a non-empty existing database.
    ///
    /// Navis is not released yet, so we do not maintain a migration chain. An
    /// existing database with a different root schema is deleted and rebuilt
    /// by `open()` instead of silently running with half-old tables.
    fn existing_root_schema_version(conn: &Connection) -> Result<Option<i32>> {
        let user_table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM sqlite_master
                 WHERE type='table'
                   AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .context("无法检查数据库表状态")?;

        if user_table_count == 0 {
            return Ok(None);
        }

        Ok(Some(schema::get_current_version(conn)?))
    }

    fn delete_development_database(db_path: &Path) -> Result<()> {
        Self::remove_file_if_exists(db_path)?;

        for suffix in ["-wal", "-shm"] {
            let sidecar = db_path.with_file_name(format!(
                "{}{}",
                db_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .context("数据库文件名不是有效 UTF-8，无法删除旧开发数据库 sidecar")?,
                suffix
            ));
            Self::remove_file_if_exists(&sidecar)?;
        }

        tracing::warn!(
            old_path = %db_path.display(),
            "Old development database was deleted before root schema rebuild"
        );
        Ok(())
    }

    fn remove_file_if_exists(path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("无法删除旧开发数据库文件: {}", path.display()))?;
        }
        Ok(())
    }

    /// 创建所有数据库表
    ///
    /// 通用原语表：`kv_store`（KV 原语）、`cache_meta`（缓存元数据）、
    /// `audit_log`（内核审计落盘）与 `schema_version`（Schema 标记）。
    ///
    /// 业务 schema：以下 sessions / messages / memories / api_keys / jobs 等
    /// 业务表为历史遗留的根 Schema 宿主，逐表标注"业务 schema（属业务域，渐进迁出）"，
    /// 后续迁出到对应业务域后，本函数回归纯落盘原语建表。
    fn create_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            -- KV 存储表（通用原语）
            CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                encrypted BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME
            );

            -- 业务 schema：属业务域（session），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话表
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                worktree_path TEXT,
                name TEXT,
                status TEXT DEFAULT 'active',
                model TEXT,
                provider_id TEXT,
                model_id TEXT,
                system_prompt TEXT,
                total_tokens INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                archived BOOLEAN DEFAULT FALSE,
                archived_at DATETIME,
                metadata TEXT
            );

            -- 会话索引
            CREATE INDEX IF NOT EXISTS idx_sessions_worktree
                ON sessions(worktree_path);

            -- 业务 schema：属业务域（message），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 消息表
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                token_count INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            -- 业务 schema：属业务域（session/timeline），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Agent timeline part表：一轮 assistant 回复中的 thinking / text / tool / permission / summary 步骤
            CREATE TABLE IF NOT EXISTS agent_timeline_parts (
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
                FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
                UNIQUE(session_id, turn_id, sequence)
            );
            CREATE INDEX IF NOT EXISTS idx_agent_timeline_parts_turn
                ON agent_timeline_parts(session_id, turn_id, sequence);
            CREATE INDEX IF NOT EXISTS idx_agent_timeline_parts_message
                ON agent_timeline_parts(message_id, sequence);
            CREATE INDEX IF NOT EXISTS idx_agent_timeline_parts_call
                ON agent_timeline_parts(session_id, turn_id, call_id);

            -- 业务 schema：属业务域（session/changes），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话文件变更事实表：由 edit/write 等真实工具写入，供 Review / Diff / Revert / Share 复用
            CREATE TABLE IF NOT EXISTS session_changes (
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
                operation TEXT NOT NULL,
                before_content TEXT,
                after_content TEXT,
                diff TEXT,
                insertions INTEGER NOT NULL DEFAULT 0,
                deletions INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active',
                created_at DATETIME NOT NULL,
                reverted_at DATETIME,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE,
                FOREIGN KEY (agent_timeline_part_id) REFERENCES agent_timeline_parts(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_changes_session_turn
                ON session_changes(session_id, turn_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_session_changes_step
                ON session_changes(agent_timeline_part_id);
            CREATE INDEX IF NOT EXISTS idx_session_changes_status
                ON session_changes(session_id, status);
            -- 业务 schema：属业务域（memory），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- AI 记忆表
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                scope_type TEXT NOT NULL DEFAULT 'global',
                scope_id TEXT,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                content TEXT NOT NULL,
                confidence REAL DEFAULT 1.0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                expires_at DATETIME
            );
            CREATE INDEX IF NOT EXISTS idx_memories_scope
                ON memories(scope_type, scope_id, category, key);

            -- 业务 schema：属业务域（memory），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- FTS5 全文搜索虚拟表：用于 memory.similar 语义搜索
            -- 仅索引 key 和 content 字段，通过子查询去重后关联主表
            -- 使用 unicode61 tokenizer 支持中英文混合分词
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                key,
                content,
                tokenize='unicode61 remove_diacritics 2'
            );

            -- 缓存元数据表
            CREATE TABLE IF NOT EXISTS cache_meta (
                cache_type TEXT NOT NULL,
                key TEXT NOT NULL,
                size_bytes INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                accessed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                access_count INTEGER DEFAULT 0,
                expires_at DATETIME,
                PRIMARY KEY (cache_type, key)
            );

            -- 业务 schema：属业务域（session），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话快照表
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                name TEXT NOT NULL,
                context TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_session
                ON snapshots(session_id, created_at DESC);

            -- 业务 schema：属业务域（session），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话断点检查点表
            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                checkpoint_type TEXT NOT NULL,
                agent_state TEXT,
                execution_context TEXT NOT NULL,
                anchor_message_id TEXT NOT NULL,
                compression_snapshot TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_checkpoints_session
                ON checkpoints(session_id, created_at DESC);

            -- 业务 schema：属业务域（session），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话软压缩范围表
            CREATE TABLE IF NOT EXISTS compacted_ranges (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                start_message_id TEXT NOT NULL,
                end_message_id TEXT NOT NULL,
                tail_start_message_id TEXT,
                summary_message_id TEXT NOT NULL,
                summary_part_id TEXT,
                token_before INTEGER NOT NULL,
                token_after INTEGER NOT NULL,
                trigger TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (summary_message_id) REFERENCES messages(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_compacted_ranges_session
                ON compacted_ranges(session_id, created_at DESC);

            -- 业务 schema：属业务域（session/events），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 会话事件事实源表：用于 Headless SSE、Share、Review/Revert 补历史
            CREATE TABLE IF NOT EXISTS session_events (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT,
                worktree_path TEXT,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at DATETIME NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_events_session_seq
                ON session_events(session_id, seq);
            CREATE INDEX IF NOT EXISTS idx_session_events_type_seq
                ON session_events(event_type, seq);

            -- 通用原语：内核审计事实源表，记录 Pipeline / Policy / Registry 等通用操作轨迹
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                schema_version INTEGER NOT NULL,
                trace_id TEXT NOT NULL,
                span_id TEXT NOT NULL,
                parent_span_id TEXT,
                scope TEXT NOT NULL,
                source TEXT NOT NULL,
                operation_id TEXT NOT NULL,
                action TEXT NOT NULL,
                policy_decision TEXT,
                duration_ms INTEGER,
                input_digest TEXT NOT NULL,
                output_digest TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_log_trace
                ON audit_log(trace_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_audit_log_scope
                ON audit_log(scope, created_at);

            -- 业务 schema：属业务域（security/auth），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Auth API Keys 表
            CREATE TABLE IF NOT EXISTS api_keys (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                name TEXT NOT NULL,
                key_encrypted TEXT NOT NULL,
                base_url TEXT,
                is_valid TEXT DEFAULT 'unknown',
                last_validated DATETIME,
                expires_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- API Keys 索引
            CREATE INDEX IF NOT EXISTS idx_api_keys_provider
                ON api_keys(provider);

            -- 业务 schema：属业务域（security/auth），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Auth Git Credentials 表
            CREATE TABLE IF NOT EXISTS git_credentials (
                id TEXT PRIMARY KEY,
                repo_pattern TEXT NOT NULL,
                credential_type TEXT NOT NULL,
                username TEXT,
                secret_encrypted TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- 业务 schema：属业务域（security/auth），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Provider 活跃密钥表
            CREATE TABLE IF NOT EXISTS provider_active_keys (
                provider TEXT PRIMARY KEY,
                key_id TEXT NOT NULL,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (key_id) REFERENCES api_keys(id) ON DELETE CASCADE
            );

            -- 业务 schema：属业务域（autonomy），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Autonomy Job 定义表：保存调度/webhook/manual 触发入口
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL CHECK (kind IN ('scheduled', 'webhook', 'manual')),
                status TEXT NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'active', 'paused', 'completed', 'disabled', 'error')),
                trigger TEXT NOT NULL,
                prompt TEXT NOT NULL,
                worktree_path TEXT,
                next_run_at DATETIME,
                last_run_at DATETIME,
                max_retries INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}'
            );
            CREATE INDEX IF NOT EXISTS idx_jobs_status_next_run
                ON jobs(status, next_run_at);
            CREATE INDEX IF NOT EXISTS idx_jobs_kind
                ON jobs(kind);

            -- 业务 schema：属业务域（autonomy），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- Autonomy JobRun 表：每次执行的事实记录
            CREATE TABLE IF NOT EXISTS job_runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled', 'interrupted')),
                trigger_event_id TEXT,
                task_id TEXT,
                session_id TEXT,
                turn_id TEXT,
                attempt INTEGER NOT NULL DEFAULT 1,
                lease_owner TEXT,
                started_at DATETIME,
                finished_at DATETIME,
                error TEXT,
                output_summary TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE,
                FOREIGN KEY (trigger_event_id) REFERENCES inbound_events(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_job_runs_job_created
                ON job_runs(job_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_job_runs_status
                ON job_runs(status, updated_at);
            CREATE INDEX IF NOT EXISTS idx_job_runs_task
                ON job_runs(task_id);
            CREATE INDEX IF NOT EXISTS idx_job_runs_session
                ON job_runs(session_id, created_at DESC);

            -- 业务 schema：属业务域（autonomy），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 入站事件事实源表：source + external_event_id 是幂等键
            CREATE TABLE IF NOT EXISTS inbound_events (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                external_event_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'received'
                    CHECK (status IN ('received', 'claimed', 'processed', 'ignored', 'failed')),
                received_at DATETIME NOT NULL,
                claimed_at DATETIME,
                processed_at DATETIME,
                payload TEXT NOT NULL,
                headers TEXT NOT NULL DEFAULT '{}',
                error TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                UNIQUE(source, external_event_id)
            );
            CREATE INDEX IF NOT EXISTS idx_inbound_events_status_received
                ON inbound_events(status, received_at);
            CREATE INDEX IF NOT EXISTS idx_inbound_events_source_type
                ON inbound_events(source, event_type);

            -- 业务 schema：属业务域（autonomy），渐进迁出到业务扩展；foundation/storage 只承诺落盘原语。
            -- 投递尝试表：记录 job/event 处理结果的对外投递
            CREATE TABLE IF NOT EXISTS deliveries (
                id TEXT PRIMARY KEY,
                job_run_id TEXT,
                inbound_event_id TEXT,
                target_kind TEXT NOT NULL,
                target TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'sending', 'sent', 'failed', 'cancelled')),
                attempt INTEGER NOT NULL DEFAULT 1,
                scheduled_at DATETIME NOT NULL,
                delivered_at DATETIME,
                error TEXT,
                payload TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                CHECK (job_run_id IS NOT NULL OR inbound_event_id IS NOT NULL),
                FOREIGN KEY (job_run_id) REFERENCES job_runs(id) ON DELETE CASCADE,
                FOREIGN KEY (inbound_event_id) REFERENCES inbound_events(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_deliveries_run
                ON deliveries(job_run_id, scheduled_at DESC);
            CREATE INDEX IF NOT EXISTS idx_deliveries_event
                ON deliveries(inbound_event_id, scheduled_at DESC);
            CREATE INDEX IF NOT EXISTS idx_deliveries_status
                ON deliveries(status, scheduled_at);
            ",
        )
        .context("无法创建数据库表")?;

        schema::initialize_root_schema(conn)?;

        tracing::debug!("Database tables created/verified");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory() {
        let conn = Database::open_memory();
        assert!(conn.is_ok());
    }

    #[test]
    fn test_tables_created() {
        let conn = Database::open_memory().unwrap();

        // 验证所有表已创建
        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        assert!(tables.contains(&"kv_store".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"memories".to_string()));
        assert!(tables.contains(&"cache_meta".to_string()));
        assert!(tables.contains(&"jobs".to_string()));
        assert!(tables.contains(&"job_runs".to_string()));
        assert!(tables.contains(&"inbound_events".to_string()));
        assert!(tables.contains(&"deliveries".to_string()));
        assert!(tables.contains(&"session_events".to_string()));
    }

    #[test]
    fn test_autonomy_tables_enforce_idempotent_inbound_events() {
        let conn = Database::open_memory().unwrap();
        conn.execute(
            "INSERT INTO inbound_events
                (id, source, external_event_id, event_type, status, received_at, payload)
             VALUES
                ('evt_1', 'github', 'delivery-1', 'pull_request', 'received', datetime('now'), '{}')",
            [],
        )
        .unwrap();

        let duplicate = conn.execute(
            "INSERT INTO inbound_events
                (id, source, external_event_id, event_type, status, received_at, payload)
             VALUES
                ('evt_2', 'github', 'delivery-1', 'pull_request', 'received', datetime('now'), '{}')",
            [],
        );

        assert!(duplicate.is_err());
    }

    #[test]
    fn test_existing_unstamped_development_database_is_deleted_and_rebuilt() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("navis.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE sessions (id TEXT PRIMARY KEY)", [])
                .unwrap();
        }

        let conn = Database::open(&db_path).unwrap();
        assert_eq!(
            schema::get_current_version(&conn).unwrap(),
            schema::CURRENT_VERSION
        );
        assert!(!std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry.file_name().to_string_lossy().contains(".backup-")));
    }

    #[test]
    fn test_wal_mode() {
        // 注意：内存数据库不支持 WAL 模式，这是正常的
        // WAL 模式只对文件数据库有效
        let conn = Database::open_memory().unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        // 内存数据库的 journal_mode 是 "memory"
        assert!(mode == "wal" || mode == "memory");
    }
}
