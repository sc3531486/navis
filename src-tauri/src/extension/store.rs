//! Extension 状态索引
//!
//! 基于设计文档 §07 实现，管理扩展的注册、查询、状态维护。
//! 扩展状态通过 kernel InMemoryRegistry<ExtensionCapability> 管理生命周期，
//! hook 声明索引和冲突检测为独立存储（不属于能力注册）。
//!
//! 职责：
//! - 通过 kernel InMemoryRegistry 维护所有已安装扩展的注册/启用状态
//! - 提供查询接口（list / get / get_manifest）
//! - 检查 ID 冲突、触发器前缀冲突（独立存储，非能力注册）
//! - 通过 EventBus 发布扩展状态事件

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use chrono::Utc;

use crate::kernel::{
    EventEnvelope, InMemoryRegistry, KernelContext, KernelScope, LifecycleAction, LifecycleState,
    Registry,
};
use triomphe::Arc as SharedArc;

use super::models::{
    status_to_kernel_lifecycle, ExtensionCapability, ExtensionManifest, ExtensionState,
    ExtensionStatus, HookPhase, HookRegistration, WorkModeRegistration,
};

/// 已启用扩展贡献的 Custom 工作模式。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct RegisteredWorkMode {
    pub extension_id: String,
    pub extension_name: String,
    pub mode_id: String,
    pub runtime_id: String,
    pub mode: WorkModeRegistration,
}

/// 已启用扩展贡献的应用层 Hook 声明。
///
/// 当前只维护 contract/registration，不执行 hook 模块。真正执行必须由宿主
/// hook runner 作为 Kernel Pipeline stage 或宿主管线步骤运行。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredHook {
    pub extension_id: String,
    pub extension_name: String,
    pub hook_id: String,
    pub runtime_id: String,
    pub hook: HookRegistration,
}

/// Extension 存储。
///
/// 底层使用 `kernel::InMemoryRegistry<ExtensionCapability>` 管理包的注册和生命周期。
/// hook 索引和冲突检测是声明级查询，不包含运行时执行逻辑。
/// 运行时 hook 执行通过 kernel::Policy (Constraint) 和 kernel::Pipeline (Stage) 完成。
pub struct ExtensionStore {
    /// kernel 能力注册表（extension_id -> ExtensionCapability）
    capabilities: InMemoryRegistry<ExtensionCapability>,
    /// 已启用扩展贡献的 Hook 声明（runtime_id -> RegisteredHook）。
    /// 这里只是声明索引；执行路径由宿主管线负责。
    hooks: RwLock<HashMap<String, RegisteredHook>>,
    /// 事件总线
    event_bus: Arc<dyn crate::kernel::EventBus>,
}

impl ExtensionStore {
    /// 创建新的扩展状态索引
    pub fn new(event_bus: Arc<dyn crate::kernel::EventBus>) -> Self {
        tracing::info!("Creating new ExtensionStore");
        Self {
            capabilities: InMemoryRegistry::new(),
            hooks: RwLock::new(HashMap::new()),
            event_bus,
        }
    }

    /// 注册一个新扩展
    ///
    /// # Arguments
    /// * `state` - 扩展状态
    ///
    /// # Errors
    /// 如果扩展 ID 已存在则返回错误
    pub fn register(&self, state: ExtensionState) -> Result<(), ExtensionStoreError> {
        let extension_id = state.id.clone();
        let capability = ExtensionCapability::new(state.clone());

        self.capabilities
            .register_arc(Arc::new(capability))
            .map_err(|_| ExtensionStoreError::AlreadyRegistered(extension_id.clone()))?;

        // 如果扩展注册时已经是 Enabled 状态，需要将 kernel lifecycle 从 Registered 转换到 Enabled。
        // register_arc 总是创建 Registered 状态的 entry，需要手动 transition。
        if state.status == ExtensionStatus::Enabled {
            if let Err(e) = self
                .capabilities
                .lifecycle(&extension_id, LifecycleAction::Enable)
            {
                tracing::warn!(
                    extension_id = %extension_id,
                    error = %e,
                    "Failed to transition kernel lifecycle to Enabled on register"
                );
            }
        }

        tracing::info!(extension_id = %extension_id, "Extension registered");

        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "extension.installed",
            KernelContext::new("extension", KernelScope::global()),
            Some(SharedArc::new(
                serde_json::json!({ "extensionId": extension_id }),
            )),
        )) {
            tracing::warn!(
                event = %"extension.installed",
                error = %error,
                "Failed to emit extension store event"
            );
        }

        Ok(())
    }

    /// 注销扩展
    ///
    /// # Arguments
    /// * `extension_id` - 扩展 ID
    ///
    /// # Errors
    /// 如果扩展不存在则返回错误
    pub fn unregister(&self, extension_id: &str) -> Result<(), ExtensionStoreError> {
        if !self.capabilities.is_registered(extension_id) {
            tracing::warn!(extension_id = %extension_id, "Extension not found for unregister");
            return Err(ExtensionStoreError::NotFound(extension_id.to_string()));
        }

        // 先清理 hook 声明
        let removed_hooks = self.unregister_hooks(extension_id);

        self.capabilities
            .unregister(extension_id)
            .map_err(|_| ExtensionStoreError::NotFound(extension_id.to_string()))?;

        tracing::info!(extension_id = %extension_id, "Extension unregistered");
        if removed_hooks > 0 {
            tracing::debug!(
                extension_id = %extension_id,
                removed_hooks = removed_hooks,
                "Extension hook declaration index cleared"
            );
        }
        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "extension.uninstalled",
            KernelContext::new("extension", KernelScope::global()),
            Some(SharedArc::new(
                serde_json::json!({ "extensionId": extension_id }),
            )),
        )) {
            tracing::warn!(
                event = "extension.uninstalled",
                error = %error,
                "Failed to emit extension store event"
            );
        }

        Ok(())
    }

    /// 获取扩展状态（无论 kernel lifecycle 处于什么状态）
    pub fn get(&self, extension_id: &str) -> Option<ExtensionState> {
        self.capabilities
            .get_registered(extension_id)
            .and_then(|cap| cap.to_extension_state())
    }

    /// 获取扩展清单
    pub fn get_manifest(
        &self,
        extension_id: &str,
    ) -> Result<ExtensionManifest, ExtensionStoreError> {
        self.get(extension_id)
            .map(|s| s.manifest)
            .ok_or_else(|| ExtensionStoreError::NotFound(extension_id.to_string()))
    }

    /// 列出所有扩展
    pub fn list(&self) -> Vec<ExtensionState> {
        self.capabilities
            .list()
            .iter()
            .filter_map(|info| {
                // 从 CapabilityInfo.metadata 反序列化 ExtensionState
                let state: ExtensionState =
                    serde_json::from_value(info.metadata.as_ref().clone()).ok()?;
                Some(state)
            })
            .collect()
    }

    /// 列出指定状态的扩展
    pub fn list_by_status(&self, status: &ExtensionStatus) -> Vec<ExtensionState> {
        self.list()
            .into_iter()
            .filter(|s| s.status == *status)
            .collect()
    }

    /// 列出所有已启用扩展贡献的 Custom 工作模式。
    pub fn list_enabled_work_modes(&self) -> Vec<RegisteredWorkMode> {
        self.list()
            .into_iter()
            .filter(|state| state.status == ExtensionStatus::Enabled)
            .flat_map(|state| {
                let extension_id = state.id.clone();
                let extension_name = state.manifest.name.clone();
                state
                    .manifest
                    .contributes
                    .work_modes
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |mode| {
                        let mode_id = mode.id.clone();
                        RegisteredWorkMode {
                            extension_id: extension_id.clone(),
                            extension_name: extension_name.clone(),
                            runtime_id: format!("{}/{}", extension_id, mode_id),
                            mode_id,
                            mode,
                        }
                    })
            })
            .collect()
    }

    /// 注册已启用扩展贡献的 Hook 声明。
    ///
    /// Hook ID 在同一扩展内唯一，运行时 ID 为 `<extension_id>/<hook_id>`。
    /// 这里仅建立声明索引，不加载模块、不执行 hook。
    pub fn register_hooks(
        &self,
        extension_id: &str,
        hooks: &[HookRegistration],
    ) -> Result<(), ExtensionStoreError> {
        // 检查扩展是否已启用（通过 kernel registry 的 available 状态）
        if !self.capabilities.is_available(extension_id) {
            // 进一步确认是找不到还是未启用
            if self.capabilities.get_registered(extension_id).is_none() {
                return Err(ExtensionStoreError::NotFound(extension_id.to_string()));
            }
            return Err(ExtensionStoreError::ExtensionNotEnabled(
                extension_id.to_string(),
            ));
        }

        // 获取扩展名称（用于 RegisteredHook）
        let extension_name = self
            .get(extension_id)
            .map(|s| s.manifest.name)
            .unwrap_or_default();

        let mut seen = HashSet::new();
        for hook in hooks {
            let runtime_id = extension_hook_runtime_id(extension_id, &hook.id);
            if !seen.insert(runtime_id.clone()) {
                return Err(ExtensionStoreError::HookAlreadyRegistered(runtime_id));
            }
        }

        let mut registered = self.hooks.write().unwrap();
        for hook in hooks {
            let runtime_id = extension_hook_runtime_id(extension_id, &hook.id);
            if registered.contains_key(&runtime_id) {
                return Err(ExtensionStoreError::HookAlreadyRegistered(runtime_id));
            }
        }

        for hook in hooks {
            let runtime_id = extension_hook_runtime_id(extension_id, &hook.id);
            registered.insert(
                runtime_id.clone(),
                RegisteredHook {
                    extension_id: extension_id.to_string(),
                    extension_name: extension_name.clone(),
                    hook_id: hook.id.clone(),
                    runtime_id,
                    hook: hook.clone(),
                },
            );
        }

        Ok(())
    }

    /// 注销扩展贡献的全部 Hook 声明。
    pub fn unregister_hooks(&self, extension_id: &str) -> usize {
        let mut hooks = self.hooks.write().unwrap();
        let before = hooks.len();
        hooks.retain(|_, hook| hook.extension_id != extension_id);
        before.saturating_sub(hooks.len())
    }

    /// 列出全部已注册 Hook，按 phase、priority、runtime_id 稳定排序。
    pub fn list_hooks(&self) -> Vec<RegisteredHook> {
        let hooks = self.hooks.read().unwrap();
        let mut hooks = hooks.values().cloned().collect::<Vec<_>>();
        sort_hooks(&mut hooks);
        hooks
    }

    /// 列出指定阶段的已注册 Hook 声明，按 priority 升序稳定排序。
    ///
    /// 只查询 manifest 声明，不包含运行时执行逻辑。
    /// 运行时 hook 执行必须走 kernel::Policy (Constraint) / kernel::Pipeline (Stage)。
    pub fn list_hook_declarations_by_phase(&self, phase: HookPhase) -> Vec<RegisteredHook> {
        let hooks = self.hooks.read().unwrap();
        let mut hooks = hooks
            .values()
            .filter(|hook| hook.hook.phase == phase)
            .cloned()
            .collect::<Vec<_>>();
        sort_hooks(&mut hooks);
        hooks
    }

    /// 获取指定 runtime id 的 Custom 工作模式。
    pub fn get_work_mode(&self, runtime_id: &str) -> Option<RegisteredWorkMode> {
        self.list_enabled_work_modes()
            .into_iter()
            .find(|mode| mode.runtime_id == runtime_id)
    }

    /// 更新扩展状态
    ///
    /// 对于 kernel 管理的状态转换（Enabled/Disabled），同时调用 kernel lifecycle。
    /// 对于扩展特有状态（Loading/Disabling/Unloading/Error），仅更新元数据。
    pub fn update_status(
        &self,
        extension_id: &str,
        status: ExtensionStatus,
        error: Option<String>,
    ) -> Result<(), ExtensionStoreError> {
        let cap = self
            .capabilities
            .get_registered(extension_id)
            .ok_or_else(|| ExtensionStoreError::NotFound(extension_id.to_string()))?;

        let current_extension_status = cap
            .to_extension_state()
            .map(|s| s.status)
            .unwrap_or(ExtensionStatus::Installed);

        if !is_valid_transition(&current_extension_status, &status) {
            return Err(ExtensionStoreError::NotFound(format!(
                "invalid transition from {:?} to {:?} for extension '{}'",
                current_extension_status, status, extension_id
            )));
        }

        // 从 CapabilityInfo 获取当前 kernel lifecycle state（可靠来源）
        let old_kernel_state = self
            .capabilities
            .list()
            .iter()
            .find(|info| info.id == extension_id)
            .map(|info| info.state)
            .unwrap_or(LifecycleState::Registered);

        // Extension metadata 只更新 ExtensionStatus；Kernel lifecycle 保留在 RegistryEntry。
        let mut state = cap
            .to_extension_state()
            .unwrap_or_else(|| panic!("Extension '{}' metadata corrupted", extension_id));
        state.status = status.clone();
        state.error = error;
        state.enabled_at = match status {
            ExtensionStatus::Enabled => Some(Utc::now()),
            ExtensionStatus::Disabled | ExtensionStatus::Unloading => None,
            _ => state.enabled_at,
        };
        let new_cap = Arc::new(ExtensionCapability::new(state));
        self.capabilities
            .replace_arc(new_cap)
            .map_err(|e| ExtensionStoreError::NotFound(e.to_string()))?;

        // 对于 kernel 管理的状态转换，调用 kernel lifecycle。
        // 使用 CapabilityInfo.state（旧）和 status_to_kernel_lifecycle（新）来判断是否需要转换。
        match (old_kernel_state, status_to_kernel_lifecycle(&status)) {
            (LifecycleState::Registered, Some(LifecycleAction::Enable)) => {
                // Registered -> Enabled
                if let Err(e) = self
                    .capabilities
                    .lifecycle(extension_id, LifecycleAction::Enable)
                {
                    tracing::warn!(
                        extension_id = %extension_id,
                        error = %e,
                        "Kernel lifecycle transition to Enabled failed"
                    );
                }
            }
            (LifecycleState::Enabled, Some(LifecycleAction::Disable)) => {
                // Enabled -> Registered
                if let Err(e) = self
                    .capabilities
                    .lifecycle(extension_id, LifecycleAction::Disable)
                {
                    tracing::warn!(
                        extension_id = %extension_id,
                        error = %e,
                        "Kernel lifecycle transition to Registered failed"
                    );
                }
            }
            _ => {
                // 同一 kernel 状态内转换（如 Registered -> Registered），
                // 或不需要 kernel 管理的状态（如 Error），不需要调用 kernel lifecycle。
            }
        }

        // 离开 Enabled 状态时清理 hook 声明
        if old_kernel_state == LifecycleState::Enabled && status != ExtensionStatus::Enabled {
            let removed_hooks = self.unregister_hooks(extension_id);
            if removed_hooks > 0 {
                tracing::debug!(
                    extension_id = %extension_id,
                    removed_hooks = removed_hooks,
                    "Extension hook declaration index cleared after status change"
                );
            }
        }

        tracing::debug!(
            extension_id = %extension_id,
            old_status = %current_extension_status,
            new_status = %status,
            "Extension status updated"
        );

        Ok(())
    }

    /// 检查触发器前缀是否与已注册的扩展触发器冲突
    ///
    /// # Arguments
    /// * `prefix` - 触发器前缀（如 "/pr"）
    /// * `exclude_extension_id` - 排除的扩展 ID（用于更新场景）
    pub fn check_trigger_conflict(
        &self,
        prefix: &str,
        exclude_extension_id: Option<&str>,
    ) -> Result<(), ExtensionStoreError> {
        // 内置触发器前缀
        let builtin_prefixes = ["/", "/role"];
        if builtin_prefixes.contains(&prefix) {
            tracing::warn!(prefix = %prefix, "Trigger prefix conflicts with builtin");
            return Err(ExtensionStoreError::TriggerConflict(
                prefix.to_string(),
                "builtin".to_string(),
            ));
        }

        for state in self.list_all_enabled() {
            if Some(state.id.as_str()) == exclude_extension_id {
                continue;
            }
            if let Some(ref triggers) = state.manifest.contributes.triggers {
                for trigger in triggers {
                    if trigger.prefix == prefix {
                        tracing::warn!(
                            prefix = %prefix,
                            conflict_extension = %state.id,
                            "Trigger prefix conflict detected"
                        );
                        return Err(ExtensionStoreError::TriggerConflict(
                            prefix.to_string(),
                            state.id.clone(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查命令 ID 是否与已注册的扩展命令冲突
    pub fn check_command_conflict(
        &self,
        command_id: &str,
        exclude_extension_id: Option<&str>,
    ) -> Result<(), ExtensionStoreError> {
        for state in self.list_all_enabled() {
            if Some(state.id.as_str()) == exclude_extension_id {
                continue;
            }
            if let Some(ref commands) = state.manifest.contributes.commands {
                for cmd in commands {
                    if cmd.id == command_id {
                        tracing::warn!(
                            command_id = %command_id,
                            conflict_extension = %state.id,
                            "Command ID conflict detected"
                        );
                        return Err(ExtensionStoreError::CommandConflict(
                            command_id.to_string(),
                            state.id.clone(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// 检查视图 ID 是否与已注册的扩展视图冲突
    pub fn check_view_conflict(
        &self,
        view_id: &str,
        exclude_extension_id: Option<&str>,
    ) -> Result<(), ExtensionStoreError> {
        for state in self.list_all_enabled() {
            if Some(state.id.as_str()) == exclude_extension_id {
                continue;
            }
            if let Some(ref views) = state.manifest.contributes.views {
                for view in views {
                    if view.id == view_id {
                        tracing::warn!(
                            view_id = %view_id,
                            conflict_extension = %state.id,
                            "View ID conflict detected"
                        );
                        return Err(ExtensionStoreError::ViewConflict(
                            view_id.to_string(),
                            state.id.clone(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取已注册的扩展数量
    pub fn count(&self) -> usize {
        self.capabilities.stats().entry_count
    }

    /// 检查扩展是否存在
    pub fn contains(&self, extension_id: &str) -> bool {
        self.capabilities.is_registered(extension_id)
    }

    /// 列出所有已启用扩展的 ExtensionState（kernel Available 状态）。
    fn list_all_enabled(&self) -> Vec<ExtensionState> {
        self.capabilities
            .list()
            .iter()
            .filter(|info| {
                matches!(
                    info.state,
                    crate::kernel::LifecycleState::Enabled | crate::kernel::LifecycleState::Running
                )
            })
            .filter_map(|info| {
                let state: ExtensionState =
                    serde_json::from_value(info.metadata.as_ref().clone()).ok()?;
                Some(state)
            })
            .collect()
    }
}

/// 扩展安装状态与声明索引错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExtensionStoreError {
    #[error("Extension already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Extension is not enabled: {0}")]
    ExtensionNotEnabled(String),

    #[error("Trigger prefix '{0}' conflicts with extension '{1}'")]
    TriggerConflict(String, String),

    #[error("Command ID '{0}' conflicts with extension '{1}'")]
    CommandConflict(String, String),

    #[error("View ID '{0}' conflicts with extension '{1}'")]
    ViewConflict(String, String),

    #[error("Hook already registered: {0}")]
    HookAlreadyRegistered(String),
}

/// 合法的 ExtensionStatus 状态转换
const VALID_TRANSITIONS: &[(ExtensionStatus, ExtensionStatus)] = &[
    (ExtensionStatus::Installed, ExtensionStatus::Loading),
    (ExtensionStatus::Installed, ExtensionStatus::Enabled),
    (ExtensionStatus::Disabled, ExtensionStatus::Loading),
    (ExtensionStatus::Disabled, ExtensionStatus::Enabled),
    (ExtensionStatus::Loading, ExtensionStatus::Enabled),
    (ExtensionStatus::Loading, ExtensionStatus::Error),
    (ExtensionStatus::Enabled, ExtensionStatus::Disabling),
    (ExtensionStatus::Enabled, ExtensionStatus::Disabled),
    (ExtensionStatus::Enabled, ExtensionStatus::Error),
    (ExtensionStatus::Disabling, ExtensionStatus::Disabled),
    (ExtensionStatus::Disabling, ExtensionStatus::Error),
    (ExtensionStatus::Installed, ExtensionStatus::Unloading),
    (ExtensionStatus::Disabled, ExtensionStatus::Unloading),
    (ExtensionStatus::Error, ExtensionStatus::Unloading),
    (ExtensionStatus::Error, ExtensionStatus::Loading),
    (ExtensionStatus::Error, ExtensionStatus::Enabled),
    (ExtensionStatus::Error, ExtensionStatus::Installed),
    (ExtensionStatus::Installed, ExtensionStatus::Error),
];

fn is_valid_transition(from: &ExtensionStatus, to: &ExtensionStatus) -> bool {
    VALID_TRANSITIONS.iter().any(|(f, t)| f == from && t == to)
}

fn extension_hook_runtime_id(extension_id: &str, hook_id: &str) -> String {
    format!("{}/{}", extension_id, hook_id)
}

fn sort_hooks(hooks: &mut [RegisteredHook]) {
    hooks.sort_by(|left, right| {
        format!("{:?}", left.hook.phase)
            .cmp(&format!("{:?}", right.hook.phase))
            .then_with(|| {
                left.hook
                    .priority
                    .unwrap_or(100)
                    .cmp(&right.hook.priority.unwrap_or(100))
            })
            .then_with(|| left.runtime_id.cmp(&right.runtime_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{
        BuiltinAction, CommandRegistration, ExtensionContributes, ExtensionManifest,
        ExtensionPermissions, HookRegistration, ViewRegistration,
    };
    use crate::extension::TriggerRegistration;
    use chrono::Utc;
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

    fn create_test_manifest(id: &str) -> ExtensionManifest {
        ExtensionManifest {
            id: id.to_string(),
            name: format!("Extension {}", id),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes::default(),
        }
    }

    fn create_test_state(id: &str) -> ExtensionState {
        ExtensionState {
            id: id.to_string(),
            status: ExtensionStatus::Installed,
            manifest: create_test_manifest(id),
            install_path: PathBuf::from(format!("/extensions/{}", id)),
            installed_at: Utc::now(),
            enabled_at: None,
            error: None,
        }
    }

    #[test]
    fn test_register_and_get() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let state = create_test_state("extension-a");

        store.register(state.clone()).unwrap();
        let retrieved = store.get("extension-a").unwrap();
        assert_eq!(retrieved.id, "extension-a");
        assert_eq!(retrieved.status, ExtensionStatus::Installed);
    }

    #[test]
    fn test_register_duplicate_fails() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();

        let result = store.register(create_test_state("extension-a"));
        assert!(matches!(
            result,
            Err(ExtensionStoreError::AlreadyRegistered(_))
        ));
    }

    #[test]
    fn test_unregister() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();
        assert_eq!(store.count(), 1);

        store.unregister("extension-a").unwrap();
        assert_eq!(store.count(), 0);
        assert!(store.get("extension-a").is_none());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let result = store.unregister("nonexistent");
        assert!(matches!(result, Err(ExtensionStoreError::NotFound(_))));
    }

    #[test]
    fn test_list() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();
        store.register(create_test_state("extension-b")).unwrap();

        let list = store.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_by_status() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        let mut state_a = create_test_state("extension-a");
        state_a.status = ExtensionStatus::Installed;
        store.register(state_a).unwrap();

        let mut state_b = create_test_state("extension-b");
        state_b.status = ExtensionStatus::Enabled;
        store.register(state_b).unwrap();

        let installed = store.list_by_status(&ExtensionStatus::Installed);
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "extension-a");

        let enabled = store.list_by_status(&ExtensionStatus::Enabled);
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "extension-b");
    }

    #[test]
    fn test_update_status() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();

        store
            .update_status("extension-a", ExtensionStatus::Enabled, None)
            .unwrap();

        let state = store.get("extension-a").unwrap();
        assert_eq!(state.status, ExtensionStatus::Enabled);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_update_status_with_error() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();

        store
            .update_status(
                "extension-a",
                ExtensionStatus::Error,
                Some("something failed".into()),
            )
            .unwrap();

        let state = store.get("extension-a").unwrap();
        assert_eq!(state.status, ExtensionStatus::Error);
        assert_eq!(state.error, Some("something failed".into()));
    }

    #[test]
    fn test_get_manifest() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();

        let manifest = store.get_manifest("extension-a").unwrap();
        assert_eq!(manifest.name, "Extension extension-a");

        let result = store.get_manifest("nonexistent");
        assert!(matches!(result, Err(ExtensionStoreError::NotFound(_))));
    }

    #[test]
    fn test_trigger_conflict_with_builtin() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        let result = store.check_trigger_conflict("/role", None);
        assert!(matches!(
            result,
            Err(ExtensionStoreError::TriggerConflict(_, _))
        ));

        let result = store.check_trigger_conflict("/mytrigger", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_trigger_conflict_between_extensions() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        // 注册一个带有触发器的扩展
        let manifest = ExtensionManifest {
            id: "extension-a".into(),
            name: "Extension A".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                triggers: Some(vec![TriggerRegistration {
                    prefix: "/pr".into(),
                    label: "Pull Request".into(),
                    description: "GitHub PR".into(),
                    icon: None,
                    placeholder: None,
                    search_module: "./search.js".into(),
                    select_module: "./select.js".into(),
                    scope: super::super::models::TriggerScope::Global,
                }]),
                ..Default::default()
            },
        };

        let mut state = create_test_state("extension-a");
        state.manifest = manifest;
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        // 同一前缀冲突
        let result = store.check_trigger_conflict("/pr", None);
        assert!(matches!(
            result,
            Err(ExtensionStoreError::TriggerConflict(_, _))
        ));

        // 排除自身不冲突
        let result = store.check_trigger_conflict("/pr", Some("extension-a"));
        assert!(result.is_ok());

        // 不同前缀不冲突
        let result = store.check_trigger_conflict("/issue", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_conflict() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        let manifest = ExtensionManifest {
            id: "extension-a".into(),
            name: "Extension A".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                commands: Some(vec![CommandRegistration {
                    id: "myCommand".into(),
                    label: "My Command".into(),
                    description: None,
                    icon: None,
                    category: None,
                    when: None,
                    action: BuiltinAction::OpenView {
                        view_id: "my.view".into(),
                    },
                }]),
                ..Default::default()
            },
        };

        let mut state = create_test_state("extension-a");
        state.manifest = manifest;
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        // 冲突
        let result = store.check_command_conflict("myCommand", None);
        assert!(matches!(
            result,
            Err(ExtensionStoreError::CommandConflict(_, _))
        ));

        // 排除自身
        let result = store.check_command_conflict("myCommand", Some("extension-a"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_view_conflict() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        let manifest = ExtensionManifest {
            id: "extension-a".into(),
            name: "Extension A".into(),
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
                ..Default::default()
            },
        };

        let mut state = create_test_state("extension-a");
        state.manifest = manifest;
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        // 冲突
        let result = store.check_view_conflict("test.view", None);
        assert!(matches!(
            result,
            Err(ExtensionStoreError::ViewConflict(_, _))
        ));

        // 不冲突
        let result = store.check_view_conflict("other.view", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_contains() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        assert!(!store.contains("extension-a"));

        store.register(create_test_state("extension-a")).unwrap();
        assert!(store.contains("extension-a"));
        assert!(!store.contains("extension-b"));
    }

    #[test]
    fn test_register_hooks_lists_by_phase_and_priority() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let mut state = create_test_state("extension-a");
        state.manifest.name = "Hook Extension".into();
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        store
            .register_hooks(
                "extension-a",
                &[
                    HookRegistration {
                        id: "slow".into(),
                        name: "Slow Hook".into(),
                        phase: HookPhase::PreToolUse,
                        priority: Some(200),
                        module: "./hooks/slow.js".into(),
                        when: None,
                        action: Default::default(),
                    },
                    HookRegistration {
                        id: "fast".into(),
                        name: "Fast Hook".into(),
                        phase: HookPhase::PreToolUse,
                        priority: Some(10),
                        module: "./hooks/fast.js".into(),
                        when: None,
                        action: Default::default(),
                    },
                    HookRegistration {
                        id: "compact".into(),
                        name: "Compact Hook".into(),
                        phase: HookPhase::PreCompact,
                        priority: None,
                        module: "./hooks/compact.js".into(),
                        when: None,
                        action: Default::default(),
                    },
                ],
            )
            .unwrap();

        let pre_tool = store.list_hook_declarations_by_phase(HookPhase::PreToolUse);
        assert_eq!(pre_tool.len(), 2);
        assert_eq!(pre_tool[0].hook_id, "fast");
        assert_eq!(pre_tool[0].runtime_id, "extension-a/fast");
        assert_eq!(pre_tool[0].extension_name, "Hook Extension");
        assert_eq!(pre_tool[1].hook_id, "slow");

        let compact = store.list_hook_declarations_by_phase(HookPhase::PreCompact);
        assert_eq!(compact.len(), 1);
        assert_eq!(compact[0].hook_id, "compact");
    }

    #[test]
    fn test_register_hooks_requires_enabled_extension() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        store.register(create_test_state("extension-a")).unwrap();

        let result = store.register_hooks(
            "extension-a",
            &[HookRegistration {
                id: "blocked".into(),
                name: "Blocked Hook".into(),
                phase: HookPhase::SessionStart,
                priority: None,
                module: "./hooks/blocked.js".into(),
                when: None,
                action: Default::default(),
            }],
        );

        assert!(matches!(
            result,
            Err(ExtensionStoreError::ExtensionNotEnabled(_))
        ));
        assert!(store.list_hooks().is_empty());
    }

    #[test]
    fn test_unregister_hooks_removes_only_extension_hooks() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let mut state_a = create_test_state("extension-a");
        state_a.status = ExtensionStatus::Enabled;
        let mut state_b = create_test_state("extension-b");
        state_b.status = ExtensionStatus::Enabled;
        store.register(state_a).unwrap();
        store.register(state_b).unwrap();

        let hook = |id: &str| HookRegistration {
            id: id.into(),
            name: id.into(),
            phase: HookPhase::SessionStart,
            priority: None,
            module: format!("./hooks/{}.js", id),
            when: None,
            action: Default::default(),
        };

        store.register_hooks("extension-a", &[hook("a")]).unwrap();
        store.register_hooks("extension-b", &[hook("b")]).unwrap();

        assert_eq!(store.unregister_hooks("extension-a"), 1);
        let hooks = store.list_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].extension_id, "extension-b");
    }

    #[test]
    fn test_unregister_extension_clears_hook_declarations() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let mut state = create_test_state("extension-a");
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        store
            .register_hooks(
                "extension-a",
                &[HookRegistration {
                    id: "cleanup".into(),
                    name: "Cleanup Hook".into(),
                    phase: HookPhase::SessionStart,
                    priority: None,
                    module: "./hooks/cleanup.js".into(),
                    when: None,
                    action: Default::default(),
                }],
            )
            .unwrap();

        assert_eq!(store.list_hooks().len(), 1);
        store.unregister("extension-a").unwrap();
        assert!(store.list_hooks().is_empty());
    }

    #[test]
    fn test_disabling_extension_clears_hook_declarations() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));
        let mut state = create_test_state("extension-a");
        state.status = ExtensionStatus::Enabled;
        store.register(state).unwrap();

        store
            .register_hooks(
                "extension-a",
                &[HookRegistration {
                    id: "disable-cleanup".into(),
                    name: "Disable Cleanup Hook".into(),
                    phase: HookPhase::SessionStart,
                    priority: None,
                    module: "./hooks/disable-cleanup.js".into(),
                    when: None,
                    action: Default::default(),
                }],
            )
            .unwrap();

        store
            .update_status("extension-a", ExtensionStatus::Disabled, None)
            .unwrap();

        assert!(store.list_hooks().is_empty());
    }

    #[test]
    fn test_disabled_extensions_not_conflicting() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        let manifest = ExtensionManifest {
            id: "extension-a".into(),
            name: "Extension A".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "test".into(),
            permissions: ExtensionPermissions::default(),
            contributes: ExtensionContributes {
                triggers: Some(vec![TriggerRegistration {
                    prefix: "/pr".into(),
                    label: "PR".into(),
                    description: "desc".into(),
                    icon: None,
                    placeholder: None,
                    search_module: "./s.js".into(),
                    select_module: "./s.js".into(),
                    scope: super::super::models::TriggerScope::Global,
                }]),
                ..Default::default()
            },
        };

        // 禁用的扩展不产生冲突
        let mut state = create_test_state("extension-a");
        state.manifest = manifest;
        state.status = ExtensionStatus::Disabled;
        store.register(state).unwrap();

        let result = store.check_trigger_conflict("/pr", None);
        assert!(result.is_ok());
    }

    /// B6: 验证扩展注册表完整生命周期链：注册 -> 启用 -> 调用查询 -> 禁用 -> 移除
    #[test]
    fn test_registry_lifecycle_full_chain() {
        let store = ExtensionStore::new(Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            test_runtime_handle(),
        )));

        // 1. 注册扩展（Installed 状态）
        let state = create_test_state("lifecycle-extension");
        store.register(state.clone()).unwrap();
        let retrieved = store.get("lifecycle-extension").unwrap();
        assert_eq!(retrieved.status, ExtensionStatus::Installed);
        assert_eq!(store.count(), 1);

        // 2. 启用扩展（通过 update_status 同时更新 extension status 和 kernel lifecycle）
        store
            .update_status("lifecycle-extension", ExtensionStatus::Enabled, None)
            .unwrap();
        let state = store.get("lifecycle-extension").unwrap();
        assert_eq!(state.status, ExtensionStatus::Enabled);

        // 3. 调用查询：已启用扩展应出现在 list 中且可通过 get 获取
        let all = store.list();
        assert!(
            all.iter().any(|s| s.id == "lifecycle-extension"),
            "enabled extension should appear in list"
        );
        let manifest = store.get_manifest("lifecycle-extension").unwrap();
        assert_eq!(manifest.name, "Extension lifecycle-extension");

        // 4. 禁用扩展
        store
            .update_status("lifecycle-extension", ExtensionStatus::Disabled, None)
            .unwrap();
        let state = store.get("lifecycle-extension").unwrap();
        assert_eq!(state.status, ExtensionStatus::Disabled);

        // 5. 移除扩展
        store.unregister("lifecycle-extension").unwrap();
        assert_eq!(store.count(), 0);
        assert!(
            store.get("lifecycle-extension").is_none(),
            "removed extension should not be found"
        );
    }
}
