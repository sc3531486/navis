//! Provider 分组管理模块
//!
//! 管理每个 Provider 的 API Key 分组和活跃密钥选择。
//! 每个 Provider 同一时间只有一个活跃密钥。
//!
//! # 活跃密钥选择策略
//! 1. 优先返回 provider_active_keys 表中指定的活跃密钥
//! 2. 如果没有显式 active key，返回 None

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use super::key_store;
use crate::app::infra::Encryption;

/// 设置 Provider 的活跃密钥
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `provider` - Provider 类型
/// * `key_id` - 要设为活跃的 Key ID
pub fn set_active_key(conn: &Arc<Mutex<Connection>>, provider: &str, key_id: &str) -> Result<()> {
    let conn_guard = conn.lock().unwrap();
    let now = Utc::now().to_rfc3339();

    conn_guard.execute(
        "INSERT INTO provider_active_keys (provider, key_id, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(provider) DO UPDATE SET
             key_id = excluded.key_id,
             updated_at = excluded.updated_at",
        rusqlite::params![provider, key_id, now],
    )?;

    tracing::debug!(
        provider = provider,
        key_id = key_id,
        "Active key set for provider"
    );

    Ok(())
}

/// 获取 Provider 的活跃密钥 ID
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `provider` - Provider 类型
///
/// # Returns
/// 活跃密钥的 ID，如果没有显式 active key 则返回 None
pub fn get_active_key_id(conn: &Arc<Mutex<Connection>>, provider: &str) -> Result<Option<String>> {
    let conn_guard = conn.lock().unwrap();

    // 尝试从 provider_active_keys 表获取显式 active key
    let result = conn_guard.query_row(
        "SELECT key_id FROM provider_active_keys WHERE provider = ?1",
        rusqlite::params![provider],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(key_id) => {
            // 验证 key_id 仍然存在
            let exists: bool = conn_guard
                .query_row(
                    "SELECT COUNT(*) FROM api_keys WHERE id = ?1",
                    rusqlite::params![key_id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;

            if exists {
                return Ok(Some(key_id));
            }
            // active key 已失效，直接返回 None
            return Ok(None);
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // 没有设置活跃密钥
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    }
}

/// 获取 Provider 的活跃解密密钥
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块
/// * `provider` - Provider 类型
///
/// # Returns
/// 解密后的明文密钥
pub fn get_active_decrypted_key(
    conn: &Arc<Mutex<Connection>>,
    encryption: &Option<Encryption>,
    provider: &str,
) -> Result<Option<String>> {
    let key_id = match get_active_key_id(conn, provider)? {
        Some(id) => id,
        None => return Ok(None),
    };

    key_store::get_decrypted_key(conn, encryption, &key_id)
}

/// 清除 Provider 的活跃密钥设置
pub fn clear_active_key(conn: &Arc<Mutex<Connection>>, provider: &str) -> Result<()> {
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "DELETE FROM provider_active_keys WHERE provider = ?1",
        rusqlite::params![provider],
    )?;
    Ok(())
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::db::Database;
use crate::app::infra::Encryption;
    use crate::security::auth::key_store;

    fn create_test_conn() -> (Arc<Mutex<Connection>>, Option<Encryption>) {
        let conn = Database::open_memory().unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let encryption = Encryption::new(&vec![0u8; 32]).unwrap();
        (conn, Some(encryption))
    }

    #[test]
    fn test_set_and_get_active_key() {
        let (conn, enc) = create_test_conn();

        // 添加两个密钥
        let k1 = key_store::insert_key(&conn, &enc, "openai", "Key 1", "sk-key1", None).unwrap();
        let k2 = key_store::insert_key(&conn, &enc, "openai", "Key 2", "sk-key2", None).unwrap();

        // 设置 k1 为活跃密钥
        set_active_key(&conn, "openai", &k1.id).unwrap();

        let active_id = get_active_key_id(&conn, "openai").unwrap();
        assert_eq!(active_id, Some(k1.id.clone()));

        // 切换到 k2
        set_active_key(&conn, "openai", &k2.id).unwrap();

        let active_id = get_active_key_id(&conn, "openai").unwrap();
        assert_eq!(active_id, Some(k2.id));
    }

    #[test]
    fn test_get_active_key_missing_returns_none() {
        let (conn, enc) = create_test_conn();

        // 添加密钥但不设置活跃密钥
        let _k1 = key_store::insert_key(&conn, &enc, "openai", "Key 1", "sk-key1", None).unwrap();
        let _k2 = key_store::insert_key(&conn, &enc, "openai", "Key 2", "sk-key2", None).unwrap();

        // 没有显式 active key 时应返回 None
        let active_id = get_active_key_id(&conn, "openai").unwrap();
        assert_eq!(active_id, None);
    }

    #[test]
    fn test_get_active_key_empty_provider() {
        let (conn, _enc) = create_test_conn();

        let active_id = get_active_key_id(&conn, "nonexistent").unwrap();
        assert_eq!(active_id, None);
    }

    #[test]
    fn test_get_active_decrypted_key() {
        let (conn, enc) = create_test_conn();

        let k1 = key_store::insert_key(&conn, &enc, "openai", "Key 1", "sk-my-secret-key", None)
            .unwrap();
        set_active_key(&conn, "openai", &k1.id).unwrap();

        let decrypted = get_active_decrypted_key(&conn, &enc, "openai").unwrap();
        assert_eq!(decrypted, Some("sk-my-secret-key".to_string()));
    }

    #[test]
    fn test_clear_active_key() {
        let (conn, enc) = create_test_conn();

        let k1 = key_store::insert_key(&conn, &enc, "openai", "Key 1", "sk-key1", None).unwrap();
        set_active_key(&conn, "openai", &k1.id).unwrap();

        // 确认设置了活跃密钥
        assert!(get_active_key_id(&conn, "openai").unwrap().is_some());

        // 清除活跃密钥
        clear_active_key(&conn, "openai").unwrap();

        // 不再自动回退到最新密钥
        let active = get_active_key_id(&conn, "openai").unwrap();
        assert_eq!(active, None);
    }

    #[test]
    fn test_active_key_cascade_on_delete() {
        let (conn, enc) = create_test_conn();

        let k1 = key_store::insert_key(&conn, &enc, "openai", "Key 1", "sk-key1", None).unwrap();
        let _k2 = key_store::insert_key(&conn, &enc, "openai", "Key 2", "sk-key2", None).unwrap();

        // 设置 k1 为活跃密钥
        set_active_key(&conn, "openai", &k1.id).unwrap();

        // 删除 k1
        key_store::delete_key(&conn, &k1.id).unwrap();

        // 删除 active key 后，不再自动回退到其他密钥
        let active = get_active_key_id(&conn, "openai").unwrap();
        assert_eq!(active, None);
    }

    #[test]
    fn test_multiple_providers() {
        let (conn, enc) = create_test_conn();

        let openai_key =
            key_store::insert_key(&conn, &enc, "openai", "OpenAI", "sk-openai", None).unwrap();
        let anthropic_key =
            key_store::insert_key(&conn, &enc, "anthropic", "Anthropic", "sk-ant-key", None)
                .unwrap();

        set_active_key(&conn, "openai", &openai_key.id).unwrap();
        set_active_key(&conn, "anthropic", &anthropic_key.id).unwrap();

        let openai_active = get_active_key_id(&conn, "openai").unwrap();
        let anthropic_active = get_active_key_id(&conn, "anthropic").unwrap();

        assert_eq!(openai_active, Some(openai_key.id));
        assert_eq!(anthropic_active, Some(anthropic_key.id));
    }
}
