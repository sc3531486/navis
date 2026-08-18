//! 后端扩展插件的 Cordis 形态（设计 35 D1 / 37 §一 / 38 §2.1）。
//!
//! 每个后端扩展对应一个 Cordis fiber：`ExtensionCordisPlugin` 用 `plugin_sync`
//! 构造（name = `extension:{id}`，`Inject::none()`），config 为 `Arc<ExtensionManifest>`。
//!
//! 可选 capability port 绝不进入 Inject（白板空壳下可能缺省），apply 内经
//! `ctx.get` 惰性解析：未注册的服务返回 `None`，扩展在对应 family 上 fail-closed。
//! 副作用经 `ctx.effect()`（或返回的 `PluginOutput` disposer）登记，fiber
//! `dispose()` 时逆序撤销。

use std::sync::Arc;

use anyhow::anyhow;
use cordis::{plugin_sync, Context, CordisError, ErrorCode, Fiber, Inject, PluginOutput};

use super::super::context::HostExtensionContext;
use super::super::models::ExtensionManifest;
use super::state::{apply_extension_fiber, ApplyState};
use super::ExtensionLifecycle;

// ── Capability 服务名 ────────────────────────────────────────────────────────
//
// 与 `HostExtensionContext::register_capability_service` 的 name 契约一致。
// 可选注入：宿主在对应子系统存在时才注册，扩展 apply 内缺服务时保持 fail-closed。

/// 后端扩展生命周期 capability 服务名。
pub const SERVICE_LIFECYCLE: &str = "lifecycle";
/// MCP capability 服务名。
pub const SERVICE_MCP: &str = "mcp";
/// LSP capability 服务名。
pub const SERVICE_LSP: &str = "lsp";
/// Gateway capability 服务名。
pub const SERVICE_GATEWAY: &str = "gateway";
/// Extension-owned Provider validation capability 服务名。
pub const SERVICE_PROVIDER_VALIDATION: &str = "provider_validation";
/// Extension 事件订阅 capability 服务名。
pub const SERVICE_EVENT_SUBSCRIPTION: &str = "event_subscription";
/// 后端进程管理器 capability 服务名。
pub const SERVICE_BACKEND_MANAGER: &str = "backend_manager";
/// 后端进程管理 capability 服务名（0b 契约化后统一命名）。
pub const SERVICE_BACKEND_PROCESS: &str = "backend_process";

/// 后端进程管理能力缝（0b：tool::backend 反向依赖契约化）。
///
/// 只暴露 extension lifecycle 需要的两个操作：生命周期 autostart spawn 与
/// 按扩展终止全部进程。`BackendProcessManager` 实现此端口，宿主装配时注入；
/// 未注入时声明 backend_services 的扩展 fail-closed。
pub trait BackendProcessPort: Send + Sync + 'static {
    /// 生命周期 autostart spawn：enable 流程中扩展处于 Loading，跳过 Enabled 校验。
    fn spawn_for_lifecycle(
        &self,
        store: &crate::extension::store::ExtensionStore,
        extension_id: &str,
        service: &crate::extension::models::BackendServiceRegistration,
    ) -> Result<String, String>;

    /// 终止指定扩展的全部后端进程（禁用/卸载/rollback 时清理）。无失败回报。
    fn kill_all_for_extension(&self, extension_id: &str);
}
/// WASM 组件注册表 capability 服务名。
pub const SERVICE_COMPONENT_REGISTRY: &str = "component_registry";
/// 全局策略引擎 capability 服务名。
pub const SERVICE_POLICY_ENGINE: &str = "policy_engine";
/// 后端扩展 apply 回调：接收 Cordis 上下文与扩展 manifest。
pub type CordisExtensionApply = Arc<
    dyn Fn(&Context, &ExtensionManifest) -> anyhow::Result<PluginOutput> + Send + Sync + 'static,
>;

/// 后端扩展插件的 Cordis 形态。
///
/// 由 [`HostExtensionContext::install_extension`]（或 [`install`](Self::install)）
/// 启动为一个 fiber；一个扩展对应一个 fiber，`dispose()` 逆序撤销 apply 登记的副作用。
pub struct ExtensionCordisPlugin {
    manifest: Arc<ExtensionManifest>,
    apply: CordisExtensionApply,
}

impl ExtensionCordisPlugin {
    /// 构造扩展插件：持有 manifest 与 apply 回调。
    ///
    /// `apply` 负责在 `ctx` 内惰性解析可选 capability 端口并提交 family 注册
    /// （D1c 填充注册逻辑）；副作用经 `ctx.effect` 或返回的 `PluginOutput` disposer
    /// 登记到 fiber，fiber 撤销时逆序执行。
    pub fn new<F>(manifest: ExtensionManifest, apply: F) -> Self
    where
        F: Fn(&Context, &ExtensionManifest) -> anyhow::Result<PluginOutput> + Send + Sync + 'static,
    {
        Self {
            manifest: Arc::new(manifest),
            apply: Arc::new(apply),
        }
    }

    /// 返回扩展 manifest。
    pub fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    /// 在宿主上下文中启动本插件并返回其生命周期 fiber。
    ///
    /// 使用 `plugin_sync` 构造（`Inject::none()`，绝不注入可选 capability port），
    /// config 为 `Arc<ExtensionManifest>`；fiber 启动成功后登记进宿主的 fiber map，
    /// 使 `dispose_extension` / `dispose_all` 可撤销。
    pub fn install(&self, host: &HostExtensionContext) -> anyhow::Result<Fiber> {
        let extension_id = self.manifest.id.clone();
        let extension_label = extension_id.clone();
        let apply = self.apply.clone();
        let plugin = plugin_sync::<Arc<ExtensionManifest>, _>(
            format!("extension:{}", extension_id),
            Inject::none(),
            move |ctx, config: Arc<Arc<ExtensionManifest>>| {
                apply(&ctx, &**config).map_err(|error| {
                    CordisError::with_message(
                        ErrorCode::Plugin,
                        format!("extension `{extension_label}` failed to start: {error}"),
                    )
                })
            },
        );

        let fiber = host.context().plugin(plugin, self.manifest.clone());
        match fiber.wait() {
            Ok(_) => {
                host.track_fiber(extension_id, fiber.clone())?;
                Ok(fiber)
            }
            Err(error) => {
                // apply 失败：Cordis 已在 activate 错误路径逆序执行已登记的
                // disposer（残余事实保留在 runtime_handles 账本供 retry）。这里
                // 再 dispose fiber 移除注册表记录，避免 Failed fiber 滞留宿主。
                let _ = fiber.dispose();
                Err(anyhow!(
                    "extension `{extension_id}` failed to start: {error}"
                ))
            }
        }
    }
}

/// 默认 apply：惰性解析 `ExtensionLifecycle` 服务并提交真实注册。
///
/// 生产装配时经 `SERVICE_LIFECYCLE` 注入全局生命周期（宿主在组合根注册）；
/// 缺服务时保持 fail-closed。注册逻辑在 `state::apply_extension_fiber` 内完成，
/// 副作用经 `ctx.effect` disposer 登记，fiber dispose / apply 失败时逆序撤销。
pub fn default_apply(ctx: &Context, manifest: &ExtensionManifest) -> anyhow::Result<PluginOutput> {
    let lifecycle: Option<Arc<ExtensionLifecycle>> =
        super::super::context::resolve_capability(ctx, SERVICE_LIFECYCLE)?;
    let lifecycle = lifecycle.ok_or_else(|| {
        anyhow!(
            "extension lifecycle service is not available; cannot apply '{}'",
            manifest.id
        )
    })?;
    let state = ApplyState::from_lifecycle(&lifecycle);
    apply_extension_fiber(&state, ctx, manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{ExtensionContributes, ExtensionPermissions};
    use crate::extension::skills::Skills;
    use crate::foundation::config::Config;
    use crate::kernel::EventBus;
    use cordis::FiberState;
    use std::sync::{Arc, Mutex, OnceLock};
    use tokio::runtime::Runtime;

    fn test_runtime_handle() -> tokio::runtime::Handle {
        static RUNTIME: OnceLock<Runtime> = OnceLock::new();
        RUNTIME
            .get_or_init(|| Runtime::new().expect("test tokio runtime"))
            .handle()
            .clone()
    }

    fn test_manifest(id: &str) -> ExtensionManifest {
        ExtensionManifest {
            id: id.to_string(),
            name: format!("Extension {id}"),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes::default(),
        }
    }

    #[test]
    fn install_plugin_with_default_apply_starts_and_tracks_fiber() {
        let event_bus: Arc<dyn EventBus> = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        ));
        let store = Arc::new(crate::extension::store::ExtensionStore::new(Arc::clone(
            &event_bus,
        )));
        let config = Arc::new(Mutex::new(Config::new(Arc::clone(&event_bus))));
        let skills = Arc::new(Mutex::new(
            Skills::with_event_bus(config, Arc::clone(&event_bus)).unwrap(),
        ));
        let lifecycle = ExtensionLifecycle::new(store, skills, event_bus);

        let host = HostExtensionContext::new();
        host.register_capability_service::<ExtensionLifecycle>(
            SERVICE_LIFECYCLE,
            Arc::new(lifecycle),
        )
        .unwrap();

        let plugin = ExtensionCordisPlugin::new(test_manifest("extension-a"), default_apply);
        let fiber = plugin.install(&host).unwrap();
        assert_eq!(fiber.state(), FiberState::Active);
        assert_eq!(fiber.name(), "extension:extension-a");

        // 宿主已跟踪 fiber，可按扩展 ID 撤销。
        host.dispose_extension("extension-a").unwrap();
        assert_eq!(fiber.state(), FiberState::Disposed);
    }

    #[test]
    fn default_apply_fails_closed_without_lifecycle_service() {
        let host = HostExtensionContext::new();
        let plugin = ExtensionCordisPlugin::new(test_manifest("extension-b"), default_apply);
        let error = plugin.install(&host).unwrap_err().to_string();
        assert!(error.contains("lifecycle"), "unexpected error: {error}");
    }
}
