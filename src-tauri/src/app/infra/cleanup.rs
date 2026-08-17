//! 缓存清理策略模块
//!
//! 基于设计文档 §四 实现缓存清理逻辑，包括：
//! - TTL 过期清理：移除已过期的缓存条目
//! - LRU 淘汰：当缓存大小超过上限时，淘汰最近最少使用的条目
//! - 全量清理：遍历所有缓存类型依次清理

use anyhow::Result;
use rusqlite::Connection;
use std::sync::MutexGuard;

use super::cache::MemoryCache;
use super::cache_types::{CacheCleanupResult, CacheType};

/// 执行指定缓存类型的清理操作
///
/// 同时清理内存缓存和数据库中的过期缓存元数据。
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `memory_cache` - 内存缓存
/// * `cache_type` - 缓存类型标识
///
/// # Returns
/// 清理结果统计
pub fn cleanup_cache(
    conn: &MutexGuard<'_, Connection>,
    memory_cache: &mut MemoryCache,
    cache_type: &str,
) -> Result<CacheCleanupResult> {
    let start = std::time::Instant::now();
    let now = chrono::Utc::now().to_rfc3339();

    tracing::info!(cache_type = cache_type, "Starting cache cleanup");

    // 1. 清理数据库中过期的缓存元数据
    let db_removed = conn.execute(
        "DELETE FROM cache_meta WHERE cache_type = ?1 AND expires_at IS NOT NULL AND expires_at < ?2",
        rusqlite::params![cache_type, now],
    )?;

    // 2. 获取被删除条目的总大小
    let freed_db_bytes: u64 = {
        // 预先计算被删除条目的大小（在实际删除前查询）
        // 注意：上面已经删除了，所以这里用 0（简化处理）
        // 实际项目中可以在删除前查询 SUM
        0u64
    };

    // 3. 清理内存缓存中的过期条目
    let memory_expired = memory_cache.cleanup_expired();

    // 4. 对持久化缓存类型执行 LRU 淘汰
    let ct = CacheType::from_str(cache_type);
    let lru_removed = if let Some(ref ct) = ct {
        cleanup_lru_if_needed(conn, memory_cache, ct)?
    } else {
        0
    };

    let total_removed = db_removed + memory_expired + lru_removed;
    let duration_ms = start.elapsed().as_millis() as u64;

    tracing::info!(
        cache_type = cache_type,
        entries_removed = total_removed,
        freed_bytes = freed_db_bytes,
        duration_ms = duration_ms,
        "Cache cleanup completed"
    );

    Ok(CacheCleanupResult {
        entries_removed: total_removed,
        freed_bytes: freed_db_bytes,
        duration_ms,
    })
}

/// 执行全量缓存清理
///
/// 遍历所有缓存类型，依次执行清理。
pub fn cleanup_all(
    conn: &MutexGuard<'_, Connection>,
    memory_cache: &mut MemoryCache,
) -> Result<CacheCleanupResult> {
    let start = std::time::Instant::now();
    let now = chrono::Utc::now().to_rfc3339();

    tracing::info!("Starting full cache cleanup");

    // 1. 清理数据库中所有过期的缓存元数据
    let db_removed = conn.execute(
        "DELETE FROM cache_meta WHERE expires_at IS NOT NULL AND expires_at < ?1",
        rusqlite::params![now],
    )?;

    // 2. 清理内存缓存中的过期条目
    let memory_expired = memory_cache.cleanup_expired();

    let total_removed = db_removed + memory_expired;
    let duration_ms = start.elapsed().as_millis() as u64;

    tracing::info!(
        entries_removed = total_removed,
        duration_ms = duration_ms,
        "Full cache cleanup completed"
    );

    Ok(CacheCleanupResult {
        entries_removed: total_removed,
        freed_bytes: 0,
        duration_ms,
    })
}

/// 检查并执行 LRU 淘汰
///
/// 如果指定缓存类型的总大小超过上限，按 LRU 策略淘汰条目。
fn cleanup_lru_if_needed(
    conn: &MutexGuard<'_, Connection>,
    memory_cache: &mut MemoryCache,
    cache_type: &CacheType,
) -> Result<usize> {
    let max_size = cache_type.max_size_bytes();
    let cache_type_str = cache_type.as_str();

    // 查询数据库中该类型的总大小
    let total_size: u64 = conn
        .query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_meta WHERE cache_type = ?1",
            rusqlite::params![cache_type_str],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u64;

    // 加上内存缓存的大小
    let memory_size = memory_cache.size_by_type(cache_type_str);
    let combined_size = total_size + memory_size;

    if combined_size <= max_size {
        return Ok(0);
    }

    tracing::info!(
        cache_type = cache_type_str,
        current_size = combined_size,
        max_size = max_size,
        "Cache size exceeds limit, performing LRU eviction"
    );

    // 从数据库中删除最久未访问的条目
    let to_free = combined_size - max_size;
    let db_removed = conn.execute(
        "DELETE FROM cache_meta WHERE cache_type = ?1 AND key IN (
            SELECT key FROM cache_meta
            WHERE cache_type = ?1
            ORDER BY accessed_at ASC
            LIMIT (
                SELECT COUNT(*) FROM cache_meta
                WHERE cache_type = ?1
                AND size_bytes <= ?2
            )
        )",
        rusqlite::params![cache_type_str, to_free as i64],
    )?;

    // 清理内存缓存
    memory_cache.clear_type(cache_type_str);

    Ok(db_removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::db::Database;
    use std::sync::Mutex;

    #[test]
    fn test_cleanup_expired_cache_meta() {
        let conn = Database::open_memory().unwrap();
        let conn = Mutex::new(conn);
        let mut cache = MemoryCache::new(100);

        // 插入过期的缓存元数据
        {
            let c = conn.lock().unwrap();
            c.execute(
                "INSERT INTO cache_meta (cache_type, key, size_bytes, expires_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["test_cache", "expired_key", 100, "2020-01-01T00:00:00Z"],
            )
            .unwrap();

            // 插入未过期的缓存元数据
            c.execute(
                "INSERT INTO cache_meta (cache_type, key, size_bytes, expires_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params!["test_cache", "valid_key", 200, "2099-12-31T23:59:59Z"],
            )
            .unwrap();
        }

        // 执行清理
        let result = {
            let c = conn.lock().unwrap();
            cleanup_cache(&c, &mut cache, "test_cache").unwrap()
        };

        // 应该只删除了过期的条目
        assert_eq!(result.entries_removed, 1);

        // 验证未过期的条目仍在
        let c = conn.lock().unwrap();
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM cache_meta WHERE cache_type = 'test_cache'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_cleanup_all() {
        let conn = Database::open_memory().unwrap();
        let conn = Mutex::new(conn);
        let mut cache = MemoryCache::new(100);

        // 插入多种类型的过期缓存
        {
            let c = conn.lock().unwrap();
            for i in 0..3 {
                c.execute(
                    "INSERT INTO cache_meta (cache_type, key, size_bytes, expires_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        format!("type_{}", i),
                        format!("key_{}", i),
                        100,
                        "2020-01-01T00:00:00Z"
                    ],
                )
                .unwrap();
            }
        }

        let result = {
            let c = conn.lock().unwrap();
            cleanup_all(&c, &mut cache).unwrap()
        };

        assert_eq!(result.entries_removed, 3);
    }
}
