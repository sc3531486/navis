//! App 壳内基础设施实现
//!
//! 承载从 `foundation::storage` 迁出的纯基础设施实现：
//! - `db` — SQLite 连接管理、WAL 模式、根表创建
//! - `schema` — Schema 版本标记
//! - `encryption` — AES-256-GCM 加密/解密
//! - `kv` — KV 表操作
//! - `backup` — 数据库备份/恢复
//! - `cache` — 内存 LRU 缓存
//! - `cache_types` — 缓存类型定义
//! - `audit_store` — Kernel 审计落盘适配
//! - `app_state` — 应用状态持久化
//! - `cleanup` — 缓存清理策略
//! - `storage` — 存储管理器（Storage struct）

pub mod app_state;
pub mod audit_store;
pub mod backup;
pub mod cache;
pub mod cache_types;
pub mod cleanup;
pub mod db;
pub mod encryption;
pub mod kv;
pub mod schema;
pub mod storage;

pub use app_state::{AppState, WindowState};
pub use cache_types::{CacheEntry, CacheStats, CacheType};
pub use db::Database;
pub use encryption::Encryption;
pub use storage::Storage;
