//! 领域无关受控操作执行机制（设计 35 §3.2）。
//!
//! 容器持有通用"受控操作执行机制"：Sandbox 门禁 + 审批 + 审计 + Registry 注册，
//! 不绑定任何业务领域。具体的操作定义（AI 的 file.edit / terminal，柜面的 query /
//! submit）由扩展通过 `runtime.operation.register` 注册，经 `runtime.operation.execute`
//! 执行；容器只负责机制本身。
//!
//! # 安全模型
//! - 操作 ID 全局唯一 `{extensionId}.{operationId}`，按扩展命名空间隔离，防止跨扩展冒用；
//! - 执行必须满足：扩展 Enabled → 操作已注册（fail-closed）→ 构造 `OperationRequest`
//!   `{actor:"extension:{id}"}` 过 Sandbox 门禁（审计由 `sandbox.check` 记录）；
//! - 需要用户确认的操作**不弹确认**，一律 fail-closed（与扩展桥语义一致）；
//! - `Builtin` 操作由容器直接执行（当前仅 `file.read`）；`Extension` 操作返回
//!   "需要扩展处理"信号，扩展 worker 用原语实现后自行回传。

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use crate::extension::{ExtensionStatus, ExtensionStore};
use crate::security::sandbox::permission::{
    CheckResult, OperationRequest, OperationType, PermissionLevel,
};
use crate::security::sandbox::Sandbox;
use crate::extension::types::MCP;

/// 操作执行所需的沙箱端口（领域无关契约）。
///
/// 执行受控操作时需要访问容器 Sandbox（门禁 / 审计源）。具体宿主（如
/// `tool::mcp::MCP`）在 tool 域实现本端口，避免 extension 域反向依赖业务域。
pub trait McpOperationPort {
    /// 返回容器级 Sandbox。
    fn sandbox(&self) -> Arc<Sandbox>;
}

/// 注册的操作定义（机制在容器，操作在扩展）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRegistration {
    /// 全局操作 ID："{extensionId}.{operationId}"（如 "com.example.myext.query"）
    pub id: String,
    /// 扩展 ID（命名空间）
    pub extension_id: String,
    /// 显示名
    pub label: String,
    /// 基础 Sandbox 操作类型（execute 时构造 OperationRequest 用）
    pub operation_type: OperationType,
    /// 权限等级（Unrestricted / LightCheck / StrictCheck / UserConfirm）
    pub permission_level: PermissionLevel,
    /// 参数 JSON Schema（可选，宽松）
    pub params_schema: Option<Value>,
    /// 执行方式
    pub handler_kind: OperationHandlerKind,
}

/// 操作执行方式：容器内建 vs 扩展 worker 实现。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationHandlerKind {
    /// 容器内建操作（如 file.read）——execute 时容器直接执行
    Builtin,
    /// 扩展 worker 实现——execute 时容器返回"需要扩展处理"信号，
    /// 扩展经原语实现后自行回传
    Extension,
}

/// 容器持有的受控操作注册表（按扩展命名空间隔离）。
#[derive(Default)]
pub struct OperationRegistry {
    operations: Mutex<HashMap<String, OperationRegistration>>,
}

impl OperationRegistry {
    /// 注册一个操作定义。
    ///
    /// id 必须为 `{extensionId}.{operationId}` 且前缀 == extension_id；label 非空；
    /// 同 id 重复注册拒绝。
    pub fn register(&self, reg: OperationRegistration) -> Result<(), String> {
        if reg.label.trim().is_empty() {
            return Err("Operation label must not be empty".to_string());
        }
        validate_operation_id(&reg.id, &reg.extension_id)?;
        let mut operations = self
            .operations
            .lock()
            .map_err(|_| "operation registry mutex poisoned".to_string())?;
        if operations.contains_key(&reg.id) {
            return Err(format!("Operation '{}' is already registered", reg.id));
        }
        operations.insert(reg.id.clone(), reg);
        Ok(())
    }

    /// 清理指定扩展的全部操作（生命周期禁用/卸载时调用）。返回移除数量。
    pub fn unregister_extension(&self, extension_id: &str) -> usize {
        let mut operations = match self.operations.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let before = operations.len();
        operations.retain(|_, reg| reg.extension_id != extension_id);
        before - operations.len()
    }

    /// 按全局操作 ID 查询（不存在返回 None，调用方 fail-closed）。
    pub fn get(&self, id: &str) -> Option<OperationRegistration> {
        let operations = self.operations.lock().ok()?;
        operations.get(id).cloned()
    }

    /// 列出操作；`extension_id` 为 Some 时只列出该扩展的操作，None 时列出全部。
    pub fn list(&self, extension_id: Option<&str>) -> Vec<OperationRegistration> {
        let Ok(operations) = self.operations.lock() else {
            return Vec::new();
        };
        let mut all: Vec<OperationRegistration> = operations
            .values()
            .filter(|reg| extension_id.map_or(true, |id| reg.extension_id == id))
            .cloned()
            .collect();
        all.sort_by(|a, b| a.id.cmp(&b.id));
        all
    }
}

/// 校验全局操作 ID 命名空间约束：`{extensionId}.{operationId}` 且前缀 == extension_id。
fn validate_operation_id(id: &str, extension_id: &str) -> Result<(), String> {
    let prefix = format!("{extension_id}.");
    let Some(op_id) = id.strip_prefix(&prefix) else {
        return Err(format!(
            "Operation id '{id}' must be namespaced by extension '{extension_id}'"
        ));
    };
    if op_id.is_empty() {
        return Err(format!("Operation id '{id}' has an empty operation part"));
    }
    Ok(())
}

/// 扩展 Enabled 校验（fail-closed）。
fn ensure_extension_enabled(store: &ExtensionStore, extension_id: &str) -> Result<(), String> {
    let state = store
        .get(extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }
    Ok(())
}

/// 校验 Sandbox 门禁结果；需要确认或拒绝的操作一律 fail-closed（不弹确认）。
fn require_allowed(result: &CheckResult, operation: &str) -> Result<(), String> {
    if result.allowed && !result.require_confirm {
        Ok(())
    } else {
        let reason = result
            .reason
            .clone()
            .or_else(|| result.confirm_message.clone())
            .unwrap_or_else(|| "sandbox denied".to_string());
        Err(format!("Operation '{operation}' denied by sandbox: {reason}"))
    }
}

// ============================================================================
// IPC payload（serde camelCase + deny_unknown_fields）
// ============================================================================

/// `runtime.operation.register` payload。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRegisterRequest {
    pub extension_id: String,
    pub id: String,
    pub label: String,
    pub permission_level: PermissionLevel,
    pub operation_type: OperationType,
    #[serde(default)]
    pub params_schema: Option<Value>,
    pub handler_kind: OperationHandlerKind,
}

/// `runtime.operation.execute` payload。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationExecuteRequest {
    pub extension_id: String,
    pub operation_id: String,
    pub params: Value,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub worktree: Option<String>,
}

/// `runtime.operation.list` payload。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationListRequest {
    #[serde(default)]
    pub extension_id: Option<String>,
}

// ============================================================================
// 核心逻辑（命令与扩展桥共用）
// ============================================================================

/// 受控操作注册核心（`ui_operation_register` 与桥 `runtime.operation.register` 共用）。
pub fn register_operation(
    extension_store: &ExtensionStore,
    registry: &OperationRegistry,
    request: OperationRegisterRequest,
) -> Result<OperationRegistration, String> {
    ensure_extension_enabled(extension_store, &request.extension_id)?;
    let registration = OperationRegistration {
        id: request.id,
        extension_id: request.extension_id,
        label: request.label,
        operation_type: request.operation_type,
        permission_level: request.permission_level,
        params_schema: request.params_schema,
        handler_kind: request.handler_kind,
    };
    registry.register(registration.clone())?;
    tracing::info!(
        operation_id = %registration.id,
        extension_id = %registration.extension_id,
        handler_kind = ?registration.handler_kind,
        "Controlled operation registered"
    );
    Ok(registration)
}

/// 受控操作执行核心（`ui_operation_execute` 与桥 `runtime.operation.execute` 共用）。
///
/// 流程：扩展 Enabled → 操作已注册且命名空间匹配（fail-closed）→ 构造
/// `OperationRequest{operation_type, operation_id, actor:"extension:{id}"}` 过
/// Sandbox 门禁（require_confirm/拒绝 → fail-closed；审计由 sandbox.check 记录）
/// → 按 handler_kind 派发：Builtin 容器直接执行，Extension 返回扩展处理信号。
pub async fn execute_operation(
    extension_store: &ExtensionStore,
    registry: &OperationRegistry,
    mcp: &impl McpOperationPort,
    request: OperationExecuteRequest,
) -> Result<Value, String> {
    ensure_extension_enabled(extension_store, &request.extension_id)?;

    // 操作必须已注册，且命名空间必须与调用方扩展一致（防跨扩展冒用）。
    let operation = registry
        .get(&request.operation_id)
        .ok_or_else(|| format!("Operation '{}' is not registered", request.operation_id))?;
    if operation.extension_id != request.extension_id {
        return Err(format!(
            "Operation '{}' belongs to extension '{}', not '{}'",
            operation.id, operation.extension_id, request.extension_id
        ));
    }

    // 容器级门禁：按操作声明的 Sandbox 操作类型 + 全局操作 ID 为 target。
    let mut op_request = OperationRequest::new(
        operation.operation_type.clone(),
        operation.id.clone(),
        format!("extension:{}", request.extension_id),
    );
    if let Some(session_id) = &request.session_id {
        op_request = op_request.with_session_id(session_id.clone());
    }
    if let Some(worktree) = &request.worktree {
        op_request = op_request.with_worktree(worktree.clone());
    }

    let sandbox = mcp.sandbox();
    let result = sandbox
        .check(&op_request)
        .map_err(|error| format!("Sandbox check failed: {error}"))?;
    require_allowed(&result, &operation.id)?;

    tracing::info!(
        operation_id = %operation.id,
        extension_id = %request.extension_id,
        handler_kind = ?operation.handler_kind,
        "Controlled operation executed"
    );

    match operation.handler_kind {
        OperationHandlerKind::Builtin => {
            builtin_dispatch(sandbox.as_ref(), &request, &operation).await
        }
        OperationHandlerKind::Extension => {
            // 门禁已通过；容器返回"需要扩展处理"信号，扩展 worker 用原语实现后自行回传。
            Ok(json!({
                "status": "extension_handler",
                "operationId": operation.id,
                "params": request.params,
            }))
        }
    }
}

/// 容器内建操作执行。当前仅支持 `file.read`（只读），其余 Builtin 操作 fail-closed。
async fn builtin_dispatch(
    sandbox: &Sandbox,
    request: &OperationExecuteRequest,
    operation: &OperationRegistration,
) -> Result<Value, String> {
    let op_id = operation
        .id
        .strip_prefix(&format!("{}.", operation.extension_id))
        .unwrap_or(&operation.id);
    match op_id {
        "file.read" => {
            builtin_file_read(
                sandbox,
                &request.extension_id,
                &request.params,
                request.worktree.as_deref(),
            )
            .await
        }
        _ => Err(format!("Builtin operation not implemented: {}", operation.id)),
    }
}

/// 容器内建 `file.read`：只读、受 path_manager 归一化 + Sandbox FileRead 门禁。
///
/// 与 `extension_bridge::bridge_file_read` 同一套读取管线（该函数为桥模块私有，
/// 此处按容器职责复制只读逻辑）。
async fn builtin_file_read(
    sandbox: &Sandbox,
    extension_id: &str,
    params: &Value,
    worktree: Option<&str>,
) -> Result<Value, String> {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "file.read requires a 'path' parameter".to_string())?;
    let base = worktree.map(Path::new).unwrap_or_else(|| Path::new("."));
    let resolved = crate::extension::types::PathManager::resolve(base, Path::new(path));

    let mut request = OperationRequest::new(
        OperationType::FileRead,
        resolved.display().to_string(),
        format!("extension:{extension_id}"),
    );
    if let Some(worktree) = worktree {
        request = request.with_worktree(worktree);
    }
    let result = sandbox
        .check(&request)
        .map_err(|error| format!("Sandbox check failed: {error}"))?;
    require_allowed(&result, "file.read")?;

    let resolved_display = resolved.display().to_string();
    let read_target = resolved.clone();
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read_target))
        .await
        .map_err(|join| format!("Failed to join file read task: {join}"))?
        .map_err(|error| format!("Failed to read '{resolved_display}': {error}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("File '{resolved_display}' is not valid UTF-8: {error}"))?;
    Ok(json!({ "content": text, "path": resolved_display }))
}

/// 清理扩展的全部受控操作（生命周期禁用/卸载时调用）。
pub fn clear_extension_operations(registry: &OperationRegistry, extension_id: &str) {
    let removed = registry.unregister_extension(extension_id);
    if removed > 0 {
        tracing::info!(
            extension_id = %extension_id,
            removed = removed,
            "Cleared controlled operations for extension"
        );
    }
}

// ============================================================================
// Tauri 命令
// ============================================================================

/// 注册一个受控操作（命令入口）。
#[tauri::command]
pub fn ui_operation_register(
    extension_store: State<'_, Arc<ExtensionStore>>,
    operation_store: State<'_, Arc<OperationRegistry>>,
    request: OperationRegisterRequest,
) -> Result<OperationRegistration, String> {
    register_operation(
        extension_store.inner().as_ref(),
        operation_store.inner().as_ref(),
        request,
    )
}

/// 执行一个受控操作（命令入口）。
#[tauri::command]
pub async fn ui_operation_execute(
    extension_store: State<'_, Arc<ExtensionStore>>,
    operation_store: State<'_, Arc<OperationRegistry>>,
//     mcp: State<'_, Arc<[REMOVED: MCP reference]
    mcp: State<'_, Arc<MCP>>,
    request: OperationExecuteRequest,
) -> Result<Value, String> {
    execute_operation(
        extension_store.inner().as_ref(),
        operation_store.inner().as_ref(),
        mcp.inner().as_ref(),
        request,
    )
    .await
}

/// 列出受控操作（命令入口）。
#[tauri::command]
pub fn ui_operation_list(
    operation_store: State<'_, Arc<OperationRegistry>>,
    request: OperationListRequest,
) -> Vec<OperationRegistration> {
    operation_store.inner().list(request.extension_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{
        ExtensionContributes, ExtensionManifest, ExtensionPermissions, ExtensionState,
    };
    use crate::kernel::{EventBus, InMemoryEventBus};
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    fn test_store() -> Arc<ExtensionStore> {
        let event_bus: Arc<dyn EventBus> =
            Arc::new(InMemoryEventBus::new(1000, test_runtime_handle()));
        Arc::new(ExtensionStore::new(event_bus))
    }

    fn register_test_extension(store: &Arc<ExtensionStore>, id: &str, status: ExtensionStatus) {
        store
            .register(ExtensionState {
                id: id.to_string(),
                status,
                manifest: ExtensionManifest {
                    id: id.to_string(),
                    name: format!("Extension {id}"),
                    version: "1.0.0".into(),
                    description: "test".into(),
                    author: "test".into(),
                    permissions: ExtensionPermissions::default(),
                    contributes: ExtensionContributes::default(),
                },
                install_path: PathBuf::from(format!("/extensions/{id}")),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();
    }

    /// 测试用操作端口桩：不依赖 tool 域 MCP，保持 extension 域无业务反向依赖。
    struct TestOperationPort {
        sandbox: Arc<Sandbox>,
    }

    impl McpOperationPort for TestOperationPort {
        fn sandbox(&self) -> Arc<Sandbox> {
            Arc::clone(&self.sandbox)
        }
    }

    fn test_port() -> TestOperationPort {
        let event_bus: Arc<dyn EventBus> =
            Arc::new(InMemoryEventBus::new(1000, test_runtime_handle()));
        let sandbox = Arc::new(Sandbox::new(event_bus));
        TestOperationPort { sandbox }
    }

    fn register_op(
        registry: &OperationRegistry,
        extension_id: &str,
        op: &str,
        handler_kind: OperationHandlerKind,
    ) {
        registry
            .register(OperationRegistration {
                id: format!("{extension_id}.{op}"),
                extension_id: extension_id.to_string(),
                label: format!("Op {op}"),
                operation_type: OperationType::FileRead,
                permission_level: PermissionLevel::LightCheck,
                params_schema: None,
                handler_kind,
            })
            .unwrap();
    }

    // ---- OperationRegistry：注册 / 重复拒绝 / 命名空间隔离 ----

    #[test]
    fn registry_validates_namespace_and_label() {
        let registry = OperationRegistry::default();
        let base = OperationRegistration {
            id: "ext.alpha.query".into(),
            extension_id: "ext.alpha".into(),
            label: "Query".into(),
            operation_type: OperationType::FileRead,
            permission_level: PermissionLevel::LightCheck,
            params_schema: None,
            handler_kind: OperationHandlerKind::Extension,
        };

        // 合法注册
        assert!(registry.register(base.clone()).is_ok());
        // 同 id 重复拒绝
        assert!(registry.register(base.clone()).is_err());
        // label 非空
        let mut no_label = base.clone();
        no_label.id = "ext.alpha.other".into();
        no_label.label = "   ".into();
        assert!(registry.register(no_label).is_err());
        // id 前缀必须 == extension_id
        let mut bad_prefix = base.clone();
        bad_prefix.id = "ext.beta.query".into();
        assert!(registry.register(bad_prefix).is_err());
        // 空 operationId
        let mut empty_op = base;
        empty_op.id = "ext.alpha.".into();
        assert!(registry.register(empty_op).is_err());

        // 跨扩展命名空间隔离
        let beta = OperationRegistration {
            id: "ext.beta.query".into(),
            extension_id: "ext.beta".into(),
            label: "Beta Query".into(),
            operation_type: OperationType::FileRead,
            permission_level: PermissionLevel::LightCheck,
            params_schema: None,
            handler_kind: OperationHandlerKind::Extension,
        };
        assert!(registry.register(beta).is_ok());
        assert!(registry.get("ext.alpha.query").is_some());
        assert!(registry.get("ext.beta.query").is_some());
        assert_eq!(registry.list(Some("ext.alpha")).len(), 1);
        assert_eq!(registry.list(None).len(), 2);
    }

    #[test]
    fn unregister_extension_cleans_only_matching_extension() {
        let registry = OperationRegistry::default();
        register_op(&registry, "ext.alpha", "query", OperationHandlerKind::Builtin);
        register_op(&registry, "ext.beta", "query", OperationHandlerKind::Builtin);

        assert_eq!(registry.unregister_extension("ext.alpha"), 1);
        assert!(registry.get("ext.alpha.query").is_none());
        assert!(registry.get("ext.beta.query").is_some());
        assert_eq!(registry.list(None).len(), 1);

        // 再次清理幂等
        assert_eq!(registry.unregister_extension("ext.alpha"), 0);
    }

    #[test]
    fn require_allowed_rejects_confirm_and_denied() {
        let allowed = CheckResult::allowed(PermissionLevel::LightCheck);
        assert!(require_allowed(&allowed, "op").is_ok());

        let needs_confirm = CheckResult::needs_confirm(PermissionLevel::UserConfirm, "confirm me");
        assert!(require_allowed(&needs_confirm, "op").is_err());

        let denied = CheckResult::denied(PermissionLevel::UserConfirm, "blocked");
        assert!(require_allowed(&denied, "op").is_err());
    }

    // ---- register_operation：需要扩展 Enabled ----

    #[test]
    fn register_operation_requires_enabled_extension() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Disabled);
        let registry = OperationRegistry::default();
        let request = OperationRegisterRequest {
            extension_id: "ext.alpha".into(),
            id: "ext.alpha.query".into(),
            label: "Query".into(),
            permission_level: PermissionLevel::LightCheck,
            operation_type: OperationType::FileRead,
            params_schema: None,
            handler_kind: OperationHandlerKind::Extension,
        };
        assert!(register_operation(&store, &registry, request).is_err());
    }

    // ---- execute_operation：fail-closed ----

    #[tokio::test]
    async fn execute_fails_closed_when_operation_not_registered_or_extension_disabled() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        register_test_extension(&store, "ext.disabled", ExtensionStatus::Disabled);
        let registry = OperationRegistry::default();
        let port = test_port();

        // 未注册操作 → fail-closed（即使扩展已启用）
        let unregistered = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.query".into(),
            params: json!({}),
            session_id: None,
            worktree: None,
        };
        assert!(execute_operation(&store, &registry, &port, unregistered)
            .await
            .is_err());

        // 扩展未启用 → fail-closed（即使操作已注册）
        register_op(&registry, "ext.disabled", "query", OperationHandlerKind::Extension);
        let disabled = OperationExecuteRequest {
            extension_id: "ext.disabled".into(),
            operation_id: "ext.disabled.query".into(),
            params: json!({}),
            session_id: None,
            worktree: None,
        };
        assert!(execute_operation(&store, &registry, &port, disabled).await.is_err());
    }

    #[tokio::test]
    async fn execute_rejects_operation_namespaced_by_other_extension() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        register_test_extension(&store, "ext.beta", ExtensionStatus::Enabled);
        let registry = OperationRegistry::default();
        register_op(&registry, "ext.beta", "query", OperationHandlerKind::Extension);
        let port = test_port();

        // alpha 试图执行 beta 注册的操作 → 命名空间不匹配，fail-closed
        let spoofed = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.beta.query".into(),
            params: json!({}),
            session_id: None,
            worktree: None,
        };
        assert!(execute_operation(&store, &registry, &port, spoofed).await.is_err());
    }

    #[tokio::test]
    async fn execute_rejects_when_sandbox_requires_confirm() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        let registry = OperationRegistry::default();
        // 默认 Suggest 模式下 CommandExecute 需要确认 → fail-closed（不弹确认）
        registry
            .register(OperationRegistration {
                id: "ext.alpha.run".into(),
                extension_id: "ext.alpha".into(),
                label: "Run".into(),
                operation_type: OperationType::CommandExecute,
                permission_level: PermissionLevel::UserConfirm,
                params_schema: None,
                handler_kind: OperationHandlerKind::Extension,
            })
            .unwrap();
        let port = test_port();
        let request = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.run".into(),
            params: json!({ "command": "ls" }),
            session_id: None,
            worktree: None,
        };
        assert!(execute_operation(&store, &registry, &port, request).await.is_err());
    }

    // ---- execute_operation：Extension 信号 / Builtin 执行 ----

    #[tokio::test]
    async fn extension_handler_returns_extension_handler_signal() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        let registry = OperationRegistry::default();
        // FileRead 在默认 Suggest 模式下放行，门禁通过后返回扩展处理信号
        register_op(&registry, "ext.alpha", "query", OperationHandlerKind::Extension);
        let port = test_port();
        let request = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.query".into(),
            params: json!({ "query": "SELECT 1" }),
            session_id: Some("session-1".into()),
            worktree: None,
        };
        let result = execute_operation(&store, &registry, &port, request).await.unwrap();
        assert_eq!(result["status"], "extension_handler");
        assert_eq!(result["operationId"], "ext.alpha.query");
        assert_eq!(result["params"]["query"], "SELECT 1");
    }

    #[tokio::test]
    async fn builtin_file_read_executes_container_side() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        let registry = OperationRegistry::default();
        register_op(&registry, "ext.alpha", "file.read", OperationHandlerKind::Builtin);
        let port = test_port();

        let temp_dir = std::env::temp_dir()
            .join(format!("navis-op-runtime-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file = temp_dir.join("sample.txt");
        std::fs::write(&file, "hello runtime").unwrap();

        let request = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.file.read".into(),
            // worktree 指向临时目录，sandbox 判定路径在 worktree 内放行
            params: json!({ "path": "sample.txt" }),
            session_id: None,
            worktree: Some(temp_dir.to_string_lossy().to_string()),
        };
        let result = execute_operation(&store, &registry, &port, request).await.unwrap();
        assert_eq!(result["content"], "hello runtime");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn builtin_unimplemented_returns_not_implemented() {
        let store = test_store();
        register_test_extension(&store, "ext.alpha", ExtensionStatus::Enabled);
        let registry = OperationRegistry::default();
        // FileRead 放行，但 Builtin 只实现 file.read → not implemented
        register_op(&registry, "ext.alpha", "write", OperationHandlerKind::Builtin);
        let port = test_port();
        let request = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.write".into(),
            params: json!({ "path": "/tmp/x" }),
            session_id: None,
            worktree: None,
        };
        let error = execute_operation(&store, &registry, &port, request).await.unwrap_err();
        assert!(error.contains("Builtin operation not implemented"));
    }

    // ---- payload serde ----

    #[test]
    fn payload_serializes_camel_case() {
        let request = OperationRegisterRequest {
            extension_id: "com.example.myext".into(),
            id: "com.example.myext.query".into(),
            label: "Query".into(),
            permission_level: PermissionLevel::LightCheck,
            operation_type: OperationType::FileRead,
            params_schema: Some(json!({ "type": "object" })),
            handler_kind: OperationHandlerKind::Extension,
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["extensionId"], "com.example.myext");
        assert_eq!(value["id"], "com.example.myext.query");
        assert_eq!(value["permissionLevel"], "LightCheck");
        assert_eq!(value["operationType"], "FileRead");
        assert_eq!(value["paramsSchema"]["type"], "object");
        assert_eq!(value["handlerKind"], "Extension");

        // 完整 roundtrip
        let back: OperationRegisterRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back.id, request.id);
        assert_eq!(back.permission_level, PermissionLevel::LightCheck);
        assert_eq!(back.handler_kind, OperationHandlerKind::Extension);
    }

    #[test]
    fn execute_payload_serializes_camel_case() {
        let request = OperationExecuteRequest {
            extension_id: "ext.alpha".into(),
            operation_id: "ext.alpha.query".into(),
            params: json!({ "q": 1 }),
            session_id: Some("s1".into()),
            worktree: None,
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["operationId"], "ext.alpha.query");
        assert_eq!(value["sessionId"], "s1");
        assert_eq!(value["params"]["q"], 1);
    }

    #[test]
    fn payload_denies_unknown_fields() {
        let json = serde_json::json!({
            "extensionId": "ext.alpha",
            "id": "ext.alpha.query",
            "label": "Query",
            "permissionLevel": "LightCheck",
            "operationType": "FileRead",
            "handlerKind": "Extension",
            "bogus": true,
        });
        assert!(serde_json::from_value::<OperationRegisterRequest>(json).is_err());
    }

    #[test]
    fn list_returns_only_requested_extension() {
        let registry = OperationRegistry::default();
        register_op(&registry, "ext.alpha", "query", OperationHandlerKind::Builtin);
        register_op(&registry, "ext.beta", "query", OperationHandlerKind::Builtin);

        let alpha = registry.list(Some("ext.alpha"));
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].extension_id, "ext.alpha");
        assert_eq!(registry.list(None).len(), 2);
    }
}
