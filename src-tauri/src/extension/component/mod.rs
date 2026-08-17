//! WASM 组件运行时核心（design/37 §四~§六，阶段 C1/C2 + 35 D2 执行链路）。
//!
//! 容器自持四件事：
//! 1. 契约映射：组件 entry 由 loader 校验（ExtensionUI/ExtensionBackend 下 .wasm），
//!    host 接口按组件 `capabilities` 白名单授予（host.rs，fail-closed）；
//! 2. host function 门禁：复用 Sandbox / OperationRegistry，门禁在容器（host.rs）；
//! 3. 组件生命周期：`load`（校验 + 门禁 + wasmtime 解析）→ `activate`（实例化 +
//!    注入 host 接口 + 调用 guest lifecycle.init/activate，实例运行时持久化）→
//!    `handle_message`（`message.handle` 消息路由，复用同一 Store/Instance）→
//!    `dispose` / `dispose_all_for_extension`（guest lifecycle.deactivate + 回收）；
//! 4. 组合注册：`<extension_id, component_id>` → `Arc<ComponentInstance>` 登记表。
//!
//! 安全模型（37 §七）：wasmtime 内存/trap 隔离；host function 是唯一出站通道；
//! 未声明能力 = 不注入授权 = 调用即拒绝（fail-closed）；组件崩溃不波及容器。

pub mod bindings_ext;
pub mod bindings_host;
pub mod host;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::extension::models::{
    ComponentCapabilities, ComponentKind, ComponentRegistration, ExtensionStatus,
};
use crate::extension::operation_runtime::OperationRegistry;
use crate::extension::store::ExtensionStore;
use crate::security::sandbox::{OperationRequest, OperationType, Sandbox};

use self::host::{require_allowed, HostState};

/// 已加载的组件实例（登记于 ComponentRegistry，`<extension_id, component_id>` 唯一）。
///
/// `runtime` 持有激活后的 wasmtime Store/Instance，`handle_message` 复用同一
/// 实例路由消息（组件内部状态跨消息保留）；未激活为 `None`。
pub struct ComponentInstance {
    /// 组件 ID（扩展内唯一）
    pub component_id: String,
    /// .wasm 相对路径（ExtensionUI/ 或 ExtensionBackend/ 下）
    pub entry: String,
    /// 组件类型：logic（组件轨）| native（逃生舱）
    pub kind: ComponentKind,
    /// 能力声明白名单（host 接口授予依据）
    pub capabilities: ComponentCapabilities,
    /// 编译后的组件（wasmtime 组件模型解析校验通过才登记）
    pub component: Arc<wasmtime::component::Component>,
    /// 激活后的实例运行时（Store + Instance）；未激活 / 已回收为 None。
    /// 内层锁保证同一实例的并发消息串行化（wasmtime Store 不可重入）。
    /// 仅容器内部访问（消息路由 / 回收），不对外暴露 wasmtime 对象。
    runtime: Mutex<Option<ActiveComponent>>,
    /// 加载时间
    pub loaded_at: DateTime<Utc>,
}

/// 激活后的实例运行时：wasmtime Store（承载 HostState）+ 实例。
///
/// 生命周期：`activate` 建立并持久化 → `handle_message` 复用 → `dispose` /
/// `dispose_all_for_extension` 调用 guest lifecycle.deactivate 后释放。
struct ActiveComponent {
    store: wasmtime::Store<HostState>,
    instance: wasmtime::component::Instance,
}

/// WASM 组件注册表（容器壳）。
///
/// 生命周期（37 §6.1）：
/// - `load`：扩展 Enabled → 解析 entry → Sandbox CommandExecute 门禁 → 读取 .wasm
///   并经 wasmtime 组件模型解析校验 → 登记（declared → loaded）
/// - `activate`：实例化 + 按 capabilities 注入 host 接口 + 调用 guest
///   `navis:ext/lifecycle.activate`（instantiated → activated）
/// - `dispose` / `dispose_all_for_extension`：回收登记（deactivate 回调在 C1-3 接线）
pub struct ComponentRegistry {
    sandbox: Arc<Sandbox>,
    operation_registry: Arc<OperationRegistry>,
    extension_store: Arc<ExtensionStore>,
    engine: Arc<wasmtime::Engine>,
    /// `<extension_id, component_id>` → ComponentInstance（Arc 使消息路由 / 回收
    /// 在释放注册表锁后仍可持有实例，避免 guest 调用期间持有全表锁导致重入死锁）
    instances: Mutex<HashMap<(String, String), Arc<ComponentInstance>>>,
}

impl ComponentRegistry {
    /// 创建组件注册表。依赖 Sandbox（门禁）、OperationRegistry（受控操作）与
    /// ExtensionStore（扩展状态 / 安装路径），全部为容器共享单例。
    pub fn new(
        sandbox: Arc<Sandbox>,
        operation_registry: Arc<OperationRegistry>,
        extension_store: Arc<ExtensionStore>,
    ) -> Self {
        let engine = Arc::new(wasmtime::Engine::default());
        tracing::info!("Creating ComponentRegistry");
        Self {
            sandbox,
            operation_registry,
            extension_store,
            engine,
            instances: Mutex::new(HashMap::new()),
        }
    }

    /// 加载组件并登记（declared → loaded）。
    ///
    /// 流程：扩展 Enabled → 组件 kind 为 logic → 解析 entry（防越权路径）
    /// → Sandbox CommandExecute 门禁（target=组件文件路径，actor=extension:{id}，
    /// require_confirm / 拒绝一律 fail-closed，不弹确认）→ 读取 .wasm 并经
    /// wasmtime 组件模型解析校验（非法组件在此拒绝）→ 登记。
    ///
    /// 返回 component_id（= `ComponentRegistration.id`）。
    pub fn load(&self, extension_id: &str, component: &ComponentRegistration) -> Result<String, String> {
        // 1) 扩展必须已启用（fail-closed）
        let state = self
            .extension_store
            .get(extension_id)
            .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
        if state.status != ExtensionStatus::Enabled {
            return Err(format!("Extension '{extension_id}' is not enabled"));
        }
        // 组件轨目前只支持 logic；native 逃生舱走 backendServices，不走本注册表。
        if component.kind != ComponentKind::Logic {
            return Err(format!(
                "Component '{extension_id}:{}' kind 'native' is not supported by the component registry; use backendServices",
                component.id
            ));
        }

        // 2) 解析 entry（loader 已校验路径格式；此处双重确认文件存在）
        let entry_path = state.install_path.join(&component.entry);
        if !entry_path.is_file() {
            return Err(format!(
                "Component entry '{}' does not exist at '{}'",
                component.entry,
                entry_path.display()
            ));
        }

        // 3) 容器级门禁：Sandbox CommandExecute
        let op_request = OperationRequest::new(
            OperationType::CommandExecute,
            entry_path.display().to_string(),
            format!("extension:{extension_id}"),
        );
        let result = self
            .sandbox
            .check(&op_request)
            .map_err(|error| format!("Sandbox check failed: {error}"))?;
        require_allowed(&result, &format!("component.load:{extension_id}:{}", component.id))?;

        // 4) 读取并交给 wasmtime 组件模型解析校验
        let bytes = std::fs::read(&entry_path).map_err(|error| {
            format!(
                "Failed to read component '{}': {error}",
                entry_path.display()
            )
        })?;
        let component_obj = wasmtime::component::Component::new(self.engine.as_ref(), &bytes)
            .map_err(|error| format!("Invalid WASM component '{}': {error}", component.entry))?;

        // 5) 登记（declared → loaded；实例运行时在 activate 建立）
        let instance = Arc::new(ComponentInstance {
            component_id: component.id.clone(),
            entry: component.entry.clone(),
            kind: component.kind,
            capabilities: component.capabilities.clone(),
            component: Arc::new(component_obj),
            runtime: Mutex::new(None),
            loaded_at: Utc::now(),
        });
        let mut instances = self
            .instances
            .lock()
            .map_err(|_| "component registry mutex poisoned".to_string())?;
        let key = (extension_id.to_string(), component.id.clone());
        if instances.contains_key(&key) {
            return Err(format!(
                "Component '{extension_id}:{}' is already loaded",
                component.id
            ));
        }
        instances.insert(key, instance);
        tracing::info!(
            extension_id = %extension_id,
            component_id = %component.id,
            "Component loaded"
        );
        Ok(component.id.clone())
    }

    /// 实例化组件 + 按 capabilities 注入 host 接口 + 调用 guest
    /// `navis:ext/lifecycle.init`（enable 时授予 host-handle）与
    /// `activate`（instantiated → activated），并把实例运行时持久化供
    /// `handle_message` 复用。幂等：已激活的组件直接返回 Ok。
    ///
    /// 组件未导出 navis:ext 世界（lifecycle/message）时 fail-closed。
    pub fn activate(&self, extension_id: &str, component_id: &str) -> Result<(), String> {
        let key = (extension_id.to_string(), component_id.to_string());
        let instance = {
            let instances = self
                .instances
                .lock()
                .map_err(|_| "component registry mutex poisoned".to_string())?;
            Arc::clone(
                instances
                    .get(&key)
                    .ok_or_else(|| format!("Component '{extension_id}:{component_id}' is not loaded"))?,
            )
        };

        // 幂等：已激活则直接返回（避免重复实例化覆盖既有运行时）。
        if instance
            .runtime
            .lock()
            .map_err(|_| "component runtime mutex poisoned".to_string())?
            .is_some()
        {
            return Ok(());
        }

        // 按 capabilities 构造 host 状态；host 接口实现见 host.rs（fail-closed）
        let host_state = HostState::new(
            extension_id.to_string(),
            component_id.to_string(),
            instance.capabilities.clone(),
            Arc::clone(&self.sandbox),
            Arc::clone(&self.operation_registry),
            Arc::clone(&self.extension_store),
        );
        let mut store = wasmtime::Store::new(self.engine.as_ref(), host_state);
        let mut linker = wasmtime::component::Linker::<HostState>::new(self.engine.as_ref());
        // 显式指定为 HasSelf<HostState>：HostState 直接实现全部 host 接口 Host trait，
        // bindgen 生成的 add_to_linker 以 `D::Data<'a>`（= &'a mut HostState）过 Host 约束。
        bindings_host::Host_::add_to_linker::<HostState, wasmtime::component::HasSelf<HostState>>(
            &mut linker,
            |state| state,
        )
        .map_err(|error| format!("Failed to wire host interfaces: {error}"))?;
        let instance_obj = linker
            .instantiate(&mut store, instance.component.as_ref())
            .map_err(|error| {
                format!(
                    "Failed to instantiate component '{extension_id}:{component_id}': {error}"
                )
            })?;

        // 组件未导出 navis:ext 世界（lifecycle/message）时 fail-closed
        let ext = bindings_ext::Ext::new(&mut store, &instance_obj).map_err(|error| {
            format!(
                "Component '{extension_id}:{component_id}' does not implement navis:ext world: {error}"
            )
        })?;
        // enable 时序：guest lifecycle.init（授予 host-handle）→ activate。
        // 两者均返回双层 Result：外层为 wasmtime 调用错误，内层为 WIT result<_, string>。
        ext.navis_ext_lifecycle()
            .call_init(
                &mut store,
                bindings_ext::navis::ext::types::HostHandle {},
            )
            .map_err(|error| format!("Component '{extension_id}:{component_id}' init trap: {error}"))?
            .map_err(|error| {
                format!("Component '{extension_id}:{component_id}' init returned: {error}")
            })?;
        ext.navis_ext_lifecycle()
            .call_activate(&mut store)
            .map_err(|error| {
                format!(
                    "Component '{extension_id}:{component_id}' activate trap: {error}"
                )
            })?
            .map_err(|error| {
                format!(
                    "Component '{extension_id}:{component_id}' activate returned: {error}"
                )
            })?;

        // 实例运行时持久化：供 handle_message 复用（组件内部状态跨消息保留）
        *instance
            .runtime
            .lock()
            .map_err(|_| "component runtime mutex poisoned".to_string())? =
            Some(ActiveComponent {
                store,
                instance: instance_obj,
            });
        tracing::info!(
            extension_id = %extension_id,
            component_id = %component_id,
            "Component activated"
        );
        Ok(())
    }

    /// 消息路由：把宿主 / 其他组件消息派发到目标组件的 `navis:ext/message.handle`。
    ///
    /// 复用 `activate` 持久化的实例（同一 Store/Instance，组件内部状态跨消息保留）。
    /// fail-closed：组件未加载 / 未激活（未实例化）即拒绝；组件未导出 navis:ext
    /// 世界或 handle 返回 Err / trap 时向上返回错误。
    ///
    /// 该入口承接 design/35 D2 的「`message` 接口 ↔ `route.call`」闭环：宿主侧
    /// `ui/extension_router` 在双端授权通过后调用本方法把 payload 交给组件。
    pub fn handle_message(
        &self,
        extension_id: &str,
        component_id: &str,
        payload: String,
    ) -> Result<String, String> {
        let key = (extension_id.to_string(), component_id.to_string());
        let instance = {
            let instances = self
                .instances
                .lock()
                .map_err(|_| "component registry mutex poisoned".to_string())?;
            Arc::clone(
                instances
                    .get(&key)
                    .ok_or_else(|| format!("Component '{extension_id}:{component_id}' is not loaded"))?,
            )
        };
        let mut runtime = instance
            .runtime
            .lock()
            .map_err(|_| "component runtime mutex poisoned".to_string())?;
        let active = runtime.as_mut().ok_or_else(|| {
            format!("Component '{extension_id}:{component_id}' is not active")
        })?;
        let ext = bindings_ext::Ext::new(&mut active.store, &active.instance).map_err(|error| {
            format!(
                "Component '{extension_id}:{component_id}' does not implement navis:ext world: {error}"
            )
        })?;
        ext.navis_ext_message()
            .call_handle(&mut active.store, &payload)
            .map_err(|error| {
                format!(
                    "Component '{extension_id}:{component_id}' message.handle trap: {error}"
                )
            })?
            .map_err(|error| {
                format!(
                    "Component '{extension_id}:{component_id}' message.handle returned: {error}"
                )
            })
    }

    /// 回收组件登记（disable / 卸载时调用）：先调 guest `lifecycle.deactivate`
    /// （best effort，失败只记日志）再释放实例运行时。
    pub fn dispose(&self, extension_id: &str, component_id: &str) -> Result<(), String> {
        let key = (extension_id.to_string(), component_id.to_string());
        let instance = {
            let mut instances = self
                .instances
                .lock()
                .map_err(|_| "component registry mutex poisoned".to_string())?;
            instances
                .remove(&key)
                .ok_or_else(|| format!("Component '{extension_id}:{component_id}' is not loaded"))?
        };
        deactivate_and_release(&instance);
        tracing::info!(
            extension_id = %extension_id,
            component_id = %component_id,
            "Component disposed"
        );
        Ok(())
    }

    /// 回收指定扩展的全部组件（disable / 卸载时调用，幂等）。
    /// 逐个调用 guest lifecycle.deactivate（best effort）后释放实例运行时。
    pub fn dispose_all_for_extension(&self, extension_id: &str) {
        let removed: Vec<Arc<ComponentInstance>> = {
            let mut instances = match self.instances.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let removed = instances
                .iter()
                .filter(|(key, _)| key.0 == extension_id)
                .map(|(_, instance)| Arc::clone(instance))
                .collect();
            instances.retain(|(ext, _), _| ext != extension_id);
            removed
        };
        for instance in &removed {
            deactivate_and_release(instance);
        }
        tracing::info!(
            extension_id = %extension_id,
            removed = removed.len(),
            "Disposed all components for extension"
        );
    }

    /// 查询组件是否已激活（实例运行时存在）。
    pub fn is_active(&self, extension_id: &str, component_id: &str) -> bool {
        let key = (extension_id.to_string(), component_id.to_string());
        self.instances
            .lock()
            .ok()
            .is_some_and(|guard| {
                guard.get(&key).is_some_and(|instance| {
                    instance
                        .runtime
                        .lock()
                        .map(|runtime| runtime.is_some())
                        .unwrap_or(false)
                })
            })
    }

    /// 列出登记组件；`extension_id` 为 Some 时只列该扩展，None 时列全部。
    /// 返回 `(extension_id, component_id)` 列表（已排序）。
    pub fn list(&self, extension_id: Option<&str>) -> Vec<(String, String)> {
        let Ok(instances) = self.instances.lock() else {
            return Vec::new();
        };
        let mut result: Vec<(String, String)> = instances
            .keys()
            .filter(|(ext, _)| extension_id.map_or(true, |id| id == ext))
            .cloned()
            .collect();
        result.sort();
        result
    }
}

/// 调用 guest `lifecycle.deactivate`（best effort）并释放实例运行时。
///
/// 失败只记 warn，不阻断 disable/卸载的状态转换（对齐 BackendProcessManager
/// kill_all 的容错语义）；无论结果如何都回收实例运行时。
fn deactivate_and_release(instance: &ComponentInstance) {
    let mut runtime = match instance.runtime.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(active) = runtime.as_mut() {
        let result = match bindings_ext::Ext::new(&mut active.store, &active.instance) {
            Ok(ext) => match ext.navis_ext_lifecycle().call_deactivate(&mut active.store) {
                Ok(Ok(())) => Ok(()),
                Ok(Err(error)) => Err(format!("deactivate returned: {error}")),
                Err(error) => Err(format!("deactivate trap: {error}")),
            },
            Err(error) => Err(format!("failed to rebind navis:ext world: {error}")),
        };
        if let Err(error) = result {
            tracing::warn!(
                component_id = %instance.component_id,
                error = %error,
                "Component deactivate failed (best effort)"
            );
        }
    }
    *runtime = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{
        ExtensionContributes, ExtensionManifest, ExtensionPermissions, ExtensionState,
    };
    use crate::kernel::{EventBus, InMemoryEventBus};
    use crate::security::sandbox::permission::PermissionLevel;
    use crate::extension::operation_runtime::{OperationHandlerKind, OperationRegistration};
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    use super::bindings_host::navis::host::{context, event, log, network, operation, storage, types};

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

    fn register_test_extension(
        store: &Arc<ExtensionStore>,
        id: &str,
        install_path: PathBuf,
        status: ExtensionStatus,
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
                    contributes: ExtensionContributes::default(),
                },
                install_path,
                installed_at: chrono::Utc::now(),
                enabled_at: None,
                error: None,
            })
            .unwrap();
    }

    fn test_sandbox() -> Arc<Sandbox> {
        let event_bus: Arc<dyn EventBus> =
            Arc::new(InMemoryEventBus::new(1000, test_runtime_handle()));
        Arc::new(Sandbox::new(event_bus))
    }

    /// registry 测试专用沙箱：FullAuto 审批模式 + 放行 fixture 路径的命令规则，
    /// 使 `load` 的 CommandExecute 门禁通过（host 门禁 fail-closed 测试仍用默认 Suggest 沙箱）。
    fn test_registry(store: Arc<ExtensionStore>) -> ComponentRegistry {
        let sandbox = test_sandbox();
        sandbox
            .set_approval_mode(crate::security::sandbox::ApprovalMode::FullAuto)
            .unwrap();
        sandbox
            .set_command_rules(vec![crate::security::sandbox::CommandRule {
                pattern: "component_add\\.wasm".into(),
                action: crate::security::sandbox::RuleAction::Allow,
                description: "test allow fixture component".into(),
            }])
            .unwrap();
        ComponentRegistry::new(sandbox, Arc::new(OperationRegistry::default()), store)
    }

    /// 真实 WASM 组件 fixture（tests/fixtures/component_add.wasm）：
    /// no_std guest 经 rustc 编译 + `wasm-tools component new` 包装，无 import、
    /// 导出 `add(i32, i32) -> i32`。仅用于证明 wasmtime 组件模型在 Windows 可加载并调用；
    /// 若本环境工具链不可用则该文件不存在，相关测试跳过。
    fn component_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/component_add.wasm")
    }

    /// 扩展安装目录（指向 fixture 所在目录，使 entry "component_add.wasm" 可解析）。
    fn fixture_install_path() -> PathBuf {
        component_fixture_path().parent().unwrap().to_path_buf()
    }

    /// 通用 logic 组件注册（entry 指向 fixture）。
    fn logic_component(id: &str) -> ComponentRegistration {
        ComponentRegistration {
            id: id.into(),
            entry: "component_add.wasm".into(),
            kind: ComponentKind::Logic,
            run_on: vec![],
            capabilities: Default::default(),
            autostart: false,
        }
    }

    // ---- wasmtime 组件模型硬门：加载并调用真实组件 ----

    #[test]
    fn wasmtime_loads_and_calls_real_component() {
        let path = component_fixture_path();
        if !path.is_file() {
            eprintln!(
                "SKIP: wasm32 guest 工具链不可用，未生成 fixture {}；跳过真实组件加载调用测试",
                path.display()
            );
            return;
        }
        let bytes = std::fs::read(&path).unwrap();
        let engine = wasmtime::Engine::default();
        let component = wasmtime::component::Component::new(&engine, &bytes).unwrap();

        let mut store = wasmtime::Store::new(&engine, ());
        let linker = wasmtime::component::Linker::<()>::new(&engine);
        let instance = linker.instantiate(&mut store, &component).unwrap();

        // 组件导出 `add(i32, i32) -> i32`，验证组件模型在 Windows 端到端可用。
        let add = instance
            .get_typed_func::<(i32, i32), (i32,)>(&mut store, "add")
            .unwrap();
        let result = add.call(&mut store, (2, 3)).unwrap();
        assert_eq!(result, (5,));
    }

    // ---- Registry 登记 / 门禁 / dispose ----

    #[test]
    fn registry_load_validates_gates_and_dispose() {
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        let registry = test_registry(store.clone());

        // 未启用扩展 → 拒绝
        let disabled_store = test_store();
        register_test_extension(
            &disabled_store,
            "ext.disabled",
            fixture_install_path(),
            ExtensionStatus::Disabled,
        );
        let disabled_registry = test_registry(disabled_store);
        assert!(disabled_registry
            .load("ext.disabled", &logic_component("app"))
            .unwrap_err()
            .contains("not enabled"));

        // entry 文件不存在 → 拒绝
        let missing = ComponentRegistration {
            id: "nope".into(),
            entry: "does_not_exist.wasm".into(),
            ..logic_component("app")
        };
        assert!(registry
            .load("ext.alpha", &missing)
            .unwrap_err()
            .contains("does not exist"));

        // native kind → 拒绝（逃生舱走 backendServices）
        let native = ComponentRegistration {
            kind: ComponentKind::Native,
            ..logic_component("native-server")
        };
        assert!(registry
            .load("ext.alpha", &native)
            .unwrap_err()
            .contains("not supported by the component registry"));

        // 默认 Suggest 沙箱（CommandExecute 需确认）→ load fail-closed（不弹确认）
        if component_fixture_path().is_file() {
            let suggest_sandbox = test_sandbox();
            let suggest_registry =
                ComponentRegistry::new(suggest_sandbox, Arc::new(OperationRegistry::default()), store.clone());
            assert!(suggest_registry
                .load("ext.alpha", &logic_component("app"))
                .unwrap_err()
                .contains("denied by sandbox"));
        }

        // 合法加载 + 查询 + dispose（fixture 存在时）
        if component_fixture_path().is_file() {
            let component_id = registry.load("ext.alpha", &logic_component("app")).unwrap();
            assert_eq!(component_id, "app");
            assert_eq!(
                registry.list(None),
                vec![("ext.alpha".to_string(), "app".to_string())]
            );
            assert_eq!(registry.list(Some("ext.alpha")).len(), 1);
            assert!(!registry.is_active("ext.alpha", "app"));

            // 重复加载拒绝
            assert!(registry
                .load("ext.alpha", &logic_component("app"))
                .unwrap_err()
                .contains("already loaded"));

            // dispose 后可回收；重复 dispose 拒绝
            registry.dispose("ext.alpha", "app").unwrap();
            assert!(registry.list(None).is_empty());
            assert!(!registry.is_active("ext.alpha", "app"));
            assert!(registry.dispose("ext.alpha", "app").is_err());
        }
    }

    #[test]
    fn registry_dispose_all_for_extension_is_scoped() {
        if !component_fixture_path().is_file() {
            eprintln!("SKIP: fixture 不存在，跳过 dispose_all 测试");
            return;
        }
        let store = test_store();
        register_test_extension(&store, "ext.one", fixture_install_path(), ExtensionStatus::Enabled);
        register_test_extension(&store, "ext.two", fixture_install_path(), ExtensionStatus::Enabled);
        let registry = test_registry(store);

        registry.load("ext.one", &logic_component("a")).unwrap();
        registry.load("ext.one", &logic_component("b")).unwrap();
        registry.load("ext.two", &logic_component("a")).unwrap();
        assert_eq!(registry.list(None).len(), 3);

        registry.dispose_all_for_extension("ext.one");
        assert_eq!(
            registry.list(None),
            vec![("ext.two".to_string(), "a".to_string())]
        );

        // 幂等
        registry.dispose_all_for_extension("ext.one");
        assert_eq!(registry.list(None).len(), 1);
    }

    // ---- host 门禁 fail-closed（纯 Rust，不依赖 guest）----

    fn test_host_state(capabilities: ComponentCapabilities) -> HostState {
        HostState::new(
            "ext.alpha".into(),
            "app".into(),
            capabilities,
            test_sandbox(),
            Arc::new(OperationRegistry::default()),
            test_store(),
        )
    }

    fn register_op(registry: &OperationRegistry, extension_id: &str, op: &str, op_type: OperationType) {
        registry
            .register(OperationRegistration {
                id: format!("{extension_id}.{op}"),
                extension_id: extension_id.to_string(),
                label: format!("Op {op}"),
                operation_type: op_type,
                permission_level: PermissionLevel::LightCheck,
                params_schema: None,
                handler_kind: OperationHandlerKind::Extension,
            })
            .unwrap();
    }

    #[test]
    fn host_operation_requires_registered_operation() {
        // 扩展未注册 → fail-closed
        let mut state = test_host_state(ComponentCapabilities {
            invoke: vec!["operation.execute".into()],
            ..Default::default()
        });
        let op = types::OperationRequest {
            operation: "ext.alpha.query".into(),
            target: "SELECT 1".into(),
            params: None,
        };
        assert!(operation::Host::execute(&mut state, op)
            .unwrap_err()
            .contains("not installed"));

        // 扩展已启用但操作未注册 → fail-closed
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            PathBuf::from("/extensions/ext.alpha"),
            ExtensionStatus::Enabled,
        );
        let mut state = HostState::new(
            "ext.alpha".into(),
            "app".into(),
            ComponentCapabilities {
                invoke: vec!["operation.execute".into()],
                ..Default::default()
            },
            test_sandbox(),
            Arc::new(OperationRegistry::default()),
            store,
        );
        let op = types::OperationRequest {
            operation: "ext.alpha.query".into(),
            target: "SELECT 1".into(),
            params: None,
        };
        assert!(operation::Host::execute(&mut state, op)
            .unwrap_err()
            .contains("not registered"));
    }

    #[test]
    fn host_operation_denied_when_capability_not_granted() {
        // capabilities 未声明 operation.execute → fail-closed（即使操作已注册）
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            PathBuf::from("/extensions/ext.alpha"),
            ExtensionStatus::Enabled,
        );
        let registry = Arc::new(OperationRegistry::default());
        register_op(&registry, "ext.alpha", "query", OperationType::FileRead);
        let mut state = HostState::new(
            "ext.alpha".into(),
            "app".into(),
            ComponentCapabilities::default(), // 空白名单：任何 invoke 都不授权
            test_sandbox(),
            registry,
            store,
        );
        let op = types::OperationRequest {
            operation: "ext.alpha.query".into(),
            target: "x".into(),
            params: None,
        };
        assert!(operation::Host::execute(&mut state, op)
            .unwrap_err()
            .contains("not granted invoke capability"));
    }

    #[test]
    fn host_operation_denied_when_sandbox_requires_confirm() {
        // CommandExecute 在默认 Suggest 策略下需要确认 → fail-closed（不弹确认）
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            PathBuf::from("/extensions/ext.alpha"),
            ExtensionStatus::Enabled,
        );
        let registry = Arc::new(OperationRegistry::default());
        register_op(&registry, "ext.alpha", "run", OperationType::CommandExecute);
        let mut state = HostState::new(
            "ext.alpha".into(),
            "app".into(),
            ComponentCapabilities {
                invoke: vec!["operation.execute".into()],
                ..Default::default()
            },
            test_sandbox(),
            registry,
            store,
        );
        let op = types::OperationRequest {
            operation: "ext.alpha.run".into(),
            target: "ls".into(),
            params: None,
        };
        assert!(operation::Host::execute(&mut state, op)
            .unwrap_err()
            .contains("denied by sandbox"));
    }

    #[test]
    fn host_fail_closed_defaults_for_other_interfaces() {
        // context/storage/network/event：未授权即拒绝；已授权但未接线返回 not implemented
        let mut state = test_host_state(ComponentCapabilities::default());
        assert!(context::Host::get_session(&mut state).unwrap_err().contains("not granted"));
        assert!(storage::Host::get(&mut state, "k".into(), "global".into())
            .unwrap_err()
            .contains("not granted"));
        assert!(network::Host::fetch(
            &mut state,
            types::HttpRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                body: None,
            },
        )
        .unwrap_err()
        .contains("network capability not granted"));
        assert!(event::Host::subscribe(&mut state, "session.*".into())
            .unwrap_err()
            .contains("not granted"));
        assert!(log::Host::write(&mut state, types::LogLevel::Info, "hello".into()).is_ok());

        // 已授权但未接线 → not implemented
        let mut state2 = test_host_state(ComponentCapabilities {
            invoke: vec!["context.getSession".into()],
            storage: vec!["global".into()],
            network: Some(serde_json::json!({ "type": "allowlist", "hosts": [] })),
            events: vec!["session.*".into()],
        });
        assert!(context::Host::get_session(&mut state2)
            .unwrap_err()
            .contains("not implemented"));
        assert!(storage::Host::get(&mut state2, "k".into(), "global".into())
            .unwrap_err()
            .contains("not implemented"));
        assert!(network::Host::fetch(
            &mut state2,
            types::HttpRequest {
                method: "GET".into(),
                url: "https://example.com".into(),
                headers: vec![],
                body: None,
            },
        )
        .unwrap_err()
        .contains("not implemented"));
        assert!(event::Host::emit(&mut state2, "session.completed".into(), "{}".into())
            .unwrap_err()
            .contains("not implemented"));
    }

    #[test]
    fn host_event_pattern_matches_prefix_wildcard() {
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            PathBuf::from("/extensions/ext.alpha"),
            ExtensionStatus::Enabled,
        );
        let mut state = HostState::new(
            "ext.alpha".into(),
            "app".into(),
            ComponentCapabilities {
                events: vec!["session.*".into()],
                ..Default::default()
            },
            test_sandbox(),
            Arc::new(OperationRegistry::default()),
            store,
        );
        // 精确 + 通配均被授权（未接线 → not implemented，说明已过白名单）
        let error = event::Host::subscribe(&mut state, "session.completed".into()).unwrap_err();
        assert!(error.contains("not implemented"), "unexpected: {error}");
        // 白名单外 pattern 拒绝
        let error = event::Host::subscribe(&mut state, "project.opened".into()).unwrap_err();
        assert!(error.contains("not granted"), "unexpected: {error}");
    }

    // ---- navis:ext 世界真实组件端到端（design/37 C1-5 验证轨）----

    /// navis:ext 世界 guest 组件（tests/fixtures/navis_ext.wasm）：
    /// 实现 lifecycle.init/activate/deactivate + message.handle，activate 内调用
    /// host 的 log.write 与 operation.list-operations（host 接口在真实组件中的全链路）。
    /// 若工具链缺失未生成该 fixture，文件不存在则跳过。
    fn navis_ext_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/navis_ext.wasm")
    }

    /// 最小 tracing 捕获订阅者（测试内联，无需 tracing-subscriber 依赖）：
    /// 把当前线程产生的事件字段序列化为 "字段:值" 文本，供端到端断言 host log
    /// 通路确实承载了 guest 写入的标记。
    #[derive(Default)]
    struct CaptureSubscriber(Mutex<Vec<String>>);

    struct CaptureFields<'a>(&'a mut Vec<String>);

    impl<'a> tracing::field::Visit for CaptureFields<'a> {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0.push(format!("{}={:?}", field.name(), value));
        }
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            self.0.push(format!("{}={}", field.name(), value));
        }
        fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
            self.0.push(format!("{}={}", field.name(), value));
        }
        fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
            self.0.push(format!("{}={}", field.name(), value));
        }
        fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
            self.0.push(format!("{}={}", field.name(), value));
        }
        fn record_error(
            &mut self,
            field: &tracing::field::Field,
            value: &(dyn std::error::Error + 'static),
        ) {
            self.0.push(format!("{}=err:{value}", field.name()));
        }
    }

    impl tracing::Subscriber for CaptureSubscriber {
        fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut fields = Vec::new();
            event.record(&mut CaptureFields(&mut fields));
            self.0.lock().unwrap().push(fields.join(" "));
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    #[test]
    fn registry_load_activate_navis_ext_component_end_to_end() {
        let path = navis_ext_fixture_path();
        if !path.is_file() {
            eprintln!(
                "SKIP: navis_ext.wasm 不存在（生成命令见 fixtures/guest_navis_ext.rs 顶部注释），跳过真实组件端到端测试"
            );
            return;
        }
        let store = test_store();
        register_test_extension(
            &store,
            "ext.navis",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        // FullAuto 沙箱 + 放行 fixture 路径的命令规则（load 的 CommandExecute 门禁）。
        let sandbox = test_sandbox();
        sandbox
            .set_approval_mode(crate::security::sandbox::ApprovalMode::FullAuto)
            .unwrap();
        sandbox
            .set_command_rules(vec![crate::security::sandbox::CommandRule {
                pattern: "navis_ext\\.wasm".into(),
                action: crate::security::sandbox::RuleAction::Allow,
                description: "test allow navis_ext fixture".into(),
            }])
            .unwrap();
        // 注册一个属于 ext.navis 的操作 → guest activate 内 list_operations 可见。
        let op_registry = Arc::new(OperationRegistry::default());
        register_op(&op_registry, "ext.navis", "query", OperationType::FileRead);
        let registry = ComponentRegistry::new(sandbox, op_registry, store);

        let component = ComponentRegistration {
            id: "navis_ext".into(),
            entry: "navis_ext.wasm".into(),
            kind: ComponentKind::Logic,
            run_on: vec![],
            // guest activate 需调用 operation.list（host.rs list_operations 白名单）；
            // log.write 无能力门槛（host.rs 直接放行）。
            capabilities: ComponentCapabilities {
                invoke: vec!["operation.list".into()],
                ..Default::default()
            },
            autostart: false,
        };

        // load 成功路径：登记 + 组件解析校验通过
        let component_id = registry.load("ext.navis", &component).unwrap();
        assert_eq!(component_id, "navis_ext");
        assert_eq!(
            registry.list(Some("ext.navis")),
            vec![("ext.navis".to_string(), "navis_ext".to_string())]
        );
        assert!(!registry.is_active("ext.navis", "navis_ext"));

        // activate 成功路径：真实组件实例化 + guest activate 执行 + host 接口全链路。
        // guest 在 activate 内只有在 log.write 成功（Err 会向上传播）且
        // list_operations 返回本扩展注册操作时才返回 Ok，因此此处 Ok 即证明
        // host log/operation 通路可用；再以 tracing 捕获断言日志内容确实承载标记。
        let capture = Arc::new(CaptureSubscriber::default());
        tracing::subscriber::with_default(capture.clone(), || {
            registry.activate("ext.navis", "navis_ext").unwrap();
        });
        assert!(
            registry.is_active("ext.navis", "navis_ext"),
            "真实组件 activate 后应 active=true"
        );

        let events = capture.0.lock().unwrap().clone();
        assert!(
            events
                .iter()
                .any(|line| line.contains("navis_ext.activate:host-log-ok")),
            "host log.write 未捕获到 guest 标记：{events:?}"
        );
        assert!(
            events
                .iter()
                .any(|line| line.contains("operations=1") && line.contains("first=ext.navis.query")),
            "host operation.list 未承载已注册操作：{events:?}"
        );

        // dispose 后不再 active（禁用组件、卸载时回收路径）
        registry.dispose("ext.navis", "navis_ext").unwrap();
        assert!(!registry.is_active("ext.navis", "navis_ext"));
        assert!(registry.list(None).is_empty());
    }

    // ---- message 路由（35 D2：`message` 接口 ↔ `route.call` 的组件侧入口）----

    #[test]
    fn handle_message_fails_closed_for_unloaded_component() {
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        let registry = test_registry(store);

        let error = registry
            .handle_message("ext.alpha", "app", "ping".into())
            .unwrap_err();
        assert!(error.contains("not loaded"), "unexpected: {error}");
    }

    #[test]
    fn handle_message_fails_closed_for_inactive_component() {
        if !component_fixture_path().is_file() {
            eprintln!("SKIP: fixture 不存在，跳过未激活消息路由测试");
            return;
        }
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        let registry = test_registry(store);
        registry.load("ext.alpha", &logic_component("app")).unwrap();

        // 已加载未激活（未实例化）→ 消息 fail-closed
        let error = registry
            .handle_message("ext.alpha", "app", "ping".into())
            .unwrap_err();
        assert!(error.contains("not active"), "unexpected: {error}");

        // dispose 后再路由 → 未加载 fail-closed
        registry.dispose("ext.alpha", "app").unwrap();
        let error = registry
            .handle_message("ext.alpha", "app", "ping".into())
            .unwrap_err();
        assert!(error.contains("not loaded"), "unexpected: {error}");
    }

    #[test]
    fn handle_message_targets_the_requested_component() {
        if !component_fixture_path().is_file() {
            eprintln!("SKIP: fixture 不存在，跳过目标组件路由测试");
            return;
        }
        let store = test_store();
        register_test_extension(
            &store,
            "ext.alpha",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        let registry = test_registry(store);
        registry.load("ext.alpha", &logic_component("app")).unwrap();

        // 路由到不存在的组件 ID → fail-closed 且错误携带目标组件身份
        let error = registry
            .handle_message("ext.alpha", "other", "ping".into())
            .unwrap_err();
        assert!(error.contains("ext.alpha:other"), "unexpected: {error}");
    }

    #[test]
    fn handle_message_echoes_payload_end_to_end() {
        let path = navis_ext_fixture_path();
        if !path.is_file() {
            eprintln!(
                "SKIP: navis_ext.wasm 不存在（生成命令见 fixtures/guest_navis_ext.rs 顶部注释），跳过消息路由端到端测试"
            );
            return;
        }
        let store = test_store();
        register_test_extension(
            &store,
            "ext.navis",
            fixture_install_path(),
            ExtensionStatus::Enabled,
        );
        let sandbox = test_sandbox();
        sandbox
            .set_approval_mode(crate::security::sandbox::ApprovalMode::FullAuto)
            .unwrap();
        sandbox
            .set_command_rules(vec![crate::security::sandbox::CommandRule {
                pattern: "navis_ext\\.wasm".into(),
                action: crate::security::sandbox::RuleAction::Allow,
                description: "test allow navis_ext fixture".into(),
            }])
            .unwrap();
        let op_registry = Arc::new(OperationRegistry::default());
        register_op(&op_registry, "ext.navis", "query", OperationType::FileRead);
        let registry = ComponentRegistry::new(sandbox, op_registry, store);

        let component = ComponentRegistration {
            id: "navis_ext".into(),
            entry: "navis_ext.wasm".into(),
            kind: ComponentKind::Logic,
            run_on: vec!["message".into()],
            capabilities: ComponentCapabilities {
                invoke: vec!["operation.list".into()],
                ..Default::default()
            },
            autostart: false,
        };
        registry.load("ext.navis", &component).unwrap();
        registry.activate("ext.navis", "navis_ext").unwrap();
        assert!(registry.is_active("ext.navis", "navis_ext"));

        // guest message.handle 回显 payload：往返验证消息通路 + 实例持久化。
        // 多次消息复用同一 Store/Instance（第二次消息仍 active，状态保留）。
        let response = registry
            .handle_message("ext.navis", "navis_ext", "ping-42".into())
            .unwrap();
        assert_eq!(response, "ping-42");
        assert!(registry.is_active("ext.navis", "navis_ext"));
        let response = registry
            .handle_message("ext.navis", "navis_ext", r#"{"hello":1}"#.into())
            .unwrap();
        assert_eq!(response, r#"{"hello":1}"#);

        // dispose 触发 guest deactivate 并回收运行时 → 路由 fail-closed
        registry.dispose("ext.navis", "navis_ext").unwrap();
        assert!(
            registry
                .handle_message("ext.navis", "navis_ext", "ping".into())
                .is_err()
        );
        assert!(!registry.is_active("ext.navis", "navis_ext"));
    }
}
