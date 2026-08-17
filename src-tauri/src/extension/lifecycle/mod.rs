//! Extension 生命周期管理
//!
//! 基于设计文档 §07 五、扩展生命周期 实现。
//!
//! 职责：
//! - 管理扩展的完整生命周期：Installed -> Loading -> Enabled -> Disabling -> Disabled -> Unloading
//! - 启用扩展时把 contributes 交给对应宿主子系统，或登记 UI/Hook 声明索引
//! - 禁用扩展时注销 contributes 声明和宿主子系统资源
//! - 通过 Kernel EventBus 发布生命周期事件
//!
//! 本模块不执行 hook 脚本，不承载 provider/tool/transport 运行能力；
//! 这些能力必须进入 MCP/Gateway/Kernel Pipeline 等宿主链路。

mod contributions;
pub mod cordis;
mod event;
mod families;
mod install;
mod register;
mod state;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::extension::types::CustomProtocolConfig;
use crate::extension::types::{ApiProtocol, ProviderConfig};
use crate::extension::models::{
    LSPServerConfig, LanguageSource, ViewRegistration, WorkModeRegistration,
};
use crate::extension::provider_validation::ExtensionProviderValidationPort;
use crate::extension::skills::Skills;
use crate::kernel::EventBus;
use crate::extension::types::{MCPServerConfig, ToolDefinitionOverride};

use super::context::HostExtensionContext;
use super::store::ExtensionStore;
use super::component::ComponentRegistry;

use contributions::UiContributionRegistrar;
pub(crate) use event::{
    EventSubscriptionPort, ExtensionSubscriptionLedger, KernelEventSubscriptionAdapter,
};
use families::{install_builtin_handlers, ContributionFamilyRegistry};

pub use cordis::ExtensionCordisPlugin;

pub(crate) use families::ContributionFamilyHandler;

// ── Shared ID prefixes ──────────────────────────────────────────────────────

/// 扩展 Gateway provider ID 前缀：`extension:`
const EXTENSION_PROVIDER_PREFIX: &str = "extension:";
/// MCP override server ID 识别前缀：`extension:`
const MCP_OVERRIDE_SERVER_PREFIX: &str = "extension:";

/// 生成 Extension provider 的唯一 runtime ID：`extension:{extension_id}/{provider_id}`。
///
/// Provider ID 是生命周期资源主键，注册、回滚和禁用必须统一使用该函数生成的值。
pub(crate) fn extension_provider_id(
    extension_id: &str,
    provider_id: &str,
) -> anyhow::Result<String> {
    if extension_id.trim().is_empty() || provider_id.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Extension provider ID segments must not be empty"
        ));
    }
    if extension_id.contains('/') || provider_id.contains('/') {
        return Err(anyhow::anyhow!(
            "Extension provider ID segments must not contain '/'"
        ));
    }
    Ok(format!(
        "{}{}/{}",
        EXTENSION_PROVIDER_PREFIX, extension_id, provider_id
    ))
}

/// Gateway 能力端口。生命周期只依赖扩展 Provider 的注册/注销能力。
pub trait GatewayCapabilityPort: Send + Sync {
    fn upsert_provider(&self, config: ProviderConfig) -> anyhow::Result<()>;
    fn set_provider_capabilities(
        &self,
        provider_id: &str,
        capabilities: crate::extension::types::CapabilitySet,
    ) -> anyhow::Result<()>;
    fn remove_provider_capabilities(&self, provider_id: &str) -> anyhow::Result<()>;
    fn remove_provider(&self, id: &str) -> anyhow::Result<()>;
    fn acquire_protocol(&self, owner: &str, protocol: &ApiProtocol) -> anyhow::Result<()>;
    fn register_custom_protocol(
        &self,
        owner: &str,
        config: CustomProtocolConfig,
    ) -> anyhow::Result<()>;
    fn release_protocol(&self, owner: &str, protocol: &ApiProtocol) -> anyhow::Result<()>;
}

/// MCP 能力端口。生命周期只依赖扩展 Server、工具声明和覆盖操作。
pub trait McpCapabilityPort: Send + Sync {
    fn add_server(&self, config: MCPServerConfig) -> anyhow::Result<()>;
    fn start_server(&self, id: &str) -> anyhow::Result<()>;
    fn remove_server(&self, id: &str) -> anyhow::Result<()>;
    fn register_tool(&self, tool: crate::extension::types::ToolDefinition)
        -> anyhow::Result<()>;
    fn unregister_server_tools(&self, server_id: &str) -> anyhow::Result<usize>;
    fn apply_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
        override_: ToolDefinitionOverride,
    ) -> anyhow::Result<()>;
    fn remove_tool_override(
        &self,
        owner: &str,
        server_id: &str,
        tool_name: &str,
    ) -> anyhow::Result<()>;
}

/// LSP 能力端口。生命周期只依赖语言注册/注销能力。
pub trait LspCapabilityPort: Send + Sync {
    fn register_language(
        &self,
        config: LSPServerConfig,
        source: LanguageSource,
    ) -> anyhow::Result<()>;
    fn unregister_language(&self, language_id: &str, owner: &str) -> anyhow::Result<()>;
}

// ── Struct + Builder ─────────────────────────────────────────────────────────

/// Extension 生命周期管理。
///
/// enable/disable 操作委托给 kernel::Registry.lifecycle()。
/// 副作用（注册 Provider/Tool/Skill/Constraint）通过对应子系统的内核 Registry 完成。
///
/// 本模块不执行 hook 脚本，不承载 provider/tool/transport 运行能力；
/// 这些能力必须进入 MCP/Gateway/Kernel Pipeline 等宿主链路。
pub struct ExtensionLifecycle {
    /// 扩展状态存储
    pub(crate) store: Arc<ExtensionStore>,
    /// Skills 共享状态
    pub(crate) skills: Option<Arc<Mutex<Skills>>>,
    /// 事件总线
    pub(crate) event_bus: Arc<dyn EventBus>,
    /// MCP 工具引擎。未注入时生命周期仍可用于纯 UI / Skills 扩展测试。
    pub(crate) mcp: Option<Arc<dyn McpCapabilityPort>>,
    /// LSP 宿主。未注入时带语言 contribution 的扩展必须 fail-closed。
    pub(crate) lsp: Option<Arc<dyn LspCapabilityPort>>,
    /// 后端进程管理器。未注入时声明 backend_services 的扩展必须 fail-closed。
    pub(crate) backend_manager: Option<Arc<dyn crate::extension::lifecycle::cordis::BackendProcessPort>>,
    /// WASM 组件注册表。未注入时声明 components 的扩展必须 fail-closed。
    pub(crate) component_registry: Option<Arc<ComponentRegistry>>,
    /// 全局策略引擎。注入后 enable/disable 时自动注册/注销扩展权限约束。
    pub(crate) policy_engine: Option<Arc<crate::kernel::PolicyEngine>>,
    /// Gateway 宿主。注入后 enable/disable 时自动注册/注销扩展 Provider 适配器。
    pub(crate) gateway: Option<Arc<dyn GatewayCapabilityPort>>,
    /// Extension-owned Provider validation registry. Validation contracts are
    /// lifecycle resources and must be removed together with their Provider.
    pub(crate) provider_validation: Option<Arc<dyn ExtensionProviderValidationPort>>,
    /// Extension runtime 事件订阅宿主。当前仅提供隔离端口，声明式订阅
    /// 在 runtime handler contract 落地前不会注册。
    pub(crate) event_subscriptions: Option<Arc<dyn EventSubscriptionPort>>,
    /// Extension-owned subscription ledger；只记录已经成功注册的 opaque 句柄。
    pub(crate) subscription_ledger: Arc<Mutex<ExtensionSubscriptionLedger>>,
    /// Declarative UI contribution registrar.
    pub(crate) ui_contributions: Arc<Mutex<UiContributionRegistrar>>,
    /// Contribution family handlers; each family owns its own transaction boundary.
    /// Arc 使 apply/cleanup disposer 能在 'static 闭包内共享 handler 注册表。
    pub(crate) contribution_families: Arc<ContributionFamilyRegistry>,
    /// 已提交的运行时资源句柄。disable 只消费这里保存的 opaque 资源事实，
    /// 不根据可变 manifest 重新推导运行时 ID。
    pub(crate) runtime_handles: Arc<Mutex<HashMap<String, ExtensionRuntimeHandle>>>,
    /// Cordis 宿主运行时。每个启用扩展对应一个 fiber：enable 安装 fiber（apply
    /// 经 `ctx.effect` 登记 cleanup disposer），disable 经 fiber dispose 逆序撤销
    /// 副作用。未注入时每个生命周期自持一个空宿主。
    pub(crate) cordis_host: Arc<HostExtensionContext>,
}

/// Extension enable 成功后保存的运行时资源句柄。
///
/// 句柄只包含宿主注册时返回/确认的完整 ID 和撤销参数，生命周期注销不再
/// 依赖 manifest 重新拼接 ID，因此即使 manifest 后续被更新，清理仍然精确。
#[derive(Debug, Clone, Default)]
pub(crate) struct ExtensionRuntimeHandle {
    pub(crate) provider_ids: Vec<String>,
    pub(crate) provider_capability_ids: Vec<String>,
    pub(crate) provider_validation_ids: Vec<String>,
    pub(crate) protocols: Vec<crate::extension::types::ApiProtocol>,
    pub(crate) tool_server_ids: Vec<String>,
    pub(crate) tool_overrides: Vec<(String, String)>,
    pub(crate) mcp_servers: Vec<String>,
    pub(crate) skills: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) ui_contribution_registered: bool,
    pub(crate) projection: ExtensionRuntimeProjection,
}

/// Lifecycle-owned projection of resources that were actually registered.
///
/// This snapshot is the runtime fact consumed by UI projection and cleanup;
/// manifest contributes are only enable-time input.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ExtensionRuntimeProjection {
    pub(crate) contribution_counts: ExtensionContributionCounts,
    pub(crate) views: Vec<ViewRegistration>,
    pub(crate) work_modes: Vec<WorkModeRegistration>,
    pub(crate) zones: Vec<super::models::ZoneRegistration>,
    pub(crate) scripts: Vec<super::models::ScriptRegistration>,
    pub(crate) toolbar_items: Vec<super::models::ToolbarItemRegistration>,
    pub(crate) statusbar_items: Vec<super::models::StatusBarItemRegistration>,
    pub(crate) inline_extensions: Vec<super::models::InlineExtensionRegistration>,
    pub(crate) configuration: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ExtensionContributionCounts {
    pub(crate) work_modes: usize,
    pub(crate) views: usize,
    pub(crate) menus: usize,
    pub(crate) commands: usize,
    pub(crate) keybindings: usize,
    pub(crate) triggers: usize,
    pub(crate) mcp_servers: usize,
    pub(crate) providers: usize,
    pub(crate) zones: usize,
    pub(crate) scripts: usize,
    pub(crate) toolbar_items: usize,
    pub(crate) statusbar_items: usize,
    pub(crate) inline_extensions: usize,
    pub(crate) configuration: usize,
}

impl ExtensionRuntimeHandle {
    /// Reconcile the read-only projection with resources that are still owned
    /// by this lifecycle transaction.
    pub(crate) fn reconcile_projection(&mut self) {
        self.projection.contribution_counts.providers = self.provider_ids.len();
        self.projection.contribution_counts.mcp_servers = self.mcp_servers.len();

        if !self.ui_contribution_registered {
            self.projection.views.clear();
            self.projection.work_modes.clear();
            self.projection.zones.clear();
            self.projection.scripts.clear();
            self.projection.toolbar_items.clear();
            self.projection.statusbar_items.clear();
            self.projection.inline_extensions.clear();
            self.projection.configuration = None;
            self.projection.contribution_counts.work_modes = 0;
            self.projection.contribution_counts.views = 0;
            self.projection.contribution_counts.menus = 0;
            self.projection.contribution_counts.commands = 0;
            self.projection.contribution_counts.keybindings = 0;
            self.projection.contribution_counts.triggers = 0;
            self.projection.contribution_counts.zones = 0;
            self.projection.contribution_counts.scripts = 0;
            self.projection.contribution_counts.toolbar_items = 0;
            self.projection.contribution_counts.statusbar_items = 0;
            self.projection.contribution_counts.inline_extensions = 0;
            self.projection.contribution_counts.configuration = 0;
        }

        if self.is_empty() {
            self.projection = ExtensionRuntimeProjection::default();
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.provider_ids.is_empty()
            && self.provider_capability_ids.is_empty()
            && self.provider_validation_ids.is_empty()
            && self.protocols.is_empty()
            && self.tool_server_ids.is_empty()
            && self.tool_overrides.is_empty()
            && self.mcp_servers.is_empty()
            && self.skills.is_empty()
            && self.languages.is_empty()
            && !self.ui_contribution_registered
            && self.projection == ExtensionRuntimeProjection::default()
    }
}

impl ExtensionLifecycle {
    /// 创建新的生命周期管理器
    pub fn new(
        store: Arc<ExtensionStore>,
        skills: Arc<Mutex<Skills>>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self::with_skills(store, Some(skills), event_bus)
    }

    /// 创建不装配业务 Skills 的容器白板生命周期。
    ///
    /// 白板容器启动（NAVIS_WHITEBOARD=1）时没有业务 Skills 状态，扩展生命周期
    /// 仍可装配 UI / 命令 / 组件 / MCP / LSP 等平台能力；声明 `contributes.skills`
    /// 的扩展在缺少 Skills 宿主时 fail-closed。
    pub fn new_without_skills(
        store: Arc<ExtensionStore>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self::with_skills(store, None, event_bus)
    }

    fn with_skills(
        store: Arc<ExtensionStore>,
        skills: Option<Arc<Mutex<Skills>>>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        tracing::info!("Creating ExtensionLifecycle");
        let mut contribution_families = ContributionFamilyRegistry::default();
        install_builtin_handlers(&mut contribution_families);
        Self {
            store,
            skills,
            event_bus,
            mcp: None,
            lsp: None,
            backend_manager: None,
            component_registry: None,
            policy_engine: None,
            gateway: None,
            provider_validation: None,
            event_subscriptions: None,
            subscription_ledger: Arc::new(Mutex::new(ExtensionSubscriptionLedger::default())),
            ui_contributions: Arc::new(Mutex::new(UiContributionRegistrar::default())),
            contribution_families: Arc::new(contribution_families),
            runtime_handles: Arc::new(Mutex::new(HashMap::new())),
            cordis_host: Arc::new(HostExtensionContext::new()),
        }
    }

    /// 注入 MCP 引擎，使扩展 MCP server 和工具覆盖进入 MCP 宿主链路。
    pub fn with_mcp(mut self, mcp: Arc<dyn McpCapabilityPort>) -> Self {
        self.mcp = Some(mcp);
        self
    }

    /// 注入 LSP 宿主，使扩展语言贡献进入 LSP Kernel-backed Registry。
    pub fn with_lsp(mut self, lsp: Arc<dyn LspCapabilityPort>) -> Self {
        self.lsp = Some(lsp);
        self
    }

    /// 注入后端进程管理器，使扩展 `contributes.backend_services` 进入
    /// 独立进程容器（spawn/kill 由管理器按 `(extension_id, service_id)` 管理）。
    pub fn with_backend_manager(
        mut self,
        manager: Arc<dyn crate::extension::lifecycle::cordis::BackendProcessPort>,
    ) -> Self {
        self.backend_manager = Some(manager);
        self
    }

    /// 注入 WASM 组件注册表，使扩展 `contributes.components` 进入组件轨
    /// （wasmtime 实例化 + Sandbox CommandExecute 门禁 + host 接口注入）。
    pub fn with_component_registry(mut self, registry: Arc<ComponentRegistry>) -> Self {
        self.component_registry = Some(registry);
        self
    }

    /// 注入全局 PolicyEngine，enable/disable 时自动注册/注销扩展权限约束。
    pub fn with_policy_engine(mut self, engine: Arc<crate::kernel::PolicyEngine>) -> Self {
        self.policy_engine = Some(engine);
        self
    }

    /// 注入 Gateway 宿主，enable/disable 时自动注册/注销扩展 Provider 适配器。
    pub fn with_gateway(mut self, gateway: Arc<dyn GatewayCapabilityPort>) -> Self {
        self.gateway = Some(gateway);
        self
    }

    /// 注入 Extension-owned Provider validation registry。
    pub fn with_provider_validation(
        mut self,
        provider_validation: Arc<dyn ExtensionProviderValidationPort>,
    ) -> Self {
        self.provider_validation = Some(provider_validation);
        self
    }

    /// 注入 Extension 事件订阅端口。
    ///
    /// 该端口不会让 lifecycle 直接依赖 Kernel EventBus；只有明确的
    /// Extension runtime handler 入口才能调用订阅注册流程。
    pub fn with_event_subscription_port(mut self, port: Arc<dyn EventSubscriptionPort>) -> Self {
        self.event_subscriptions = Some(port);
        self
    }

    /// 注入 Cordis 宿主运行时。
    ///
    /// 生产装配时传入全局 `HostExtensionContext`，使 enable 安装的 fiber 与
    /// disable 的 fiber dispose 指向同一个宿主（fiber 按扩展 ID 跟踪）。未注入
    /// 时生命周期自持一个空宿主，测试与白板空壳场景同样可用。
    pub fn with_cordis(mut self, host: Arc<HostExtensionContext>) -> Self {
        self.cordis_host = host;
        self
    }

    /// Add one contribution family handler at the composition root.
    #[allow(dead_code)]
    pub(crate) fn with_contribution_family_handler(
        mut self,
        handler: Arc<dyn ContributionFamilyHandler>,
    ) -> anyhow::Result<Self> {
        Arc::make_mut(&mut self.contribution_families).insert_handler(handler)?;
        Ok(self)
    }

    /// Lock the two lifecycle ledgers in the only supported order.
    ///
    /// Runtime handles and subscription records describe one committed
    /// lifecycle transaction. Any code that needs both must acquire the
    /// runtime handle lock first and the subscription ledger lock second.
    pub(crate) fn lock_runtime_and_subscription_ledger(
        &self,
    ) -> anyhow::Result<(
        MutexGuard<'_, HashMap<String, ExtensionRuntimeHandle>>,
        MutexGuard<'_, ExtensionSubscriptionLedger>,
    )> {
        let handles = self.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        let ledger = self.subscription_ledger.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension subscription ledger: {error}")
        })?;
        Ok((handles, ledger))
    }

    pub(crate) fn contribution_context<'a>(
        &'a self,
        extension_id: &'a str,
    ) -> families::ContributionContext<'a> {
        families::ContributionContext {
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

    /// Return the committed runtime projection for UI/read-only hosts.
    pub(crate) fn runtime_projection(
        &self,
        extension_id: &str,
    ) -> anyhow::Result<Option<ExtensionRuntimeProjection>> {
        let handles = self.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        Ok(handles
            .get(extension_id)
            .map(|handle| handle.projection.clone()))
    }

    /// Return all committed runtime projections keyed by extension ID.
    pub(crate) fn runtime_projections(
        &self,
    ) -> anyhow::Result<HashMap<String, ExtensionRuntimeProjection>> {
        let handles = self.runtime_handles.lock().map_err(|error| {
            anyhow::anyhow!("Failed to lock Extension runtime handles: {error}")
        })?;
        Ok(handles
            .iter()
            .map(|(id, handle)| (id.clone(), handle.projection.clone()))
            .collect())
    }
}

// ── Shared helper functions ──────────────────────────────────────────────────

/// 扩展工具统一 server_id：`extension:{extension_id}`
///
/// 所有同一扩展声明的工具共享此 server_id，禁用时按此 id 整体移除。
pub(crate) fn extension_tool_server_id(extension_id: &str) -> String {
    format!("extension:{}", extension_id)
}

/// 扩展 MCP server id：`extension:{extension_id}/{server_name}`
pub(crate) fn extension_mcp_server_id(extension_id: &str, server_name: &str) -> String {
    format!("extension:{}/{}", extension_id, server_name)
}

/// 合法的状态转换
/// Installed -> Loading -> Enabled
/// Enabled -> Disabling -> Disabled
/// Disabled -> Loading -> Enabled  (重新启用)
/// Any -> Unloading  (卸载前)
///
/// 状态转换合法性由 ExtensionStore::is_valid_transition 和 kernel InMemoryRegistry.lifecycle 共同保证。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::*;
    use crate::extension::skills::Skills;
    use crate::foundation::config::Config;
//     use [REMOVED: MCP reference]
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

    fn create_test_state(id: &str, status: ExtensionStatus) -> ExtensionState {
        ExtensionState {
            id: id.to_string(),
            status,
            manifest: ExtensionManifest {
                id: id.to_string(),
                name: format!("Extension {}", id),
                version: "1.0.0".into(),
                description: "test".into(),
                author: "test".into(),
                permissions: ExtensionPermissions::default(),
                contributes: ExtensionContributes::default(),
            },
            install_path: PathBuf::from(format!("/extensions/{}", id)),
            installed_at: chrono::Utc::now(),
            enabled_at: None,
            error: None,
        }
    }

    fn setup() -> (ExtensionLifecycle, Arc<ExtensionStore>) {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus);
        (lifecycle, store)
    }

    #[test]
    fn test_enable_installed_extension() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Installed))
            .unwrap();

        lifecycle.enable("extension-a").unwrap();

        let state = store.get("extension-a").unwrap();
        assert_eq!(state.status, ExtensionStatus::Enabled);
    }

    #[test]
    fn test_enable_disabled_extension() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Disabled))
            .unwrap();

        lifecycle.enable("extension-a").unwrap();

        let state = store.get("extension-a").unwrap();
        assert_eq!(state.status, ExtensionStatus::Enabled);
    }

    #[test]
    fn test_enable_already_enabled_fails() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Enabled))
            .unwrap();

        let result = lifecycle.enable("extension-a");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid transition"));
    }

    #[test]
    fn test_enable_nonexistent_fails() {
        let (lifecycle, _registry) = setup();
        let result = lifecycle.enable("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_disable_enabled_extension() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Enabled))
            .unwrap();

        lifecycle.disable("extension-a").unwrap();

        let state = store.get("extension-a").unwrap();
        assert_eq!(state.status, ExtensionStatus::Disabled);
    }

    #[test]
    fn test_disable_installed_extension_fails() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Installed))
            .unwrap();

        let result = lifecycle.disable("extension-a");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid transition"));
    }

    #[test]
    fn test_disable_already_disabled_fails() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Disabled))
            .unwrap();

        let result = lifecycle.disable("extension-a");
        assert!(result.is_err());
    }

    #[test]
    fn test_enable_disable_cycle() {
        let (lifecycle, store) = setup();
        store
            .register(create_test_state("extension-a", ExtensionStatus::Installed))
            .unwrap();

        // 启用
        lifecycle.enable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Enabled
        );

        // 禁用
        lifecycle.disable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Disabled
        );

        // 重新启用
        lifecycle.enable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Enabled
        );
    }

    #[test]
    fn test_lifecycle_transitions_through_store() {
        let (lifecycle, store) = setup();

        // Installed -> (enable) -> Loading -> Enabled -> (disable) -> Disabling -> Disabled
        store
            .register(create_test_state("extension-a", ExtensionStatus::Installed))
            .unwrap();

        // Installed -> Loading -> Enabled
        lifecycle.enable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Enabled
        );

        // Enabled -> Disabling -> Disabled
        lifecycle.disable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Disabled
        );

        // Disabled -> Loading -> Enabled (重新启用)
        lifecycle.enable("extension-a").unwrap();
        assert_eq!(
            store.get("extension-a").unwrap().status,
            ExtensionStatus::Enabled
        );

        // Error -> (enable resets to Installed) -> Loading -> Enabled.
        // Use a fresh lifecycle/store so the simulated error has no live
        // contribution registration left behind by the previous cycle.
        let (error_lifecycle, error_store) = setup();
        error_store
            .register(create_test_state("extension-error", ExtensionStatus::Error))
            .unwrap();

        assert_eq!(
            error_store.get("extension-error").unwrap().status,
            ExtensionStatus::Error
        );
        error_lifecycle.enable("extension-error").unwrap();
        assert_eq!(
            error_store.get("extension-error").unwrap().status,
            ExtensionStatus::Enabled
        );
    }

    #[test]
    fn test_enable_with_contributes_emits_events() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));

        // 创建一个带有 contributes 的扩展
        let manifest = ExtensionManifest {
            id: "extension-b".into(),
            name: "Extension B".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                views: Some(vec![ViewRegistration {
                    id: "test.view".into(),
                    name: "Test View".into(),
                    icon: None,
                    entry: None,
                    zone: None,
            placement: Some("rightWorkspace".into()),
                    renderer: crate::extension::host_view::HOST_PANEL_RENDERER.into(),
                    config: None,
                    activation_events: vec![],
                    allow_close: None,
                    default_visible: None,
                }]),
                commands: Some(vec![CommandRegistration {
                    id: "test.cmd".into(),
                    label: "Test Command".into(),
                    description: None,
                    icon: None,
                    category: None,
                    when: None,
                    action: BuiltinAction::OpenView {
                        view_id: "test.view".into(),
                    },
                }]),

                ..Default::default()
            },
        };

        let state = ExtensionState {
            id: "extension-b".into(),
            status: ExtensionStatus::Installed,
            manifest,
            install_path: PathBuf::from("/extensions/extension-b"),
            installed_at: chrono::Utc::now(),
            enabled_at: None,
            error: None,
        };

        store.register(state).unwrap();

        // 用计数器跟踪事件
        let event_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = Arc::clone(&event_count);
        event_bus
            .subscribe(
                None,
                None,
                Arc::new(move |_event| {
                    count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }),
            )
            .unwrap();

        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus);
        lifecycle.enable("extension-b").unwrap();

        let mut total_events = event_count.load(std::sync::atomic::Ordering::SeqCst);
        for _ in 0..50 {
            if total_events >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            total_events = event_count.load(std::sync::atomic::Ordering::SeqCst);
        }
        assert!(
            total_events >= 2,
            "Expected enabling and enabled events, got {}",
            total_events
        );
    }

    #[test]
    fn test_ui_contributes_register_and_unregister_as_one_projection_unit() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));

        let mut state = create_test_state("extension-ui-declarations", ExtensionStatus::Installed);
        state.manifest.contributes.views = Some(vec![ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: Some("rightWorkspace".into()),
            renderer: crate::extension::host_view::HOST_PANEL_RENDERER.into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);
        state.manifest.contributes.commands = Some(vec![CommandRegistration {
            id: "test.cmd".into(),
            label: "Test Command".into(),
            description: None,
            icon: None,
            category: None,
            when: None,
            action: BuiltinAction::OpenView {
                view_id: "test.view".into(),
            },
        }]);

        state.manifest.contributes.hooks = Some(vec![HookRegistration {
            id: "guard-bash".into(),
            name: "Guard Bash".into(),
            phase: HookPhase::PreToolUse,
            priority: Some(20),
            module: "./hooks/guard-bash.js".into(),
            when: Some("tool.name == 'bash'".into()),
            action: Default::default(),
        }]);
        store.register(state).unwrap();

        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), skills, Arc::clone(&event_bus));
        lifecycle.enable("extension-ui-declarations").unwrap();

        let registered = lifecycle
            .ui_contributions
            .lock()
            .unwrap()
            .get("extension-ui-declarations");
        assert!(registered.is_some());
        let registered = registered.unwrap();
        assert_eq!(registered.views.len(), 1);
        assert_eq!(registered.commands.len(), 1);

        lifecycle.disable("extension-ui-declarations").unwrap();

        assert!(lifecycle
            .ui_contributions
            .lock()
            .unwrap()
            .get("extension-ui-declarations")
            .is_none());

        assert!(lifecycle
            .runtime_projection("extension-ui-declarations")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_enable_registers_extension_skill_and_disable_unregisters_it() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));

        let manifest = ExtensionManifest {
            id: "extension-skill".into(),
            name: "Extension Skill".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                skills: Some(vec![crate::extension::models::SkillDefinition {
                    id: "pr-summary".into(),
                    name: "pr-summary".into(),
                    description: Some("Summarize a pull request".into()),
                    config: serde_json::json!({
                        "mode": "standard",
                        "trigger": "/pr-summary",
                        "tools": ["git"],
                        "content": "Summarize the current pull request."
                    }),
                }]),
                ..Default::default()
            },
        };

        store
            .register(ExtensionState {
                id: "extension-skill".into(),
                status: ExtensionStatus::Installed,
                manifest,
                install_path: PathBuf::from("/extensions/extension-skill"),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();

        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), Arc::clone(&skills), event_bus);
        lifecycle.enable("extension-skill").unwrap();
        assert!(skills
            .lock()
            .unwrap()
            .get("extension:extension-skill/pr-summary")
            .is_some());

        lifecycle.disable("extension-skill").unwrap();
        assert!(skills
            .lock()
            .unwrap()
            .get("extension:extension-skill/pr-summary")
            .is_none());
    }

    #[test]
    fn test_enable_and_disable_declares_extension_hooks() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state("extension-hooks", ExtensionStatus::Installed);
        state.manifest.contributes.hooks = Some(vec![HookRegistration {
            id: "guard-bash".into(),
            name: "Guard Bash".into(),
            phase: HookPhase::PreToolUse,
            priority: Some(20),
            module: "./hooks/guard-bash.js".into(),
            when: Some("tool.name == 'bash'".into()),
            action: Default::default(),
        }]);
        store.register(state).unwrap();

        lifecycle.enable("extension-hooks").unwrap();
        let hooks = store.list_hook_declarations_by_phase(HookPhase::PreToolUse);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].runtime_id, "extension-hooks/guard-bash");

        lifecycle.disable("extension-hooks").unwrap();
        assert!(store
            .list_hook_declarations_by_phase(HookPhase::PreToolUse)
            .is_empty());
    }

    #[test]
    fn test_enable_fails_closed_on_duplicate_hook_declarations() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state("extension-duplicate-hooks", ExtensionStatus::Installed);
        state.manifest.contributes.hooks = Some(vec![
            HookRegistration {
                id: "same".into(),
                name: "Same A".into(),
                phase: HookPhase::PreToolUse,
                priority: Some(10),
                module: "./hooks/a.js".into(),
                when: None,
                action: Default::default(),
            },
            HookRegistration {
                id: "same".into(),
                name: "Same B".into(),
                phase: HookPhase::PostToolUse,
                priority: Some(20),
                module: "./hooks/b.js".into(),
                when: None,
                action: Default::default(),
            },
        ]);
        store.register(state).unwrap();

        let result = lifecycle.enable("extension-duplicate-hooks");

        assert!(result.is_err());
        assert_eq!(
            store.get("extension-duplicate-hooks").unwrap().status,
            ExtensionStatus::Error
        );
        assert!(store.list_hooks().is_empty());
    }

    #[test]
    fn test_enable_fails_closed_on_unsupported_runtime_contributes() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state("extension-middleware", ExtensionStatus::Installed);
        state.manifest.contributes.middlewares = Some(vec![MiddlewareRegistration {
            id: "custom-middleware".into(),
            name: "Custom Middleware".into(),
            phase: MiddlewarePhase::PreRequest,
            module: "./middleware.js".into(),
        }]);
        store.register(state).unwrap();

        let result = lifecycle.enable("extension-middleware");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("unsupported runtime contributes"));
        assert!(message.contains("gateway.middleware"));
        assert_eq!(
            store.get("extension-middleware").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_runtime_only_contributions_fail_closed_with_family_name() {
        let cases: Vec<(&str, ExtensionContributes, &str)> = vec![
            (
                "extension-transport-adapter",
                ExtensionContributes {
                    transport_adapters: Some(vec![TransportAdapterRegistration {
                        id: "custom-transport".into(),
                        name: "Custom Transport".into(),
                        transport_type: "custom".into(),
                        module: "./transport.js".into(),
                    }]),
                    ..ExtensionContributes::default()
                },
                "mcp.transport_adapter",
            ),
            (
                "extension-notification-channel",
                ExtensionContributes {
                    notification_channels: Some(vec![NotificationChannelRegistration {
                        id: "custom-channel".into(),
                        name: "Custom Channel".into(),
                        description: "Custom notification channel".into(),
                        config_schema: serde_json::json!({"type": "object"}),
                        module: "./notification.js".into(),
                    }]),
                    ..ExtensionContributes::default()
                },
                "notification.channel",
            ),
            (
                "extension-editor-theme",
                ExtensionContributes {
                    themes: Some(vec![ThemeRegistration {
                        id: "custom-theme".into(),
                        name: "Custom Theme".into(),
                        theme_type: ThemeType::Dark,
                        module: "./theme.js".into(),
                    }]),
                    ..ExtensionContributes::default()
                },
                "editor.theme",
            ),
        ];

        for (extension_id, contributes, family) in cases {
            let (lifecycle, store) = setup();
            let mut state = create_test_state(extension_id, ExtensionStatus::Installed);
            state.manifest.contributes = contributes;
            store.register(state).unwrap();

            let result = lifecycle.enable(extension_id);

            assert!(result.is_err(), "{family} should fail closed");
            let message = result.unwrap_err().to_string();
            assert!(message.contains("unsupported runtime contributes"), "{message}");
            assert!(message.contains(family), "{message}");
            assert_eq!(store.get(extension_id).unwrap().status, ExtensionStatus::Error);
        }
    }

    #[test]
    fn test_event_subscriptions_declared_accept_enable_ledger_stays_empty() {
        let (lifecycle, store) = setup();
        let mut state =
            create_test_state("extension-event-subscription", ExtensionStatus::Installed);
        state.manifest.contributes.event_subscriptions =
            Some(vec![EventSubscriptionRegistration {
                id: "session-completed".into(),
                topic: "session.completed".into(),
                scope_key: Some("session:active".into()),
                handler: EventHandlerReference {
                    module: "./runtime/events".into(),
                    export: "onSessionCompleted".into(),
                },
            }]);
        store.register(state).unwrap();

        // 声明式 event_subscriptions 现在由白名单桥 listen() 承接（阶段 1），
        // enable 不再 fail-closed；订阅在扩展运行时经桥建立，ledger 保持空。
        let result = lifecycle.enable("extension-event-subscription");

        assert!(result.is_ok());
        assert!(lifecycle
            .subscription_ledger
            .lock()
            .unwrap()
            .records("extension-event-subscription")
            .is_empty());
        assert_eq!(
            store.get("extension-event-subscription").unwrap().status,
            ExtensionStatus::Enabled
        );
    }

    #[test]
    fn test_enable_fails_closed_when_mcp_host_missing() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state("extension-mcp-no-host", ExtensionStatus::Installed);
        state.manifest.contributes.mcp_servers =
            Some(vec![crate::extension::models::MCPServerConfig {
                name: "browser".into(),
                config: serde_json::json!({
                    "transport": "stdio",
                    "command": "node",
                    "auto_start": false
                }),
            }]);
        store.register(state).unwrap();

        let result = lifecycle.enable("extension-mcp-no-host");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("MCP contributes"));
        assert_eq!(
            store.get("extension-mcp-no-host").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_fails_closed_when_lsp_host_missing() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state("extension-lsp-no-host", ExtensionStatus::Installed);
        state.manifest.contributes.languages = Some(vec![LanguageRegistration {
            language_id: "gleam".into(),
            display_name: "Gleam".into(),
            extensions: vec![".gleam".into()],
            server_command: "gleam".into(),
            server_args: Some(vec!["lsp".into()]),
            initialization_options: None,
        }]);
        store.register(state).unwrap();

        let result = lifecycle.enable("extension-lsp-no-host");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("LSP languages"));
        assert_eq!(
            store.get("extension-lsp-no-host").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_and_disable_registers_editor_languages_via_lsp_port() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));

        #[derive(Default)]
        struct RecordingLsp {
            registered: Mutex<Vec<LSPServerConfig>>,
            unregistered: Mutex<Vec<String>>,
        }
        impl LspCapabilityPort for RecordingLsp {
            fn register_language(
                &self,
                config: LSPServerConfig,
                _source: LanguageSource,
            ) -> anyhow::Result<()> {
                self.registered.lock().unwrap().push(config);
                Ok(())
            }

            fn unregister_language(&self, language_id: &str, _owner: &str) -> anyhow::Result<()> {
                self.unregistered.lock().unwrap().push(language_id.to_string());
                Ok(())
            }
        }
        let lsp = Arc::new(RecordingLsp::default());

        let mut state = create_test_state("extension-editor-language", ExtensionStatus::Installed);
        state.manifest.contributes.editor_languages = Some(vec![EditorLanguageRegistration {
            id: "mylang".into(),
            name: "My Lang".into(),
            extensions: vec![".my".into()],
            module: "./syntax/mylang.js".into(),
        }]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_lsp(lsp.clone());
        lifecycle.enable("extension-editor-language").unwrap();

        let registered = lsp.registered.lock().unwrap();
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0].language_id, "mylang");
        assert_eq!(registered[0].language_names, vec!["My Lang"]);
        assert_eq!(registered[0].file_extensions, vec![".my"]);
        drop(registered);

        lifecycle.disable("extension-editor-language").unwrap();
        assert_eq!(
            lsp.unregistered.lock().unwrap().as_slice(),
            &["mylang".to_string()]
        );
    }

    #[test]
    fn test_enable_fails_closed_when_editor_language_host_missing() {
        let (lifecycle, store) = setup();
        let mut state = create_test_state(
            "extension-editor-language-no-host",
            ExtensionStatus::Installed,
        );
        state.manifest.contributes.editor_languages = Some(vec![EditorLanguageRegistration {
            id: "mylang".into(),
            name: "My Lang".into(),
            extensions: vec![".my".into()],
            module: "./syntax/mylang.js".into(),
        }]);
        store.register(state).unwrap();

        let result = lifecycle.enable("extension-editor-language-no-host");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("editor languages"));
        assert!(message.contains("LSP host is not available"));
        assert_eq!(
            store
                .get("extension-editor-language-no-host")
                .unwrap()
                .status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_and_disable_registers_lsp_language() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lsp = Arc::new(crate::extension::types::LspManager::new(Arc::clone(&event_bus)).unwrap());

        let mut state = create_test_state("extension-lsp-language", ExtensionStatus::Installed);
        state.manifest.contributes.languages = Some(vec![LanguageRegistration {
            language_id: "gleam".into(),
            display_name: "Gleam".into(),
            extensions: vec![".gleam".into()],
            server_command: "gleam".into(),
            server_args: Some(vec!["lsp".into()]),
            initialization_options: Some(serde_json::json!({ "compileTarget": "erlang" })),
        }]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_lsp(lsp.clone());
        lifecycle.enable("extension-lsp-language").unwrap();

        let language = lsp.registry().get_config("gleam").unwrap();
        assert_eq!(language.language_names, vec!["Gleam"]);
        assert_eq!(language.file_extensions, vec![".gleam"]);
        assert_eq!(language.server_command, "gleam");
        assert_eq!(language.server_args, vec!["lsp"]);
        assert_eq!(
            language.initialization_options,
            Some(serde_json::json!({ "compileTarget": "erlang" }))
        );
        assert_eq!(
            lsp.registry().get_source("gleam"),
            Some(crate::extension::types::LanguageSource::Extension {
                owner: "extension-lsp-language".to_string()
            })
        );

        lifecycle.disable("extension-lsp-language").unwrap();
        assert!(lsp.registry().get_config("gleam").is_none());
        assert_eq!(lsp.registry().get_source("gleam"), None);
    }

    #[test]
    fn test_enable_fails_closed_when_lsp_language_overrides_builtin() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lsp = Arc::new(crate::extension::types::LspManager::new(Arc::clone(&event_bus)).unwrap());

        let mut state = create_test_state("extension-lsp-builtin", ExtensionStatus::Installed);
        state.manifest.contributes.languages = Some(vec![LanguageRegistration {
            language_id: "rust".into(),
            display_name: "Rust Override".into(),
            extensions: vec![".rs".into()],
            server_command: "custom-rust-analyzer".into(),
            server_args: None,
            initialization_options: None,
        }]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_lsp(lsp.clone());
        let result = lifecycle.enable("extension-lsp-builtin");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("builtin language"));
        assert_eq!(
            store.get("extension-lsp-builtin").unwrap().status,
            ExtensionStatus::Error
        );
        assert_eq!(
            lsp.registry().get_source("rust"),
            Some(crate::extension::types::LanguageSource::Builtin)
        );
    }

    #[test]
    fn test_enable_rolls_back_lsp_languages_when_later_registration_fails() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lsp = Arc::new(crate::extension::types::LspManager::new(Arc::clone(&event_bus)).unwrap());

        let mut state = create_test_state("extension-lsp-partial", ExtensionStatus::Installed);
        state.manifest.contributes.languages = Some(vec![
            LanguageRegistration {
                language_id: "gleam".into(),
                display_name: "Gleam".into(),
                extensions: vec![".gleam".into()],
                server_command: "gleam".into(),
                server_args: Some(vec!["lsp".into()]),
                initialization_options: None,
            },
            LanguageRegistration {
                language_id: "rust".into(),
                display_name: "Rust Override".into(),
                extensions: vec![".rs".into()],
                server_command: "custom-rust-analyzer".into(),
                server_args: None,
                initialization_options: None,
            },
        ]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_lsp(lsp.clone());
        let result = lifecycle.enable("extension-lsp-partial");

        assert!(result.is_err());
        assert!(lsp.registry().get_config("gleam").is_none());
        assert_eq!(
            lsp.registry().get_source("rust"),
            Some(crate::extension::types::LanguageSource::Builtin)
        );
        assert_eq!(
            store.get("extension-lsp-partial").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_rolls_back_mcp_server_when_later_skill_registration_fails() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let mcp = Arc::new(MCP::init_for_test().unwrap());

        let mut state = create_test_state("extension-partial-mcp", ExtensionStatus::Installed);
        state.manifest.contributes.mcp_servers =
            Some(vec![crate::extension::models::MCPServerConfig {
                name: "browser".into(),
                config: serde_json::json!({
                    "transport": "stdio",
                    "command": "node",
                    "auto_start": false
                }),
            }]);
        state.manifest.contributes.skills = Some(vec![SkillDefinition {
            id: "broken".into(),
            name: "Broken".into(),
            description: None,
            config: serde_json::json!({
                "content": ""
            }),
        }]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_mcp(mcp.clone());
        let result = lifecycle.enable("extension-partial-mcp");

        assert!(result.is_err());
        assert!(!mcp
            .list_servers()
            .iter()
            .any(|server| server.id == "extension:extension-partial-mcp/browser"));
        assert_eq!(
            store.get("extension-partial-mcp").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_rolls_back_skill_when_later_lsp_registration_fails() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lsp = Arc::new(crate::extension::types::LspManager::new(Arc::clone(&event_bus)).unwrap());

        let mut state = create_test_state("extension-partial-skill", ExtensionStatus::Installed);
        state.manifest.contributes.skills = Some(vec![SkillDefinition {
            id: "good".into(),
            name: "Good".into(),
            description: Some("Good skill".into()),
            config: serde_json::json!({
                "content": "Use this extension skill."
            }),
        }]);
        state.manifest.contributes.languages = Some(vec![LanguageRegistration {
            language_id: "rust".into(),
            display_name: "Rust Override".into(),
            extensions: vec![".rs".into()],
            server_command: "custom-rust-analyzer".into(),
            server_args: None,
            initialization_options: None,
        }]);
        store.register(state).unwrap();

        let lifecycle = ExtensionLifecycle::new(Arc::clone(&store), Arc::clone(&skills), event_bus)
            .with_lsp(lsp.clone());
        let result = lifecycle.enable("extension-partial-skill");

        assert!(result.is_err());
        assert!(skills
            .lock()
            .unwrap()
            .store()
            .get("extension:extension-partial-skill/good")
            .is_none());
        assert_eq!(
            lsp.registry().get_source("rust"),
            Some(crate::extension::types::LanguageSource::Builtin)
        );
        assert_eq!(
            store.get("extension-partial-skill").unwrap().status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_applies_mcp_tool_overrides() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let mcp = Arc::new(MCP::init_for_test().unwrap());

        let manifest = ExtensionManifest {
            id: "extension-mcp-override".into(),
            name: "Extension MCP Override".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                mcp_tool_overrides: Some(vec![McpToolOverride {
                    server: "builtin".into(),
                    tool: "fs.read_file".into(),
                    model_name: Some("read_doc".into()),
                    user_visible: Some(true),
                    display_name: Some("Read documentation".into()),
                    description: Some("Read a documentation file".into()),
                    renderer: Some("docs.read".into()),
                    detail_view: Some("markdown".into()),
                    declared_risk: Some("read".into()),
                }]),
                ..Default::default()
            },
        };

        store
            .register(ExtensionState {
                id: "extension-mcp-override".into(),
                status: ExtensionStatus::Installed,
                manifest,
                install_path: PathBuf::from("/extensions/extension-mcp-override"),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_mcp(mcp.clone());
        lifecycle.enable("extension-mcp-override").unwrap();

        let tool = mcp.get_tool("fs.read_file").unwrap();
        assert_eq!(tool.model_name.as_deref(), Some("read_doc"));
        assert!(tool.user_visible);
        assert_eq!(tool.ui_hint.unwrap().title, "Read documentation");
        assert_eq!(tool.description, "Read a documentation file");
        let renderer_hint = tool.renderer_hint.unwrap();
        assert_eq!(renderer_hint.renderer, "docs.read");
        assert_eq!(renderer_hint.detail_view.as_deref(), Some("markdown"));
    }

    #[test]
    fn test_enable_fails_closed_on_unmatched_mcp_tool_override() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let mcp = Arc::new(MCP::init_for_test().unwrap());

        let mut state = create_test_state(
            "extension-mcp-unmatched-override",
            ExtensionStatus::Installed,
        );
        state.manifest.contributes.mcp_tool_overrides = Some(vec![McpToolOverride {
            server: "builtin".into(),
            tool: "missing.tool".into(),
            model_name: Some("missing_tool".into()),
            user_visible: Some(true),
            display_name: None,
            description: None,
            renderer: None,
            detail_view: None,
            declared_risk: None,
        }]);
        store.register(state).unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_mcp(mcp.clone());
        let result = lifecycle.enable("extension-mcp-unmatched-override");

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("did not match registered tool"));
        assert_eq!(
            store
                .get("extension-mcp-unmatched-override")
                .unwrap()
                .status,
            ExtensionStatus::Error
        );
    }

    #[test]
    fn test_enable_and_disable_registers_mcp_server() {
        let event_bus: Arc<dyn crate::kernel::EventBus> = Arc::new(
            crate::kernel::InMemoryEventBus::new(1000, test_runtime_handle()),
        );
        let store = Arc::new(ExtensionStore::new(Arc::clone(&event_bus)));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let handle = test_runtime_handle();
        let _guard = handle.enter();
        let mcp = Arc::new(MCP::init_for_test().unwrap());

        let manifest = ExtensionManifest {
            id: "extension-mcp-server".into(),
            name: "Extension MCP Server".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                mcp_servers: Some(vec![crate::extension::models::MCPServerConfig {
                    name: "browser".into(),
                    config: serde_json::json!({
                        "transport": "stdio",
                        "command": "node",
                        "auto_start": false
                    }),
                }]),
                ..Default::default()
            },
        };

        store
            .register(ExtensionState {
                id: "extension-mcp-server".into(),
                status: ExtensionStatus::Installed,
                manifest,
                install_path: PathBuf::from("/extensions/extension-mcp-server"),
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();

        let lifecycle =
            ExtensionLifecycle::new(Arc::clone(&store), skills, event_bus).with_mcp(mcp.clone());
        lifecycle.enable("extension-mcp-server").unwrap();
        assert!(mcp
            .list_servers()
            .iter()
            .any(|server| server.id == "extension:extension-mcp-server/browser"));

        lifecycle.disable("extension-mcp-server").unwrap();
        assert!(!mcp
            .list_servers()
            .iter()
            .any(|server| server.id == "extension:extension-mcp-server/browser"));
    }
}
