//! 扩展白名单桥（阶段 1）。
//!
//! iframe / Worker 内的扩展 UI 通过宿主桥（前端 `bridge.ts`）调用本模块命令。
//! 每个桥请求都经过三层校验：
//! 1. 扩展必须处于 Enabled 状态；
//! 2. 目标命令必须声明在 `manifest.capabilities.invoke` 白名单中；
//! 3. 实际文件/命令操作构造 `OperationRequest{actor:"extension:{id}"}`
//!    复用 `security::sandbox` 门禁（白名单 / 权限分级 / 审计）。
//!
//! 未声明能力 → 拒绝 + 审计；fail-closed。
/// 桥请求。扩展 UI 侧 `window.__NAVIS__.invoke(cmd, args)`。
/// 桥响应。
/// 桥支持的受限命令注册表。只有这里列出的命令可被桥派发；
/// 且仍须同时满足 capabilities.invoke 白名单与 Sandbox 门禁。
/// 事件订阅能力必须严格窄于 invoke 能力。
/// 声明可以授予精确主题或 `session.*` 这类前缀通配符；请求通配符不得扩大声明范围。
/// 高频扩展事件必须使用流通道，不能绕过能力校验使用低频 EventBus/Tauri 事件桥。
/// 构造扩展 actor 的 Sandbox 操作请求。
/// 校验 Sandbox 门禁结果；需要确认的操作直接拒绝（扩展桥不弹确认）。
/// 为扩展桥授权低频 Tauri/Kernel 事件订阅。
///
/// 浏览器端只有在此命令成功后才执行实际的 `listen()` 调用，确保事件授权
/// 在 Rust 侧完成，防止 iframe 绕过能力校验直接使用通用前端事件 API。
/// 读取 worktree 文本文件。只读、受 path_manager 归一化 + Sandbox FileRead 门禁。
/// 读取当前 session 快照（只读投影）。
/// 白名单桥派发入口（iframe / Worker）。
///
/// 流程：校验 Enabled → 查 `capabilities.invoke` 白名单 → 受限命令分发 →
/// 构造 `OperationRequest{actor:"extension:{id}"}` 过 Sandbox 门禁 → 派发。
/// 未声明能力或门禁拒绝 → fail-closed 返回错误并审计。
/// 受控操作注册桥派发（`runtime.operation.register`）。
///
/// args 内的 `extensionId` 必须与桥调用方扩展一致，防止跨扩展冒名注册。
/// 受控操作执行桥派发（`runtime.operation.execute`）。
///
/// args 内的 `extensionId` 必须与桥调用方扩展一致，防止跨扩展冒名执行。
/// 受控操作列表桥派发（`runtime.operation.list`）。

use crate::extension::types::MCP;













use std::path::Path;

use std::sync::Arc;



use serde::{Deserialize, Serialize};

use serde_json::{json, Value};

use tauri::State;



use crate::extension::models::{CapabilityDeclaration, ExtensionStatus};

use crate::extension::{ExtensionLifecycle, ExtensionStore};

use crate::app::infra::Storage;

use crate::security::sandbox::permission::{CheckResult, OperationRequest, OperationType};

use crate::security::sandbox::Sandbox;

use crate::ui::extension_network::{ui_extension_network_proxy, ExtensionNetworkRequest};

use crate::ui::extension_router::{ui_extension_route_call, ExtensionRouteRequest};

use crate::ui::extension_storage::{

    ui_extension_storage_clear, ui_extension_storage_delete, ui_extension_storage_get,

    ui_extension_storage_set, ExtensionEphemeralStorage, ExtensionStorageClearRequest,

    ExtensionStorageRequest,

};

use crate::ui::extensions::{ui_extension_discovery_query, ExtensionDiscoveryQuery};

use crate::extension::operation_runtime::{

    execute_operation, register_operation, OperationExecuteRequest, OperationListRequest,

    OperationRegisterRequest, OperationRegistry,

};




#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(deny_unknown_fields)]

pub struct BridgeInvokeRequest {

    /// 发起请求的扩展 ID。

    pub extension_id: String,

    /// 目标宿主命令名（须在 capabilities.invoke 白名单中）。

    pub cmd: String,

    /// 命令参数。

    pub args: Value,

}




#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(rename_all = "camelCase")]

pub struct BridgeInvokeResponse {

    pub ok: bool,

    pub data: Option<Value>,

    pub error: Option<String>,

}





fn supported_bridge_commands() -> &'static [&'static str] {

    &[

        // 文件与上下文（阶段 1）

        "file.read",

        "context.getSession",

        "context.getActiveProject",

        // 扩展发现（阶段 9）

        "extensions.query",

        // 跨扩展路由（阶段 7）

        "route.call",

        // 扩展 KV 存储（阶段 8）

        "storage.get",

        "storage.set",

        "storage.delete",

        "storage.clear",

        // 网络代理（阶段 8）

        "network.fetch",

        // 领域无关受控操作执行机制（设计 35 §3.2）

        "runtime.operation.execute",

        "runtime.operation.register",

        "runtime.operation.list",

    ]

}



fn extension_enabled(store: &ExtensionStore, extension_id: &str) -> bool {

    store

        .get(extension_id)

        .is_some_and(|state| state.status == ExtensionStatus::Enabled)

}



fn invoke_whitelisted(capabilities: &Option<CapabilityDeclaration>, cmd: &str) -> bool {

    capabilities

        .as_ref()

        .is_some_and(|caps| caps.invoke.iter().any(|allowed| allowed == cmd))

}






fn event_pattern_whitelisted(capabilities: &Option<CapabilityDeclaration>, requested: &str) -> bool {

    let requested = requested.trim();

    if requested.is_empty() || is_high_frequency_event_pattern(requested) {

        return false;

    }



    capabilities.as_ref().is_some_and(|caps| {

        caps.events.iter().any(|declared| {

            let declared = declared.trim();

            if declared == "*" {

                return !requested.contains('*');

            }

            if !requested.contains('*') && declared == requested {

                return true;

            }

            declared.strip_suffix('*').is_some_and(|prefix| {

                !prefix.is_empty()

                    && !requested.contains('*')

                    && requested.starts_with(prefix)

            })

        })

    })

}





fn is_high_frequency_event_pattern(pattern: &str) -> bool {

    ["agent", "terminal", "task"].iter().any(|kind| {

        pattern == *kind

            || pattern.starts_with(&format!("{kind}."))

            || pattern.starts_with(&format!("{kind}:"))

    })

}




fn sandbox_request(

    extension_id: &str,

    operation: OperationType,

    target: impl Into<String>,

    worktree: Option<String>,

) -> OperationRequest {

    let mut request = OperationRequest::new(

        operation,

        target,

        format!("extension:{extension_id}"),

    );

    if let Some(worktree) = worktree {

        request = request.with_worktree(worktree);

    }

    request

}




fn require_allowed(result: &CheckResult, cmd: &str) -> Result<(), String> {

    if result.allowed && !result.require_confirm {

        Ok(())

    } else {

        let reason = result

            .reason

            .clone()

            .or(result.confirm_message.clone())

            .unwrap_or_else(|| "sandbox denied".to_string());

        Err(format!("Bridge command '{cmd}' denied by sandbox: {reason}"))

    }

}








#[derive(Debug, Clone, Serialize, Deserialize)]

#[serde(deny_unknown_fields)]

pub struct BridgeEventAuthorizationRequest {

    pub extension_id: String,

    pub pattern: String,

}



#[tauri::command]

pub async fn ui_extension_bridge_authorize_event(

    extension_store: State<'_, Arc<ExtensionStore>>,

    request: BridgeEventAuthorizationRequest,

) -> Result<(), String> {

    if !extension_enabled(&extension_store, &request.extension_id) {

        tracing::warn!(

            extension_id = %request.extension_id,

            pattern = %request.pattern,

            "Bridge event subscription rejected: extension is not enabled"

        );

        return Err(format!("Extension '{}' is not enabled", request.extension_id));

    }



    let manifest = extension_store

        .get_manifest(&request.extension_id)

        .map_err(|error| format!("Extension '{}' manifest not found: {error}", request.extension_id))?;

    if !event_pattern_whitelisted(&manifest.contributes.capabilities, &request.pattern) {

        tracing::warn!(

            extension_id = %request.extension_id,

            pattern = %request.pattern,

            "Bridge event subscription rejected: pattern is not authorized"

        );

        return Err(format!(

            "Event pattern '{}' is not declared in capabilities.events or requires a stream channel",

            request.pattern

        ));

    }



    Ok(())

}




async fn bridge_file_read(

    sandbox: &Sandbox,

    extension_id: &str,

    args: &Value,

) -> Result<Value, String> {

    let path = args

        .get("path")

        .and_then(Value::as_str)

        .ok_or_else(|| "file.read requires a 'path' argument".to_string())?;

    let worktree = args

        .get("worktree")

        .and_then(Value::as_str)

        .map(str::to_string);



    let resolved = {

        let base = worktree

            .as_ref()

            .map(|root| Path::new(root))

            .unwrap_or_else(|| Path::new("."));

        crate::extension::types::PathManager::resolve(base, Path::new(path))

    };



    let request =

        sandbox_request(extension_id, OperationType::FileRead, resolved.display().to_string(), worktree);

    let result = sandbox

        .check(&request)

        .map_err(|error| format!("Sandbox check failed: {error}"))?;

    require_allowed(&result, "file.read")?;



    let resolved_display = resolved.display().to_string();

    let read_target = resolved.clone();

    let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read_target))

        .await

        .map_err(|join| format!("Failed to join file read task: {join}"))?

        .map_err(|error| format!("Failed to read '{}': {error}", resolved_display))?;

    let text = String::from_utf8(bytes)

        .map_err(|error| format!("File '{}' is not valid UTF-8: {error}", resolved_display))?;

    Ok(json!({ "content": text, "path": resolved_display }))

}




fn bridge_context_get_session() -> Result<Value, String> {

    // 由前端注入真实快照；桥保持无状态只读。返回统一形状。

    Ok(json!({}))

}



fn bridge_context_get_active_project() -> Result<Value, String> {

    Ok(json!({}))

}








#[tauri::command]

pub async fn ui_extension_bridge_invoke(

    extension_store: State<'_, Arc<ExtensionStore>>,

    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    mcp: State<'_, Arc<MCP>>,


    storage: State<'_, Arc<Storage>>,

    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,

    operation_store: State<'_, Arc<OperationRegistry>>,

    request: BridgeInvokeRequest,

) -> Result<BridgeInvokeResponse, String> {

    let extension_id = &request.extension_id;

    let cmd = &request.cmd;



    // 1. Enabled 校验

    if !extension_enabled(&extension_store, extension_id) {

        tracing::warn!(

            extension_id = %extension_id,

            cmd = %cmd,

            "Bridge invoke rejected: extension is not enabled"

        );

        return Ok(denied_response(format!("Extension '{extension_id}' is not enabled")));

    }



    // 2. capabilities.invoke 白名单校验

    let manifest = match extension_store.get_manifest(extension_id) {

        Ok(manifest) => manifest,

        Err(error) => return Ok(denied_response(format!("Extension '{extension_id}' manifest not found: {error}"))),

    };

    if !invoke_whitelisted(&manifest.contributes.capabilities, cmd) {

        tracing::warn!(

            extension_id = %extension_id,

            cmd = %cmd,

            "Bridge invoke rejected: command not in capabilities.invoke whitelist"

        );

        return Ok(denied_response(format!(

            "Bridge command '{cmd}' is not declared in capabilities.invoke"

        )));

    }



    // 3. 受限命令注册表校验（fail-closed）

    if !supported_bridge_commands().contains(&cmd.as_str()) {

        tracing::warn!(

            extension_id = %extension_id,

            cmd = %cmd,

            "Bridge invoke rejected: command not in supported bridge registry"

        );

        return Ok(denied_response(format!(

            "Bridge command '{cmd}' is not supported by the bridge registry"

        )));

    }



    // 4. Sandbox 门禁 + 派发

    let sandbox = mcp.sandbox();

    let result = match cmd.as_str() {

        "file.read" => bridge_file_read(&sandbox, extension_id, &request.args).await,

        "context.getSession" => bridge_context_get_session(),

        "context.getActiveProject" => bridge_context_get_active_project(),

        "extensions.query" => bridge_extension_discovery_query(&extension_store, &lifecycle, &request.args),

        "route.call" => bridge_extension_route_call(&extension_store, mcp.clone(), &request.args),

        "storage.get" => bridge_storage_get(&extension_store, &storage, &ephemeral, mcp.clone(), &request.args),

        "storage.set" => bridge_storage_set(&extension_store, &storage, &ephemeral, mcp.clone(), &request.args),

        "storage.delete" => bridge_storage_delete(&extension_store, &storage, &ephemeral, mcp.clone(), &request.args),

        "storage.clear" => bridge_storage_clear(&extension_store, &storage, &ephemeral, mcp.clone(), &request.args),

        "network.fetch" => bridge_network_fetch(&extension_store, mcp.clone(), &request.args).await,

        "runtime.operation.execute" => {

            bridge_operation_execute(extension_id, &extension_store, &operation_store, &mcp, &request.args)

                .await

        }

        "runtime.operation.register" => {

            bridge_operation_register(extension_id, &extension_store, &operation_store, &request.args)

        }

        "runtime.operation.list" => bridge_operation_list(&operation_store, &request.args),

        _ => Err(format!("Unsupported bridge command '{cmd}'")),

    };



    Ok(match result {

        Ok(data) => BridgeInvokeResponse {

            ok: true,

            data: Some(data),

            error: None,

        },

        Err(error) => {

            tracing::warn!(extension_id = %extension_id, cmd = %cmd, error = %error, "Bridge invoke failed");

            BridgeInvokeResponse {

                ok: false,

                data: None,

                error: Some(error),

            }

        }

    })

}



fn bridge_extension_discovery_query(

    extension_store: &State<'_, Arc<ExtensionStore>>,

    lifecycle: &State<'_, Arc<ExtensionLifecycle>>,

    args: &Value,

) -> Result<Value, String> {

    let query = serde_json::from_value::<ExtensionDiscoveryQuery>(args.clone())

        .map_err(|error| format!("Invalid extensions.query args: {error}"))?;

    let results = ui_extension_discovery_query(extension_store.clone(), lifecycle.clone(), query);

    Ok(serde_json::to_value(results).map_err(|error| format!("Failed to serialize discovery results: {error}"))?)

}



fn bridge_extension_route_call(

    extension_store: &State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionRouteRequest>(args.clone())

        .map_err(|error| format!("Invalid route.call args: {error}"))?;

    let response = ui_extension_route_call(extension_store.clone(), mcp, request)

        .map_err(|error| format!("Extension route call failed: {error}"))?;

    serde_json::to_value(response).map_err(|error| format!("Failed to serialize route response: {error}"))

}



fn bridge_storage_get(

    extension_store: &State<'_, Arc<ExtensionStore>>,


    storage: &State<'_, Arc<Storage>>,

    ephemeral: &State<'_, Arc<ExtensionEphemeralStorage>>,
    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionStorageRequest>(args.clone())

        .map_err(|error| format!("Invalid storage.get args: {error}"))?;

    let response =

        ui_extension_storage_get(extension_store.clone(), mcp, storage.clone(), ephemeral.clone(), request)

            .map_err(|error| format!("Storage get failed: {error}"))?;

    serde_json::to_value(response).map_err(|error| format!("Failed to serialize storage response: {error}"))

}



fn bridge_storage_set(

    extension_store: &State<'_, Arc<ExtensionStore>>,


    storage: &State<'_, Arc<Storage>>,

    ephemeral: &State<'_, Arc<ExtensionEphemeralStorage>>,
    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionStorageRequest>(args.clone())

        .map_err(|error| format!("Invalid storage.set args: {error}"))?;

    ui_extension_storage_set(extension_store.clone(), mcp, storage.clone(), ephemeral.clone(), request)

        .map_err(|error| format!("Storage set failed: {error}"))?;

    Ok(json!({ "ok": true }))

}



fn bridge_storage_delete(

    extension_store: &State<'_, Arc<ExtensionStore>>,


    storage: &State<'_, Arc<Storage>>,

    ephemeral: &State<'_, Arc<ExtensionEphemeralStorage>>,
    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionStorageRequest>(args.clone())

        .map_err(|error| format!("Invalid storage.delete args: {error}"))?;

    ui_extension_storage_delete(extension_store.clone(), mcp, storage.clone(), ephemeral.clone(), request)

        .map_err(|error| format!("Storage delete failed: {error}"))?;

    Ok(json!({ "ok": true }))

}



fn bridge_storage_clear(

    extension_store: &State<'_, Arc<ExtensionStore>>,


    storage: &State<'_, Arc<Storage>>,

    ephemeral: &State<'_, Arc<ExtensionEphemeralStorage>>,
    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionStorageClearRequest>(args.clone())

        .map_err(|error| format!("Invalid storage.clear args: {error}"))?;

    ui_extension_storage_clear(extension_store.clone(), mcp, storage.clone(), ephemeral.clone(), request)

        .map_err(|error| format!("Storage clear failed: {error}"))?;

    Ok(json!({ "ok": true }))

}



async fn bridge_network_fetch(

    extension_store: &State<'_, Arc<ExtensionStore>>,

    mcp: State<'_, Arc<MCP>>,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<ExtensionNetworkRequest>(args.clone())

        .map_err(|error| format!("Invalid network.fetch args: {error}"))?;

    let response = ui_extension_network_proxy(extension_store.clone(), mcp, request)

        .await

        .map_err(|error| format!("Network fetch failed: {error}"))?;

    serde_json::to_value(response).map_err(|error| format!("Failed to serialize network response: {error}"))

}






fn bridge_operation_register(

    extension_id: &str,

    extension_store: &State<'_, Arc<ExtensionStore>>,

    operation_store: &State<'_, Arc<OperationRegistry>>,

    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<OperationRegisterRequest>(args.clone())

        .map_err(|error| format!("Invalid runtime.operation.register args: {error}"))?;

    if request.extension_id != extension_id {

        return Err(format!(

            "runtime.operation.register extension_id mismatch: expected '{extension_id}'"

        ));

    }

    let registration = register_operation(

        extension_store.inner().as_ref(),

        operation_store.inner().as_ref(),

        request,

    )

    .map_err(|error| format!("Operation register failed: {error}"))?;

    serde_json::to_value(registration)

        .map_err(|error| format!("Failed to serialize operation registration: {error}"))

}






async fn bridge_operation_execute(

    extension_id: &str,

    extension_store: &State<'_, Arc<ExtensionStore>>,

    operation_store: &State<'_, Arc<OperationRegistry>>,


    mcp: &MCP,
    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<OperationExecuteRequest>(args.clone())

        .map_err(|error| format!("Invalid runtime.operation.execute args: {error}"))?;

    if request.extension_id != extension_id {

        return Err(format!(

            "runtime.operation.execute extension_id mismatch: expected '{extension_id}'"

        ));

    }

    execute_operation(

        extension_store.inner().as_ref(),

        operation_store.inner().as_ref(),

        mcp.inner(),

        request,

    )

    .await

    .map_err(|error| format!("Operation execute failed: {error}"))

}




fn bridge_operation_list(

    operation_store: &State<'_, Arc<OperationRegistry>>,

    args: &Value,

) -> Result<Value, String> {

    let request = serde_json::from_value::<OperationListRequest>(args.clone())

        .map_err(|error| format!("Invalid runtime.operation.list args: {error}"))?;

    let operations = operation_store.inner().list(request.extension_id.as_deref());

    serde_json::to_value(operations)

        .map_err(|error| format!("Failed to serialize operation list: {error}"))

}



fn denied_response(error: impl Into<String>) -> BridgeInvokeResponse {

    BridgeInvokeResponse {

        ok: false,

        data: None,

        error: Some(error.into()),

    }

}



#[cfg(test)]

mod tests {

    use super::*;



    #[test]

    fn bridge_command_registry_contains_file_read() {

        assert!(supported_bridge_commands().contains(&"file.read"));

    }



    #[test]

    fn bridge_command_registry_exposes_phase_7_to_9_commands() {

        for command in [

            "extensions.query",

            "route.call",

            "storage.get",

            "storage.set",

            "storage.delete",

            "storage.clear",

            "network.fetch",

        ] {

            assert!(

                supported_bridge_commands().contains(&command),

                "bridge registry must expose '{command}'"

            );

        }

    }



    #[test]

    fn bridge_command_registry_exposes_operation_runtime_commands() {

        for command in [

            "runtime.operation.execute",

            "runtime.operation.register",

            "runtime.operation.list",

        ] {

            assert!(

                supported_bridge_commands().contains(&command),

                "bridge registry must expose '{command}'"

            );

        }

    }



    #[test]

    fn invoke_whitelist_checks_exact_match() {

        let caps = Some(CapabilityDeclaration {

            invoke: vec!["file.read".into()],

            ..Default::default()

        });

        assert!(invoke_whitelisted(&caps, "file.read"));

        assert!(!invoke_whitelisted(&caps, "file.write"));

        assert!(!invoke_whitelisted(&None, "file.read"));

    }



    #[test]

    fn event_pattern_whitelist_supports_exact_and_prefix_declarations() {

        let caps = Some(CapabilityDeclaration {

            events: vec!["session.*".into(), "extension.registry.changed".into()],

            ..Default::default()

        });

        assert!(event_pattern_whitelisted(&caps, "session.completed"));

        assert!(event_pattern_whitelisted(&caps, "extension.registry.changed"));

        assert!(!event_pattern_whitelisted(&caps, "project.changed"));

        assert!(!event_pattern_whitelisted(&caps, "session.*"));

    }



    #[test]

    fn event_pattern_whitelist_rejects_high_frequency_topics() {

        let caps = Some(CapabilityDeclaration {

            events: vec!["*".into()],

            ..Default::default()

        });

        assert!(!event_pattern_whitelisted(&caps, "agent.timeline"));

        assert!(!event_pattern_whitelisted(&caps, "terminal.output"));

        assert!(event_pattern_whitelisted(&caps, "project.changed"));

    }



    #[test]

    fn sandbox_request_actor_is_namespaced() {

        let request = sandbox_request("my.ext", OperationType::FileRead, "/a/b", None);

        assert_eq!(request.actor, "extension:my.ext");

        assert!(request.is_extension());

        assert_eq!(request.extension_id(), Some("my.ext"));

    }



    #[test]

    fn require_allowed_rejects_confirm_requests() {

        let allowed = CheckResult::allowed(crate::security::sandbox::permission::PermissionLevel::LightCheck);

        assert!(require_allowed(&allowed, "file.read").is_ok());



        let needs_confirm = CheckResult::needs_confirm(

            crate::security::sandbox::permission::PermissionLevel::UserConfirm,

            "confirm me",

        );

        assert!(require_allowed(&needs_confirm, "file.read").is_err());



        let denied = CheckResult::denied(

            crate::security::sandbox::permission::PermissionLevel::UserConfirm,

            "blocked",

        );

        assert!(require_allowed(&denied, "file.read").is_err());

    }

}



