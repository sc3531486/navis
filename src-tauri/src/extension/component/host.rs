//! host function 门禁（design/37 §七，阶段 C1-2）。
//!
//! host 接口是组件唯一出站通道，门禁在容器，不可绕过（fail-closed）：
//! - 每个出站调用先过组件 `capabilities` 白名单（未声明 = 不授权 = 调用即拒绝 + 审计）；
//! - `operation.execute` 复用 OperationRegistry（操作注册）+ Sandbox：构造
//!   `OperationRequest{operation_type, target, actor:"extension:{id}"}` 过
//!   `sandbox.check()`，`require_allowed` 拒绝 / 需确认一律 fail-closed（不弹确认），
//!   语义与 `extension::operation_runtime::require_allowed` 一致（该函数为模块私有，此处复刻）；
//! - `context/storage/network/event` 未接线时提供 fail-closed 默认（未授权或 not implemented）；
//! - `log` 是组件侧唯一低风险出站，直接写入 tracing（受控输出）。

use std::sync::Arc;

use serde_json::json;

use crate::extension::models::{ComponentCapabilities, ExtensionStatus};
use crate::extension::store::ExtensionStore;
use crate::extension::operation_runtime::OperationRegistry;
use crate::security::sandbox::{CheckResult, OperationRequest, Sandbox};

use super::bindings_host::navis::host::{context, event, log, network, operation, storage, types};

/// 组件实例化时注入的 host 状态（Store 数据，即 Linker 的 `T`）。
pub struct HostState {
    pub extension_id: String,
    pub component_id: String,
    /// 组件 capabilities 白名单（host 接口授予依据，fail-closed 判定）
    pub capabilities: ComponentCapabilities,
    pub sandbox: Arc<Sandbox>,
    pub operation_registry: Arc<OperationRegistry>,
    pub extension_store: Arc<ExtensionStore>,
}

impl HostState {
    /// 构造 host 状态。`capabilities` 来自组件 manifest 声明的白名单。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extension_id: String,
        component_id: String,
        capabilities: ComponentCapabilities,
        sandbox: Arc<Sandbox>,
        operation_registry: Arc<OperationRegistry>,
        extension_store: Arc<ExtensionStore>,
    ) -> Self {
        Self {
            extension_id,
            component_id,
            capabilities,
            sandbox,
            operation_registry,
            extension_store,
        }
    }
}

// ============================================================================
// 门禁原语（fail-closed）
// ============================================================================

/// 校验组件 invoke 能力白名单（对应 capabilities.invoke），未声明即拒绝 + 审计。
fn ensure_invoke(
    component_id: &str,
    capabilities: &ComponentCapabilities,
    action: &str,
) -> Result<(), String> {
    if capabilities.invoke.iter().any(|item| item == action) {
        Ok(())
    } else {
        tracing::warn!(
            component_id = %component_id,
            action = %action,
            "Component invoke capability not granted (fail-closed)"
        );
        Err(format!(
            "Component '{component_id}' not granted invoke capability '{action}'"
        ))
    }
}

/// 校验 storage scope 白名单（对应 capabilities.storage），未声明即拒绝 + 审计。
fn ensure_storage_scope(
    component_id: &str,
    capabilities: &ComponentCapabilities,
    scope: &str,
) -> Result<(), String> {
    if capabilities.storage.iter().any(|item| item == scope) {
        Ok(())
    } else {
        tracing::warn!(
            component_id = %component_id,
            scope = %scope,
            "Component storage scope not granted (fail-closed)"
        );
        Err(format!(
            "Component '{component_id}' not granted storage scope '{scope}'"
        ))
    }
}

/// 校验 event pattern 白名单（对应 capabilities.events；支持尾部 `*` 前缀通配）。
fn ensure_event_pattern(
    component_id: &str,
    capabilities: &ComponentCapabilities,
    pattern: &str,
) -> Result<(), String> {
    let granted = capabilities.events.iter().any(|allowed| {
        if let Some(prefix) = allowed.strip_suffix('*') {
            pattern.starts_with(prefix)
        } else {
            pattern == allowed
        }
    });
    if granted {
        Ok(())
    } else {
        tracing::warn!(
            component_id = %component_id,
            pattern = %pattern,
            "Component event pattern not granted (fail-closed)"
        );
        Err(format!(
            "Component '{component_id}' not granted event pattern '{pattern}'"
        ))
    }
}

/// 扩展必须已启用（fail-closed）。
fn ensure_extension_enabled(store: &ExtensionStore, extension_id: &str) -> Result<(), String> {
    let state = store
        .get(extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }
    Ok(())
}

/// Sandbox 门禁结果判定：拒绝 / 需确认一律 fail-closed（不弹确认）。
///
/// 语义与 `extension::operation_runtime::require_allowed` 一致；该函数为模块私有不可改，
/// 此处按容器职责复刻判定逻辑。
pub(crate) fn require_allowed(result: &CheckResult, operation: &str) -> Result<(), String> {
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
// types host 接口（共享类型：bindgen 要求宿主实现，空 trait 无方法）
// ============================================================================

impl types::Host for HostState {}

// ============================================================================
// operation host 接口（受控操作执行，门禁在容器）
// ============================================================================

impl operation::Host for HostState {
    fn execute(&mut self, op: types::OperationRequest) -> Result<String, String> {
        // 1) 组件 capability 白名单
        ensure_invoke(&self.component_id, &self.capabilities, "operation.execute")?;
        // 2) 扩展必须已启用
        ensure_extension_enabled(&self.extension_store, &self.extension_id)?;
        // 3) 操作必须已注册且命名空间与调用方扩展一致（防跨扩展冒用）
        let registration = self
            .operation_registry
            .get(&op.operation)
            .ok_or_else(|| format!("Operation '{}' is not registered", op.operation))?;
        if registration.extension_id != self.extension_id {
            return Err(format!(
                "Operation '{}' belongs to extension '{}', not '{}'",
                registration.id, registration.extension_id, self.extension_id
            ));
        }
        // 4) 容器级门禁：按操作声明的 Sandbox OperationType + target + actor=extension:{id}
        let request = OperationRequest::new(
            registration.operation_type.clone(),
            op.target.clone(),
            format!("extension:{}", self.extension_id),
        );
        let result = self
            .sandbox
            .check(&request)
            .map_err(|error| format!("Sandbox check failed: {error}"))?;
        require_allowed(&result, &registration.id)?;

        // 门禁通过：返回"需要扩展处理"信号（与 operation_runtime Extension handler 语义一致）。
        tracing::info!(
            operation_id = %registration.id,
            extension_id = %self.extension_id,
            component_id = %self.component_id,
            "Controlled operation executed (host gate passed)"
        );
        Ok(json!({
            "status": "extension_handler",
            "operationId": registration.id,
            "params": op.params.unwrap_or_default(),
        })
        .to_string())
    }

    fn list_operations(&mut self) -> Vec<types::OperationDescription> {
        // fail-closed：未授权时返回空列表，不泄漏任何操作信息。
        if ensure_invoke(&self.component_id, &self.capabilities, "operation.list").is_err() {
            return Vec::new();
        }
        self.operation_registry
            .list(Some(&self.extension_id))
            .into_iter()
            .map(|reg| types::OperationDescription {
                id: reg.id,
                label: reg.label,
            })
            .collect()
    }
}

// ============================================================================
// context / storage / network / event host 接口（fail-closed 默认）
// ============================================================================

impl context::Host for HostState {
    fn get_session(&mut self) -> Result<types::SessionSnapshot, String> {
        ensure_invoke(&self.component_id, &self.capabilities, "context.getSession")?;
        Err("context.getSession not implemented".to_string())
    }

    fn get_active_project(&mut self) -> Result<types::ProjectSnapshot, String> {
        ensure_invoke(
            &self.component_id,
            &self.capabilities,
            "context.getActiveProject",
        )?;
        Err("context.getActiveProject not implemented".to_string())
    }
}

impl storage::Host for HostState {
    fn get(&mut self, key: String, scope: String) -> Result<Option<String>, String> {
        ensure_storage_scope(&self.component_id, &self.capabilities, &scope)?;
        Err(format!(
            "storage.get not implemented (scope '{scope}', key '{key}')"
        ))
    }

    fn set(&mut self, key: String, _value: String, scope: String) -> Result<(), String> {
        ensure_storage_scope(&self.component_id, &self.capabilities, &scope)?;
        Err(format!(
            "storage.set not implemented (scope '{scope}', key '{key}')"
        ))
    }

    fn delete(&mut self, key: String, scope: String) -> Result<(), String> {
        ensure_storage_scope(&self.component_id, &self.capabilities, &scope)?;
        Err(format!(
            "storage.delete not implemented (scope '{scope}', key '{key}')"
        ))
    }
}

impl network::Host for HostState {
    fn fetch(&mut self, request: types::HttpRequest) -> Result<types::HttpResponse, String> {
        // 未声明 network 能力 = 不授权（fail-closed）。
        if self.capabilities.network.is_none() {
            tracing::warn!(
                component_id = %self.component_id,
                "Component network capability not granted (fail-closed)"
            );
            return Err(format!(
                "Component '{}' network capability not granted",
                self.component_id
            ));
        }
        Err(format!("network.fetch not implemented (url '{}')", request.url))
    }
}

impl event::Host for HostState {
    fn subscribe(&mut self, pattern: String) -> Result<types::Subscription, String> {
        ensure_event_pattern(&self.component_id, &self.capabilities, &pattern)?;
        Err("event.subscribe not implemented".to_string())
    }

    fn emit(&mut self, topic: String, _payload: String) -> Result<(), String> {
        ensure_event_pattern(&self.component_id, &self.capabilities, &topic)?;
        Err("event.emit not implemented".to_string())
    }
}

// ============================================================================
// log host 接口（受控输出，直接写入 tracing）
// ============================================================================

impl log::Host for HostState {
    fn write(&mut self, level: types::LogLevel, message: String) -> Result<(), String> {
        // 组件侧受控输出：写入 tracing（tracing 宏 target 须为编译期常量，
        // 扩展/组件身份以字段承载）。
        let extension_id = &self.extension_id;
        let component_id = &self.component_id;
        match level {
            types::LogLevel::Trace => tracing::trace!(
                extension_id = %extension_id,
                component_id = %component_id,
                "{message}"
            ),
            types::LogLevel::Debug => tracing::debug!(
                extension_id = %extension_id,
                component_id = %component_id,
                "{message}"
            ),
            types::LogLevel::Info => tracing::info!(
                extension_id = %extension_id,
                component_id = %component_id,
                "{message}"
            ),
            types::LogLevel::Warn => tracing::warn!(
                extension_id = %extension_id,
                component_id = %component_id,
                "{message}"
            ),
            types::LogLevel::Error => tracing::error!(
                extension_id = %extension_id,
                component_id = %component_id,
                "{message}"
            ),
        }
        Ok(())
    }
}
