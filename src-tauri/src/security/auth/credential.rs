//! Git/第三方凭证管理模块
//!
//! 负责 Git 凭证和第三方服务凭证的数据库 CRUD 操作。
//! 凭证密钥使用 AES-256-GCM 加密存储。
//!
//! # 凭证类型
//! - `UsernamePassword` - 用户名/密码
//! - `SshKey` - SSH Key
//! - `Token` - Token（如 GitHub PAT）

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use super::{CredentialType, GitCredential};
use crate::app::infra::Encryption;

/// 插入 Git 凭证
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块
/// * `pattern` - 仓库匹配模式
/// * `cred` - Git 凭证信息
pub fn insert_credential(
    conn: &Arc<Mutex<Connection>>,
    encryption: &Option<Encryption>,
    pattern: &str,
    cred: &GitCredential,
) -> Result<()> {
    // 加密 secret
    let secret_str = String::from_utf8_lossy(&cred.secret_encrypted).to_string();
    let secret_encrypted = encrypt_secret(encryption, &secret_str)?;

    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "INSERT INTO git_credentials (id, repo_pattern, credential_type, username, secret_encrypted, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            cred.id,
            pattern,
            cred.credential_type.as_str(),
            cred.username,
            secret_encrypted,
            cred.created_at.to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// 根据仓库 URL 查找匹配的 Git 凭证
///
/// 使用 LIKE 模式匹配 repo_pattern（支持 % 通配符）。
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `encryption` - 加密模块
/// * `repo_url` - 仓库 URL
pub fn find_matching_credential(
    conn: &Arc<Mutex<Connection>>,
    _encryption: &Option<Encryption>,
    repo_url: &str,
) -> Result<Option<GitCredential>> {
    let conn_guard = conn.lock().unwrap();
    let mut stmt = conn_guard.prepare(
        "SELECT id, repo_pattern, credential_type, username, secret_encrypted, created_at
         FROM git_credentials ORDER BY created_at DESC",
    )?;

    let credentials: Vec<GitCredential> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .filter_map(
            |(id, pattern, cred_type, username, secret_enc, created_at)| {
                // 检查模式是否匹配
                if !matches_pattern(&pattern, repo_url) {
                    return None;
                }

                let credential_type = CredentialType::from_str(&cred_type).ok()?;
                let created = chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Some(GitCredential {
                    id,
                    repo_pattern: pattern,
                    credential_type,
                    username,
                    secret_encrypted: secret_enc.into_bytes(),
                    created_at: created,
                })
            },
        )
        .collect();

    // 返回第一个匹配的凭证
    Ok(credentials.into_iter().next())
}

/// 列出所有 Git 凭证
pub fn list_credentials(conn: &Arc<Mutex<Connection>>) -> Result<Vec<GitCredential>> {
    let conn_guard = conn.lock().unwrap();
    let mut stmt = conn_guard.prepare(
        "SELECT id, repo_pattern, credential_type, username, secret_encrypted, created_at
         FROM git_credentials ORDER BY created_at DESC",
    )?;

    let credentials = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(
            |(id, pattern, cred_type, username, secret_enc, created_at)| {
                let credential_type =
                    CredentialType::from_str(&cred_type).unwrap_or(CredentialType::Token);
                let created = chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                GitCredential {
                    id,
                    repo_pattern: pattern,
                    credential_type,
                    username,
                    secret_encrypted: secret_enc.into_bytes(),
                    created_at: created,
                }
            },
        )
        .collect();

    Ok(credentials)
}

/// 删除 Git 凭证
pub fn delete_credential(conn: &Arc<Mutex<Connection>>, id: &str) -> Result<()> {
    let conn_guard = conn.lock().unwrap();
    conn_guard.execute(
        "DELETE FROM git_credentials WHERE id = ?1",
        rusqlite::params![id],
    )?;
    Ok(())
}

/// 加密 secret 值
fn encrypt_secret(encryption: &Option<Encryption>, secret: &str) -> Result<String> {
    if let Some(ref enc) = encryption {
        enc.encrypt(secret.as_bytes())
    } else {
        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(secret.as_bytes()))
    }
}

/// 检查仓库 URL 是否匹配给定模式
///
/// 模式支持 % 通配符（SQL LIKE 语法），同时将简单的 glob * 转换为 %。
///
/// # Arguments
/// * `pattern` - 匹配模式（如 "github.com/*"）
/// * `url` - 仓库 URL
///
/// # Returns
/// 是否匹配
fn matches_pattern(pattern: &str, url: &str) -> bool {
    // 将 glob 风格的 * 转换为 SQL LIKE 的 %
    let like_pattern = pattern.replace('*', "%");

    // 简单的 LIKE 匹配实现
    if like_pattern.contains('%') {
        matches_like(&like_pattern, url)
    } else {
        // 精确匹配或前缀匹配
        url == pattern || url.starts_with(pattern)
    }
}

/// 简单的 SQL LIKE 匹配实现
///
/// 支持 %（匹配任意字符序列）通配符。
fn matches_like(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('%').collect();

    if parts.len() == 1 {
        // 没有通配符，精确匹配
        return text == pattern;
    }

    let mut text_pos = 0;
    let text_bytes = text.as_bytes();

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if i == 0 {
            // 第一部分必须匹配开头
            if !text[text_pos..].starts_with(part) {
                return false;
            }
            text_pos += part.len();
        } else if i == parts.len() - 1 && !part.is_empty() {
            // 最后一部分必须匹配结尾
            return text[text_pos..].ends_with(part);
        } else {
            // 中间部分，在剩余文本中查找
            if let Some(pos) = text[text_pos..].find(part) {
                text_pos += pos + part.len();
            } else {
                return false;
            }
        }
    }

    // 如果模式以 % 结尾，任何剩余文本都可以
    if pattern.ends_with('%') || text_pos <= text_bytes.len() {
        true
    } else {
        false
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::db::Database;
use crate::app::infra::Encryption;
    use crate::security::auth::generate_id;

    fn create_test_conn() -> (Arc<Mutex<Connection>>, Option<Encryption>) {
        let conn = Database::open_memory().unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let encryption = Encryption::new(&vec![0u8; 32]).unwrap();
        (conn, Some(encryption))
    }

    fn make_credential(pattern: &str, cred_type: CredentialType) -> GitCredential {
        GitCredential {
            id: generate_id(),
            repo_pattern: pattern.to_string(),
            credential_type: cred_type,
            username: None,
            secret_encrypted: Vec::new(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_list_credentials() {
        let (conn, enc) = create_test_conn();

        let cred1 = make_credential("github.com/*", CredentialType::Token);
        let cred2 = make_credential("gitlab.com/*", CredentialType::SshKey);

        insert_credential(&conn, &enc, &cred1.repo_pattern.clone(), &cred1).unwrap();
        insert_credential(&conn, &enc, &cred2.repo_pattern.clone(), &cred2).unwrap();

        let all = list_credentials(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_find_matching_credential() {
        let (conn, enc) = create_test_conn();

        let cred = make_credential("github.com/*", CredentialType::Token);
        let cred_id = cred.id.clone();
        insert_credential(&conn, &enc, "github.com/*", &cred).unwrap();

        // 匹配 github.com URL
        let found = find_matching_credential(&conn, &enc, "github.com/user/repo.git").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, cred_id);

        // 不匹配 gitlab.com URL
        let not_found = find_matching_credential(&conn, &enc, "gitlab.com/user/repo.git").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_delete_credential() {
        let (conn, enc) = create_test_conn();

        let cred = make_credential("github.com/*", CredentialType::Token);
        let cred_id = cred.id.clone();
        insert_credential(&conn, &enc, "github.com/*", &cred).unwrap();

        assert_eq!(list_credentials(&conn).unwrap().len(), 1);

        delete_credential(&conn, &cred_id).unwrap();
        assert_eq!(list_credentials(&conn).unwrap().len(), 0);
    }

    #[test]
    fn test_credential_with_username() {
        let (conn, enc) = create_test_conn();

        let mut cred = make_credential("github.com/*", CredentialType::UsernamePassword);
        cred.username = Some("testuser".to_string());

        insert_credential(&conn, &enc, "github.com/*", &cred).unwrap();

        let all = list_credentials(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].username, Some("testuser".to_string()));
        assert_eq!(all[0].credential_type, CredentialType::UsernamePassword);
    }

    #[test]
    fn test_matches_pattern() {
        // glob * 匹配
        assert!(matches_pattern("github.com/*", "github.com/user/repo.git"));
        assert!(matches_pattern("github.com/*", "github.com/org/project"));
        assert!(!matches_pattern("github.com/*", "gitlab.com/user/repo"));

        // 精确匹配
        assert!(matches_pattern("github.com", "github.com"));

        // 前缀匹配
        assert!(matches_pattern("github.com/user", "github.com/user/repo"));
    }

    #[test]
    fn test_matches_like() {
        // % 通配符
        assert!(matches_like("github.com/%", "github.com/anything"));
        assert!(matches_like("%github.com%", "api.github.com"));
        assert!(!matches_like("github.com/%", "gitlab.com/repo"));

        // 无通配符
        assert!(matches_like("exact", "exact"));
        assert!(!matches_like("exact", "not-exact"));

        // 开头和结尾通配符
        assert!(matches_like("%example%", "this-is-example-text"));
        assert!(matches_like("start%", "start-of-text"));
        assert!(matches_like("%end", "this-is-the-end"));
    }

    #[test]
    fn test_encrypted_storage() {
        let (conn, enc) = create_test_conn();

        let mut cred = make_credential("github.com/*", CredentialType::Token);
        cred.secret_encrypted = "my-secret-token".as_bytes().to_vec();

        insert_credential(&conn, &enc, "github.com/*", &cred).unwrap();

        // 直接从数据库读取
        let conn_guard = conn.lock().unwrap();
        let stored: String = conn_guard
            .query_row(
                "SELECT secret_encrypted FROM git_credentials WHERE id = ?1",
                rusqlite::params![cred.id],
                |row| row.get(0),
            )
            .unwrap();

        // 存储的值不应是明文
        assert_ne!(stored, "my-secret-token");
    }
}
