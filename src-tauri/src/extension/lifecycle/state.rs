//! 扩展启用/禁用逻辑 + ExtensionState 管理
//!
//! 包含 `enable`、`disable` 及其所有内部辅助方法。D1c：enable 经 Cordis fiber
//! 安装（apply 在 fiber 内真实注册贡献并登记 cleanup disposer），disable 经
//! fiber dispose 逆序撤销；runtime_handles 账本保留残余重试语义。

use anyhow::Result;

use super::contributions::UiContributionRegistrar;
use super::cordis::ExtensionCordisPlugin;
use super::families::{ContributionContext, ContributionFamilyRegistry};
use super::register::{
    apply_mcp_tool_overrides, extension_mcp_server_config, extension_tool_definition,
    register_lsp_languages,
};
use super::{
    EventSubscriptionPort, ExtensionLifecycle, ExtensionRuntimeHandle, ExtensionSubscriptionLedger,
    GatewayCapabilityPort, LspCapabilityPort, McpCapabilityPort,
};
use crate::extension::component::ComponentRegistry;
use crate::extension::lifecycle::cordis::BackendProcessPort;
use crate::extension::models::{ExtensionContributes, ExtensionManifest, ExtensionStatus};
use crate::extension::provider_validation::ExtensionProviderValidationPort;
use crate::extension::skills::Skills;
use crate::extension::store::ExtensionStore;
use crate::kernel::{EventEnvelope, KernelContext, KernelScope};
use cordis::{Context, PluginOutput};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use triomphe::Arc as SharedArc;

// ── Cordis apply/cleanup 共享状态 ────────────────────────────────────────────

/// Cordis apply/cleanup 共享状态。
///
/// 从 `ExtensionLifecycle` 快照的 owned Arcs。apply（enable 安装 fiber）与
/// cleanup disposer（disable / fiber 撤销）只依赖这份快照而不借用 lifecycle
/// 本身，因此 `'static` 闭包可直接捕获，fiber dispose 时即使 lifecycle 已被
/// drop 也能安全执行。
#[derive(Clone)]
pub(crate) struct ApplyState {
    pub(crate) store: Arc<ExtensionStore>,
    pub(crate) skills: Option<Arc<Mutex<Skills>>>,
    pub(crate) mcp: Option<Arc<dyn McpCapabilityPort>>,
    pub(crate) lsp: Option<Arc<dyn LspCapabilityPort>>,
    pub(crate) backend_manager: Option<Arc<dyn BackendProcessPort>>,
    pub(crate) component_registry: Option<Arc<ComponentRegistry>>,
    pub(crate) gateway: Option<Arc<dyn GatewayCapabilityPort>>,
    pub(crate) provider_validation: Option<Arc<dyn ExtensionProviderValidationPort>>,
    pub(crate) event_subscriptions: Option<Arc<dyn EventSubscriptionPort>>,
    pub(crate) subscription_ledger: Arc<Mutex<ExtensionSubscriptionLedger>>,
    pub(crate) ui_contributions: Arc<Mutex<UiContributionRegistrar>>,
    pub(crate) contribution_families: Arc<ContributionFamilyRegistry>,
    pub(crate) runtime_handles: Arc<Mutex<HashMap<String, ExtensionRuntimeHandle>>>,
}

impl ApplyState {
    /// 从生命周期快照 owned Arcs。
    pub(crate) fn from_lifecycle(lifecycle: &ExtensionLifecycle) -> Self {
        Self {
            store: Arc::clone(&lifecycle.store),
            skills: lifecycle.skills.clone(),
            mcp: lifecycle.mcp.clone(),
            lsp: lifecycle.lsp.clone(),
            backend_manager: lifecycle.backend_manager.clone(),
            component_registry: lifecycle.component_registry.clone(),
            gateway: lifecycle.gateway.clone(),
            provider_validation: lifecycle.provider_validation.clone(),
            event_subscriptions: lifecycle.event_subscriptions.clone(),
            subscription_ledger: Arc::clone(&lifecycle.subscription_ledger),
            ui_contributions: Arc::clone(&lifecycle.ui_contributions),
            contribution_families: Arc::clone(&lifecycle.contribution_families),
            runtime_handles: Arc::clone(&lifecycle.runtime_handles),
        }
    }

    /// 构造 contribution family 上下文（与 `ExtensionLifecycle::contribution_context` 一致）。
    pub(crate) fn context<'a>(&'a self, extension_id: &'a str) -> ContributionContext<'a> {
        ContributionContext {
            extension_id,
            store: self.store.as_ref(),
            gateway: self.gateway.as_deref(),
            provider_validation: self.provider_validation.as_deref(),
            ui_contributions: self.ui_contributions.as_ref(),
            lsp: self.lsp.as_deref(),
            backend_manager: self.backend_manager.as_deref(),
            component_registry: self.component_registry.as_deref(),
        }
    }
}

/// Lock the two lifecycle ledgers in the only supported order.
fn lock_runtime_and_subscription_ledger(
    state: &ApplyState,
) -> anyhow::Result<(
    MutexGuard<'_, HashMap<String, ExtensionRuntimeHandle>>,
    MutexGuard<'_, ExtensionSubscriptionLedger>,
)> {
    let handles = state.runtime_handles.lock().map_err(|error| {
        anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
    })?;
    let ledger = state.subscription_ledger.lock().map_err(|error| {
        anyhow::anyhow!("Failed to lock Extension subscription ledger: {error}")
    })?;
    Ok((handles, ledger))
}

fn has_runtime_handle(state: &ApplyState, extension_id: &str) -> Result<bool> {
    let (handles, ledger) = lock_runtime_and_subscription_ledger(state)?;
    Ok(handles.contains_key(extension_id) || ledger.contains_extension(extension_id))
}

/// 向句柄账本登记一条事实；句柄必须已存在（apply 早期插入的空句柄）。
fn record_fact(
    handles: &Mutex<HashMap<String, ExtensionRuntimeHandle>>,
    extension_id: &str,
    record: impl FnOnce(&mut ExtensionRuntimeHandle),
) -> Result<()> {
    let mut handles = handles.lock().map_err(|error| {
        anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
    })?;
    let handle = handles.get_mut(extension_id).ok_or_else(|| {
        anyhow::anyhow!("Extension '{}' has no runtime handle", extension_id)
    })?;
    record(handle);
    Ok(())
}

/// 在 Cordis fiber apply 内执行真实注册（D1c）。
///
/// 副作用分两类：
/// - 声明资源（MCP / Skills / LSP / family plan）：提交到 `runtime_handles`
///   句柄账本；
/// - cleanup disposer：apply 一开始就经 `ctx.effect` 登记，持有 `ApplyState`
///   快照；fiber dispose（正常禁用）与 apply 失败（Cordis 自动逆序撤销）都会
///   触发它，按句柄事实逆序清理宿主资源并移除账本条目。
pub(crate) fn apply_extension_fiber(
    state: &ApplyState,
    ctx: &Context,
    manifest: &ExtensionManifest,
) -> Result<PluginOutput> {
    let extension_id = manifest.id.clone();
    let contributes = manifest.contributes.clone();

    // 幂等守卫：扩展必须尚无运行时账本，才允许开启新 fiber 事务。
    {
        let (handles, ledger) = lock_runtime_and_subscription_ledger(state)?;
        if handles.contains_key(&extension_id) || ledger.contains_extension(&extension_id) {
            return Err(anyhow::anyhow!(
                "Extension '{}' already owns runtime resources",
                extension_id
            ));
        }
    }

    // 先在账本登记空句柄：apply 失败时 Cordis 逆序执行 disposer，disposer
    // 需要读到句柄事实来移除条目；成功则句柄在 fiber 生命周期内保持完整。
    state
        .runtime_handles
        .lock()
        .map_err(|error| anyhow::anyhow!("Failed to lock Extension runtime handles: {error}"))?
        .insert(extension_id.clone(), ExtensionRuntimeHandle::default());

    // 登记 cleanup disposer：必须在任何注册之前，保证 apply 失败也触发回滚。
    let disposer_state = state.clone();
    let disposer_id = extension_id.clone();
    ctx.effect(
        format!("extension:{extension_id}:cleanup"),
        move || -> cordis::Result<()> {
            cleanup_runtime_resources(&disposer_state, &disposer_id).map_err(|error| {
                cordis::CordisError::with_message(
                    cordis::ErrorCode::Plugin,
                    format!("extension `{disposer_id}` cleanup failed: {error}"),
                )
            })
        },
    )
    .map_err(|error| {
        anyhow::anyhow!("Failed to register cleanup effect for extension '{extension_id}': {error}")
    })?;

    // MCP Servers
    if let Some(ref mcp_servers) = contributes.mcp_servers {
        for server in mcp_servers {
            tracing::debug!(
                extension_id = %extension_id,
                mcp_server = %server.name,
                "Registering MCP server"
            );
            let mcp = state.mcp.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Extension '{}' declares MCP server '{}' but MCP host is not available",
                    extension_id,
                    server.name
                )
            })?;
            let config = extension_mcp_server_config(&extension_id, server)?;
            let server_id = config.id.clone();
            let auto_start = config.auto_start;
            mcp.add_server(config)?;
            record_fact(&state.runtime_handles, &extension_id, |handle| {
                handle.mcp_servers.push(server_id.clone());
                handle.projection.contribution_counts.mcp_servers += 1;
            })?;
            if auto_start {
                if let Err(error) = mcp.start_server(&server_id) {
                    return Err(error);
                }
            }
        }
    }

    // MCP tool overrides
    if let Some(ref overrides) = contributes.mcp_tool_overrides {
        let mcp = state.mcp.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Extension '{}' declares MCP tool overrides but MCP host is not available",
                extension_id
            )
        })?;
        let mut handles = state.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        let handle = handles.get_mut(&extension_id).ok_or_else(|| {
            anyhow::anyhow!("Extension '{}' has no runtime handle", extension_id)
        })?;
        apply_mcp_tool_overrides(&**mcp, &extension_id, overrides, handle)?;
    }

    // 扩展声明的 MCP 工具。tool_server_id 先入账本，注册中途失败也能精确撤销。
    if let Some(ref tools) = contributes.tools {
        let mcp = state.mcp.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Extension '{}' declares tools but MCP host is not available",
                extension_id
            )
        })?;
        let tool_server_id = super::extension_tool_server_id(&extension_id);
        record_fact(&state.runtime_handles, &extension_id, |handle| {
            handle.tool_server_ids.push(tool_server_id.clone());
        })?;
        for tool_reg in tools {
            tracing::debug!(
                extension_id = %extension_id,
                tool_name = %tool_reg.name,
                "Registering extension tool"
            );
            let tool_def = extension_tool_definition(&extension_id, tool_reg)?;
            if let Err(error) = mcp.register_tool(tool_def) {
                return Err(error);
            }
        }
    }

    // Skills
    if let Some(ref skills) = contributes.skills {
        let skills_host = state.skills.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Skills host is not available for extension `{extension_id}`")
        })?;
        for skill in skills {
            tracing::debug!(
                extension_id = %extension_id,
                skill = %skill.id,
                "Registering skill"
            );
            if let Err(error) = skills_host
                .lock()
                .map_err(|error| anyhow::anyhow!("Failed to lock Skills state: {}", error))?
                .register_extension_skill(&extension_id, skill)
            {
                return Err(error);
            }
            record_fact(&state.runtime_handles, &extension_id, |handle| {
                handle.skills.push(skill.id.clone());
            })?;
        }
    }

    // LSP languages
    if let Some(ref languages) = contributes.languages {
        let lsp = state.lsp.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Extension '{}' declares LSP languages but LSP host is not available",
                extension_id
            )
        })?;
        let mut handles = state.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        let handle = handles.get_mut(&extension_id).ok_or_else(|| {
            anyhow::anyhow!("Extension '{}' has no runtime handle", extension_id)
        })?;
        register_lsp_languages(&extension_id, languages, &**lsp, handle)?;
    }

    // Contribution families own normalize/validate/prepare/commit. The
    // lifecycle only orchestrates the prepared plan.
    let context = state.context(&extension_id);
    let plan = state
        .contribution_families
        .prepare_plan(&context, &contributes)?;
    {
        let mut handles = state.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        let handle = handles.get_mut(&extension_id).ok_or_else(|| {
            anyhow::anyhow!("Extension '{}' has no runtime handle", extension_id)
        })?;
        plan.commit(&context, handle)?;
    }

    // Hooks 在扩展进入 Enabled 后登记到 ExtensionStore 声明索引；
    // 这里不加载/执行 hook 模块。

    Ok(PluginOutput::none())
}

/// 按句柄账本事实逆序清理宿主资源（fiber disposer / do_disable 手动路径）。
///
/// 每个资源只有在宿主清理成功后才从句柄移除；清理失败的事实保留在句柄/ledger
/// 中，作为残余资源供下次 retry 精确消费。全部清理完成且句柄为空时移除账本
/// 条目。任何清理错误都会汇总返回。
fn cleanup_runtime_resources(state: &ApplyState, extension_id: &str) -> Result<()> {
    let mut cleanup_errors = Vec::new();
    let (mut runtime_handle, subscription_records) = {
        let (handles, ledger) = lock_runtime_and_subscription_ledger(state)?;
        (
            handles.get(extension_id).cloned(),
            ledger.records(extension_id),
        )
    };
    if !subscription_records.is_empty() {
        if let Some(port) = &state.event_subscriptions {
            for record in subscription_records.iter().rev() {
                if let Err(error) =
                    port.unsubscribe_extension(extension_id, &record.subscription_id)
                {
                    cleanup_errors.push(format!(
                        "Event subscription '{}': {}",
                        record.subscription_id, error
                    ));
                } else {
                    let mut ledger = state.subscription_ledger.lock().map_err(|error| {
                        anyhow::anyhow!(
                            "Failed to lock Extension subscription ledger after cleanup: {error}"
                        )
                    })?;
                    ledger.remove(extension_id, &record.subscription_id);
                }
            }
        } else {
            cleanup_errors.push(
                "Event subscriptions exist but Extension event host is not available".to_string(),
            );
        }
    }

    if let Some(handle) = runtime_handle.as_mut() {
        let context = state.context(extension_id);
        state
            .contribution_families
            .disable(&context, handle, &mut cleanup_errors);
    }

    let has_ui_registration = state
        .ui_contributions
        .lock()
        .map_err(|error| {
            anyhow::anyhow!("Failed to lock UI contribution registrar during cleanup: {error}")
        })?
        .contains_extension(extension_id);
    let has_runtime_resources = runtime_handle.is_some()
        || !subscription_records.is_empty()
        || has_ui_registration;
    if !has_runtime_resources && has_runtime_handle(state, extension_id)? {
        cleanup_errors.push(format!(
            "Extension '{}' has residual lifecycle resources without a runtime ledger",
            extension_id
        ));
    }

    if let Some(handle) = runtime_handle.as_mut() {
        if let Some(mcp) = &state.mcp {
            for (server_id, tool_name) in handle.tool_overrides.clone().into_iter().rev() {
                if let Err(error) = mcp.remove_tool_override(extension_id, &server_id, &tool_name) {
                    cleanup_errors.push(format!(
                        "MCP tool override '{}:{}': {}",
                        server_id, tool_name, error
                    ));
                } else {
                    handle.tool_overrides.retain(|candidate| {
                        candidate != &(server_id.clone(), tool_name.clone())
                    });
                }
            }
            for server_id in handle.tool_server_ids.clone().into_iter().rev() {
                if let Err(error) = mcp.unregister_server_tools(&server_id) {
                    cleanup_errors.push(format!("MCP tools '{}': {}", server_id, error));
                } else {
                    handle
                        .tool_server_ids
                        .retain(|candidate| candidate != &server_id);
                }
            }
            for server_id in handle.mcp_servers.clone().into_iter().rev() {
                if let Err(error) = mcp.remove_server(&server_id) {
                    cleanup_errors.push(format!("MCP server '{}': {}", server_id, error));
                } else {
                    handle
                        .mcp_servers
                        .retain(|candidate| candidate != &server_id);
                }
            }
        } else if !handle.tool_overrides.is_empty()
            || !handle.tool_server_ids.is_empty()
            || !handle.mcp_servers.is_empty()
        {
            cleanup_errors
                .push("MCP resources exist but MCP host is not available".to_string());
        }

        if !handle.skills.is_empty() {
            match state.skills.as_ref() {
                Some(skills_host) => match skills_host.lock() {
                    Ok(mut skills_state) => {
                        for skill_id in handle.skills.clone().into_iter().rev() {
                            if let Err(error) =
                                skills_state.unregister_extension_skill(extension_id, &skill_id)
                            {
                                cleanup_errors.push(format!("Skill '{}': {}", skill_id, error));
                            } else {
                                handle.skills.retain(|candidate| candidate != &skill_id);
                            }
                        }
                    }
                    Err(error) => cleanup_errors.push(format!("Skills state lock: {error}")),
                },
                None => cleanup_errors
                    .push("Skills host is not available for skill cleanup".to_string()),
            }
        }

        if let Some(lsp) = &state.lsp {
            for language_id in handle.languages.clone().into_iter().rev() {
                if let Err(error) = lsp.unregister_language(&language_id, extension_id) {
                    cleanup_errors.push(format!("LSP language '{}': {}", language_id, error));
                } else {
                    handle
                        .languages
                        .retain(|candidate| candidate != &language_id);
                }
            }
        } else if !handle.languages.is_empty() {
            cleanup_errors
                .push("LSP resources exist but LSP host is not available".to_string());
        }
    }

    if let Some(handle) = runtime_handle {
        let mut handle = handle;
        handle.reconcile_projection();
        let mut handles = state.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles after cleanup: {error}")
        })?;
        if cleanup_errors.is_empty() && handle.is_empty() {
            handles.remove(extension_id);
        } else {
            handles.insert(extension_id.to_string(), handle);
        }
    }

    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Extension '{}' cleanup failed: {}",
            extension_id,
            cleanup_errors.join("; ")
        ))
    }
}

impl ExtensionLifecycle {
    /// 启用扩展
    ///
    /// 状态转换：Installed/Disabled/Error -> Loading -> Enabled
    /// enable 操作委托给 kernel InMemoryRegistry.lifecycle(Enable)，
    /// Loading/Unloading 等扩展特有状态在调用 kernel lifecycle 前后处理。
    ///
    /// # Arguments
    /// * `extension_id` - 扩展 ID
    pub fn enable(&self, extension_id: &str) -> Result<()> {
        tracing::info!(extension_id = %extension_id, "Enabling extension");

        // 获取当前状态
        let current = self
            .store
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension '{}' not found", extension_id))?;

        let current_status = current.status.clone();

        // 校验状态转换合法性（仅允许从 Installed/Disabled/Error 进入 Loading）
        match current_status {
            ExtensionStatus::Installed | ExtensionStatus::Disabled | ExtensionStatus::Error => {}
            _ => {
                return Err(anyhow::anyhow!(
                    "Cannot enable extension '{}': invalid transition from {:?}",
                    extension_id,
                    current_status
                ));
            }
        }

        // Error 状态可能还保留上一次失败操作的运行时资源。先重试清理，
        // 清理不完整时保持 Error，避免重新启用覆盖 residual handle。
        if current_status == ExtensionStatus::Error {
            if self.has_runtime_handle(extension_id)? {
                self.do_disable(extension_id, &current.manifest.contributes)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "Cannot recover extension '{}' before enabling: {}",
                            extension_id,
                            error
                        )
                    })?;
            }
            self.unregister_extension_permission_constraint(extension_id);
            self.store
                .update_status(extension_id, ExtensionStatus::Installed, None)?;
        }

        // 发送 enabling 事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "extension.enabling",
            KernelContext::new("extension", KernelScope::global()),
            Some(SharedArc::new(
                serde_json::json!({ "extensionId": extension_id }),
            )),
        )) {
            tracing::warn!(
                event = "extension.enabling",
                error = %error,
                "Failed to emit extension lifecycle event"
            );
        }

        // 转换到 Loading 状态（扩展特有，仅更新元数据）
        self.store
            .update_status(extension_id, ExtensionStatus::Loading, None)?;

        // 执行启用逻辑。Hook 声明索引必须等扩展进入 Enabled 后再建立，
        // 避免 ExtensionStore 在 Loading 状态承载"可运行声明"。
        let enable_result = self.do_enable(extension_id, &current.manifest);

        match enable_result {
            Ok(()) => {
                // 转换到 Enabled 状态（会触发 kernel lifecycle Registered -> Enabled）
                self.store
                    .update_status(extension_id, ExtensionStatus::Enabled, None)?;

                // 注册扩展权限约束到 PolicyEngine（Task 3）
                self.register_extension_permission_constraint(
                    extension_id,
                    &current.manifest.permissions,
                );

                if let Err(error) = self
                    .register_enabled_hook_declarations(extension_id, &current.manifest.contributes)
                {
                    let error_msg = error.to_string();
                    if let Err(cleanup_error) =
                        self.do_disable(extension_id, &current.manifest.contributes)
                    {
                        tracing::warn!(
                            extension_id = %extension_id,
                            error = %cleanup_error,
                            "Failed to clean up extension contributes after hook declaration failure"
                        );
                    }
                    // 注销权限约束
                    self.unregister_extension_permission_constraint(extension_id);

                    let _ = self.store.update_status(
                        extension_id,
                        ExtensionStatus::Error,
                        Some(error_msg.clone()),
                    );

                    if let Err(event_error) = self.event_bus.emit(EventEnvelope::new(
                        "extension.error",
                        KernelContext::new("extension", KernelScope::global()),
                        Some(SharedArc::new(serde_json::json!({
                            "extensionId": extension_id,
                            "error": error_msg
                        }))),
                    )) {
                        tracing::warn!(
                            event = "extension.error",
                            error = %event_error,
                            "Failed to emit extension lifecycle event"
                        );
                    }

                    tracing::error!(
                        extension_id = %extension_id,
                        error = %error_msg,
                        "Failed to register enabled extension hook declarations"
                    );
                    return Err(error);
                }

                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.enabled",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(
                        serde_json::json!({ "extensionId": extension_id }),
                    )),
                )) {
                    tracing::warn!(
                        event = "extension.enabled",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }

                tracing::info!(extension_id = %extension_id, "Extension enabled successfully");
                Ok(())
            }
            Err(e) => {
                // 注销权限约束
                self.unregister_extension_permission_constraint(extension_id);

                // 转换到 Error 状态
                let error_msg = e.to_string();
                let _ = self.store.update_status(
                    extension_id,
                    ExtensionStatus::Error,
                    Some(error_msg.clone()),
                );

                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.error",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "extensionId": extension_id,
                        "error": error_msg
                    }))),
                )) {
                    tracing::warn!(
                        event = "extension.error",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }

                tracing::error!(extension_id = %extension_id, error = %error_msg, "Failed to enable extension");
                Err(e)
            }
        }
    }

    /// 禁用扩展
    ///
    /// 状态转换：Enabled -> Disabling -> Disabled
    /// disable 操作委托给 kernel InMemoryRegistry.lifecycle(Disable)，
    /// Disabling 等扩展特有状态在调用 kernel lifecycle 前后处理。
    ///
    /// # Arguments
    /// * `extension_id` - 扩展 ID
    pub fn disable(&self, extension_id: &str) -> Result<()> {
        tracing::info!(extension_id = %extension_id, "Disabling extension");

        let current = self
            .store
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension '{}' not found", extension_id))?;

        let current_status = current.status.clone();

        let retrying_error_cleanup =
            current_status == ExtensionStatus::Error && self.has_runtime_handle(extension_id)?;

        // Enabled 执行正常禁用；Error 仅允许对 residual runtime handle 重试清理。
        if current_status != ExtensionStatus::Enabled && !retrying_error_cleanup {
            return Err(anyhow::anyhow!(
                "Cannot disable extension '{}': invalid transition from {:?}",
                extension_id,
                current_status
            ));
        }

        if retrying_error_cleanup {
            return self.retry_error_cleanup(extension_id, &current.manifest.contributes);
        }

        // 发送 disabling 事件
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "extension.disabling",
            KernelContext::new("extension", KernelScope::global()),
            Some(SharedArc::new(
                serde_json::json!({ "extensionId": extension_id }),
            )),
        )) {
            tracing::warn!(
                event = "extension.disabling",
                error = %error,
                "Failed to emit extension lifecycle event"
            );
        }

        // 转换到 Disabling 状态（扩展特有，仅更新元数据）
        self.store
            .update_status(extension_id, ExtensionStatus::Disabling, None)?;

        // 执行禁用逻辑
        let disable_result = self.do_disable(extension_id, &current.manifest.contributes);

        match disable_result {
            Ok(()) => {
                // 转换到 Disabled 状态（会触发 kernel lifecycle Enabled -> Registered）
                self.store
                    .update_status(extension_id, ExtensionStatus::Disabled, None)?;

                // 注销扩展权限约束
                self.unregister_extension_permission_constraint(extension_id);

                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.disabled",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(
                        serde_json::json!({ "extensionId": extension_id }),
                    )),
                )) {
                    tracing::warn!(
                        event = "extension.disabled",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }

                tracing::info!(extension_id = %extension_id, "Extension disabled successfully");
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                let _ = self.store.update_status(
                    extension_id,
                    ExtensionStatus::Error,
                    Some(error_msg.clone()),
                );

                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.error",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "extensionId": extension_id,
                        "error": error_msg
                    }))),
                )) {
                    tracing::warn!(
                        event = "extension.error",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }

                tracing::error!(extension_id = %extension_id, error = %error_msg, "Failed to disable extension");
                Err(e)
            }
        }
    }

    /// 在删除扩展文件前完成生命周期清理并确认没有残留运行时资源。
    ///
    /// Installer 只负责文件和 ExtensionStore，不负责宿主资源撤销；所有卸载
    /// 调用方必须先经过此入口。Error 状态也会尝试清理 residual resources，
    /// 只有清理完成后才允许进入 Installer。
    pub fn prepare_uninstall(&self, extension_id: &str) -> Result<()> {
        let current = self
            .store
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension '{}' not found", extension_id))?;

        let recover_error_state = current.status == ExtensionStatus::Error;
        match current.status {
            ExtensionStatus::Enabled => self.disable(extension_id)?,
            ExtensionStatus::Error => {
                if self.has_runtime_handle(extension_id)? {
                    self.retry_error_cleanup(extension_id, &current.manifest.contributes)?;
                }
            }
            ExtensionStatus::Installed | ExtensionStatus::Disabled => {}
            status => {
                return Err(anyhow::anyhow!(
                    "Extension '{}' cannot be uninstalled while it is {:?}",
                    extension_id,
                    status
                ));
            }
        }

        let (has_runtime_handle, has_subscriptions) = {
            let (handles, ledger) = self.lock_runtime_and_subscription_ledger()?;
            (
                handles.contains_key(extension_id),
                ledger.contains_extension(extension_id),
            )
        };
        let has_ui_registration = self
            .ui_contributions
            .lock()
            .map_err(|error| anyhow::anyhow!("Failed to lock UI contribution registrar: {error}"))?
            .contains_extension(extension_id);

        if has_runtime_handle || has_subscriptions || has_ui_registration {
            return Err(anyhow::anyhow!(
                "Extension '{}' still owns runtime resources and cannot be uninstalled",
                extension_id
            ));
        }

        if recover_error_state {
            self.store
                .update_status(extension_id, ExtensionStatus::Disabled, None)?;
        }

        Ok(())
    }

    /// 执行启用逻辑：安装 Cordis fiber，apply 在 fiber 内真实注册贡献。
    ///
    /// apply 捕获 `ApplyState` 快照（不借用 self）；`plugin.install` 内部
    /// `fiber.wait()` 失败（apply 返回 Err）时 Cordis 自动逆序执行已登记的
    /// disposer，残余事实保留在账本供 retry。
    fn do_enable(&self, extension_id: &str, manifest: &ExtensionManifest) -> Result<()> {
        tracing::debug!(extension_id = %extension_id, "Running enable logic");

        self.ensure_enable_preflight(extension_id, &manifest.contributes)?;
        if self.has_runtime_handle(extension_id)? {
            return Err(anyhow::anyhow!(
                "Extension '{}' still owns residual runtime resources",
                extension_id
            ));
        }

        let state = ApplyState::from_lifecycle(self);
        let plugin = ExtensionCordisPlugin::new(
            manifest.clone(),
            move |ctx, manifest| apply_extension_fiber(&state, ctx, manifest),
        );
        let _fiber = plugin.install(&self.cordis_host)?;
        Ok(())
    }

    fn retry_error_cleanup(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        match self.do_disable(extension_id, contributes) {
            Ok(()) => {
                self.unregister_extension_permission_constraint(extension_id);
                self.store
                    .update_status(extension_id, ExtensionStatus::Installed, None)?;
                tracing::info!(
                    extension_id = %extension_id,
                    "Residual extension resources cleaned up"
                );
                Ok(())
            }
            Err(error) => {
                let error_message = error.to_string();
                let _ = self.store.update_status(
                    extension_id,
                    ExtensionStatus::Error,
                    Some(error_message),
                );
                Err(error)
            }
        }
    }

    fn has_runtime_handle(&self, extension_id: &str) -> Result<bool> {
        let (handles, ledger) = self.lock_runtime_and_subscription_ledger()?;
        Ok(handles.contains_key(extension_id) || ledger.contains_extension(extension_id))
    }

    fn ensure_enable_preflight(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        let context = self.contribution_context(extension_id);
        self.contribution_families
            .preflight(&context, contributes)?;

        let needs_mcp = contributes
            .mcp_servers
            .as_ref()
            .is_some_and(|servers| !servers.is_empty())
            || contributes
                .mcp_tool_overrides
                .as_ref()
                .is_some_and(|overrides| !overrides.is_empty())
            || contributes
                .tools
                .as_ref()
                .is_some_and(|tools| !tools.is_empty());

        if needs_mcp && self.mcp.is_none() {
            return Err(anyhow::anyhow!(
                "Extension '{}' declares MCP contributes but MCP host is not available",
                extension_id
            ));
        }

        if contributes
            .languages
            .as_ref()
            .is_some_and(|languages| !languages.is_empty())
            && self.lsp.is_none()
        {
            return Err(anyhow::anyhow!(
                "Extension '{}' declares LSP languages but LSP host is not available",
                extension_id
            ));
        }

        Ok(())
    }

    /// 执行禁用逻辑
    ///
    /// 优先经 Cordis fiber dispose 逆序撤销（其 disposer 消费账本事实）；fiber
    /// 已不存在（失败启用残余 / 上次 dispose 后的残余）时手动消费账本清理。
    fn do_disable(&self, extension_id: &str, contributes: &ExtensionContributes) -> Result<()> {
        tracing::debug!(extension_id = %extension_id, "Running disable logic");
        let state = ApplyState::from_lifecycle(self);

        let manual_cleanup = match self.cordis_host.take_extension_fiber(extension_id)? {
            Some(fiber) => {
                if let Err(error) = fiber.dispose() {
                    tracing::warn!(
                        extension_id = %extension_id,
                        error = %error,
                        "Extension fiber dispose failed; falling back to manual cleanup"
                    );
                }
                // dispose 不传播 disposer 错误；残余检查在下方账本面完成。
                None
            }
            None => Some(cleanup_runtime_resources(&state, extension_id)),
        };

        // 声明级清理：trigger/hook 事件与 hook 索引（无论资源清理结果都执行）。
        if let Some(ref triggers) = contributes.triggers {
            for trigger in triggers {
                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.trigger.removed",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "extensionId": extension_id,
                        "prefix": trigger.prefix
                    }))),
                )) {
                    tracing::warn!(
                        event = "extension.trigger.removed",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }
            }
        }

        if let Some(ref hooks) = contributes.hooks {
            let removed = self.store.unregister_hooks(extension_id);
            tracing::debug!(
                extension_id = %extension_id,
                removed,
                "Removed extension hook declarations"
            );

            for hook in hooks {
                if let Err(error) = self.event_bus.emit(EventEnvelope::new(
                    "extension.hook.removed",
                    KernelContext::new("extension", KernelScope::global()),
                    Some(SharedArc::new(serde_json::json!({
                        "extensionId": extension_id,
                        "hookId": hook.id,
                        "phase": format!("{:?}", hook.phase)
                    }))),
                )) {
                    tracing::warn!(
                        event = "extension.hook.removed",
                        error = %error,
                        "Failed to emit extension lifecycle event"
                    );
                }
            }
        }

        if let Some(error) = manual_cleanup {
            error?;
        }

        // 残余面判定：fiber dispose 的 disposer 错误不向 dispose 传播，这里用
        // 账本面判断是否仍有残余资源待重试。
        if self.has_runtime_handle(extension_id)? {
            return Err(anyhow::anyhow!(
                "Extension '{}' still owns residual runtime resources",
                extension_id
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::lifecycle::event::{
        ExtensionEventHandler, ExtensionSubscriptionRecord,
    };
    use crate::extension::lifecycle::{
        EventSubscriptionPort, LspCapabilityPort, McpCapabilityPort,
    };
    use crate::extension::models::{
        ExtensionManifest, ExtensionPermissions, ExtensionState, LSPServerConfig, LanguageSource,
        MCPServerConfig,
    };
    use crate::foundation::config::Config;
    use crate::kernel::{SubscriptionId, Topic};
    use crate::domains::ai_platform::mcp::protocol::{
        MCPServerConfig as HostMcpServerConfig, ToolDefinition, ToolDefinitionOverride,
    };
    use crate::domains::ai_platform::mcp::MCP;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    struct FailingSubscriptionPort {
        fail_next_unsubscribe: AtomicBool,
    }

    impl FailingSubscriptionPort {
        fn new() -> Self {
            Self {
                fail_next_unsubscribe: AtomicBool::new(true),
            }
        }
    }

    impl EventSubscriptionPort for FailingSubscriptionPort {
        fn subscribe_extension(
            &self,
            _extension_id: &str,
            _topic: String,
            _scope_key: Option<String>,
            _handler: ExtensionEventHandler,
        ) -> anyhow::Result<SubscriptionId> {
            Ok(SubscriptionId::new("unused"))
        }

        fn unsubscribe_extension(
            &self,
            _extension_id: &str,
            _subscription_id: &SubscriptionId,
        ) -> anyhow::Result<()> {
            if self.fail_next_unsubscribe.swap(false, Ordering::SeqCst) {
                Err(anyhow::anyhow!("simulated unsubscribe failure"))
            } else {
                Ok(())
            }
        }
    }
    struct FailingCleanupMcp {
        servers: Mutex<Vec<String>>,
        fail_next_remove: AtomicBool,
    }

    impl FailingCleanupMcp {
        fn new() -> Self {
            Self {
                servers: Mutex::new(Vec::new()),
                fail_next_remove: AtomicBool::new(true),
            }
        }

        fn has_server(&self, id: &str) -> bool {
            self.servers
                .lock()
                .unwrap()
                .iter()
                .any(|server| server == id)
        }
    }

    impl McpCapabilityPort for FailingCleanupMcp {
        fn add_server(&self, config: HostMcpServerConfig) -> anyhow::Result<()> {
            self.servers.lock().unwrap().push(config.id);
            Ok(())
        }

        fn start_server(&self, _id: &str) -> anyhow::Result<()> {
            Ok(())
        }

        fn remove_server(&self, id: &str) -> anyhow::Result<()> {
            if self.fail_next_remove.swap(false, Ordering::SeqCst) {
                return Err(anyhow::anyhow!("simulated cleanup failure"));
            }
            let mut servers = self.servers.lock().unwrap();
            let position = servers
                .iter()
                .position(|server| server == id)
                .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", id))?;
            servers.remove(position);
            Ok(())
        }

        fn register_tool(&self, _tool: ToolDefinition) -> anyhow::Result<()> {
            Ok(())
        }

        fn unregister_server_tools(&self, _server_id: &str) -> anyhow::Result<usize> {
            Ok(0)
        }

        fn apply_tool_override(
            &self,
            _owner: &str,
            _server_id: &str,
            _tool_name: &str,
            _override_: ToolDefinitionOverride,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn remove_tool_override(
            &self,
            _owner: &str,
            _server_id: &str,
            _tool_name: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct FailingLsp;

    impl LspCapabilityPort for FailingLsp {
        fn register_language(
            &self,
            _config: LSPServerConfig,
            _source: LanguageSource,
        ) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("simulated LSP registration failure"))
        }

        fn unregister_language(&self, _language_id: &str, _owner: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn create_lifecycle(
        id: &str,
        mcp: Arc<dyn McpCapabilityPort>,
    ) -> (
        ExtensionLifecycle,
        Arc<crate::extension::store::ExtensionStore>,
    ) {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(crate::extension::store::ExtensionStore::new(Arc::clone(
            &event_bus,
        )));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        store
            .register(ExtensionState {
                id: id.to_string(),
                status: ExtensionStatus::Installed,
                manifest: ExtensionManifest {
                    id: id.to_string(),
                    name: format!("Extension {id}"),
                    version: "1.0.0".into(),
                    description: "test".into(),
                    author: "test".into(),
                    permissions: ExtensionPermissions::default(),
                    contributes: ExtensionContributes {
                        mcp_servers: Some(vec![MCPServerConfig {
                            name: "browser".into(),
                            config: serde_json::json!({
                                "transport": "stdio",
                                "command": "node",
                                "auto_start": false
                            }),
                        }]),
                        ..Default::default()
                    },
                },
                install_path: PathBuf::from(format!("/extensions/{id}")),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_mcp(mcp);
        (lifecycle, store)
    }

    #[test]
    fn error_residual_handle_can_be_retried_with_disable() {
        let _guard = test_runtime_handle().enter();
        let mcp = Arc::new(FailingCleanupMcp::new());
        let (lifecycle, store) = create_lifecycle("extension-residual-disable", mcp.clone());

        lifecycle.enable("extension-residual-disable").unwrap();
        assert!(mcp.has_server("extension:extension-residual-disable/browser"));

        // 禁用时 disposer 的 remove_server 失败 → 残余句柄保留，状态回到 Error。
        let result = lifecycle.disable("extension-residual-disable");
        assert!(result.is_err());
        assert_eq!(
            store.get("extension-residual-disable").unwrap().status,
            ExtensionStatus::Error
        );
        assert!(mcp.has_server("extension:extension-residual-disable/browser"));
        assert!(lifecycle
            .runtime_handles
            .lock()
            .unwrap()
            .contains_key("extension-residual-disable"));

        // retry：fiber 已随上次 dispose 消费，走手动清理路径，remove 成功。
        lifecycle.disable("extension-residual-disable").unwrap();

        assert_eq!(
            store.get("extension-residual-disable").unwrap().status,
            ExtensionStatus::Installed
        );
        assert!(!mcp.has_server("extension:extension-residual-disable/browser"));
        assert!(!lifecycle
            .runtime_handles
            .lock()
            .unwrap()
            .contains_key("extension-residual-disable"));
    }

    #[test]
    fn error_residual_handle_is_cleaned_before_enable_retry() {
        let _guard = test_runtime_handle().enter();
        let mcp = Arc::new(FailingCleanupMcp::new());
        let (lifecycle, store) = create_lifecycle("extension-residual-enable", mcp.clone());

        lifecycle.enable("extension-residual-enable").unwrap();
        assert!(lifecycle.disable("extension-residual-enable").is_err());
        assert_eq!(
            store.get("extension-residual-enable").unwrap().status,
            ExtensionStatus::Error
        );

        // enable 前先清理残余，再安装新 fiber。
        lifecycle.enable("extension-residual-enable").unwrap();

        assert_eq!(
            store.get("extension-residual-enable").unwrap().status,
            ExtensionStatus::Enabled
        );
        assert!(mcp.has_server("extension:extension-residual-enable/browser"));
        lifecycle.disable("extension-residual-enable").unwrap();
        assert!(!mcp.has_server("extension:extension-residual-enable/browser"));
    }

    #[test]
    fn failed_subscription_unsubscribe_remains_in_ledger_for_retry() {
        let _guard = test_runtime_handle().enter();
        let mcp = Arc::new(MCP::init_for_test().unwrap());
        let (mut lifecycle, _store) = create_lifecycle("extension-subscription-retry", mcp);
        let port = Arc::new(FailingSubscriptionPort::new());
        lifecycle = lifecycle.with_event_subscription_port(port);
        let extension_id = "extension-subscription-retry";
        let record = ExtensionSubscriptionRecord {
            subscription_id: SubscriptionId::new("subscription-retry"),
            topic: Topic::new("extension.test"),
            scope_key: None,
        };
        lifecycle
            .subscription_ledger
            .lock()
            .unwrap()
            .record_many(extension_id, std::slice::from_ref(&record))
            .unwrap();
        lifecycle
            .runtime_handles
            .lock()
            .unwrap()
            .insert(extension_id.into(), ExtensionRuntimeHandle::default());

        let contributes = ExtensionContributes::default();
        assert!(lifecycle.do_disable(extension_id, &contributes).is_err());
        assert_eq!(
            lifecycle
                .subscription_ledger
                .lock()
                .unwrap()
                .records(extension_id),
            vec![record.clone()]
        );

        lifecycle.do_disable(extension_id, &contributes).unwrap();
        assert!(lifecycle
            .subscription_ledger
            .lock()
            .unwrap()
            .records(extension_id)
            .is_empty());
        assert!(!lifecycle
            .runtime_handles
            .lock()
            .unwrap()
            .contains_key(extension_id));
    }
    #[test]
    fn failed_enable_persists_residual_handle_when_rollback_cleanup_fails() {
        let _guard = test_runtime_handle().enter();
        let mcp = Arc::new(FailingCleanupMcp::new());
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(crate::extension::store::ExtensionStore::new(Arc::clone(
            &event_bus,
        )));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let id = "extension-rollback-residual";
        store
            .register(ExtensionState {
                id: id.into(),
                status: ExtensionStatus::Installed,
                manifest: ExtensionManifest {
                    id: id.into(),
                    name: "Rollback residual".into(),
                    version: "1.0.0".into(),
                    description: "test".into(),
                    author: "test".into(),
                    permissions: ExtensionPermissions::default(),
                    contributes: ExtensionContributes {
                        mcp_servers: Some(vec![MCPServerConfig {
                            name: "browser".into(),
                            config: serde_json::json!({
                                "transport": "stdio",
                                "command": "node",
                                "auto_start": false
                            }),
                        }]),
                        languages: Some(vec![crate::extension::models::LanguageRegistration {
                            language_id: "rollback-language".into(),
                            display_name: "Rollback language".into(),
                            extensions: vec![".rollback".into()],
                            server_command: "rollback-lsp".into(),
                            server_args: None,
                            initialization_options: None,
                        }]),
                        ..Default::default()
                    },
                },
                install_path: PathBuf::from(format!("/extensions/{id}")),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();

        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus)
            .with_mcp(mcp.clone())
            .with_lsp(Arc::new(FailingLsp));
        assert!(lifecycle.enable(id).is_err());
        assert_eq!(store.get(id).unwrap().status, ExtensionStatus::Error);
        assert!(mcp.has_server("extension:extension-rollback-residual/browser"));
        assert!(lifecycle.runtime_handles.lock().unwrap().contains_key(id));

        lifecycle.disable(id).unwrap();
        assert_eq!(store.get(id).unwrap().status, ExtensionStatus::Installed);
        assert!(!mcp.has_server("extension:extension-rollback-residual/browser"));
        assert!(!lifecycle.runtime_handles.lock().unwrap().contains_key(id));
    }
}
