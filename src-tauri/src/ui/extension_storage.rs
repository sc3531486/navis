//! 扩展 KV 存储 IPC。

//!

//! 这是 34-extension-ui-open-architecture 的生产出口：扩展 UI 只能访问自己

//! manifest.contributes.storage 声明过的 scope，所有持久化 key 都收口到

//! `extension:{id}:{scope}:...` 命名空间，避免跨扩展污染。未声明 storage 时

//! fail-closed。

//!

//! 持久化介质（架构裁决 35 C0-5 / 34 §2.5）：global / worktree 走 extension 域

//! facade `storage::ExtensionStorage`（源文件 `extension/storage.rs`，文件系统目录、

//! 每扩展独立目录），不再绑定宿主 `foundation::storage::Storage` 上帝对象。ephemeral

//! 受 Tauri State 装配限制（`app::mod.rs` 当前 manage 的是 `ExtensionEphemeralStorage`），

//! 仍走进程内 `HashMap`。

//!

//! 兼容性约束：为不破坏 `extension_bridge.rs` 的直接调用签名（bridge 仍按

//! `(store, mcp, storage, ephemeral, req)` 传参）以及 `app::mod.rs` 的 manage 装配，

//! 命令保留 `storage: State<Arc<Storage>>` 参数（内部已不再使用）。facade 经

//! `#[path]` 声明为本模块子模块，避免改动 `extension/mod.rs`；facade 本身无共享

//! 可变状态（ephemeral 由 `ephemeral` State 承载），命令按扩展安装目录的父目录

//! 现场构造。

use crate::extension::types::MCP;

use std::collections::HashMap;

use std::path::PathBuf;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use serde_json::Value;

use tauri::State;

use crate::extension::models::StorageScope;

use crate::extension::{ExtensionState, ExtensionStatus, ExtensionStore};

use crate::app::infra::Storage;

use crate::security::sandbox::permission::{CheckResult, OperationRequest, OperationType};

use crate::security::sandbox::Sandbox;

// use [REMOVED: MCP reference]

#[path = "../extension/storage.rs"]
pub(crate) mod storage;

pub(crate) use storage::{
    scope_name, validate_key, validate_storage_value, worktree_hash, ExtensionStorage,
    MAX_STORAGE_TTL_MS,
};

/// 进程内 ephemeral 存储（`StorageScope::Ephemeral`）。

///

/// 生命周期禁用/卸载时由 `clear_extension_ephemeral` 清理；持久化（global / worktree）

/// 已迁移到 `storage::ExtensionStorage` 文件目录，本类型仅承载内存态。

#[derive(Debug, Default)]

pub struct ExtensionEphemeralStorage {
    values: Mutex<HashMap<String, Value>>,
}

impl ExtensionEphemeralStorage {
    fn get(&self, key: &str) -> Result<Option<Value>, String> {
        let values = self
            .values
            .lock()
            .map_err(|_| "extension ephemeral storage mutex poisoned".to_string())?;

        Ok(values.get(key).cloned())
    }

    fn set(&self, key: String, value: Value) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "extension ephemeral storage mutex poisoned".to_string())?;

        values.insert(key, value);

        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "extension ephemeral storage mutex poisoned".to_string())?;

        values.remove(key);

        Ok(())
    }

    fn clear_prefix(&self, prefix: &str) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "extension ephemeral storage mutex poisoned".to_string())?;

        values.retain(|key, _| !key.starts_with(prefix));

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ExtensionStorageRequest {
    pub extension_id: String,

    #[serde(default = "default_storage_scope")]
    pub scope: String,

    pub key: String,

    #[serde(default)]
    pub value: Option<Value>,

    #[serde(default)]
    pub worktree: Option<String>,

    #[serde(default)]
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ExtensionStorageClearRequest {
    pub extension_id: String,

    #[serde(default = "default_storage_scope")]
    pub scope: String,

    #[serde(default)]
    pub worktree: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ExtensionStorageValueResponse {
    pub value: Option<Value>,
}

fn default_storage_scope() -> String {
    "global".to_string()
}

fn parse_scope(scope: &str) -> Result<StorageScope, String> {
    match scope {
        "global" => Ok(StorageScope::Global),

        "worktree" => Ok(StorageScope::Worktree),

        "ephemeral" => Ok(StorageScope::Ephemeral),

        other => Err(format!("Unsupported extension storage scope '{other}'")),
    }
}

/// manifest 声明校验（第一道防线）：扩展已安装、已启用、且声明了该 scope。

///

/// 返回 `ExtensionState` 供 facade 根目录推导（安装目录的父目录即 `<app_data>/extensions`）。

fn ensure_storage_allowed(
    store: &ExtensionStore,

    extension_id: &str,

    scope: &StorageScope,
) -> Result<ExtensionState, String> {
    let state = store
        .get(extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;

    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }

    let Some(declared) = &state.manifest.contributes.storage else {
        return Err(format!(
            "Extension '{extension_id}' did not declare storage scopes"
        ));
    };

    if !declared.scopes.iter().any(|item| item == scope) {
        return Err(format!(
            "Extension '{extension_id}' did not declare '{}' storage scope",
            scope_name(scope)
        ));
    }

    Ok(state)
}

/// 校验 Sandbox 门禁结果；扩展存储写操作不弹确认，需要确认或拒绝一律 fail-closed。

fn require_allowed(result: &CheckResult, operation: &str) -> Result<(), String> {
    if result.allowed && !result.require_confirm {
        Ok(())
    } else {
        let reason = result
            .reason
            .clone()
            .or_else(|| result.confirm_message.clone())
            .unwrap_or_else(|| "sandbox denied".to_string());

        Err(format!(
            "Extension storage '{operation}' denied by sandbox: {reason}"
        ))
    }
}

/// 构造 `OperationRequest{actor:"extension:{id}"}` 过 Sandbox 门禁。

///

/// manifest `ensure_storage_allowed` 是第一道校验（声明检查），这里是纵深防御：

/// 写操作走审批模式/访问控制门禁，`sandbox.check` 内部同时写入 kernel 审计。

fn sandbox_storage_check(
    sandbox: &Sandbox,

    extension_id: &str,

    operation: OperationType,

    target: String,
) -> Result<(), String> {
    let request = OperationRequest::new(operation, target, format!("extension:{extension_id}"));

    let result = sandbox
        .check(&request)
        .map_err(|error| format!("Sandbox check failed: {error}"))?;

    require_allowed(&result, &request.operation.to_string())
}

/// 清理扩展的全部 ephemeral 存储（`extension:{extension_id}:` 前缀）。

///

/// 生命周期禁用/卸载时调用。当前由 UI 命令接线（`ui_set_extension_enabled` /

/// `ui_uninstall_extension`）；`ExtensionLifecycle` 内部暂无法访问该 Tauri State，

/// 其余禁用路径（enable 回滚、Error 重试清理）的清理缺口见任务报告。

pub fn clear_extension_ephemeral(
    ephemeral: &ExtensionEphemeralStorage,
    extension_id: &str,
) -> Result<(), String> {
    let prefix = format!("extension:{extension_id}:");

    ephemeral.clear_prefix(&prefix)
}

/// Sandbox 门禁目标命名空间 key（`extension:{id}:{scope}:...`）。

///

/// 仅用于构造 `OperationRequest` 的 target；实际落盘 key 由 facade 内部按

/// 文件目录布局处理。

fn storage_key(
    extension_id: &str,
    scope: &StorageScope,
    worktree: Option<&str>,
    key: &str,
) -> Result<String, String> {
    validate_key(key)?;

    match scope {
        StorageScope::Global => Ok(format!("extension:{extension_id}:global:{key}")),

        StorageScope::Worktree => Ok(format!(
            "extension:{extension_id}:worktree:{}:{key}",
            worktree_hash(worktree)?
        )),

        StorageScope::Ephemeral => Ok(format!("extension:{extension_id}:ephemeral:{key}")),
    }
}

/// Sandbox 门禁目标命名空间前缀（clear 用）。

fn storage_prefix(
    extension_id: &str,
    scope: &StorageScope,
    worktree: Option<&str>,
) -> Result<String, String> {
    match scope {
        StorageScope::Global => Ok(format!("extension:{extension_id}:global:")),

        StorageScope::Worktree => Ok(format!(
            "extension:{extension_id}:worktree:{}:",
            worktree_hash(worktree)?
        )),

        StorageScope::Ephemeral => Ok(format!("extension:{extension_id}:ephemeral:")),
    }
}

/// 由已安装扩展状态构造文件存储 facade。

///

/// base_dir 取扩展安装目录的父目录（装配时为 `<app_data>/extensions`），对应

/// facade 的 `{base_dir}/{extension_id}/storage/...` 布局，即扩展存储落在其

/// 自身安装目录下的 `storage/` 子目录（目录即生命周期，卸载删目录即干净）。

fn facade_for_state(state: &ExtensionState) -> Result<ExtensionStorage, String> {
    let base_dir = state
        .install_path
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!(
                "Extension '{}' install path has no parent directory",
                state.id
            )
        })?;

    Ok(ExtensionStorage::new(base_dir))
}

#[tauri::command]

pub fn ui_extension_storage_get(
    extension_store: State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,

    _storage: State<'_, Arc<Storage>>,

    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,

    request: ExtensionStorageRequest,
) -> Result<ExtensionStorageValueResponse, String> {
    let scope = parse_scope(&request.scope)?;

    let state = ensure_storage_allowed(
        extension_store.inner().as_ref(),
        &request.extension_id,
        &scope,
    )?;

    let key = storage_key(
        &request.extension_id,
        &scope,
        request.worktree.as_deref(),
        &request.key,
    )?;

    // 只读访问：轻量 Sandbox FileRead 门禁 + 审计（manifest 声明校验是第一道防线）

    sandbox_storage_check(
        mcp.sandbox(),
        &request.extension_id,
        OperationType::FileRead,
        key.clone(),
    )?;

    let value = if scope == StorageScope::Ephemeral {
        ephemeral.get(&key)?
    } else {
        let facade = facade_for_state(&state)?;

        facade.get(
            &request.extension_id,
            &scope,
            request.worktree.as_deref(),
            &request.key,
        )?
    };

    Ok(ExtensionStorageValueResponse { value })
}

#[tauri::command]

pub fn ui_extension_storage_set(
    extension_store: State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,

    _storage: State<'_, Arc<Storage>>,

    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,

    request: ExtensionStorageRequest,
) -> Result<(), String> {
    let scope = parse_scope(&request.scope)?;

    let state = ensure_storage_allowed(
        extension_store.inner().as_ref(),
        &request.extension_id,
        &scope,
    )?;

    let key = storage_key(
        &request.extension_id,
        &scope,
        request.worktree.as_deref(),
        &request.key,
    )?;

    // 写操作：Sandbox FileWrite 门禁（actor=extension:{id}），拒绝/需确认一律 fail-closed

    sandbox_storage_check(
        mcp.sandbox(),
        &request.extension_id,
        OperationType::FileWrite,
        key.clone(),
    )?;

    let value = request
        .value
        .ok_or_else(|| "storage.set requires value".to_string())?;

    // TTL 上限校验（所有 scope 一致；facade 对文件 scope 内部会再次校验）

    if request.ttl_ms.is_some_and(|ttl| ttl > MAX_STORAGE_TTL_MS) {
        return Err(format!(
            "Extension storage ttl cannot exceed {MAX_STORAGE_TTL_MS} ms"
        ));
    }

    if scope == StorageScope::Ephemeral {
        validate_storage_value(&value)?;

        ephemeral.set(key, value)
    } else {
        let facade = facade_for_state(&state)?;

        facade.set(
            &request.extension_id,
            &scope,
            request.worktree.as_deref(),
            &request.key,
            &value,
            request.ttl_ms,
        )
    }
}

#[tauri::command]

pub fn ui_extension_storage_delete(
    extension_store: State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,

    _storage: State<'_, Arc<Storage>>,

    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,

    request: ExtensionStorageRequest,
) -> Result<(), String> {
    let scope = parse_scope(&request.scope)?;

    let state = ensure_storage_allowed(
        extension_store.inner().as_ref(),
        &request.extension_id,
        &scope,
    )?;

    let key = storage_key(
        &request.extension_id,
        &scope,
        request.worktree.as_deref(),
        &request.key,
    )?;

    // 删除操作：Sandbox FileDelete 门禁（actor=extension:{id}），拒绝/需确认一律 fail-closed

    sandbox_storage_check(
        mcp.sandbox(),
        &request.extension_id,
        OperationType::FileDelete,
        key.clone(),
    )?;

    if scope == StorageScope::Ephemeral {
        ephemeral.delete(&key)
    } else {
        let facade = facade_for_state(&state)?;

        facade.delete(
            &request.extension_id,
            &scope,
            request.worktree.as_deref(),
            &request.key,
        )
    }
}

#[tauri::command]

pub fn ui_extension_storage_clear(
    extension_store: State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,

    _storage: State<'_, Arc<Storage>>,

    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,

    request: ExtensionStorageClearRequest,
) -> Result<(), String> {
    let scope = parse_scope(&request.scope)?;

    let state = ensure_storage_allowed(
        extension_store.inner().as_ref(),
        &request.extension_id,
        &scope,
    )?;

    let prefix = storage_prefix(&request.extension_id, &scope, request.worktree.as_deref())?;

    // 清空操作：Sandbox FileDelete 门禁（actor=extension:{id}），拒绝/需确认一律 fail-closed

    sandbox_storage_check(
        mcp.sandbox(),
        &request.extension_id,
        OperationType::FileDelete,
        prefix.clone(),
    )?;

    if scope == StorageScope::Ephemeral {
        ephemeral.clear_prefix(&prefix)
    } else {
        let facade = facade_for_state(&state)?;

        facade.clear(&request.extension_id, &scope, request.worktree.as_deref())
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    use crate::kernel::InMemoryEventBus;

    use serde_json::json;

    use std::sync::OnceLock;

    use tokio::runtime::Runtime;

    fn test_sandbox() -> Sandbox {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();

        let runtime = RUNTIME.get_or_init(|| Runtime::new().expect("test tokio runtime"));

        Sandbox::new(Arc::new(InMemoryEventBus::new(
            1000,
            runtime.handle().clone(),
        )))
    }

    #[test]

    fn storage_write_fails_closed_when_sandbox_requires_confirm() {
        // 默认 Suggest 模式下 FileWrite/FileDelete 需要确认，扩展写操作 fail-closed。

        let sandbox = test_sandbox();

        assert!(sandbox_storage_check(
            &sandbox,
            "ext.alpha",
            OperationType::FileWrite,
            "extension:ext.alpha:global:key".into(),
        )
        .is_err());

        assert!(sandbox_storage_check(
            &sandbox,
            "ext.alpha",
            OperationType::FileDelete,
            "extension:ext.alpha:global:key".into(),
        )
        .is_err());
    }

    #[test]

    fn storage_read_passes_sandbox_in_default_mode() {
        // 默认 Suggest 模式下 FileRead 放行，只读访问保持可用。

        let sandbox = test_sandbox();

        assert!(sandbox_storage_check(
            &sandbox,
            "ext.alpha",
            OperationType::FileRead,
            "extension:ext.alpha:global:key".into(),
        )
        .is_ok());
    }

    #[test]

    fn storage_write_passes_sandbox_in_full_auto_mode() {
        let sandbox = test_sandbox();

        sandbox
            .set_approval_mode(crate::security::sandbox::ApprovalMode::FullAuto)
            .unwrap();

        assert!(sandbox_storage_check(
            &sandbox,
            "ext.alpha",
            OperationType::FileWrite,
            "extension:ext.alpha:global:key".into(),
        )
        .is_ok());
    }

    #[test]

    fn clear_extension_ephemeral_removes_only_matching_extension() {
        let ephemeral = ExtensionEphemeralStorage::default();

        ephemeral
            .set("extension:alpha:ephemeral:foo".into(), json!({"v": 1}))
            .unwrap();

        ephemeral
            .set("extension:beta:ephemeral:foo".into(), json!({"v": 2}))
            .unwrap();

        clear_extension_ephemeral(&ephemeral, "alpha").unwrap();

        assert!(ephemeral
            .get("extension:alpha:ephemeral:foo")
            .unwrap()
            .is_none());

        assert_eq!(
            ephemeral.get("extension:beta:ephemeral:foo").unwrap(),
            Some(json!({"v": 2}))
        );
    }

    #[test]

    fn storage_key_namespaces_scopes() {
        let global = storage_key("ext.alpha", &StorageScope::Global, None, "theme").unwrap();

        assert_eq!(global, "extension:ext.alpha:global:theme");

        let ephemeral = storage_key("ext.alpha", &StorageScope::Ephemeral, None, "theme").unwrap();

        assert_eq!(ephemeral, "extension:ext.alpha:ephemeral:theme");

        let worktree =
            storage_key("ext.alpha", &StorageScope::Worktree, Some("/repo"), "theme").unwrap();

        assert!(worktree.starts_with("extension:ext.alpha:worktree:"));

        assert!(worktree.ends_with(":theme"));

        // worktree 缺失参数 → 拒绝

        assert!(storage_key("ext.alpha", &StorageScope::Worktree, None, "theme").is_err());
    }

    #[test]

    fn storage_key_rejects_unsafe_keys() {
        assert!(storage_key("ext.alpha", &StorageScope::Global, None, "..").is_err());

        assert!(storage_key("ext.alpha", &StorageScope::Global, None, "/lead").is_err());

        assert!(storage_key("ext.alpha", &StorageScope::Global, None, "a\\b").is_err());
    }
}
