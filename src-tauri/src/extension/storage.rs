//! 扩展域存储 facade。
//!
//! 架构裁决（设计 35 C0-5 / 34 §2.5）：扩展存储从"统一 SQLite kv 前缀隔离"改为
//! "每个扩展独立目录"。本 facade 是 extension 域的存储收口，底层介质是文件系统目录，
//! **不依赖**宿主 `foundation::storage::Storage` 上帝对象：
//! - global：`{base_dir}/{extension_id}/storage/global/{key}.json`
//! - worktree：`{base_dir}/{extension_id}/storage/worktree/{worktree_hash}/{key}.json`
//! - ephemeral：进程内 `HashMap`（本 facade 自持；IPC 层受 Tauri State 装配限制仍走
//!   `ui::extension_storage::ExtensionEphemeralStorage`，见该模块说明）。
//!
//! 目录即生命周期：卸载扩展时删除 `{base_dir}/{extension_id}/storage` 即干净。
//! key → 文件名采用 percent 编码（保留可读性同时规避 Windows 文件名非法字符与
//! 路径分隔符）；TTL 用 `{"value": ..., "expires_at": ...}` 单文件 JSON 包装，
//! 无额外元数据文件。除 ephemeral 的 `Mutex<HashMap>` 外无共享可变状态，
//! 每次读/写直接操作文件系统。

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extension::loader::is_valid_extension_id;
use crate::extension::models::StorageScope;

/// 单值大小上限（34 §2.5 迁移）。
const MAX_STORAGE_VALUE_BYTES: usize = 256 * 1024;
/// 单值 JSON 深度上限（34 §2.5 迁移）。
const MAX_STORAGE_VALUE_DEPTH: usize = 16;
/// TTL 上限：30 天（34 §2.5 迁移）。
pub(crate) const MAX_STORAGE_TTL_MS: u64 = 1000 * 60 * 60 * 24 * 30;

/// 落盘值包装：有 TTL 时 `expires_at` 为 unix 毫秒，否则为 `null`。
///
/// 统一包装（含无 TTL 值）保证读取侧无歧义：外层对象必然是本结构。
#[derive(Debug, Serialize, Deserialize)]
struct StoredValue {
    value: Value,
    expires_at: Option<u64>,
}

/// 扩展域存储 facade。
///
/// 根目录由构造函数注入（装配时为 `<app_data>/extensions`）；内部无共享可变状态，
/// `Clone`（Arc 包装）由调用方按需进行。
#[derive(Debug)]
pub struct ExtensionStorage {
    /// 扩展存储根目录（`{base_dir}/{extension_id}/storage/...`）。
    base_dir: PathBuf,
    /// 进程内 ephemeral 存储。
    ephemeral: Mutex<HashMap<String, Value>>,
}

impl ExtensionStorage {
    /// 以存储根目录创建 facade。
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            ephemeral: Mutex::new(HashMap::new()),
        }
    }

    /// 读取存储值。key 缺失 / 已过期 → `None`；其余 IO / 解析错误 → `Err`。
    pub fn get(
        &self,
        extension_id: &str,
        scope: &StorageScope,
        worktree: Option<&str>,
        key: &str,
    ) -> Result<Option<Value>, String> {
        validate_key(key)?;
        if *scope == StorageScope::Ephemeral {
            return self.ephemeral_get(extension_id, key);
        }
        let path = self.storage_path(extension_id, scope, worktree, key)?;
        match fs::read(&path) {
            Ok(bytes) => {
                let stored: StoredValue = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("Failed to parse '{}': {error}", path.display()))?;
                let now = now_millis();
                if stored.expires_at.is_some_and(|expires_at| expires_at <= now) {
                    // 过期即清理：读取时删除并视为不存在。
                    let _ = fs::remove_file(&path);
                    return Ok(None);
                }
                Ok(Some(stored.value))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("Failed to read '{}': {error}", path.display())),
        }
    }

    /// 写入存储值。`ttl_ms` 需 ≤ `MAX_STORAGE_TTL_MS`；值需通过 `validate_storage_value`。
    pub fn set(
        &self,
        extension_id: &str,
        scope: &StorageScope,
        worktree: Option<&str>,
        key: &str,
        value: &Value,
        ttl_ms: Option<u64>,
    ) -> Result<(), String> {
        validate_key(key)?;
        validate_storage_value(value)?;
        if ttl_ms.is_some_and(|ttl| ttl > MAX_STORAGE_TTL_MS) {
            return Err(format!("Extension storage ttl cannot exceed {MAX_STORAGE_TTL_MS} ms"));
        }
        if *scope == StorageScope::Ephemeral {
            return self.ephemeral_set(extension_id, key, value.clone());
        }
        let path = self.storage_path(extension_id, scope, worktree, key)?;
        let stored = StoredValue {
            value: value.clone(),
            expires_at: ttl_ms.map(|ttl| now_millis().saturating_add(ttl)),
        };
        let bytes = serde_json::to_vec(&stored)
            .map_err(|error| format!("Failed to serialize storage value: {error}"))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create '{}': {error}", parent.display()))?;
        }
        fs::write(&path, bytes)
            .map_err(|error| format!("Failed to write '{}': {error}", path.display()))
    }

    /// 删除存储值。key 不存在时幂等返回 `Ok`。
    pub fn delete(
        &self,
        extension_id: &str,
        scope: &StorageScope,
        worktree: Option<&str>,
        key: &str,
    ) -> Result<(), String> {
        validate_key(key)?;
        if *scope == StorageScope::Ephemeral {
            return self.ephemeral_delete(extension_id, key);
        }
        let path = self.storage_path(extension_id, scope, worktree, key)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Failed to delete '{}': {error}", path.display())),
        }
    }

    /// 清空该扩展在给定 scope 下的全部存储。
    ///
    /// - global：删除 `storage/global/` 目录；
    /// - worktree：删除 `storage/worktree/{hash}/` 目录；
    /// - ephemeral：按 `extension:{id}:ephemeral:` 前缀清理内存。
    pub fn clear(
        &self,
        extension_id: &str,
        scope: &StorageScope,
        worktree: Option<&str>,
    ) -> Result<(), String> {
        if *scope == StorageScope::Ephemeral {
            return self.ephemeral_clear(extension_id);
        }
        let root = self.extension_storage_dir(extension_id)?;
        let dir = match scope {
            StorageScope::Global => root.join("global"),
            StorageScope::Worktree => {
                let hash = worktree_hash(worktree)?;
                root.join("worktree").join(hash)
            }
            StorageScope::Ephemeral => unreachable!("handled above"),
        };
        remove_dir_all_if_exists(&dir)
    }

    /// 清空该扩展全部存储：ephemeral + 删除 `{base_dir}/{extension_id}/storage` 目录。
    ///
    /// 卸载扩展时调用（目录即生命周期）；目录不存在时幂等返回 `Ok`。
    pub fn clear_extension(&self, extension_id: &str) -> Result<(), String> {
        self.ephemeral_clear(extension_id)?;
        let dir = self.extension_storage_dir(extension_id)?;
        remove_dir_all_if_exists(&dir)
    }

    /// 单 key 落盘路径。
    fn storage_path(
        &self,
        extension_id: &str,
        scope: &StorageScope,
        worktree: Option<&str>,
        key: &str,
    ) -> Result<PathBuf, String> {
        validate_key(key)?;
        let root = self.extension_storage_dir(extension_id)?;
        let file_name = format!("{}.json", encode_key_file_name(key));
        match scope {
            StorageScope::Global => Ok(root.join("global").join(file_name)),
            StorageScope::Worktree => {
                let hash = worktree_hash(worktree)?;
                Ok(root.join("worktree").join(hash).join(file_name))
            }
            StorageScope::Ephemeral => Err("ephemeral scope has no file path".to_string()),
        }
    }

    /// 该扩展的存储目录 `{base_dir}/{extension_id}/storage`。
    fn extension_storage_dir(&self, extension_id: &str) -> Result<PathBuf, String> {
        if !is_valid_extension_id(extension_id) {
            return Err(format!("Invalid extension id '{extension_id}'"));
        }
        Ok(self.base_dir.join(extension_id).join("storage"))
    }

    fn ephemeral_key(extension_id: &str, key: &str) -> String {
        format!("extension:{extension_id}:ephemeral:{key}")
    }

    fn ephemeral_prefix(extension_id: &str) -> String {
        format!("extension:{extension_id}:ephemeral:")
    }

    fn ephemeral_get(&self, extension_id: &str, key: &str) -> Result<Option<Value>, String> {
        let map = self
            .ephemeral
            .lock()
            .map_err(|_| "extension storage mutex poisoned".to_string())?;
        Ok(map.get(&Self::ephemeral_key(extension_id, key)).cloned())
    }

    fn ephemeral_set(&self, extension_id: &str, key: &str, value: Value) -> Result<(), String> {
        let mut map = self
            .ephemeral
            .lock()
            .map_err(|_| "extension storage mutex poisoned".to_string())?;
        map.insert(Self::ephemeral_key(extension_id, key), value);
        Ok(())
    }

    fn ephemeral_delete(&self, extension_id: &str, key: &str) -> Result<(), String> {
        let mut map = self
            .ephemeral
            .lock()
            .map_err(|_| "extension storage mutex poisoned".to_string())?;
        map.remove(&Self::ephemeral_key(extension_id, key));
        Ok(())
    }

    fn ephemeral_clear(&self, extension_id: &str) -> Result<(), String> {
        let prefix = Self::ephemeral_prefix(extension_id);
        let mut map = self
            .ephemeral
            .lock()
            .map_err(|_| "extension storage mutex poisoned".to_string())?;
        map.retain(|key, _| !key.starts_with(&prefix));
        Ok(())
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn remove_dir_all_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to remove '{}': {error}", path.display())),
    }
}

/// key 校验（34 §2.5 迁移）：1..512 字节，禁 `..`/`\`/前导 `/`/控制字符。
///
/// 校验通过后 key 到文件名仍需 percent 编码（见 `encode_key_file_name`），以规避
/// Windows 文件名非法字符（`:` `?` `*` 等）与路径分隔符（`/`）。
pub(crate) fn validate_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() || key.len() > 512 {
        return Err("Extension storage key must be 1..512 bytes".to_string());
    }
    if key.contains("..") || key.contains('\\') || key.starts_with('/') || key.chars().any(char::is_control) {
        return Err("Extension storage key contains unsafe path-like characters".to_string());
    }
    Ok(())
}

/// 值校验（34 §2.5 迁移）：深度 ≤16、单值 ≤256KB、对象 key ≤512 且禁控制字符。
pub(crate) fn validate_storage_value(value: &Value) -> Result<(), String> {
    validate_json_value(value, 0)
}

fn validate_json_value(value: &Value, depth: usize) -> Result<(), String> {
    if depth > MAX_STORAGE_VALUE_DEPTH {
        return Err(format!(
            "Extension storage value exceeds max JSON depth of {MAX_STORAGE_VALUE_DEPTH}"
        ));
    }
    match value {
        Value::Array(items) => {
            for item in items {
                validate_json_value(item, depth + 1)?;
            }
        }
        Value::Object(entries) => {
            for (key, item) in entries {
                if key.len() > 512 || key.chars().any(char::is_control) {
                    return Err("Extension storage object contains an invalid key".to_string());
                }
                validate_json_value(item, depth + 1)?;
            }
        }
        _ => {}
    }
    let encoded = serde_json::to_vec(value).map_err(|error| format!("Invalid JSON storage value: {error}"))?;
    if encoded.len() > MAX_STORAGE_VALUE_BYTES {
        return Err(format!("Extension storage value exceeds {MAX_STORAGE_VALUE_BYTES} bytes"));
    }
    Ok(())
}

/// worktree 标识 → 16 位 hex hash（用作目录名）。worktree scope 必须携带 worktree。
pub(crate) fn worktree_hash(worktree: Option<&str>) -> Result<String, String> {
    let worktree =
        worktree.ok_or_else(|| "worktree storage scope requires a worktree argument".to_string())?;
    let mut hasher = DefaultHasher::new();
    worktree.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// scope 名称（目录名 / IPC 错误消息共用）。
pub(crate) fn scope_name(scope: &StorageScope) -> &'static str {
    match scope {
        StorageScope::Global => "global",
        StorageScope::Worktree => "worktree",
        StorageScope::Ephemeral => "ephemeral",
    }
}

/// key → 安全文件名：保留 `[A-Za-z0-9._-]`，其余字节 percent 编码（`%XX`）。
///
/// 规避 Windows 文件名非法字符（`:` `?` `*` 等）与路径分隔符（`/`），保证 key
/// 到文件名单射、可逆、不产生子目录。
fn encode_key_file_name(key: &str) -> String {
    let mut encoded = String::with_capacity(key.len());
    for &byte in key.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_storage() -> (ExtensionStorage, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = ExtensionStorage::new(dir.path().to_path_buf());
        (storage, dir)
    }

    #[test]
    fn global_set_get_roundtrip() {
        let (storage, _dir) = test_storage();
        let scope = StorageScope::Global;
        storage
            .set("ext.alpha", &scope, None, "theme", &json!({"mode": "dark", "size": 2}), None)
            .unwrap();
        assert_eq!(
            storage.get("ext.alpha", &scope, None, "theme").unwrap(),
            Some(json!({"mode": "dark", "size": 2}))
        );
        // 覆盖写
        storage
            .set("ext.alpha", &scope, None, "theme", &json!("light"), None)
            .unwrap();
        assert_eq!(storage.get("ext.alpha", &scope, None, "theme").unwrap(), Some(json!("light")));
    }

    #[test]
    fn missing_key_returns_none_and_delete_is_idempotent() {
        let (storage, _dir) = test_storage();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "nope").unwrap(), None);
        assert!(storage.delete("ext.alpha", &StorageScope::Global, None, "nope").is_ok());
    }

    #[test]
    fn worktree_scope_is_isolated_by_worktree_hash() {
        let (storage, _dir) = test_storage();
        let scope = StorageScope::Worktree;
        storage.set("ext.alpha", &scope, Some("/repo/a"), "key", &json!(1), None).unwrap();
        storage.set("ext.alpha", &scope, Some("/repo/b"), "key", &json!(2), None).unwrap();
        assert_eq!(storage.get("ext.alpha", &scope, Some("/repo/a"), "key").unwrap(), Some(json!(1)));
        assert_eq!(storage.get("ext.alpha", &scope, Some("/repo/b"), "key").unwrap(), Some(json!(2)));
    }

    #[test]
    fn worktree_scope_requires_worktree_argument() {
        let (storage, _dir) = test_storage();
        assert!(storage.set("ext.alpha", &StorageScope::Worktree, None, "key", &json!(1), None).is_err());
        assert!(storage.clear("ext.alpha", &StorageScope::Worktree, None).is_err());
    }

    #[test]
    fn delete_removes_key() {
        let (storage, _dir) = test_storage();
        storage.set("ext.alpha", &StorageScope::Global, None, "a", &json!(1), None).unwrap();
        storage.delete("ext.alpha", &StorageScope::Global, None, "a").unwrap();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "a").unwrap(), None);
    }

    #[test]
    fn clear_removes_only_target_scope() {
        let (storage, _dir) = test_storage();
        storage.set("ext.alpha", &StorageScope::Global, None, "g", &json!(1), None).unwrap();
        storage.set("ext.alpha", &StorageScope::Worktree, Some("/w"), "w", &json!(2), None).unwrap();
        storage.clear("ext.alpha", &StorageScope::Worktree, Some("/w")).unwrap();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Worktree, Some("/w"), "w").unwrap(), None);
        // global 不受影响
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "g").unwrap(), Some(json!(1)));
        // 其他 worktree hash 不受影响
        storage.set("ext.alpha", &StorageScope::Worktree, Some("/w2"), "w", &json!(3), None).unwrap();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Worktree, Some("/w2"), "w").unwrap(), Some(json!(3)));
    }

    #[test]
    fn clear_extension_removes_everything_and_keeps_others() {
        let (storage, _dir) = test_storage();
        storage.set("ext.alpha", &StorageScope::Global, None, "g", &json!(1), None).unwrap();
        storage.set("ext.alpha", &StorageScope::Worktree, Some("/w"), "w", &json!(2), None).unwrap();
        storage.set("ext.beta", &StorageScope::Global, None, "g", &json!(3), None).unwrap();
        storage.clear_extension("ext.alpha").unwrap();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "g").unwrap(), None);
        assert_eq!(storage.get("ext.alpha", &StorageScope::Worktree, Some("/w"), "w").unwrap(), None);
        // 其他扩展不受影响
        assert_eq!(storage.get("ext.beta", &StorageScope::Global, None, "g").unwrap(), Some(json!(3)));
        // 幂等
        assert!(storage.clear_extension("ext.alpha").is_ok());
    }

    #[test]
    fn ttl_expiry_removes_file_and_returns_none() {
        let (storage, dir) = test_storage();
        storage
            .set("ext.alpha", &StorageScope::Global, None, "k", &json!(1), Some(60_000))
            .unwrap();
        // 未过期读正常返回
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "k").unwrap(), Some(json!(1)));
        // 手工把 expires_at 改为过去，触发过期清理
        let file = dir
            .path()
            .join("ext.alpha")
            .join("storage")
            .join("global")
            .join(format!("{}.json", encode_key_file_name("k")));
        let expired = StoredValue {
            value: json!(1),
            expires_at: Some(1),
        };
        fs::write(&file, serde_json::to_vec(&expired).unwrap()).unwrap();
        assert_eq!(storage.get("ext.alpha", &StorageScope::Global, None, "k").unwrap(), None);
        assert!(!file.exists(), "expired file should be removed on read");
    }

    #[test]
    fn ttl_upper_bound_rejected() {
        let (storage, _dir) = test_storage();
        assert!(storage
            .set("ext.alpha", &StorageScope::Global, None, "k", &json!(1), Some(MAX_STORAGE_TTL_MS + 1))
            .is_err());
    }

    #[test]
    fn unsafe_keys_rejected() {
        let (storage, _dir) = test_storage();
        for bad in ["..", "a/../b", "a\\b", "/lead", "ctrl\u{0007}"] {
            assert!(
                storage.set("ext.alpha", &StorageScope::Global, None, bad, &json!(1), None).is_err(),
                "key should be rejected: {bad:?}"
            );
        }
    }

    #[test]
    fn keys_with_special_characters_roundtrip_on_windows() {
        // `:` `?` `/` 等在 Windows 上是文件名非法字符；percent 编码后应可正常读写。
        let (storage, _dir) = test_storage();
        for key in ["a/b", "a:b", "a?b", "space key", "中文键"] {
            storage.set("ext.alpha", &StorageScope::Global, None, key, &json!(key.to_string()), None).unwrap();
            assert_eq!(
                storage.get("ext.alpha", &StorageScope::Global, None, key).unwrap(),
                Some(json!(key.to_string())),
                "key roundtrip: {key}"
            );
        }
    }

    #[test]
    fn invalid_extension_id_rejected() {
        let (storage, _dir) = test_storage();
        assert!(storage.set("../evil", &StorageScope::Global, None, "k", &json!(1), None).is_err());
    }

    #[test]
    fn value_validation_depth_and_size() {
        let (storage, _dir) = test_storage();
        let deep = (0..20).fold(json!(1), |acc, _| json!([acc]));
        assert!(storage.set("ext.alpha", &StorageScope::Global, None, "deep", &deep, None).is_err());
        let big = json!({ "x": "x".repeat(300 * 1024) });
        assert!(storage.set("ext.alpha", &StorageScope::Global, None, "big", &big, None).is_err());
        let bad_object_key = serde_json::from_str::<Value>(r#"{"a\u0000b": 1}"#).unwrap();
        assert!(storage.set("ext.alpha", &StorageScope::Global, None, "bad", &bad_object_key, None).is_err());
    }

    #[test]
    fn ephemeral_scope_uses_in_memory_map() {
        let (storage, _dir) = test_storage();
        let scope = StorageScope::Ephemeral;
        storage.set("ext.alpha", &scope, None, "k", &json!(1), None).unwrap();
        assert_eq!(storage.get("ext.alpha", &scope, None, "k").unwrap(), Some(json!(1)));
        storage.delete("ext.alpha", &scope, None, "k").unwrap();
        assert_eq!(storage.get("ext.alpha", &scope, None, "k").unwrap(), None);
        storage.set("ext.alpha", &scope, None, "k", &json!(1), None).unwrap();
        storage.set("ext.beta", &scope, None, "k", &json!(2), None).unwrap();
        storage.clear("ext.alpha", &scope, None).unwrap();
        assert_eq!(storage.get("ext.alpha", &scope, None, "k").unwrap(), None);
        assert_eq!(storage.get("ext.beta", &scope, None, "k").unwrap(), Some(json!(2)));
    }

    #[test]
    fn on_disk_layout_matches_design() {
        let (storage, dir) = test_storage();
        storage.set("ext.alpha", &StorageScope::Global, None, "theme", &json!({"mode": "dark"}), None).unwrap();
        storage.set("ext.alpha", &StorageScope::Worktree, Some("/repo/a"), "w", &json!(1), None).unwrap();
        let base = dir.path().join("ext.alpha").join("storage");
        // global 文件直接落在 `storage/global/{key}.json`
        assert!(base.join("global").join("theme.json").is_file());
        // worktree 落在 `storage/worktree/{worktree_hash}/{key}.json`
        let hash = worktree_hash(Some("/repo/a")).unwrap();
        assert!(base.join("worktree").join(hash).join("w.json").is_file());
    }

    #[test]
    fn scope_name_matches_directory_names() {
        assert_eq!(scope_name(&StorageScope::Global), "global");
        assert_eq!(scope_name(&StorageScope::Worktree), "worktree");
        assert_eq!(scope_name(&StorageScope::Ephemeral), "ephemeral");
    }
}
