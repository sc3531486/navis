//! Extension contribution family orchestration.
//!
//! Each family owns the manifest-to-runtime transaction for one contribution
//! surface. The lifecycle only prepares and commits a family plan; host-specific
//! registration stays behind the family handler contract.

use std::any::Any;
use std::sync::Arc;

use anyhow::{anyhow, Result};

use super::contributions::{
    normalize_registration, prepare_registration, validate_registration, UiContributionRegistration,
};
use super::register::{
    build_gateway_plan, register_gateway_plan, unregister_gateway_resources, GatewayPlan,
};
use super::{
    ExtensionRuntimeHandle, GatewayCapabilityPort, LspCapabilityPort,
};
use crate::extension::lifecycle::cordis::BackendProcessPort;
use crate::extension::provider_validation::ExtensionProviderValidationPort;
use crate::extension::component::ComponentRegistry;
use crate::extension::models::{
    BackendServiceRegistration, ComponentRegistration, EditorLanguageRegistration,
    ExtensionContributes, LSPServerConfig, LanguageSource,
};
use crate::extension::store::ExtensionStore;
use std::sync::Mutex;

use super::contributions::UiContributionRegistrar;

pub(crate) const GATEWAY_CONTRIBUTION_FAMILY: &str = "gateway";
pub(crate) const UI_CONTRIBUTION_FAMILY: &str = "ui";
pub(crate) const EDITOR_CONTRIBUTION_FAMILY: &str = "editor";
pub(crate) const BACKEND_SERVICE_CONTRIBUTION_FAMILY: &str = "backend_service";
pub(crate) const COMPONENT_CONTRIBUTION_FAMILY: &str = "component";

/// Context shared by contribution families.
///
/// Only the ports required by the registered families are exposed. A family
/// cannot reach the complete lifecycle object or mutate unrelated ledgers.
pub(crate) struct ContributionContext<'a> {
    pub(crate) extension_id: &'a str,
    /// Extension state index；backend family spawn 需解析 install_path 与 Enabled。
    pub(crate) store: &'a ExtensionStore,
    pub(crate) gateway: Option<&'a dyn GatewayCapabilityPort>,
    pub(crate) provider_validation: Option<&'a dyn ExtensionProviderValidationPort>,
    pub(crate) ui_contributions: &'a Mutex<UiContributionRegistrar>,
    pub(crate) lsp: Option<&'a dyn LspCapabilityPort>,
    /// 后端进程管理器；backend_service family 的 preflight/commit/disable 使用。
    /// 未注入时声明 backend_services 的扩展必须 fail-closed。
    pub(crate) backend_manager: Option<&'a dyn BackendProcessPort>,
    /// WASM 组件注册表；component family 的 preflight/commit/rollback/disable 使用。
    /// registry 在装配时注入 ExtensionStore（app/mod.rs），load 无需重复传入 store。
    /// 未注入时声明 components 的扩展必须 fail-closed。
    pub(crate) component_registry: Option<&'a ComponentRegistry>,
}

impl ContributionContext<'_> {
    fn unregister_ui_contributions(&self) -> anyhow::Result<()> {
        let mut registrar = self.ui_contributions.lock().map_err(|error| {
            anyhow!("Failed to lock UI contribution registrar during unregister: {error}")
        })?;
        registrar.unregister_extension(self.extension_id);
        Ok(())
    }
}

pub(crate) struct NormalizedContribution {
    family: &'static str,
    payload: Box<dyn Any + Send + Sync>,
}

impl NormalizedContribution {
    fn new<T>(family: &'static str, payload: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self {
            family,
            payload: Box::new(payload),
        }
    }

    fn family(&self) -> &'static str {
        self.family
    }

    fn downcast_ref<T>(&self) -> Result<&T>
    where
        T: Any + Send + Sync,
    {
        self.payload.downcast_ref::<T>().ok_or_else(|| {
            anyhow!(
                "Contribution family '{}' produced an invalid normalized payload",
                self.family
            )
        })
    }

    fn into_payload<T>(self) -> Result<T>
    where
        T: Any + Send + Sync,
    {
        self.payload
            .downcast::<T>()
            .map(|payload| *payload)
            .map_err(|_| {
                anyhow!(
                    "Contribution family '{}' produced an invalid normalized payload",
                    self.family
                )
            })
    }
}

pub(crate) struct PreparedContribution {
    family: &'static str,
    payload: Box<dyn Any + Send + Sync>,
}

impl PreparedContribution {
    fn new<T>(family: &'static str, payload: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self {
            family,
            payload: Box::new(payload),
        }
    }

    fn into_payload<T>(self) -> Result<T>
    where
        T: Any + Send + Sync,
    {
        self.payload
            .downcast::<T>()
            .map(|payload| *payload)
            .map_err(|_| {
                anyhow!(
                    "Contribution family '{}' produced an invalid prepared payload",
                    self.family
                )
            })
    }
}

/// Contract implemented by one contribution family.
pub(crate) trait ContributionFamilyHandler: Send + Sync {
    fn family(&self) -> &'static str;

    fn preflight(
        &self,
        _context: &ContributionContext<'_>,
        _contributes: &ExtensionContributes,
    ) -> Result<()> {
        Ok(())
    }

    fn normalize(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>>;

    fn validate(
        &self,
        context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()>;

    fn prepare(
        &self,
        context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution>;

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()>;

    fn rollback(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
    );

    fn disable(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
        cleanup_errors: &mut Vec<String>,
    );
}

struct PreparedFamily {
    handler: Arc<dyn ContributionFamilyHandler>,
    prepared: PreparedContribution,
}

pub(crate) struct ContributionPlan {
    entries: Vec<PreparedFamily>,
}

impl ContributionPlan {
    pub(crate) fn commit(
        mut self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let mut committed = Vec::new();
        for entry in self.entries.drain(..) {
            let handler = Arc::clone(&entry.handler);
            match handler.commit(context, entry.prepared, handle) {
                Ok(()) => committed.push(handler),
                Err(error) => {
                    for handler in committed.into_iter().rev() {
                        handler.rollback(context, handle);
                    }
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn rollback(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
    ) {
        for entry in self.entries.iter().rev() {
            entry.handler.rollback(context, handle);
        }
    }
}

#[derive(Clone)]
pub(crate) struct ContributionFamilyRegistry {
    handlers: Vec<Arc<dyn ContributionFamilyHandler>>,
}

impl Default for ContributionFamilyRegistry {
    fn default() -> Self {
        Self { handlers: Vec::new() }
    }
}

pub(crate) fn install_builtin_handlers(registry: &mut ContributionFamilyRegistry) {
    registry
        .insert_handler(Arc::new(GatewayContributionFamilyHandler))
        .expect("built-in Gateway contribution family must be unique");
    registry
        .insert_handler(Arc::new(UiContributionFamilyHandler))
        .expect("built-in UI contribution family must be unique");
    registry
        .insert_handler(Arc::new(EditorContributionFamilyHandler))
        .expect("built-in Editor contribution family must be unique");
    registry
        .insert_handler(Arc::new(BackendServiceFamilyHandler))
        .expect("built-in Backend service contribution family must be unique");
    registry
        .insert_handler(Arc::new(ComponentFamilyHandler))
        .expect("built-in Component contribution family must be unique");
    for handler in unsupported_runtime_handlers() {
        registry
            .insert_handler(handler)
            .expect("built-in unsupported contribution family must be unique");
    }
}

impl ContributionFamilyRegistry {
    pub(crate) fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        for handler in &self.handlers {
            handler.preflight(context, contributes)?;
        }
        Ok(())
    }

    pub(crate) fn insert_handler(
        &mut self,
        handler: Arc<dyn ContributionFamilyHandler>,
    ) -> Result<()> {
        let family = handler.family();
        if self
            .handlers
            .iter()
            .any(|candidate| candidate.family() == family)
        {
            return Err(anyhow!(
                "Contribution family '{}' is already registered",
                family
            ));
        }
        self.handlers.push(handler);
        Ok(())
    }

    pub(crate) fn prepare_plan(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<ContributionPlan> {
        let mut entries = Vec::new();
        for handler in &self.handlers {
            let Some(normalized) = handler.normalize(context.extension_id, contributes)? else {
                continue;
            };
            if normalized.family() != handler.family() {
                return Err(anyhow!(
                    "Contribution family handler '{}' returned payload for '{}'",
                    handler.family(),
                    normalized.family()
                ));
            }
            handler.validate(context, &normalized)?;
            entries.push(PreparedFamily {
                handler: Arc::clone(handler),
                prepared: handler.prepare(context, normalized)?,
            });
        }
        Ok(ContributionPlan { entries })
    }

    pub(crate) fn disable(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
        cleanup_errors: &mut Vec<String>,
    ) {
        for handler in self.handlers.iter().rev() {
            handler.disable(context, handle, cleanup_errors);
        }
    }
}

/// Gateway family owns manifest normalization, host validation, registration and cleanup.
///
/// The lifecycle never coordinates individual Gateway resources. It only invokes this
/// handler through the generic contribution-family transaction.
struct GatewayContributionFamilyHandler;

impl ContributionFamilyHandler for GatewayContributionFamilyHandler {
    fn family(&self) -> &'static str {
        GATEWAY_CONTRIBUTION_FAMILY
    }

    fn normalize(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        let Some(gateway) = contributes.gateway.as_ref() else {
            return Ok(None);
        };
        Ok(Some(NormalizedContribution::new(
            self.family(),
            build_gateway_plan(extension_id, gateway)?,
        )))
    }

    fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        let plan = contributes
            .gateway
            .as_ref()
            .map(|gateway| build_gateway_plan(context.extension_id, gateway))
            .transpose()?;
        let Some(plan) = plan else {
            return Ok(());
        };
        if context.gateway.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares Gateway contributions but Gateway host is not available",
                context.extension_id
            ));
        }
        if !plan.providers.is_empty() && context.provider_validation.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares Gateway Providers but Provider validation host is not available",
                context.extension_id
            ));
        }
        Ok(())
    }

    fn validate(
        &self,
        context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()> {
        let plan = normalized.downcast_ref::<GatewayPlan>()?;
        if context.gateway.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares Gateway contributions but Gateway host is not available",
                context.extension_id
            ));
        }
        if !plan.providers.is_empty() && context.provider_validation.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares Gateway Providers but Provider validation host is not available",
                context.extension_id
            ));
        }
        Ok(())
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        Ok(PreparedContribution::new(
            self.family(),
            normalized.into_payload::<GatewayPlan>()?,
        ))
    }

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let plan = prepared.into_payload::<GatewayPlan>()?;
        let gateway = context.gateway.ok_or_else(|| {
            anyhow!(
                "Extension '{}' declares Gateway contributions but Gateway host is not available",
                context.extension_id
            )
        })?;
        register_gateway_plan(
            context.extension_id,
            plan,
            gateway,
            context.provider_validation,
            handle,
        )
    }

    fn rollback(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
    ) {
        // Consume the same Gateway ownership facts used by the outer cleanup.
        // This keeps family rollback explicit while preserving residual handles
        // when a host cleanup operation fails.
        let mut cleanup_errors = Vec::new();
        unregister_gateway_resources(
            context.extension_id,
            handle,
            context.gateway,
            context.provider_validation,
            &mut cleanup_errors,
        );
        for error in cleanup_errors {
            tracing::warn!(
                extension_id = %context.extension_id,
                error = %error,
                "Failed to roll back extension Gateway resource"
            );
        }
    }

    fn disable(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
        cleanup_errors: &mut Vec<String>,
    ) {
        unregister_gateway_resources(
            context.extension_id,
            handle,
            context.gateway,
            context.provider_validation,
            cleanup_errors,
        );
    }
}

struct UiContributionFamilyHandler;

impl ContributionFamilyHandler for UiContributionFamilyHandler {
    fn family(&self) -> &'static str {
        UI_CONTRIBUTION_FAMILY
    }

    fn normalize(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        let registration = normalize_registration(extension_id, contributes);
        if registration.is_empty() {
            return Ok(None);
        }
        Ok(Some(NormalizedContribution::new(self.family(), registration)))
    }

    fn validate(
        &self,
        _context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()> {
        validate_registration(normalized.downcast_ref()?)
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        let registration: UiContributionRegistration = normalized.into_payload()?;
        // Keep prepare as the single pure validation boundary for direct
        // callers too; the registry already validates before reaching here.
        let registration = prepare_registration(&registration)?;
        Ok(PreparedContribution::new(self.family(), registration))
    }

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let registration: UiContributionRegistration = prepared.into_payload()?;
        let projection = registration.clone();
        let mut registrar = context
            .ui_contributions
            .lock()
            .map_err(|error| anyhow!("Failed to lock UI contribution registrar: {}", error))?;
        registrar.commit_registration(registration)?;
        drop(registrar);

        let view_count = projection.views.len();
        let work_mode_count = projection.work_modes.len();
        let command_count = projection.commands.len();
        let menu_count = projection.menus.len();
        let keybinding_count = projection.keybindings.len();
        let zone_count = projection.zones.len();
        let script_count = projection.scripts.len();
        let toolbar_count = projection.toolbar_items.len();
        let statusbar_count = projection.statusbar_items.len();
        let inline_count = projection.inline_extensions.len();
        let configuration_count = usize::from(projection.configuration.is_some());
        handle.ui_contribution_registered = true;
        handle.projection.views = projection.views;
        handle.projection.work_modes = projection.work_modes;
        handle.projection.zones = projection.zones;
        handle.projection.scripts = projection.scripts;
        handle.projection.toolbar_items = projection.toolbar_items;
        handle.projection.statusbar_items = projection.statusbar_items;
        handle.projection.inline_extensions = projection.inline_extensions;
        handle.projection.configuration = projection.configuration;
        handle.projection.contribution_counts.views = view_count;
        handle.projection.contribution_counts.commands = command_count;
        handle.projection.contribution_counts.menus = menu_count;
        handle.projection.contribution_counts.keybindings = keybinding_count;
        handle.projection.contribution_counts.work_modes = work_mode_count;
        handle.projection.contribution_counts.zones = zone_count;
        handle.projection.contribution_counts.scripts = script_count;
        handle.projection.contribution_counts.toolbar_items = toolbar_count;
        handle.projection.contribution_counts.statusbar_items = statusbar_count;
        handle.projection.contribution_counts.inline_extensions = inline_count;
        handle.projection.contribution_counts.configuration = configuration_count;
        Ok(())
    }

    fn rollback(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
    ) {
        if !handle.ui_contribution_registered {
            return;
        }
        if let Err(error) = context.unregister_ui_contributions() {
            tracing::warn!(
                extension_id = %context.extension_id,
                error = %error,
                "Failed to roll back UI contribution family"
            );
            return;
        }
        handle.ui_contribution_registered = false;
        handle.projection.views.clear();
        handle.projection.work_modes.clear();
        handle.projection.zones.clear();
        handle.projection.scripts.clear();
        handle.projection.toolbar_items.clear();
        handle.projection.statusbar_items.clear();
        handle.projection.inline_extensions.clear();
        handle.projection.configuration = None;
        handle.projection.contribution_counts.views = 0;
        handle.projection.contribution_counts.work_modes = 0;
        handle.projection.contribution_counts.menus = 0;
        handle.projection.contribution_counts.commands = 0;
        handle.projection.contribution_counts.keybindings = 0;
        handle.projection.contribution_counts.zones = 0;
        handle.projection.contribution_counts.scripts = 0;
        handle.projection.contribution_counts.toolbar_items = 0;
        handle.projection.contribution_counts.statusbar_items = 0;
        handle.projection.contribution_counts.inline_extensions = 0;
        handle.projection.contribution_counts.configuration = 0;
    }

    fn disable(
        &self,
        context: &ContributionContext<'_>,
        handle: &mut ExtensionRuntimeHandle,
        cleanup_errors: &mut Vec<String>,
    ) {
        if !handle.ui_contribution_registered {
            return;
        }
        if let Err(error) = context.unregister_ui_contributions() {
            cleanup_errors.push(format!("UI contributions: {}", error));
            return;
        }
        handle.ui_contribution_registered = false;
    }
}

struct EditorContributionFamilyHandler;

impl ContributionFamilyHandler for EditorContributionFamilyHandler {
    fn family(&self) -> &'static str {
        EDITOR_CONTRIBUTION_FAMILY
    }

    fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        if count_editor_languages(contributes) == 0 {
            return Ok(());
        }
        if context.lsp.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares editor languages but LSP host is not available",
                context.extension_id
            ));
        }
        Ok(())
    }

    fn normalize(
        &self,
        _extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        if count_editor_languages(contributes) == 0 {
            return Ok(None);
        }
        let languages = contributes.editor_languages.clone().unwrap_or_default();
        Ok(Some(NormalizedContribution::new(self.family(), languages)))
    }

    fn validate(
        &self,
        context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()> {
        let languages = normalized.downcast_ref::<Vec<EditorLanguageRegistration>>()?;
        for language in languages {
            if language.id.trim().is_empty() {
                return Err(anyhow!(
                    "Extension '{}' declares an editor language with an empty id",
                    context.extension_id
                ));
            }
        }
        Ok(())
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        Ok(PreparedContribution::new(
            self.family(),
            normalized.into_payload::<Vec<EditorLanguageRegistration>>()?,
        ))
    }

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let languages = prepared.into_payload::<Vec<EditorLanguageRegistration>>()?;
        let lsp = context.lsp.ok_or_else(|| {
            anyhow!(
                "Extension '{}' declares editor languages but LSP host is not available",
                context.extension_id
            )
        })?;
        for language in &languages {
            lsp.register_language(
                editor_language_server_config(language),
                LanguageSource::Extension {
                    owner: context.extension_id.to_string(),
                },
            )
            .map_err(|error| {
                anyhow!(
                    "Extension '{}' failed to register editor language '{}': {}",
                    context.extension_id,
                    language.id,
                    error
                )
            })?;
            // 与 contributes.languages 共用 handle.languages：enable 中途失败
            // 或 disable 时由外层 cleanup 经 lsp.unregister_language 注销。
            handle.languages.push(language.id.clone());
        }
        Ok(())
    }

    fn rollback(
        &self,
        _context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
    ) {
        // 语言资源所有权已记录在 handle.languages，由外层 rollback/disable 统一注销。
    }

    fn disable(
        &self,
        _context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
        _cleanup_errors: &mut Vec<String>,
    ) {
    }
}

/// Backend service family owns manifest normalization, host validation,
/// autostart spawn and process cleanup (设计 35 §3.4).
///
/// 后端扩展承载纯后端逻辑，独立进程运行。enable 时按 `autostart` 决定
/// 是否立即 spawn；进程由 `BackendProcessManager` 按 `(extension_id,
/// service_id)` 管理，spawn 前必须过 Sandbox CommandExecute 门禁
/// （fail-closed，不弹确认）。
struct BackendServiceFamilyHandler;

impl ContributionFamilyHandler for BackendServiceFamilyHandler {
    fn family(&self) -> &'static str {
        BACKEND_SERVICE_CONTRIBUTION_FAMILY
    }

    fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        if count_backend_services(contributes) == 0 {
            return Ok(());
        }
        if context.backend_manager.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares backend services but backend host is not available",
                context.extension_id
            ));
        }
        Ok(())
    }

    fn normalize(
        &self,
        _extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        let Some(services) = contributes.backend_services.as_ref() else {
            return Ok(None);
        };
        if services.is_empty() {
            return Ok(None);
        }
        Ok(Some(NormalizedContribution::new(
            self.family(),
            services.clone(),
        )))
    }

    fn validate(
        &self,
        context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()> {
        let services = normalized.downcast_ref::<Vec<BackendServiceRegistration>>()?;
        for service in services {
            if service.id.trim().is_empty() {
                return Err(anyhow!(
                    "Extension '{}' declares a backend service with an empty id",
                    context.extension_id
                ));
            }
            if service.entry.trim().is_empty() {
                return Err(anyhow!(
                    "Extension '{}' declares backend service '{}' with an empty entry",
                    context.extension_id,
                    service.id
                ));
            }
        }
        Ok(())
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        Ok(PreparedContribution::new(
            self.family(),
            normalized.into_payload::<Vec<BackendServiceRegistration>>()?,
        ))
    }

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        _handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let services = prepared.into_payload::<Vec<BackendServiceRegistration>>()?;
        let manager = context.backend_manager.ok_or_else(|| {
            anyhow!(
                "Extension '{}' declares backend services but backend host is not available",
                context.extension_id
            )
        })?;
        for service in &services {
            if service.autostart {
                if let Err(error) =
                    manager.spawn_for_lifecycle(context.store, context.extension_id, service)
                {
                    // 保持 enable 原子性：本 family 先前已拉起的进程一并清理。
                    manager.kill_all_for_extension(context.extension_id);
                    return Err(anyhow!(
                        "Extension '{}' failed to start backend service '{}': {}",
                        context.extension_id,
                        service.id,
                        error
                    ));
                }
            } else {
                tracing::debug!(
                    extension_id = %context.extension_id,
                    service_id = %service.id,
                    "Backend service declared for on-demand spawn"
                );
            }
        }
        Ok(())
    }

    fn rollback(
        &self,
        context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
    ) {
        if let Some(manager) = context.backend_manager {
            manager.kill_all_for_extension(context.extension_id);
        }
    }

    fn disable(
        &self,
        context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
        _cleanup_errors: &mut Vec<String>,
    ) {
        // 禁用/卸载时终止该扩展全部后端进程。kill_all 内部已 warn 记录
        // 单进程失败，不阻断状态转换。
        if let Some(manager) = context.backend_manager {
            manager.kill_all_for_extension(context.extension_id);
        }
    }
}

/// Component family owns manifest normalization, host validation, wasm load
/// and activation（37 §5.x 组件轨）.
///
/// WASM 组件轨：逻辑扩展统一编译为 wasm32-wasip2 组件。enable 时经
/// `ComponentRegistry.load` 校验 Enabled → 解析 entry → Sandbox CommandExecute
/// 门禁 → 加载 .wasm → 登记；`autostart` 或 `run_on` 含 "activation"（启用即激活）
/// 或 "message"（须保持实例化以接收消息）的组件立即 activate。任一组件
/// load/activate 失败即 `dispose_all_for_extension` 清理本 family 已登记组件，
/// 保持 enable 原子性（fail-closed）。rollback/disable 按扩展整体销毁组件实例。
struct ComponentFamilyHandler;

impl ContributionFamilyHandler for ComponentFamilyHandler {
    fn family(&self) -> &'static str {
        COMPONENT_CONTRIBUTION_FAMILY
    }

    fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        if count_components(contributes) == 0 {
            return Ok(());
        }
        if context.component_registry.is_none() {
            return Err(anyhow!(
                "Extension '{}' declares components but component host is not available",
                context.extension_id
            ));
        }
        Ok(())
    }

    fn normalize(
        &self,
        _extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        let Some(components) = contributes.components.as_ref() else {
            return Ok(None);
        };
        if components.is_empty() {
            return Ok(None);
        }
        Ok(Some(NormalizedContribution::new(
            self.family(),
            components.clone(),
        )))
    }

    fn validate(
        &self,
        context: &ContributionContext<'_>,
        normalized: &NormalizedContribution,
    ) -> Result<()> {
        let components = normalized.downcast_ref::<Vec<ComponentRegistration>>()?;
        for component in components {
            if component.id.trim().is_empty() {
                return Err(anyhow!(
                    "Extension '{}' declares a component with an empty id",
                    context.extension_id
                ));
            }
            if component.entry.trim().is_empty() {
                return Err(anyhow!(
                    "Extension '{}' declares component '{}' with an empty entry",
                    context.extension_id,
                    component.id
                ));
            }
        }
        Ok(())
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        Ok(PreparedContribution::new(
            self.family(),
            normalized.into_payload::<Vec<ComponentRegistration>>()?,
        ))
    }

    fn commit(
        &self,
        context: &ContributionContext<'_>,
        prepared: PreparedContribution,
        _handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        let components = prepared.into_payload::<Vec<ComponentRegistration>>()?;
        let registry = context.component_registry.ok_or_else(|| {
            anyhow!(
                "Extension '{}' declares components but component host is not available",
                context.extension_id
            )
        })?;
        for component in &components {
            // ComponentRegistry 持有 ExtensionStore（new 时注入，见 app/mod.rs 装配），
            // load 只需 extension_id + ComponentRegistration。
            let component_id = match registry.load(context.extension_id, component)
            {
                Ok(component_id) => component_id,
                Err(error) => {
                    // 保持 enable 原子性：本 family 先前已登记/激活的组件一并清理。
                    registry.dispose_all_for_extension(context.extension_id);
                    return Err(anyhow!(
                        "Extension '{}' failed to load component '{}': {}",
                        context.extension_id,
                        component.id,
                        error
                    ));
                }
            };
            if component.autostart
                || component
                    .run_on
                    .iter()
                    .any(|event| {
                        // autostart 或 runOn 含 "activation"（启用即激活）或
                        // "message"（须保持实例化以接收消息）时立即 activate。
                        event.as_str() == "activation" || event.as_str() == "message"
                    })
            {
                if let Err(error) = registry.activate(context.extension_id, &component_id) {
                    // 保持 enable 原子性：激活失败同样清理本 family 全部组件。
                    registry.dispose_all_for_extension(context.extension_id);
                    return Err(anyhow!(
                        "Extension '{}' failed to activate component '{}': {}",
                        context.extension_id,
                        component.id,
                        error
                    ));
                }
            }
        }
        Ok(())
    }

    fn rollback(
        &self,
        context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
    ) {
        // 组件资源由 ComponentRegistry 按扩展统一管理；回滚即整扩展 dispose。
        if let Some(registry) = context.component_registry {
            registry.dispose_all_for_extension(context.extension_id);
        }
    }

    fn disable(
        &self,
        context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
        _cleanup_errors: &mut Vec<String>,
    ) {
        // 禁用/卸载时销毁该扩展全部组件实例。dispose_all 内部记录失败，不阻断状态转换。
        if let Some(registry) = context.component_registry {
            registry.dispose_all_for_extension(context.extension_id);
        }
    }
}

/// 把编辑器语言模式声明投影为 LSP 语言能力目录条目。
///
/// editor_languages 是 CodeMirror 语言模式声明（id/name/extensions/module），
/// 本身不承载 LSP server 进程；此处经 LspCapabilityPort 注册为语言能力，
/// 使编辑器可按扩展名/语言名匹配。ES Module 语法模块路径由前端 Editor
/// runtime（26-Editor）后续承接，LSPServerConfig 不表达 module 字段。
fn editor_language_server_config(language: &EditorLanguageRegistration) -> LSPServerConfig {
    LSPServerConfig {
        language_id: language.id.clone(),
        language_names: vec![language.name.clone()],
        file_extensions: language.extensions.clone(),
        server_command: String::new(),
        server_args: Vec::new(),
        initialization_options: None,
        capabilities_required: Vec::new(),
    }
}

struct UnsupportedContributionFamilyHandler {
    family: &'static str,
    host: &'static str,
    count: fn(&ExtensionContributes) -> usize,
    /// 拒绝原因/验收备注。存在时 preflight 拒绝前先打 warn 日志，
    /// 用于说明"阶段 N 验收：为何保持 fail-closed"。
    note: Option<&'static str>,
}

impl ContributionFamilyHandler for UnsupportedContributionFamilyHandler {
    fn family(&self) -> &'static str {
        self.family
    }

    fn preflight(
        &self,
        context: &ContributionContext<'_>,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        let count = (self.count)(contributes);
        if count == 0 {
            return Ok(());
        }
        if let Some(note) = self.note {
            tracing::warn!(
                extension_id = %context.extension_id,
                family = self.family,
                host = self.host,
                count = count,
                note = %note,
                "Rejected unsupported runtime contributes"
            );
        }
        Err(anyhow!(
            "Extension '{}' declares unsupported runtime contributes '{}' -> {} ({})",
            context.extension_id,
            self.family,
            self.host,
            count
        ))
    }

    fn normalize(
        &self,
        _extension_id: &str,
        _contributes: &ExtensionContributes,
    ) -> Result<Option<NormalizedContribution>> {
        Ok(None)
    }

    fn validate(
        &self,
        _context: &ContributionContext<'_>,
        _normalized: &NormalizedContribution,
    ) -> Result<()> {
        Err(anyhow!(
            "Unsupported contribution family '{}' cannot produce a runtime plan",
            self.family
        ))
    }

    fn prepare(
        &self,
        _context: &ContributionContext<'_>,
        _normalized: NormalizedContribution,
    ) -> Result<PreparedContribution> {
        Err(anyhow!(
            "Unsupported contribution family '{}' cannot be prepared",
            self.family
        ))
    }

    fn commit(
        &self,
        _context: &ContributionContext<'_>,
        _prepared: PreparedContribution,
        _handle: &mut ExtensionRuntimeHandle,
    ) -> Result<()> {
        Err(anyhow!(
            "Unsupported contribution family '{}' cannot be committed",
            self.family
        ))
    }

    fn rollback(
        &self,
        _context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
    ) {
    }

    fn disable(
        &self,
        _context: &ContributionContext<'_>,
        _handle: &mut ExtensionRuntimeHandle,
        _cleanup_errors: &mut Vec<String>,
    ) {
    }
}

macro_rules! contribution_vec_counter {
    ($name:ident, $field:ident) => {
        fn $name(contributes: &ExtensionContributes) -> usize {
            contributes.$field.as_ref().map_or(0, Vec::len)
        }
    };
}

contribution_vec_counter!(count_middlewares, middlewares);
contribution_vec_counter!(count_transport_adapters, transport_adapters);
contribution_vec_counter!(count_themes, themes);
contribution_vec_counter!(count_editor_languages, editor_languages);
contribution_vec_counter!(count_editor_extensions, editor_extensions);
contribution_vec_counter!(count_notification_channels, notification_channels);
contribution_vec_counter!(count_behaviors, behaviors);
contribution_vec_counter!(count_context_providers, context_providers);
contribution_vec_counter!(count_search_providers, search_providers);
contribution_vec_counter!(count_file_watchers, file_watchers);
contribution_vec_counter!(count_roles, roles);
contribution_vec_counter!(count_tray_items, tray_items);
contribution_vec_counter!(count_layout_overrides, layout_overrides);
contribution_vec_counter!(count_backend_services, backend_services);
contribution_vec_counter!(count_components, components);


fn unsupported_runtime_handlers() -> Vec<Arc<dyn ContributionFamilyHandler>> {
    [
        (
            "gateway.middleware",
            "Gateway Kernel Pipeline",
            count_middlewares as fn(&ExtensionContributes) -> usize,
            // 阶段 7 验收（B-P0-1）：contributes.middlewares 的 module 是扩展 JS，
            // 而 GatewayMiddleware 是 Rust trait（ai/gateway/middleware.rs:107），
            // 需要 worker 轨把 JS middleware 桥接为可执行的 Arc<dyn GatewayMiddleware>。
            // 当前无该运行轨，直接 add_extension 只会注册一个不执行扩展 JS 的空壳，
            // 违反 07 铁律（未接入真实 Pipeline 链路的贡献必须 fail-closed）。
            // 因此保持 fail-closed，待 worker 轨落地后接线。
            Some(
                "阶段 7 验收：JS middleware 需 worker 轨承载为 Rust GatewayMiddleware，暂未接线，保持 fail-closed",
            ),
        ),
        (
            "mcp.transport_adapter",
            "MCP transport registry",
            count_transport_adapters,
            // 阶段 7 验收（B-P0-2）：contributes.transport_adapters 的 module 是扩展 JS，
            // 而 ServerManager::register_transport（server_manager.rs:379）需要
            // Box<dyn TransportAdapter>（tool/mcp/transport/adapter_trait.rs:23）。
            // 无法从声明直接构造 Rust 传输实现，需 worker 轨桥接；保持 fail-closed。
            Some(
                "阶段 7 验收：JS transport adapter 需 worker 轨承载为 Rust TransportAdapter，暂未接线，保持 fail-closed",
            ),
        ),
        ("editor.theme", "Editor theme host", count_themes, None),
        ("editor.extension", "Editor extension host", count_editor_extensions, None),
        ("notification.channel", "Notification host", count_notification_channels, None),
        ("ui.behavior", "UI behavior host", count_behaviors, None),
        ("context.provider", "Context host", count_context_providers, None),
        ("search.provider", "Search host", count_search_providers, None),
        ("file.watcher", "File watcher host", count_file_watchers, None),
        ("extension.role", "Role host", count_roles, None),
        ("tray.item", "Window tray host", count_tray_items, None),
        // layout_overrides 此前 enable 时被静默忽略（不在承接 family、不在拒绝列表），
        // 违反 fail-closed 铁律（34 §10.3）。阶段 6 LayoutOverride 宿主落地前显式拒绝。
        ("layout.override", "Layout override host", count_layout_overrides, None),
    ]
    .into_iter()
    .map(|(family, host, count, note)| {
        Arc::new(UnsupportedContributionFamilyHandler { family, host, count, note })
            as Arc<dyn ContributionFamilyHandler>
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::lifecycle::contributions::UiContributionRegistrar;
    use crate::extension::lifecycle::{ExtensionLifecycle, ExtensionRuntimeHandle};
    use crate::extension::models::{
        BackendProtocol, BackendTransport, ComponentCapabilities, ComponentKind,
        ComponentRegistration, EditorLanguageRegistration, ExtensionManifest, ExtensionPermissions,
        ExtensionState, ExtensionStatus, LayoutOverrideRegistration,
    };
    use crate::extension::skills::Skills;
    use crate::extension::store::ExtensionStore;
    use crate::foundation::config::Config;
    use crate::kernel::{EventBus, InMemoryEventBus};
    use crate::security::sandbox::permission::ApprovalMode;
    use crate::security::sandbox::{CommandRule, RuleAction, Sandbox};
    use crate::domains::editor::backend::BackendProcessManager;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, OnceLock};
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

    /// 放行一切命令的测试沙箱（FullAuto + allow-all 规则）。
    fn allow_all_sandbox() -> Arc<Sandbox> {
        let event_bus: Arc<dyn EventBus> =
            Arc::new(InMemoryEventBus::new(1000, test_runtime_handle()));
        let sandbox = Arc::new(Sandbox::new(event_bus));
        sandbox.set_approval_mode(ApprovalMode::FullAuto).unwrap();
        sandbox
            .set_command_rules(vec![CommandRule {
                pattern: ".*".into(),
                action: RuleAction::Allow,
                description: "test allow-all".into(),
            }])
            .unwrap();
        sandbox
    }

    /// 无害的后端入口文件名（跨平台）。
    fn backend_harness_name() -> &'static str {
        if cfg!(windows) {
            "echo.cmd"
        } else {
            "echo.sh"
        }
    }

    /// 在 install_path 下写入无害后端入口（保持几秒存活）。
    fn write_backend_harness(install_dir: &Path) {
        let backend_dir = install_dir.join("ExtensionBackend");
        std::fs::create_dir_all(&backend_dir).unwrap();
        let entry = backend_dir.join(backend_harness_name());
        if cfg!(windows) {
            std::fs::write(&entry, "@echo off\r\nping 127.0.0.1 -n 5 > nul\r\n").unwrap();
        } else {
            std::fs::write(&entry, "#!/bin/sh\nsleep 5\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
    }

    fn backend_service(id: &str, autostart: bool) -> BackendServiceRegistration {
        BackendServiceRegistration {
            id: id.into(),
            entry: format!("ExtensionBackend/{}", backend_harness_name()),
            transport: BackendTransport::Stdio,
            protocol: BackendProtocol::JsonRpc,
            args: Vec::new(),
            env: HashMap::new(),
            autostart,
        }
    }

    fn create_test_state(id: &str, install_path: PathBuf) -> ExtensionState {
        ExtensionState {
            id: id.to_string(),
            status: ExtensionStatus::Installed,
            manifest: ExtensionManifest {
                id: id.to_string(),
                name: format!("Extension {}", id),
                version: "1.0.0".into(),
                description: "test".into(),
                author: "test".into(),
                permissions: ExtensionPermissions::default(),
                contributes: ExtensionContributes::default(),
            },
            install_path,
            installed_at: chrono::Utc::now(),
            enabled_at: None,
            error: None,
        }
    }

    struct RecordingLsp(Mutex<Vec<LSPServerConfig>>, Mutex<Vec<String>>);

    impl RecordingLsp {
        fn new() -> Self {
            Self(Mutex::new(Vec::new()), Mutex::new(Vec::new()))
        }
    }

    impl LspCapabilityPort for RecordingLsp {
        fn register_language(
            &self,
            config: LSPServerConfig,
            _source: LanguageSource,
        ) -> anyhow::Result<()> {
            self.0.lock().unwrap().push(config);
            Ok(())
        }

        fn unregister_language(&self, language_id: &str, _owner: &str) -> anyhow::Result<()> {
            self.1.lock().unwrap().push(language_id.to_string());
            Ok(())
        }
    }

    fn context_with_lsp<'a>(
        store: &'a ExtensionStore,
        registrar: &'a Mutex<UiContributionRegistrar>,
        lsp: Option<&'a dyn LspCapabilityPort>,
    ) -> ContributionContext<'a> {
        ContributionContext {
            extension_id: "example",
            store,
            gateway: None,
            provider_validation: None,
            ui_contributions: registrar,
            lsp,
            backend_manager: None,
            component_registry: None,
        }
    }

    fn editor_contributes() -> ExtensionContributes {
        let mut contributes = ExtensionContributes::default();
        contributes.editor_languages = Some(vec![EditorLanguageRegistration {
            id: "mylang".into(),
            name: "My Lang".into(),
            extensions: vec![".my".into()],
            module: "./syntax/mylang.js".into(),
        }]);
        contributes
    }

    #[test]
    fn editor_language_family_registers_languages_via_lsp_port() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let lsp = RecordingLsp::new();
        let context = context_with_lsp(&store, &registrar, Some(&lsp));
        let mut registry = ContributionFamilyRegistry::default();
        registry
            .insert_handler(Arc::new(EditorContributionFamilyHandler))
            .unwrap();
        let contributes = editor_contributes();

        let plan = registry.prepare_plan(&context, &contributes).unwrap();
        let mut handle = ExtensionRuntimeHandle::default();
        plan.commit(&context, &mut handle).unwrap();

        let registered = lsp.0.lock().unwrap();
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].language_id, "mylang");
        assert_eq!(registered[0].language_names, vec!["My Lang"]);
        assert_eq!(registered[0].file_extensions, vec![".my"]);
        assert_eq!(handle.languages, vec!["mylang"]);
    }

    #[test]
    fn editor_language_family_fails_closed_without_lsp_host() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = context_with_lsp(&store, &registrar, None);
        let contributes = editor_contributes();

        let error = EditorContributionFamilyHandler
            .preflight(&context, &contributes)
            .unwrap_err()
            .to_string();
        assert!(error.contains("editor languages"));
        assert!(error.contains("LSP host is not available"));
    }

    #[test]
    fn default_registry_always_prepares_ui_family_without_mutating_registrar() {
        let contributes = ExtensionContributes::default();
        let registration = normalize_registration("example", &contributes);
        let mut registrar = UiContributionRegistrar::default();

        validate_registration(&registration).unwrap();
        let prepared = prepare_registration(&registration).unwrap();
        assert_eq!(prepared.extension_id, "example");
        assert!(!registrar.contains_extension("example"));
        registrar.commit_registration(prepared).unwrap();
        assert!(registrar.contains_extension("example"));
    }

    #[test]
    fn registry_rejects_duplicate_family_ids() {
        let mut registry = ContributionFamilyRegistry::default();
        install_builtin_handlers(&mut registry);
        let error = match registry.insert_handler(Arc::new(UiContributionFamilyHandler)) {
            Ok(()) => panic!("duplicate family registration should fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("already registered"));
    }

    #[test]
    fn layout_overrides_are_explicitly_fail_closed() {
        let mut contributes = ExtensionContributes::default();
        contributes.layout_overrides = Some(vec![LayoutOverrideRegistration {
            target: "rightWorkspace".into(),
            position: None,
            offset: None,
            size: None,
            z_index: None,
            transition: None,
        }]);

        let handlers = unsupported_runtime_handlers();
        let layout = handlers
            .iter()
            .find(|handler| handler.family() == "layout.override")
            .expect("layout.override must be an explicit unsupported family");
        assert_eq!(count_layout_overrides(&contributes), 1);

        let store = test_store();
        let context = ContributionContext {
            extension_id: "example",
            store: &store,
            gateway: None,
            provider_validation: None,
            ui_contributions: &Mutex::new(UiContributionRegistrar::default()),
            lsp: None,
            backend_manager: None,
            component_registry: None,
        };
        let error = layout.preflight(&context, &contributes).unwrap_err().to_string();
        assert!(error.contains("unsupported runtime contributes"));
        assert!(error.contains("layout.override"));
    }

    // ---- Backend service family ----

    #[test]
    fn backend_service_family_autostart_spawns_process() {
        // 已启用扩展 + 可执行入口 + 放行 CommandExecute 的沙箱。
        let dir = std::env::temp_dir().join(format!(
            "navis-backend-lifecycle-spawn-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_backend_harness(&dir);

        let store = test_store();
        let mut state = create_test_state("example", dir.clone());
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        let manager = BackendProcessManager::new(allow_all_sandbox());
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = ContributionContext {
            extension_id: "example",
            store: &store,
            gateway: None,
            provider_validation: None,
            ui_contributions: &registrar,
            lsp: None,
            backend_manager: Some(&manager),
            component_registry: None,
        };

        let mut contributes = ExtensionContributes::default();
        contributes.backend_services = Some(vec![backend_service("search", true)]);

        let mut registry = ContributionFamilyRegistry::default();
        registry
            .insert_handler(Arc::new(BackendServiceFamilyHandler))
            .unwrap();
        let plan = registry.prepare_plan(&context, &contributes).unwrap();
        let mut handle = ExtensionRuntimeHandle::default();
        plan.commit(&context, &mut handle).unwrap();

        // autostart 服务已 spawn 并登记。
        assert!(manager.is_running("example", "search"));
        assert!(manager.list(None).contains(&("example".to_string(), "search".to_string())));

        manager.kill_all_for_extension("example");
        assert!(manager.list(None).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backend_service_family_fails_closed_without_backend_host() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = context_with_lsp(&store, &registrar, None);
        let mut contributes = ExtensionContributes::default();
        contributes.backend_services = Some(vec![backend_service("search", true)]);

        let error = BackendServiceFamilyHandler
            .preflight(&context, &contributes)
            .unwrap_err()
            .to_string();
        assert!(error.contains("backend services"));
        assert!(error.contains("backend host is not available"));
    }

    #[test]
    fn backend_service_enable_fails_closed_without_backend_host() {
        // 生命周期未注入 backend_manager：声明 backend_services 的扩展 enable 必须失败。
        let (lifecycle, store) = test_lifecycle_with_store();
        let mut state = create_test_state("backend-no-host", PathBuf::from("/extensions/backend-no-host"));
        state.manifest.contributes.backend_services = Some(vec![backend_service("search", true)]);
        store.register(state).unwrap();

        let result = lifecycle.enable("backend-no-host");
        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("backend services"));
        assert!(message.contains("backend host is not available"));
        assert_eq!(
            store.get("backend-no-host").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn backend_service_enable_autostart_spawns_and_disable_kills() {
        // 生命周期注入 backend_manager：enable 拉进程、disable 清进程。
        let dir = std::env::temp_dir().join(format!(
            "navis-backend-lifecycle-enable-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_backend_harness(&dir);

        let (lifecycle, store) = test_lifecycle_with_store();
        let mut state = create_test_state("backend-autostart", dir.clone());
        state.manifest.contributes.backend_services = Some(vec![backend_service("search", true)]);
        store.register(state).unwrap();

        let manager = Arc::new(BackendProcessManager::new(allow_all_sandbox()));
        let lifecycle = lifecycle.with_backend_manager(manager.clone());

        lifecycle.enable("backend-autostart").unwrap();
        assert!(manager.is_running("backend-autostart", "search"));

        lifecycle.disable("backend-autostart").unwrap();
        assert!(!manager.is_running("backend-autostart", "search"));
        assert!(manager.list(None).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn test_lifecycle_with_store() -> (ExtensionLifecycle, Arc<ExtensionStore>) {
        let event_bus: Arc<dyn EventBus> =
            Arc::new(InMemoryEventBus::new(1000, test_runtime_handle()));
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        (
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus),
            store,
        )
    }

    // ---- Component family ----

    fn component(id: &str, entry: &str, autostart: bool) -> ComponentRegistration {
        ComponentRegistration {
            id: id.into(),
            entry: entry.into(),
            kind: ComponentKind::Logic,
            run_on: Vec::new(),
            capabilities: ComponentCapabilities::default(),
            autostart,
        }
    }

    #[test]
    fn component_family_preflight_fails_closed_without_component_host() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = context_with_lsp(&store, &registrar, None);
        let mut contributes = ExtensionContributes::default();
        contributes.components = Some(vec![component(
            "app",
            "ExtensionUI/scripts/app.component.wasm",
            true,
        )]);

        let error = ComponentFamilyHandler
            .preflight(&context, &contributes)
            .unwrap_err()
            .to_string();
        assert!(error.contains("declares components"));
        assert!(error.contains("component host is not available"));
    }

    #[test]
    fn component_family_normalize_returns_component_registrations() {
        let mut contributes = ExtensionContributes::default();
        contributes.components = Some(vec![component("app", "ExtensionUI/app.wasm", false)]);

        let normalized = ComponentFamilyHandler
            .normalize("example", &contributes)
            .unwrap()
            .expect("components must normalize to a contribution");
        let components = normalized
            .downcast_ref::<Vec<ComponentRegistration>>()
            .unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].id, "app");

        // 空 components 与缺省 contributes 均不产出贡献。
        assert!(ComponentFamilyHandler
            .normalize("example", &ExtensionContributes::default())
            .unwrap()
            .is_none());
    }

    #[test]
    fn component_family_validate_rejects_empty_id_and_entry() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = context_with_lsp(&store, &registrar, None);

        let empty_id = ComponentFamilyHandler
            .normalize(
                "example",
                &ExtensionContributes {
                    components: Some(vec![component("", "ExtensionUI/app.wasm", false)]),
                    ..Default::default()
                },
            )
            .unwrap()
            .unwrap();
        let error = ComponentFamilyHandler
            .validate(&context, &empty_id)
            .unwrap_err()
            .to_string();
        assert!(error.contains("empty id"));

        let empty_entry = ComponentFamilyHandler
            .normalize(
                "example",
                &ExtensionContributes {
                    components: Some(vec![component("app", "", false)]),
                    ..Default::default()
                },
            )
            .unwrap()
            .unwrap();
        let error = ComponentFamilyHandler
            .validate(&context, &empty_entry)
            .unwrap_err()
            .to_string();
        assert!(error.contains("empty entry"));
    }

    #[test]
    fn component_family_commit_fails_closed_without_component_host() {
        let store = test_store();
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = context_with_lsp(&store, &registrar, None);
        let mut contributes = ExtensionContributes::default();
        contributes.components = Some(vec![component("app", "ExtensionUI/app.wasm", true)]);

        let mut family_registry = ContributionFamilyRegistry::default();
        family_registry
            .insert_handler(Arc::new(ComponentFamilyHandler))
            .unwrap();
        let plan = family_registry.prepare_plan(&context, &contributes).unwrap();
        let mut handle = ExtensionRuntimeHandle::default();
        let error = plan
            .commit(&context, &mut handle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("component host is not available"));
    }

    #[test]
    fn component_family_commit_routes_components_to_registry_load() {
        // 注入真实 component host 后，commit 把组件交给 ComponentRegistry.load 登记；
        // 入口 wasm 缺失 → load 失败 → commit 返回带组件 ID 的错误（验证接线与
        // 错误传播）。load 成功 + activate 的端到端验证依赖并行 ComponentRegistry
        // 的真实 wasm 组件 fixture，由 C1-5 端到端验证承接。
        let dir = std::env::temp_dir().join(format!(
            "navis-component-lifecycle-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let store = test_store();
        let mut state = create_test_state("example", dir.clone());
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        let registry = Arc::new(crate::extension::component::ComponentRegistry::new(
            allow_all_sandbox(),
            Arc::new(crate::extension::operation_runtime::OperationRegistry::default()),
            Arc::clone(&store),
        ));
        let registrar = Mutex::new(UiContributionRegistrar::default());
        let context = ContributionContext {
            extension_id: "example",
            store: &store,
            gateway: None,
            provider_validation: None,
            ui_contributions: &registrar,
            lsp: None,
            backend_manager: None,
            component_registry: Some(&registry),
        };

        let mut contributes = ExtensionContributes::default();
        contributes.components = Some(vec![component(
            "app",
            "ExtensionUI/missing.component.wasm",
            true,
        )]);

        let mut family_registry = ContributionFamilyRegistry::default();
        family_registry
            .insert_handler(Arc::new(ComponentFamilyHandler))
            .unwrap();
        let plan = family_registry.prepare_plan(&context, &contributes).unwrap();
        let mut handle = ExtensionRuntimeHandle::default();
        let error = plan
            .commit(&context, &mut handle)
            .unwrap_err()
            .to_string();
        assert!(error.contains("failed to load component 'app'"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn component_enable_injects_registry_and_fails_closed_on_load_error() {
        // 生命周期经 with_component_registry 注入 host：声明 components 的扩展
        // enable 时 preflight 通过、commit 把组件交给 registry.load；入口缺失 →
        // enable 失败且扩展状态回到 Error（保持原子性）。
        let (lifecycle, store) = test_lifecycle_with_store();
        let mut state = create_test_state(
            "component-host",
            PathBuf::from("/extensions/component-host"),
        );
        state.manifest.contributes.components = Some(vec![component(
            "app",
            "ExtensionUI/missing.component.wasm",
            true,
        )]);
        store.register(state).unwrap();

        let registry = Arc::new(crate::extension::component::ComponentRegistry::new(
            allow_all_sandbox(),
            Arc::new(crate::extension::operation_runtime::OperationRegistry::default()),
            Arc::clone(&store),
        ));
        let lifecycle = lifecycle.with_component_registry(registry);

        let result = lifecycle.enable("component-host");
        let message = result.unwrap_err().to_string();
        assert!(message.contains("failed to load component 'app'"));
        assert_eq!(
            store.get("component-host").unwrap().status,
            ExtensionStatus::Error
        );
    }
}
