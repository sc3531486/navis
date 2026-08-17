//! API Key 存储模块
//!
//! 负责 API Key 的数据库 CRUD 操作，支持加密存储。
//! 所有密钥使用 AES-256-GCM 加密后存储，读取时按需解密。

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex, MutexGuard};

use super::{generate_id, ApiKey, ValidationStatus};
use crate::app::infra::Encryption;

/// 插入新的 API Key
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块
/// * `provider` - Provider 类型
/// * `name` - 用户自定义名称
/// * `key` - 明文密钥
/// * `base_url` - 自定义 API 地址
pub fn insert_key(
    conn: &Arc<Mutex<Connection>>,
    encryption: &Option<Encryption>,
    provider: &str,
    name: &str,
    key: &str,
    base_url: Option<&str>,
) -> Result<ApiKey> {
    let id = generate_id();
    let now = Utc::now();

    // 加密密钥
    let key_encrypted_str = if let Some(ref enc) = encryption {
        enc.encrypt(key.as_bytes())?
    } else {
        tracing::warn!("No encryption module, storing key in base64 only");
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key.as_bytes())
    };

    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "INSERT INTO api_keys (id, provider, name, key_encrypted, base_url, is_valid, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            id,
            provider,
            name,
            key_encrypted_str,
            base_url,
            ValidationStatus::Unknown.as_str(),
            now.to_rfc3339(),
        ],
    )?;

    Ok(ApiKey {
        id,
        provider: provider.to_string(),
        name: name.to_string(),
        key_encrypted: key_encrypted_str.into_bytes(),
        base_url: base_url.map(|s| s.to_string()),
        is_valid: ValidationStatus::Unknown,
        last_validated: None,
        expires_at: None,
        created_at: now,
    })
}

/// 根据 ID 获取 API Key 信息
pub fn get_key_by_id(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<Option<ApiKey>> {
    let conn_guard = conn.lock().unwrap();
    get_key_by_id_inner(&conn_guard, id)
}

/// 内部函数：根据 ID 获取 API Key 信息
fn get_key_by_id_inner(conn: &MutexGuard<'_, Connection>, id: &str) -> Result<Option<ApiKey>> {
    let result = conn.query_row(
        "SELECT id, provider, name, key_encrypted, base_url, is_valid, last_validated, expires_at, created_at
         FROM api_keys WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        },
    );

    match result {
        Ok((
            id,
            provider,
            name,
            key_enc,
            base_url,
            is_valid,
            last_validated,
            expires_at,
            created_at,
        )) => Ok(Some(ApiKey {
            id,
            provider,
            name,
            key_encrypted: key_enc.into_bytes(),
            base_url,
            is_valid: ValidationStatus::from_str(&is_valid),
            last_validated: last_validated.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            expires_at: expires_at.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            }),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| Utc::now()),
        })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// 获取解密后的 API Key 明文
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块
/// * `id` - Key ID
///
/// # Returns
/// 解密后的明文密钥。解密失败时记录错误日志并返回 None。
pub fn get_decrypted_key(
    conn: &Arc<Mutex<Connection>>,
    encryption: &Option<Encryption>,
    id: &str,
) -> Result<Option<String>> {
    let conn_guard = conn.lock().unwrap();
    let api_key = match get_key_by_id_inner(&conn_guard, id)? {
        Some(k) => k,
        None => return Ok(None),
    };

    decrypt_key_value(encryption, &api_key.key_encrypted)
}

/// 解密密钥值
fn decrypt_key_value(
    encryption: &Option<Encryption>,
    key_encrypted: &[u8],
) -> Result<Option<String>> {
    let key_enc_str = String::from_utf8_lossy(key_encrypted).to_string();

    if let Some(ref enc) = encryption {
        match enc.decrypt(&key_enc_str) {
            Ok(plaintext) => {
                let key = String::from_utf8(plaintext)
                    .map_err(|e| anyhow::anyhow!("密钥解码失败: {}", e))?;
                Ok(Some(key))
            }
            Err(e) => {
                tracing::error!("密钥解密失败: {}", e);
                Ok(None)
            }
        }
    } else {
        // 无加密模块时尝试 base64 解码
        use base64::Engine;
        match base64::engine::general_purpose::STANDARD.decode(&key_enc_str) {
            Ok(bytes) => {
                let key =
                    String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("密钥解码失败: {}", e))?;
                Ok(Some(key))
            }
            Err(e) => {
                tracing::error!("密钥 base64 解码失败: {}", e);
                Ok(None)
            }
        }
    }
}

/// 获取指定 Provider 的所有 API Keys
pub fn get_keys_by_provider(conn: &Arc<Mutex<Connection>>, provider: &str) -> Result<Vec<ApiKey>> {
    let conn_guard = conn.lock().unwrap();
    let mut stmt = conn_guard.prepare(
        "SELECT id, provider, name, key_encrypted, base_url, is_valid, last_validated, expires_at, created_at
         FROM api_keys WHERE provider = ?1 ORDER BY created_at DESC"
    )?;

    let keys = stmt
        .query_map(rusqlite::params![provider], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(
            |(
                id,
                provider,
                name,
                key_enc,
                base_url,
                is_valid,
                last_validated,
                expires_at,
                created_at,
            )| {
                ApiKey {
                    id,
                    provider,
                    name,
                    key_encrypted: key_enc.into_bytes(),
                    base_url,
                    is_valid: ValidationStatus::from_str(&is_valid),
                    last_validated: last_validated.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    }),
                    expires_at: expires_at.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    }),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }
            },
        )
        .collect();

    Ok(keys)
}

/// 列出所有 API Keys
pub fn list_keys(conn: &Arc<Mutex<Connection>>) -> Result<Vec<ApiKey>> {
    let conn_guard = conn.lock().unwrap();
    let mut stmt = conn_guard.prepare(
        "SELECT id, provider, name, key_encrypted, base_url, is_valid, last_validated, expires_at, created_at
         FROM api_keys ORDER BY created_at DESC"
    )?;

    let keys = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(
            |(
                id,
                provider,
                name,
                key_enc,
                base_url,
                is_valid,
                last_validated,
                expires_at,
                created_at,
            )| {
                ApiKey {
                    id,
                    provider,
                    name,
                    key_encrypted: key_enc.into_bytes(),
                    base_url,
                    is_valid: ValidationStatus::from_str(&is_valid),
                    last_validated: last_validated.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    }),
                    expires_at: expires_at.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    }),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }
            },
        )
        .collect();

    Ok(keys)
}

/// 删除 API Key
pub fn delete_key(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<()> {
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute("DELETE FROM api_keys WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

/// 更新密钥校验状态
pub fn update_validation_status(
    conn: &Arc<Mutex<Connection>>,
    id: &str,
    status: &ValidationStatus,
) -> Result<()> {
    let conn_guard = conn.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    conn_guard.execute(
        "UPDATE api_keys SET is_valid = ?1, last_validated = ?2 WHERE id = ?3",
        rusqlite::params![status.as_str(), now, id],
    )?;
    Ok(())
}

/// 轮转密钥：更新加密密钥并重置校验状态
pub fn rotate_key(
    conn: &Arc<Mutex<Connection>>,
    encryption: &Option<Encryption>,
    id: &str,
    new_key: &str,
) -> Result<()> {
    // 加密新密钥
    let key_encrypted_str = if let Some(ref enc) = encryption {
        enc.encrypt(new_key.as_bytes())?
    } else {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(new_key.as_bytes())
    };

    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "UPDATE api_keys SET key_encrypted = ?1, is_valid = ?2, last_validated = NULL WHERE id = ?3",
        rusqlite::params![key_encrypted_str, ValidationStatus::Unknown.as_str(), id],
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

    fn create_test_conn() -> (Arc<Mutex<Connection>>, Option<Encryption>) {
        let conn = Database::open_memory().unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let encryption = Encryption::new(&vec![0u8; 32]).unwrap();
        (conn, Some(encryption))
    }

    #[test]
    fn test_insert_and_get_key() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(&conn, &enc, "openai", "Test Key", "sk-abc123", None).unwrap();

        assert_eq!(api_key.provider, "openai");
        assert_eq!(api_key.name, "Test Key");
        assert_eq!(api_key.is_valid, ValidationStatus::Unknown);
        assert!(api_key.base_url.is_none());

        // 通过 ID 读取
        let found = get_key_by_id(&conn, &api_key.id).unwrap().unwrap();
        assert_eq!(found.id, api_key.id);
        assert_eq!(found.provider, "openai");
    }

    #[test]
    fn test_insert_key_with_base_url() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(
            &conn,
            &enc,
            "custom",
            "Custom",
            "key123",
            Some("https://api.test.com"),
        )
        .unwrap();

        assert_eq!(api_key.base_url, Some("https://api.test.com".to_string()));
    }

    #[test]
    fn test_get_decrypted_key() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(&conn, &enc, "openai", "Test", "sk-secret-key-123", None).unwrap();

        let decrypted = get_decrypted_key(&conn, &enc, &api_key.id).unwrap();
        assert_eq!(decrypted, Some("sk-secret-key-123".to_string()));
    }

    #[test]
    fn test_get_decrypted_key_nonexistent() {
        let (conn, enc) = create_test_conn();

        let result = get_decrypted_key(&conn, &enc, "nonexistent-id").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_list_keys() {
        let (conn, enc) = create_test_conn();

        insert_key(&conn, &enc, "openai", "Key 1", "sk-1", None).unwrap();
        insert_key(&conn, &enc, "anthropic", "Key 2", "sk-ant-2", None).unwrap();
        insert_key(&conn, &enc, "openai", "Key 3", "sk-3", None).unwrap();

        let all = list_keys(&conn).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_keys_by_provider() {
        let (conn, enc) = create_test_conn();

        insert_key(&conn, &enc, "openai", "Key 1", "sk-1", None).unwrap();
        insert_key(&conn, &enc, "anthropic", "Key 2", "sk-ant-2", None).unwrap();
        insert_key(&conn, &enc, "openai", "Key 3", "sk-3", None).unwrap();

        let openai_keys = get_keys_by_provider(&conn, "openai").unwrap();
        assert_eq!(openai_keys.len(), 2);
        assert!(openai_keys.iter().all(|k| k.provider == "openai"));

        let anthropic_keys = get_keys_by_provider(&conn, "anthropic").unwrap();
        assert_eq!(anthropic_keys.len(), 1);
    }

    #[test]
    fn test_delete_key() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(&conn, &enc, "openai", "To Delete", "sk-delete", None).unwrap();
        assert_eq!(list_keys(&conn).unwrap().len(), 1);

        delete_key(&conn, &api_key.id).unwrap();
        assert_eq!(list_keys(&conn).unwrap().len(), 0);
        assert!(get_key_by_id(&conn, &api_key.id).unwrap().is_none());
    }

    #[test]
    fn test_update_validation_status() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(&conn, &enc, "openai", "Test", "sk-test", None).unwrap();
        assert_eq!(api_key.is_valid, ValidationStatus::Unknown);

        update_validation_status(&conn, &api_key.id, &ValidationStatus::Valid).unwrap();

        let updated = get_key_by_id(&conn, &api_key.id).unwrap().unwrap();
        assert_eq!(updated.is_valid, ValidationStatus::Valid);
        assert!(updated.last_validated.is_some());
    }

    #[test]
    fn test_rotate_key() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(&conn, &enc, "openai", "Rotate", "sk-old-key", None).unwrap();

        // 验证旧密钥
        let old_decrypted = get_decrypted_key(&conn, &enc, &api_key.id).unwrap();
        assert_eq!(old_decrypted, Some("sk-old-key".to_string()));

        // 轮转密钥
        rotate_key(&conn, &enc, &api_key.id, "sk-new-key").unwrap();

        // 验证新密钥
        let new_decrypted = get_decrypted_key(&conn, &enc, &api_key.id).unwrap();
        assert_eq!(new_decrypted, Some("sk-new-key".to_string()));

        // 校验状态应为 Unknown
        let updated = get_key_by_id(&conn, &api_key.id).unwrap().unwrap();
        assert_eq!(updated.is_valid, ValidationStatus::Unknown);
        assert!(updated.last_validated.is_none());
    }

    #[test]
    fn test_key_encrypted_in_database() {
        let (conn, enc) = create_test_conn();

        let api_key = insert_key(
            &conn,
            &enc,
            "openai",
            "Encrypted",
            "sk-plaintext-secret",
            None,
        )
        .unwrap();

        // 直接从数据库读取加密值
        let conn_guard = conn.lock().unwrap();
        let stored: String = conn_guard
            .query_row(
                "SELECT key_encrypted FROM api_keys WHERE id = ?1",
                rusqlite::params![api_key.id],
                |row| row.get(0),
            )
            .unwrap();

        // 存储的值不应是明文
        assert_ne!(stored, "sk-plaintext-secret");
    }

    #[test]
    fn test_no_encryption_module() {
        let conn = Database::open_memory().unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let enc: Option<Encryption> = None;

        let api_key = insert_key(&conn, &enc, "openai", "No Enc", "sk-test", None).unwrap();

        // 无加密模块时使用 base64
        let decrypted = get_decrypted_key(&conn, &enc, &api_key.id).unwrap();
        assert_eq!(decrypted, Some("sk-test".to_string()));
    }
}
