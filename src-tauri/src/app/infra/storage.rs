//! 存储管理器实现
//!
//! 承载 `Storage` 结构体及其实现，从 `foundation::storage` 迁入。
//! 这是基础设施实现的核心组件，管理数据库连接、加密、审计等。

use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::kernel::{AuditRecorder, EventBus, EventEnvelope, KernelContext, KernelScope};

use super::{AppState, Database, Encryption, audit_store, kv};
use crate::extension::types::MemoryStore;

/// 存储管理器
pub struct Storage {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
    /// 加密模块
    encryption: Option<Encryption>,
    /// 事件总线
    event_bus: Arc<dyn EventBus>,
    /// Kernel 审计记录器。结构化审计事实统一通过 AuditSink 写入。
    audit_recorder: AuditRecorder,
}

impl Storage {
    pub(crate) fn from_shared_connection(
        conn: Arc<Mutex<Connection>>,
        encryption_key: Option<&str>,
        event_bus: Arc<dyn EventBus>,
    ) -> Result<Self> {
        let encryption = match encryption_key {
            Some(key) => Some(Encryption::new(key.as_bytes())?),
            None => None,
        };

        let audit_recorder =
            AuditRecorder::new(Arc::new(audit_store::StorageAuditSink::new(conn.clone())));

        let storage = Self {
            conn,
            encryption,
            event_bus,
            audit_recorder,
        };

        tracing::info!("Storage created successfully");
        Ok(storage)
    }

    /// 创建新的存储管理器
    ///
    /// # Arguments
    /// * `db_path` - 数据库文件路径
    /// * `encryption_key` - 加密密钥（可选）
    /// * `event_bus` - 事件总线
    pub fn new(
        db_path: &Path,
        encryption_key: Option<&str>,
        event_bus: Arc<dyn EventBus>,
    ) -> Result<Self> {
        tracing::info!(path = %db_path.display(), "Creating new Storage");

        let conn = Arc::new(Mutex::new(Database::open(db_path)?));
        Self::from_shared_connection(conn, encryption_key, event_bus)
    }

    /// 创建内存存储（仅用于测试）。
    #[cfg(test)]
    pub fn new_memory(encryption_key: Option<&str>, event_bus: Arc<dyn EventBus>) -> Result<Self> {
        tracing::debug!("Creating in-memory Storage");

        let conn = Arc::new(Mutex::new(Database::open_memory()?));
        let storage = Self::from_shared_connection(conn, encryption_key, event_bus)?;
        tracing::debug!("In-memory Storage created");
        Ok(storage)
    }

    /// 暴露底层数据库连接（基础设施原语）。
    ///
    /// 业务 facade（SessionStore 等）已迁出 foundation 至业务域；
    /// 业务域通过此访问器构造自己的存储入口。
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// 获取记忆领域存储入口。
    ///
    /// 业务 facade（MemoryStore）：承载 memories 业务 schema 访问，
    /// 属业务域概念，渐进迁出到业务域；本模块暂保留连接宿主能力。
    pub fn memory_store(&self) -> MemoryStore {
        MemoryStore::new(self.conn.clone())
    }

    /// 获取加密模块
    pub fn encryption(&self) -> Option<&Encryption> {
        self.encryption.as_ref()
    }

    /// 获取事件总线
    pub fn event_bus(&self) -> &Arc<dyn EventBus> {
        &self.event_bus
    }

    /// 获取内核审计记录器。
    pub fn audit_recorder(&self) -> AuditRecorder {
        self.audit_recorder.clone()
    }


    /// 读取通用 KV 值。领域 facade 应负责 key 命名空间隔离；这里仅暴露 Storage 统一入口。
    pub fn kv_get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("storage connection mutex poisoned"))?;
        kv::kv_get(&conn, &self.encryption, key)
    }

    /// 写入通用 KV 值。领域 facade 应负责 key 命名空间隔离；这里仅暴露 Storage 统一入口。
    pub fn kv_set(
        &self,
        key: &str,
        value: &serde_json::Value,
        ttl: Option<std::time::Duration>,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("storage connection mutex poisoned"))?;
        kv::kv_set(&conn, &self.encryption, key, value, ttl)
    }

    /// 删除通用 KV 值。
    pub fn kv_delete(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("storage connection mutex poisoned"))?;
        kv::kv_delete(&conn, key)
    }

    /// 按前缀列出通用 KV 值。
    pub fn kv_list(&self, prefix: &str) -> Result<Vec<(String, serde_json::Value)>> {
        let conn = self.conn.lock().map_err(|_| anyhow::anyhow!("storage connection mutex poisoned"))?;
        kv::kv_list(&conn, &self.encryption, prefix)
    }

    // ======================================================================
    // AppState 持久化（从 Config 模块迁移到 Storage 模块）
    // ======================================================================

    /// 加载应用状态
    ///
    /// 业务耦合：直接读写 `kv_store` 表与 `'app_state'` key（绕过 `kv_*` 原语，
    /// 属业务 schema 用法）。渐进迁出到业务域；暂保留以兼容现有调用。
    /// 从 KV 存储中读取应用状态，如果不存在则返回默认值
    pub fn load_app_state(&self) -> Result<AppState> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE key = 'app_state'")?;

        let result = stmt.query_row([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let state = AppState::from_json(&json);
                tracing::debug!("App state loaded from storage");
                Ok(state)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tracing::debug!("No app state found, returning default");
                Ok(AppState::default())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// 保存应用状态
    ///
    /// 业务耦合：直接写入 `kv_store` 表与 `'app_state'` key（绕过 `kv_*` 原语，
    /// 属业务 schema 用法）。渐进迁出到业务域；暂保留以兼容现有调用。
    /// 将应用状态序列化为 JSON 并存储到 KV 存储中
    pub fn save_app_state(&self, state: &AppState) -> Result<()> {
        let json = state.to_json();
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value, encrypted, created_at, updated_at)
             VALUES ('app_state', ?1, FALSE, ?2, ?2)",
            rusqlite::params![json, now],
        )?;

        tracing::info!("App state saved to storage");

        // 发送事件
        let topic = "appstate.saved";
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "appstate.saved",
            KernelContext::new("storage", KernelScope::global()),
            None,
        )) {
            tracing::warn!(
                topic = %topic,
                error = %error,
                "Failed to publish storage event"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_creation() {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        let storage = Storage::new_memory(None, event_bus);
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_storage_with_encryption() {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        let storage =
            Storage::new_memory(Some("test-key-123-very-long-key-for-testing"), event_bus);
        match storage {
            Ok(s) => {
                assert!(s.encryption().is_some());
            }
            Err(e) => {
                panic!("Failed to create storage with encryption: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_storage_connection() {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        let storage = Storage::new_memory(None, event_bus).unwrap();

        let conn = storage.connection();

        // 测试连接可用
        let result: Result<i32, _> = conn.query_row("SELECT 1", [], |row| row.get(0));
        match result {
            Ok(val) => assert_eq!(val, 1),
            Err(e) => panic!("Connection test failed: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_app_state_save_and_load() {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        let storage = Storage::new_memory(None, event_bus).unwrap();

        // 默认状态
        let state = storage.load_app_state().unwrap();
        assert!(state.first_launch);

        // 保存自定义状态
        let mut new_state = AppState::default();
        new_state.last_active_session = Some("sess_001".to_string());
        new_state.last_worktree = Some("/path/to/worktree".to_string());
        new_state.first_launch = false;

        storage.save_app_state(&new_state).unwrap();

        // 加载并验证
        let loaded = storage.load_app_state().unwrap();
        assert_eq!(loaded.last_active_session, Some("sess_001".to_string()));
        assert_eq!(loaded.last_worktree, Some("/path/to/worktree".to_string()));
        assert!(!loaded.first_launch);
    }
}
