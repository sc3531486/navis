//! 扩展间路由 IPC。
//!
//! 只开放显式导出（extensionExports）并且调用方 capabilities.extensionCalls
//! 允许的面。用于 SendMessage / 跨扩展 view.open / command.execute 的宿主裁决。
//!
//! 审计：每一条路由决策（允许/拒绝）都写入 kernel AuditRecorder，action 使用
//! 命名空间 `extension.route.call`，policy_decision 记录 caller/target/action/结果。

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;

use crate::extension::{ExtensionStatus, ExtensionStore};
use crate::kernel::{AuditRecord, AuditRecorder, AuditStatus, KernelContext, KernelScope};
use crate::domains::ai_platform::mcp::MCP;

/// 跨扩展路由审计的 action 命名空间。
const ROUTE_AUDIT_ACTION: &str = "extension.route.call";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRouteRequest {
    pub caller_extension_id: String,
    pub target: String,
    pub action: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRouteResponse {
    pub ok: bool,
    #[serde(default)]
    pub data: Value,
}

fn split_target(target: &str) -> Result<(&str, &str), String> {
    target
        .split_once(':')
        .ok_or_else(|| "Extension route target must use '{extensionId}:{localId}'".to_string())
}

fn caller_allowed(store: &ExtensionStore, caller_id: &str, target_extension_id: &str, action: &str) -> Result<(), String> {
    let caller = store
        .get(caller_id)
        .ok_or_else(|| format!("Caller extension '{caller_id}' is not installed"))?;
    if caller.status != ExtensionStatus::Enabled {
        return Err(format!("Caller extension '{caller_id}' is not enabled"));
    }
    let allowed = caller
        .manifest
        .contributes
        .capabilities
        .as_ref()
        .is_some_and(|caps| {
            caps.extension_calls.iter().any(|call| {
                (call.target == target_extension_id || call.target == "*")
                    && (call.actions.iter().any(|item| item == action) || call.actions.iter().any(|item| item == "*"))
            })
        });
    if allowed { Ok(()) } else { Err(format!("Caller extension '{caller_id}' is not allowed to perform '{action}' on '{target_extension_id}'")) }
}

fn target_exported(store: &ExtensionStore, target_extension_id: &str, local_id: &str, action: &str) -> Result<(), String> {
    let target = store
        .get(target_extension_id)
        .ok_or_else(|| format!("Target extension '{target_extension_id}' is not installed"))?;
    if target.status != ExtensionStatus::Enabled {
        return Err(format!("Target extension '{target_extension_id}' is not enabled"));
    }
    let exports = target.manifest.contributes.extension_exports.as_ref();
    let exported = match action {
        "view.open" | "view.toggle" => exports.is_some_and(|exports| exports.views.iter().any(|view| view == local_id)),
        "command.execute" | "message.send" => exports.is_some_and(|exports| exports.commands.iter().any(|command| command == local_id)),
        _ => false,
    };
    if exported { Ok(()) } else { Err(format!("Target '{target_extension_id}:{local_id}' does not export action '{action}'")) }
}

/// 记录跨扩展路由审计（失败仅打日志，不影响路由结果）。
fn record_route_audit(
    audit: &AuditRecorder,
    request: &ExtensionRouteRequest,
    status: AuditStatus,
    reason: Option<&str>,
) {
    let ctx = KernelContext::new("extension.router", KernelScope::global());
    let decision = json!({
        "callerExtensionId": request.caller_extension_id,
        "target": request.target,
        "action": request.action,
        "allowed": status == AuditStatus::Success,
        "reason": reason,
    });
    let record = AuditRecord::new(&ctx, uuid::Uuid::new_v4().to_string(), ROUTE_AUDIT_ACTION, status)
        .with_policy_decision(decision);
    if let Err(error) = audit.record_owned(record) {
        tracing::warn!(error = %error, "Failed to record extension route audit");
    }
}

/// 路由裁决（纯函数，便于单元测试）。
///
/// 双端授权 + 显式导出校验；任何拒绝都写入审计（fail-closed）。
fn route_call(
    store: &ExtensionStore,
    audit: &AuditRecorder,
    request: ExtensionRouteRequest,
) -> Result<ExtensionRouteResponse, String> {
    let decision: Result<ExtensionRouteResponse, String> = (|| {
        let (target_extension_id, local_id) = split_target(&request.target)?;
        caller_allowed(store, &request.caller_extension_id, target_extension_id, &request.action)?;
        target_exported(store, target_extension_id, local_id, &request.action)?;
        Ok(ExtensionRouteResponse {
            ok: true,
            data: json!({
                "targetExtensionId": target_extension_id,
                "localId": local_id,
                "action": request.action,
                "payload": request.payload,
            }),
        })
    })();

    match &decision {
        Ok(_) => record_route_audit(audit, &request, AuditStatus::Success, None),
        Err(reason) => record_route_audit(audit, &request, AuditStatus::Failed, Some(reason)),
    }
    decision
}

#[tauri::command]
pub fn ui_extension_route_call(
    extension_store: State<'_, Arc<ExtensionStore>>,
    mcp: State<'_, Arc<MCP>>,
    request: ExtensionRouteRequest,
) -> Result<ExtensionRouteResponse, String> {
    // 通过 MCP 宿主持有的 Sandbox 获取注入的 kernel AuditRecorder
    // （app/mod.rs 在组装 Sandbox 时已注入 StorageAuditSink）。
    let audit = mcp.sandbox().audit_recorder();
    route_call(extension_store.inner().as_ref(), &audit, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{
        CapabilityDeclaration, ExtensionCall, ExtensionContributes, ExtensionExports,
        ExtensionManifest, ExtensionPermissions, ExtensionState,
    };
    use crate::kernel::{EventBus, InMemoryAuditSink, InMemoryEventBus};
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

    fn register(
        store: &Arc<ExtensionStore>,
        id: &str,
        status: ExtensionStatus,
        contributes: ExtensionContributes,
    ) {
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
                    contributes,
                },
                install_path: PathBuf::from(format!("/extensions/{id}")),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();
    }

    fn test_audit() -> (AuditRecorder, Arc<InMemoryAuditSink>) {
        let sink = Arc::new(InMemoryAuditSink::new());
        (AuditRecorder::new(sink.clone()), sink)
    }

    fn caller_contributes(target: &str, actions: Vec<&str>) -> ExtensionContributes {
        ExtensionContributes {
            capabilities: Some(CapabilityDeclaration {
                extension_calls: vec![ExtensionCall {
                    target: target.to_string(),
                    actions: actions.into_iter().map(str::to_string).collect(),
                }],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn target_contributes(views: Vec<&str>, commands: Vec<&str>) -> ExtensionContributes {
        ExtensionContributes {
            extension_exports: Some(ExtensionExports {
                views: views.into_iter().map(str::to_string).collect(),
                commands: commands.into_iter().map(str::to_string).collect(),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn split_target_requires_namespaced_target() {
        assert_eq!(split_target("a:b").unwrap(), ("a", "b"));
        assert!(split_target("missing-separator").is_err());
    }

    #[test]
    fn caller_allowed_honors_exact_and_wildcard() {
        let store = test_store();
        register(&store, "caller", ExtensionStatus::Enabled, caller_contributes("target", vec!["view.open"]));
        register(&store, "wildcard", ExtensionStatus::Enabled, caller_contributes("*", vec!["*"]));
        register(&store, "disabled", ExtensionStatus::Disabled, caller_contributes("target", vec!["view.open"]));

        // 精确匹配 target + action
        assert!(caller_allowed(&store, "caller", "target", "view.open").is_ok());
        // 未授权 action
        assert!(caller_allowed(&store, "caller", "target", "command.execute").is_err());
        // target `*` + action `*` 全通配
        assert!(caller_allowed(&store, "wildcard", "target", "view.open").is_ok());
        assert!(caller_allowed(&store, "wildcard", "other", "command.execute").is_ok());
        // caller 未启用
        assert!(caller_allowed(&store, "disabled", "target", "view.open").is_err());
        // caller 未安装
        assert!(caller_allowed(&store, "missing", "target", "view.open").is_err());
    }

    #[test]
    fn target_exported_requires_enabled_target_and_export() {
        let store = test_store();
        register(&store, "target", ExtensionStatus::Enabled, target_contributes(vec!["panel"], vec!["run"]));
        register(&store, "stale", ExtensionStatus::Disabled, target_contributes(vec!["panel"], vec!["run"]));

        // 命中显式导出
        assert!(target_exported(&store, "target", "panel", "view.open").is_ok());
        assert!(target_exported(&store, "target", "run", "command.execute").is_ok());
        // 未导出 localId
        assert!(target_exported(&store, "target", "hidden", "view.open").is_err());
        // 未知 action fail-closed
        assert!(target_exported(&store, "target", "panel", "unknown.action").is_err());
        // target 未启用
        assert!(target_exported(&store, "stale", "panel", "view.open").is_err());
        // target 未安装
        assert!(target_exported(&store, "missing", "panel", "view.open").is_err());
    }

    #[test]
    fn route_call_authorizes_both_sides_and_audits_success() {
        let store = test_store();
        register(&store, "caller", ExtensionStatus::Enabled, caller_contributes("target", vec!["view.open"]));
        register(&store, "target", ExtensionStatus::Enabled, target_contributes(vec!["panel"], vec![]));

        let (audit, sink) = test_audit();
        let response = route_call(
            &store,
            &audit,
            ExtensionRouteRequest {
                caller_extension_id: "caller".into(),
                target: "target:panel".into(),
                action: "view.open".into(),
                payload: json!({}),
            },
        )
        .unwrap();

        assert!(response.ok);
        assert_eq!(response.data["targetExtensionId"], "target");
        assert_eq!(response.data["localId"], "panel");

        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].action, ROUTE_AUDIT_ACTION);
        assert_eq!(records[0].status, AuditStatus::Success);
        assert_eq!(
            records[0].policy_decision.as_ref().unwrap()["allowed"],
            json!(true)
        );
    }

    #[test]
    fn route_call_rejects_unexported_target_and_audits_failure() {
        let store = test_store();
        register(&store, "caller", ExtensionStatus::Enabled, caller_contributes("target", vec!["command.execute"]));
        register(&store, "target", ExtensionStatus::Enabled, target_contributes(vec![], vec!["run"]));

        let (audit, sink) = test_audit();
        let error = route_call(
            &store,
            &audit,
            ExtensionRouteRequest {
                caller_extension_id: "caller".into(),
                target: "target:hidden".into(),
                action: "command.execute".into(),
                payload: json!({}),
            },
        )
        .unwrap_err();

        assert!(error.contains("does not export action"));

        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, AuditStatus::Failed);
        assert_eq!(
            records[0].policy_decision.as_ref().unwrap()["allowed"],
            json!(false)
        );
        assert_eq!(
            records[0].policy_decision.as_ref().unwrap()["reason"],
            json!(error)
        );
    }

    #[test]
    fn route_call_fails_closed_when_target_disabled() {
        let store = test_store();
        register(&store, "caller", ExtensionStatus::Enabled, caller_contributes("target", vec!["view.open"]));
        register(&store, "target", ExtensionStatus::Disabled, target_contributes(vec!["panel"], vec![]));

        let (audit, sink) = test_audit();
        let error = route_call(
            &store,
            &audit,
            ExtensionRouteRequest {
                caller_extension_id: "caller".into(),
                target: "target:panel".into(),
                action: "view.open".into(),
                payload: json!({}),
            },
        )
        .unwrap_err();

        assert!(error.contains("is not enabled"));

        let records = sink.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, AuditStatus::Failed);
        assert_eq!(
            records[0].policy_decision.as_ref().unwrap()["callerExtensionId"],
            json!("caller")
        );
    }
}