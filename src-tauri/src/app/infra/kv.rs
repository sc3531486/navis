//! KV 存储模块
//!
//! 基于设计文档 §3.1 实现通用键值对持久化存储。
//! 支持 TTL 过期、前缀查询、加密存储等功能。
//!
//! # 存储格式
//! - 键：TEXT 类型的主键
//! - 值：TEXT 类型的 JSON 字符串
//! - 加密：可选 AES-256-GCM 加密，加密后值为 base64 编码字符串
//! - TTL：通过 expires_at 字段实现，NULL 表示永不过期

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::MutexGuard;
use std::time::Duration;

use super::encryption::Encryption;

/// KV 存储操作（实现为 Storage 的方法）
///
/// 所有 KV 操作通过 Storage 结构体的公共方法暴露。
/// 此模块提供 KV 相关的内部实现函数。

/// 读取 KV 值
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块（可选）
/// * `key` - 键名
///
/// # Returns
/// 值（如果存在且未过期）
pub fn kv_get(
    conn: &MutexGuard<'_, Connection>,
    encryption: &Option<Encryption>,
    key: &str,
) -> Result<Option<Value>> {
    tracing::debug!(key = key, "KV get");

    let now = Utc::now().to_rfc3339();

    let result = conn.query_row(
        "SELECT value, encrypted, expires_at FROM kv_store WHERE key = ?1",
        rusqlite::params![key],
        |row| {
            let value: String = row.get(0)?;
            let encrypted: bool = row.get(1)?;
            let expires_at: Option<String> = row.get(2)?;
            Ok((value, encrypted, expires_at))
        },
    );

    match result {
        Ok((value, encrypted, expires_at)) => {
            // 检查是否过期
            if let Some(ref exp) = expires_at {
                if exp.as_str() <= now.as_str() {
                    tracing::debug!(key = key, "KV entry expired");
                    // 异步删除过期条目
                    let _ = conn.execute(
                        "DELETE FROM kv_store WHERE key = ?1",
                        rusqlite::params![key],
                    );
                    return Ok(None);
                }
            }

            // 解密（如果需要）
            let plaintext = if encrypted {
                if let Some(ref enc) = encryption {
                    enc.decrypt(&value)?
                } else {
                    anyhow::bail!("数据已加密但未提供加密密钥");
                }
            } else {
                value.into_bytes()
            };

            let json_value: Value = serde_json::from_slice(&plaintext)?;
            Ok(Some(json_value))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// 写入 KV 值
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块（可选）
/// * `key` - 键名
/// * `value` - 值
/// * `ttl` - TTL（可选）
pub fn kv_set(
    conn: &MutexGuard<'_, Connection>,
    encryption: &Option<Encryption>,
    key: &str,
    value: &Value,
    ttl: Option<Duration>,
) -> Result<()> {
    tracing::debug!(key = key, ttl = ?ttl, "KV set");

    let json_str = serde_json::to_string(value)?;
    let now = Utc::now();

    // 如果有加密模块，加密数据
    let (stored_value, is_encrypted) = if let Some(ref enc) = encryption {
        let encrypted = enc.encrypt(json_str.as_bytes())?;
        (encrypted, true)
    } else {
        (json_str, false)
    };

    // 计算过期时间
    let expires_at = ttl.map(|duration| {
        (now + chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::seconds(0)))
            .to_rfc3339()
    });

    let now_str = now.to_rfc3339();

    conn.execute(
        "INSERT INTO kv_store (key, value, encrypted, created_at, updated_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?4, ?5)
         ON CONFLICT(key) DO UPDATE SET
             value = excluded.value,
             encrypted = excluded.encrypted,
             updated_at = excluded.updated_at,
             expires_at = excluded.expires_at",
        rusqlite::params![key, stored_value, is_encrypted, now_str, expires_at],
    )?;

    tracing::debug!(key = key, "KV entry saved");
    Ok(())
}

/// 删除 KV 条目
pub fn kv_delete(conn: &MutexGuard<'_, Connection>, key: &str) -> Result<()> {
    tracing::debug!(key = key, "KV delete");

    let deleted = conn.execute(
        "DELETE FROM kv_store WHERE key = ?1",
        rusqlite::params![key],
    )?;

    if deleted == 0 {
        tracing::debug!(key = key, "KV entry not found for deletion");
    }

    Ok(())
}

/// 按前缀列出 KV 条目
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块（可选）
/// * `prefix` - 键前缀
///
/// # Returns
/// 匹配的 (key, value) 列表
pub fn kv_list(
    conn: &MutexGuard<'_, Connection>,
    encryption: &Option<Encryption>,
    prefix: &str,
) -> Result<Vec<(String, Value)>> {
    tracing::debug!(prefix = prefix, "KV list");

    let now = Utc::now().to_rfc3339();
    let pattern = format!("{}%", prefix);

    let mut stmt = conn.prepare(
        "SELECT key, value, encrypted, expires_at FROM kv_store
         WHERE key LIKE ?1
         ORDER BY key",
    )?;

    let entries = stmt
        .query_map(rusqlite::params![pattern], |row| {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            let encrypted: bool = row.get(2)?;
            let expires_at: Option<String> = row.get(3)?;
            Ok((key, value, encrypted, expires_at))
        })?
        .filter_map(|r| r.ok())
        .filter_map(|(key, value, encrypted, expires_at)| {
            // 过滤过期条目
            if let Some(ref exp) = expires_at {
                if exp.as_str() <= now.as_str() {
                    return None;
                }
            }

            // 解密
            let plaintext = if encrypted {
                if let Some(ref enc) = encryption {
                    enc.decrypt(&value).ok()?
                } else {
                    return None;
                }
            } else {
                value.into_bytes()
            };

            let json_value: Value = serde_json::from_slice(&plaintext).ok()?;
            Some((key, json_value))
        })
        .collect();

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::db::Database;
    use serde_json::json;
    use std::sync::Mutex;

    fn setup() -> (Mutex<Connection>, Option<Encryption>) {
        let conn = Database::open_memory().unwrap();
        (Mutex::new(conn), None)
    }

    #[test]
    fn test_kv_set_get() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        kv_set(&conn, &enc, "test_key", &json!("hello"), None).unwrap();
        let result = kv_get(&conn, &enc, "test_key").unwrap();
        assert_eq!(result, Some(json!("hello")));
    }

    #[test]
    fn test_kv_get_nonexistent() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        let result = kv_get(&conn, &enc, "nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_kv_overwrite() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        kv_set(&conn, &enc, "key", &json!("v1"), None).unwrap();
        kv_set(&conn, &enc, "key", &json!("v2"), None).unwrap();

        let result = kv_get(&conn, &enc, "key").unwrap();
        assert_eq!(result, Some(json!("v2")));
    }

    #[test]
    fn test_kv_delete() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        kv_set(&conn, &enc, "key", &json!("value"), None).unwrap();
        kv_delete(&conn, "key").unwrap();

        let result = kv_get(&conn, &enc, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_kv_delete_nonexistent() {
        let (conn, _enc) = setup();
        let conn = conn.lock().unwrap();

        // 删除不存在的键不应报错
        kv_delete(&conn, "nonexistent").unwrap();
    }

    #[test]
    fn test_kv_list_prefix() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        kv_set(&conn, &enc, "app.name", &json!("Navis Go"), None).unwrap();
        kv_set(&conn, &enc, "app.version", &json!("1.0"), None).unwrap();
        kv_set(&conn, &enc, "user.name", &json!("test"), None).unwrap();

        let results = kv_list(&conn, &enc, "app.").unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(k, _)| k.starts_with("app.")));
    }

    #[test]
    fn test_kv_list_empty_prefix() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        kv_set(&conn, &enc, "key1", &json!(1), None).unwrap();
        kv_set(&conn, &enc, "key2", &json!(2), None).unwrap();

        let results = kv_list(&conn, &enc, "").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_kv_complex_value() {
        let (conn, enc) = setup();
        let conn = conn.lock().unwrap();

        let value = json!({
            "name": "Navis Go",
            "version": "1.0.0",
            "features": ["ai", "storage", "cache"]
        });

        kv_set(&conn, &enc, "config", &value, None).unwrap();
        let result = kv_get(&conn, &enc, "config").unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_kv_with_encryption() {
        let conn = Database::open_memory().unwrap();
        let conn = Mutex::new(conn);
        let key = vec![42u8; 32];
        let enc = Some(Encryption::new(&key).unwrap());

        {
            let c = conn.lock().unwrap();
            kv_set(&c, &enc, "secret", &json!("my_api_key"), None).unwrap();
        }

        // 验证数据库中的值是加密的
        let stored_value: String = {
            let c = conn.lock().unwrap();
            c.query_row(
                "SELECT value FROM kv_store WHERE key = 'secret'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_ne!(stored_value, "\"my_api_key\"");

        // 但通过 API 读取时可以正确解密
        let c = conn.lock().unwrap();
        let result = kv_get(&c, &enc, "secret").unwrap();
        assert_eq!(result, Some(json!("my_api_key")));
    }
}
