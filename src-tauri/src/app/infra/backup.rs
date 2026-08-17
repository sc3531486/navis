//! 备份/恢复模块
//!
//! 基于设计文档 §3.1 实现数据库备份和恢复功能。
//!
//! # 备份策略
//! - 使用 SQLite 的 `VACUUM INTO` 命令创建一致的数据库副本
//! - 备份文件为完整的 SQLite 数据库文件
//! - 加密数据在备份中保持加密状态
//!
//! # 恢复策略
//! - `restore()` - 恢复非加密数据（会话、消息、记忆等）
//! - `restore_encrypted()` - 使用主密码解密并恢复加密数据
//! - 恢复操作会替换当前数据库文件

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::MutexGuard;

/// 执行数据库备份
///
/// 使用 SQLite 的 VACUUM INTO 命令创建一致的数据库快照。
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `backup_path` - 备份文件路径
pub fn backup(conn: &MutexGuard<'_, Connection>, backup_path: &Path) -> Result<()> {
    tracing::info!(path = %backup_path.display(), "Starting database backup");

    // 确保备份目录存在
    if let Some(parent) = backup_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("无法创建备份目录: {}", parent.display()))?;
    }

    // 使用 VACUUM INTO 创建一致的数据库副本
    let path_str = backup_path.to_str().context("备份路径包含无效字符")?;

    // 转义路径中的单引号
    let escaped_path = path_str.replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}'", escaped_path))
        .context("执行 VACUUM INTO 失败")?;

    // 验证备份文件已创建
    if !backup_path.exists() {
        anyhow::bail!("备份文件未成功创建: {}", backup_path.display());
    }

    let file_size = std::fs::metadata(backup_path).map(|m| m.len()).unwrap_or(0);

    tracing::info!(
        path = %backup_path.display(),
        size_bytes = file_size,
        "Database backup completed"
    );

    Ok(())
}

/// 从备份恢复数据库
///
/// # Arguments
/// * `conn` - 当前数据库连接（恢复前需要关闭）
/// * `db_path` - 当前数据库文件路径
/// * `backup_path` - 备份文件路径
pub fn restore(
    conn: &MutexGuard<'_, Connection>,
    db_path: &Path,
    backup_path: &Path,
) -> Result<()> {
    tracing::info!(
        backup_path = %backup_path.display(),
        db_path = %db_path.display(),
        "Starting database restore"
    );

    // 验证备份文件存在
    if !backup_path.exists() {
        anyhow::bail!("备份文件不存在: {}", backup_path.display());
    }

    // 验证备份文件是有效的 SQLite 数据库
    verify_backup(backup_path)?;

    // 先执行 checkpoint 确保 WAL 数据写入主文件
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .context("无法执行 WAL checkpoint")?;

    // 关闭当前连接的 WAL 模式文件
    // 注意：在实际使用中，调用者需要先关闭 Storage 的连接
    // 这里我们直接复制文件覆盖

    // 复制备份文件到数据库位置
    std::fs::copy(backup_path, db_path).with_context(|| {
        format!(
            "无法复制备份文件: {} -> {}",
            backup_path.display(),
            db_path.display()
        )
    })?;

    // 如果存在 WAL 和 SHM 文件，删除它们
    let wal_path = db_path.with_extension("db-wal");
    let shm_path = db_path.with_extension("db-shm");
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&shm_path);

    tracing::info!("Database restore completed");
    Ok(())
}

/// 验证备份文件是否是有效的 SQLite 数据库
fn verify_backup(path: &Path) -> Result<()> {
    let conn =
        Connection::open(path).with_context(|| format!("无法打开备份文件: {}", path.display()))?;

    // 检查是否包含必要的表
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .context("无法查询备份文件的表结构")?;

        let mut rows = stmt.query([]).context("无法读取表名")?;
        let mut tables = Vec::new();
        while let Some(row) = rows.next()? {
            let table_name: String = row.get(0)?;
            tables.push(table_name);
        }
        tables
    };

    let required_tables = ["kv_store", "sessions", "messages", "memories", "cache_meta"];
    for table in &required_tables {
        if !tables.contains(&table.to_string()) {
            anyhow::bail!("备份文件缺少必要表: {}", table);
        }
    }

    tracing::debug!(path = %path.display(), "Backup file verified");
    Ok(())
}

/// 获取备份信息
pub fn get_backup_info(path: &Path) -> Result<BackupInfo> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("无法读取备份文件信息: {}", path.display()))?;

    let conn =
        Connection::open(path).with_context(|| format!("无法打开备份文件: {}", path.display()))?;

    // 统计各表的记录数
    let session_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap_or(0);
    let message_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .unwrap_or(0);
    let memory_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(BackupInfo {
        path: path.to_string_lossy().to_string(),
        size_bytes: metadata.len(),
        session_count: session_count as usize,
        message_count: message_count as usize,
        memory_count: memory_count as usize,
        created_at: metadata.modified().ok().map(|t| {
            let datetime: chrono::DateTime<chrono::Utc> = t.into();
            datetime.to_rfc3339()
        }),
    })
}

/// 备份信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupInfo {
    /// 备份文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size_bytes: u64,
    /// 会话数
    pub session_count: usize,
    /// 消息数
    pub message_count: usize,
    /// 记忆数
    pub memory_count: usize,
    /// 创建时间
    pub created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::db::Database;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[test]
    fn test_backup_and_verify() {
        let conn = Database::open_memory().unwrap();
        let conn = Mutex::new(conn);
        let temp_dir = tempdir().unwrap();
        let backup_path = temp_dir.path().join("backup.db");

        // 插入一些测试数据
        {
            let c = conn.lock().unwrap();
            c.execute(
                "INSERT INTO sessions (id, status) VALUES ('test_1', 'active')",
                [],
            )
            .unwrap();
        }

        // 备份
        let c = conn.lock().unwrap();
        backup(&c, &backup_path).unwrap();

        // 验证备份文件
        assert!(backup_path.exists());
        verify_backup(&backup_path).unwrap();
    }

    #[test]
    fn test_backup_file_path() {
        let conn = Database::open_memory().unwrap();
        let conn = Mutex::new(conn);
        let temp_dir = tempdir().unwrap();
        let backup_path = temp_dir.path().join("subdir").join("backup.db");

        let c = conn.lock().unwrap();
        backup(&c, &backup_path).unwrap();
        assert!(backup_path.exists());
    }

    #[test]
    fn test_backup_nonexistent_path() {
        let path = Path::new("/nonexistent/path/backup.db");
        let result = verify_backup(path);
        assert!(result.is_err());
    }
}
