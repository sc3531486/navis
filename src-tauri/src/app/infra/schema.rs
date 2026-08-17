//! 数据库根 Schema 标记模块（纯落盘原语）。
//!
//! Navis 当前尚未发布，不维护历史版本迁移链。根 Schema 由
//! `Database::create_tables` 一次性创建；这里仅记录当前 Schema 版本，
//! 让启动和测试能明确判断当前数据库是否按根 Schema 初始化完成。
//!
//! # 职责边界（C0-6 收敛）
//! 本模块只维护根 Schema 版本标记（`schema_version` 表 + `CURRENT_VERSION`），
//! **不含任何业务建表声明**。历史遗留的业务表建表声明位于 `db.rs::create_tables`，
//! 已逐表标注"业务 schema（属业务域，渐进迁出）"，**不要在本模块新增业务表**。

use anyhow::{Context, Result};
use rusqlite::Connection;

pub const CURRENT_VERSION: i32 = 12;
pub const CURRENT_DESCRIPTION: &str = "Navis Go root schema";

pub fn get_current_version(conn: &Connection) -> Result<i32> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !exists {
        return Ok(0);
    }

    let version = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get::<_, Option<i32>>(0)
        })
        .unwrap_or(None)
        .unwrap_or(0);
    Ok(version)
}

pub fn initialize_root_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at DATETIME NOT NULL
        );",
    )
    .context("无法创建 schema_version 表")?;

    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version, description, applied_at)
         VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![CURRENT_VERSION, CURRENT_DESCRIPTION],
    )
    .context("无法记录当前数据库 Schema 版本")?;

    tracing::info!(version = CURRENT_VERSION, "Database root schema is ready");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn records_root_schema_version() {
        let conn = Mutex::new(Database::open_memory().unwrap());
        let guard = conn.lock().unwrap();

        initialize_root_schema(&guard).unwrap();

        assert_eq!(get_current_version(&guard).unwrap(), CURRENT_VERSION);
    }
}
