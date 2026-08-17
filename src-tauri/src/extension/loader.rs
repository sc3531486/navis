//! Extension 加载器
//!
//! 基于设计文档 §07 实现扩展清单的加载与校验。
//!
//! 职责：
//! - 从目录加载 extension.json
//! - 校验清单完整性
//! - 校验权限声明
//! - 校验 contributes 跨类型引用完整性

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::host_view::validate_extension_view;
use super::models::{
    BackendServiceRegistration, BuiltinAction, CommandRegistration, ComponentRegistration,
    EventSubscriptionRegistration, ExtensionManifest, ExtensionPermissions, HookRegistration,
    TriggerRegistration, ViewRegistration, WorkModeRegistration,
};

/// 扩展加载器
///
/// 负责从文件系统读取 extension.json，解析并校验。
pub struct ExtensionLoader;

pub fn is_valid_extension_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && id != "."
        && id != ".."
}

impl ExtensionLoader {
    /// 创建新的扩展加载器
    pub fn new() -> Self {
        Self
    }

    /// 从指定路径加载扩展清单
    ///
    /// # Arguments
    /// * `extension_dir` - 扩展目录路径（包含 extension.json）
    pub fn load_manifest(&self, extension_dir: &Path) -> Result<ExtensionManifest> {
        let manifest_path = extension_dir.join("extension.json");
        tracing::debug!(path = %manifest_path.display(), "Loading extension manifest");

        let content =
            fs::read_to_string(&manifest_path).context("Failed to read extension.json")?;

        let manifest: ExtensionManifest =
            serde_json::from_str(&content).context("Failed to parse extension.json")?;

        // 校验清单
        self.validate_manifest(&manifest)
            .context("Manifest validation failed")?;

        tracing::debug!(
            extension_id = %manifest.id,
            version = %manifest.version,
            "Extension manifest loaded and validated"
        );

        Ok(manifest)
    }

    /// 从 JSON 字符串解析扩展清单
    pub fn parse_manifest(&self, json: &str) -> Result<ExtensionManifest> {
        let manifest: ExtensionManifest =
            serde_json::from_str(json).context("Failed to parse manifest JSON")?;

        self.validate_manifest(&manifest)
            .context("Manifest validation failed")?;

        Ok(manifest)
    }

    /// 校验已解析的清单（供外部使用）
    pub fn validate(&self, manifest: &ExtensionManifest) -> Result<()> {
        self.validate_manifest(manifest)
    }

    /// 校验清单完整性
    fn validate_manifest(&self, manifest: &ExtensionManifest) -> Result<()> {
        // 基本字段校验
        if manifest.id.is_empty() {
            return Err(anyhow::anyhow!("Extension ID is empty"));
        }
        if !is_valid_extension_id(&manifest.id) {
            return Err(anyhow::anyhow!(
                "Extension ID contains unsupported characters"
            ));
        }
        if manifest.name.is_empty() {
            return Err(anyhow::anyhow!("Extension name is empty"));
        }
        if manifest.version.is_empty() {
            return Err(anyhow::anyhow!("Extension version is empty"));
        }

        // 权限校验
        self.validate_permissions(&manifest.permissions)?;

        // contributes 引用校验
        self.validate_contributes(&manifest.contributes)?;

        // 触发器前缀格式校验
        if let Some(ref triggers) = manifest.contributes.triggers {
            self.validate_triggers(triggers)?;
        }

        // Custom 工作模式校验
        if let Some(ref work_modes) = manifest.contributes.work_modes {
            self.validate_work_modes(work_modes)?;
        }

        // 命令声明式 action 校验
        if let Some(ref commands) = manifest.contributes.commands {
            self.validate_commands(commands)?;
        }

        // Hook ID 和模块路径校验
        if let Some(ref hooks) = manifest.contributes.hooks {
            self.validate_hooks(hooks)?;
        }

        // EventBus 订阅声明只在 loader 层做合同校验；runtime 尚未落地时，
        // lifecycle 必须 fail-closed，不在这里或其他加载路径注册 EventBus。
        if let Some(ref subscriptions) = manifest.contributes.event_subscriptions {
            self.validate_event_subscriptions(subscriptions)?;
        }

        // capabilities 白名单校验（34 §3.4）：invoke 引用必须存在且命令名合法，
        // events/read 为 pattern 字符串，extension_calls target 为合法扩展/命名空间。
        if let Some(ref capabilities) = manifest.contributes.capabilities {
            self.validate_capabilities(manifest, capabilities)?;
        }

        // extension_exports 引用校验（34 §2.4）。
        if let Some(ref exports) = manifest.contributes.extension_exports {
            self.validate_extension_exports(manifest, exports)?;
        }

        // 后端扩展服务声明校验（35 §3.4）。
        if let Some(ref backend_services) = manifest.contributes.backend_services {
            self.validate_backend_services(backend_services)?;
        }

        // WASM 组件轨声明校验（37 §5.1）：entry 必须位于 ExtensionUI/ 或 ExtensionBackend/ 下。
        if let Some(ref components) = manifest.contributes.components {
            self.validate_components(components)?;
        }

        Ok(())
    }

    /// 校验权限声明
    fn validate_permissions(&self, permissions: &ExtensionPermissions) -> Result<()> {
        // 校验资源限制值合理性
        if permissions.resources.max_memory_mb == 0 {
            return Err(anyhow::anyhow!("max_memory_mb must be greater than 0"));
        }
        if permissions.resources.max_cpu_percent <= 0.0
            || permissions.resources.max_cpu_percent > 100.0
        {
            return Err(anyhow::anyhow!(
                "max_cpu_percent must be between 0 (exclusive) and 100 (inclusive)"
            ));
        }
        if permissions.resources.timeout_ms == 0 {
            return Err(anyhow::anyhow!("timeout_ms must be greater than 0"));
        }
        Ok(())
    }

    /// 校验 contributes 内部引用完整性
    fn validate_contributes(
        &self,
        contributes: &super::models::ExtensionContributes,
    ) -> Result<()> {
        // 收集所有 commands.id 用于引用校验
        let command_ids: Vec<String> = contributes
            .commands
            .as_ref()
            .map(|cmds| cmds.iter().map(|c| c.id.clone()).collect())
            .unwrap_or_default();

        // 收集所有 views.id 用于引用校验
        let view_ids: Vec<String> = contributes
            .views
            .as_ref()
            .map(|vs| vs.iter().map(|v| v.id.clone()).collect())
            .unwrap_or_default();
        if let Some(ref views) = contributes.views {
            for view in views {
                self.validate_view(view)?;
            }
        }

        // 收集所有 roles.id
        let role_ids: Vec<String> = contributes
            .roles
            .as_ref()
            .map(|roles| roles.iter().map(|role| role.id.clone()).collect())
            .unwrap_or_default();

        // 收集所有 skills.id
        let skill_ids: Vec<String> = contributes
            .skills
            .as_ref()
            .map(|skills| skills.iter().map(|skill| skill.id.clone()).collect())
            .unwrap_or_default();

        // 收集所有 context_providers.id
        let context_provider_ids: Vec<String> = contributes
            .context_providers
            .as_ref()
            .map(|providers| {
                providers
                    .iter()
                    .map(|provider| provider.id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // 收集所有 scripts.id 用于 RunScript 引用校验
        let script_ids: Vec<String> = contributes
            .scripts
            .as_ref()
            .map(|scripts| scripts.iter().map(|script| script.id.clone()).collect())
            .unwrap_or_default();

        // 校验 menus.command 引用 commands.id
        if let Some(ref menus) = contributes.menus {
            for menu in menus {
                if !command_ids.contains(&menu.command) {
                    return Err(anyhow::anyhow!(
                        "Menu '{}' references non-existent command '{}'",
                        menu.id,
                        menu.command
                    ));
                }
            }
        }

        // 校验 keybindings.command 引用 commands.id
        if let Some(ref keybindings) = contributes.keybindings {
            for kb in keybindings {
                if !command_ids.contains(&kb.command) {
                    return Err(anyhow::anyhow!(
                        "Keybinding key '{}' references non-existent command '{}'",
                        kb.key,
                        kb.command
                    ));
                }
            }
        }

        // 校验 toolbar_items.command 引用 commands.id
        if let Some(ref toolbar_items) = contributes.toolbar_items {
            for item in toolbar_items {
                if !command_ids.contains(&item.command) {
                    return Err(anyhow::anyhow!(
                        "ToolbarItem '{}' references non-existent command '{}'",
                        item.id,
                        item.command
                    ));
                }
            }
        }

        // 校验 tray_items.command 引用 commands.id
        if let Some(ref tray_items) = contributes.tray_items {
            for item in tray_items {
                if !command_ids.contains(&item.command) {
                    return Err(anyhow::anyhow!(
                        "TrayItem '{}' references non-existent command '{}'",
                        item.id,
                        item.command
                    ));
                }
            }
        }

        // 校验 statusbar_items.command（可选）引用 commands.id
        if let Some(ref statusbar_items) = contributes.statusbar_items {
            for item in statusbar_items {
                if let Some(ref cmd) = item.command {
                    if !command_ids.contains(cmd) {
                        return Err(anyhow::anyhow!(
                            "StatusBarItem '{}' references non-existent command '{}'",
                            item.id,
                            cmd
                        ));
                    }
                }
            }
        }

        // 校验 commands 中 BuiltinAction 的引用
        if let Some(ref commands) = contributes.commands {
            for cmd in commands {
                match &cmd.action {
                    BuiltinAction::OpenView { view_id }
                    | BuiltinAction::ToggleView { view_id }
                    | BuiltinAction::OpenDialog { view_id, .. } => {
                        let action_name = match &cmd.action {
                            BuiltinAction::OpenView { .. } => "OpenView",
                            BuiltinAction::ToggleView { .. } => "ToggleView",
                            BuiltinAction::OpenDialog { .. } => "OpenDialog",
                            _ => unreachable!(),
                        };
                        if !view_id.contains(':') && !view_ids.contains(view_id) {
                            return Err(anyhow::anyhow!(
                                "Command '{}' {} references non-existent view '{}'",
                                cmd.id,
                                action_name,
                                view_id
                            ));
                        }
                        if !view_id.contains(':') {
                            self.validate_command_view_action(
                                contributes,
                                &cmd.id,
                                action_name,
                                view_id,
                            )?;
                        }
                    }
                    BuiltinAction::RunScript { script_id, .. } => {
                        if script_id.trim().is_empty() {
                            return Err(anyhow::anyhow!(
                                "Command '{}' RunScript references empty script id",
                                cmd.id
                            ));
                        }
                        if !script_id.contains(':') && !script_ids.contains(script_id) {
                            return Err(anyhow::anyhow!(
                                "Command '{}' RunScript references non-existent script '{}'",
                                cmd.id,
                                script_id
                            ));
                        }
                    }
                    BuiltinAction::SendMessage { target, .. } => {
                        if target.trim().is_empty() {
                            return Err(anyhow::anyhow!(
                                "Command '{}' SendMessage target must not be empty",
                                cmd.id
                            ));
                        }
                    }
                }
            }
        }

        // 校验 work_modes 引用自身 contributes 中声明的资源。
        // 内建 role / skill / command / context policy 由对应子系统在注册阶段校验。
        if let Some(ref work_modes) = contributes.work_modes {
            for mode in work_modes {
                if let Some(ref role) = mode.role {
                    let builtin_roles = ["developer", "technical-writer", "assistant"];
                    if !builtin_roles.contains(&role.as_str()) && !role_ids.contains(role) {
                        return Err(anyhow::anyhow!(
                            "WorkMode '{}' references non-existent role '{}'",
                            mode.id,
                            role
                        ));
                    }
                }

                if let Some(ref commands) = mode.commands {
                    for command_id in commands {
                        if !command_ids.contains(command_id) {
                            return Err(anyhow::anyhow!(
                                "WorkMode '{}' references non-existent command '{}'",
                                mode.id,
                                command_id
                            ));
                        }
                    }
                }

                if let Some(ref skills) = mode.skills {
                    let builtin_skills = [
                        "commit", "refactor", "test-gen", "bug-fix", "review", "explain", "doc-gen",
                    ];
                    for skill_id in skills {
                        if !builtin_skills.contains(&skill_id.as_str())
                            && !skill_ids.contains(skill_id)
                        {
                            return Err(anyhow::anyhow!(
                                "WorkMode '{}' references non-existent skill '{}'",
                                mode.id,
                                skill_id
                            ));
                        }
                    }
                }

                if let Some(ref entry_view) = mode.entry_view {
                    if !view_ids.contains(entry_view) {
                        return Err(anyhow::anyhow!(
                            "WorkMode '{}' references non-existent entry_view '{}'",
                            mode.id,
                            entry_view
                        ));
                    }
                }

                if let Some(ref default_views) = mode.default_views {
                    for view_id in default_views {
                        if !view_ids.contains(view_id) {
                            return Err(anyhow::anyhow!(
                                "WorkMode '{}' references non-existent default_view '{}'",
                                mode.id,
                                view_id
                            ));
                        }
                    }
                }

                if let Some(ref context_policy) = mode.context_policy {
                    let builtin_context_policies = ["code", "cowork", "default"];
                    if !builtin_context_policies.contains(&context_policy.as_str())
                        && !context_provider_ids.contains(context_policy)
                    {
                        return Err(anyhow::anyhow!(
                            "WorkMode '{}' references non-existent context_policy '{}'",
                            mode.id,
                            context_policy
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_view(&self, view: &ViewRegistration) -> Result<()> {
        validate_extension_view(view)
    }

    fn validate_command_view_action(
        &self,
        contributes: &super::models::ExtensionContributes,
        command_id: &str,
        action_name: &str,
        view_id: &str,
    ) -> Result<()> {
        let Some(view) = contributes
            .views
            .as_ref()
            .and_then(|views| views.iter().find(|view| view.id == view_id))
        else {
            return Ok(());
        };
        self.validate_view(view).with_context(|| {
            format!(
                "Command '{}' {} references unsupported view '{}'",
                command_id, action_name, view_id
            )
        })
    }

    /// 校验触发器前缀格式
    fn validate_triggers(&self, triggers: &[TriggerRegistration]) -> Result<()> {
        for trigger in triggers {
            // 前缀必须以 / 开头
            if !trigger.prefix.starts_with('/') {
                return Err(anyhow::anyhow!(
                    "Trigger prefix '{}' must start with '/'",
                    trigger.prefix
                ));
            }

            // 前缀只允许 / + 小写字母/数字/短横线/点号
            let prefix_body = &trigger.prefix[1..];
            if prefix_body.is_empty() {
                return Err(anyhow::anyhow!(
                    "Trigger prefix '/' is too short, must have at least one character after '/'"
                ));
            }
            if !prefix_body
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
            {
                return Err(anyhow::anyhow!(
                    "Trigger prefix '{}' can only contain lowercase letters, digits, '-' or '.' after '/'",
                    trigger.prefix
                ));
            }
        }
        Ok(())
    }

    /// 校验 Custom 工作模式基本字段
    fn validate_work_modes(&self, work_modes: &[WorkModeRegistration]) -> Result<()> {
        let mut seen = HashSet::new();
        for mode in work_modes {
            if mode.id.is_empty() {
                return Err(anyhow::anyhow!("WorkMode ID is empty"));
            }
            if mode.id.contains(':') {
                return Err(anyhow::anyhow!(
                    "WorkMode '{}' must not contain ':'; runtime id is custom:<extensionId>/{}",
                    mode.id,
                    mode.id
                ));
            }
            if !seen.insert(mode.id.as_str()) {
                return Err(anyhow::anyhow!("Duplicate WorkMode ID '{}'", mode.id));
            }
        }
        Ok(())
    }

    /// 命令 action 在类型层面必须存在，具体变体由前面的贡献校验处理。
    fn validate_commands(&self, _commands: &[CommandRegistration]) -> Result<()> {
        Ok(())
    }

    /// 校验 Hook 声明。Hook ID 在同一扩展内必须唯一。
    fn validate_hooks(&self, hooks: &[HookRegistration]) -> Result<()> {
        let mut seen = HashSet::new();
        for hook in hooks {
            if hook.id.trim().is_empty() {
                return Err(anyhow::anyhow!("Hook ID is empty"));
            }
            if !seen.insert(hook.id.as_str()) {
                return Err(anyhow::anyhow!("Duplicate Hook ID '{}'", hook.id));
            }
            if hook.module.trim().is_empty() {
                return Err(anyhow::anyhow!("Hook '{}' module is empty", hook.id));
            }
        }
        Ok(())
    }

    /// 校验 EventBus 订阅声明。
    ///
    /// 这些声明仍然只是受控 DTO。这里只验证 manifest 合同，不解析、加载或执行
    /// handler，也不向 EventBus 注册订阅。
    fn validate_event_subscriptions(
        &self,
        subscriptions: &[EventSubscriptionRegistration],
    ) -> Result<()> {
        let mut seen_ids = HashSet::new();

        for subscription in subscriptions {
            let id = subscription.id.trim();
            if id.is_empty() {
                return Err(anyhow::anyhow!("Event subscription ID is empty"));
            }
            if id != subscription.id || !is_valid_extension_id(id) {
                return Err(anyhow::anyhow!(
                    "Event subscription ID '{}' contains unsupported characters",
                    subscription.id
                ));
            }
            if !seen_ids.insert(id) {
                return Err(anyhow::anyhow!(
                    "Duplicate Event subscription ID '{}'",
                    subscription.id
                ));
            }

            self.validate_event_topic(&subscription.topic, id)?;
            self.validate_scope_key(subscription.scope_key.as_deref(), id)?;
            self.validate_handler_module(&subscription.handler.module, id)?;
            self.validate_handler_export(&subscription.handler.export, id)?;
        }

        Ok(())
    }

    fn validate_event_topic(&self, topic: &str, subscription_id: &str) -> Result<()> {
        if topic.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' topic is empty",
                subscription_id
            ));
        }
        if topic != topic.trim()
            || topic.chars().any(|character| !character.is_ascii())
            || topic
                .chars()
                .any(|character| character.is_ascii_whitespace())
            || topic.chars().any(|character| character.is_ascii_control())
        {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' topic must be an ASCII topic without whitespace",
                subscription_id
            ));
        }

        let segments: Vec<&str> = topic.split('.').collect();
        if segments.iter().any(|segment| {
            segment.is_empty()
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        }) {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' topic '{}' must contain only dot-separated name segments",
                subscription_id,
                topic
            ));
        }

        Ok(())
    }

    fn validate_scope_key(&self, scope_key: Option<&str>, subscription_id: &str) -> Result<()> {
        let Some(scope_key) = scope_key else {
            return Ok(());
        };

        if scope_key.trim().is_empty()
            || scope_key != scope_key.trim()
            || scope_key
                .chars()
                .any(|character| character.is_ascii_control())
            || scope_key
                .chars()
                .any(|character| character.is_ascii_whitespace())
        {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' scopeKey must be a non-empty value without whitespace",
                subscription_id
            ));
        }

        Ok(())
    }

    fn validate_handler_module(&self, module: &str, subscription_id: &str) -> Result<()> {
        if module.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' handler module is empty",
                subscription_id
            ));
        }
        if module != module.trim()
            || !module.starts_with("./")
            || module.contains('\\')
            || module.contains("://")
            || module.contains('?')
            || module.contains('#')
            || module.chars().any(|character| character.is_ascii_control())
            || Path::new(module).is_absolute()
        {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' handler module '{}' must be a relative './' path",
                subscription_id,
                module
            ));
        }

        let segments: Vec<&str> = module.split('/').collect();
        if segments.first().copied() != Some(".")
            || segments.len() < 2
            || segments[1..].iter().any(|segment| {
                segment.is_empty() || *segment == "." || *segment == ".." || segment.contains(':')
            })
        {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' handler module '{}' contains an unsafe path segment",
                subscription_id,
                module
            ));
        }

        Ok(())
    }

    fn validate_handler_export(&self, export: &str, subscription_id: &str) -> Result<()> {
        let mut characters = export.chars();
        let Some(first) = characters.next() else {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' handler export is empty",
                subscription_id
            ));
        };

        if export != export.trim() || !is_valid_handler_identifier(first, characters) {
            return Err(anyhow::anyhow!(
                "Event subscription '{}' handler export '{}' must be a valid export name",
                subscription_id,
                export
            ));
        }

        Ok(())
    }

    /// 校验 capabilities 白名单声明。
    ///
    /// `invoke` 中的命令名必须存在于本 manifest 的 `contributes.commands` 或属于
    /// 宿主已注册命令格式；事件/读取 pattern 必须是合法命名空间。未声明任何
    /// 能力时，扩展 UI 只能做纯静态渲染。
    fn validate_capabilities(
        &self,
        manifest: &ExtensionManifest,
        capabilities: &super::models::CapabilityDeclaration,
    ) -> Result<()> {
        let command_ids: Vec<&str> = manifest
            .contributes
            .commands
            .as_ref()
            .map(|cmds| cmds.iter().map(|command| command.id.as_str()).collect())
            .unwrap_or_default();

        for command in &capabilities.invoke {
            if command.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "Capabilities invoke contains an empty command name"
                ));
            }
            if !is_valid_invoke_command_name(command) {
                return Err(anyhow::anyhow!(
                    "Capabilities invoke command '{command}' contains unsupported characters"
                ));
            }
            // invoke 引用的命令必须已在本扩展声明或属于宿主命令。
            // 宿主命令名集合无法在此静态枚举，运行时由桥派发前再次校验；
            // 这里保证扩展自声明命令引用存在。
            if !command_ids.contains(&command.as_str())
                && !command.starts_with("host:")
                && !command.starts_with("file.")
                && !command.starts_with("context.")
                && !command.starts_with("clipboard.")
            {
                tracing::debug!(
                    extension_id = %manifest.id,
                    command = %command,
                    "Capabilities invoke references a host command resolved at runtime"
                );
            }
        }

        for event in &capabilities.events {
            self.validate_event_topic(event, "capabilities.events")?;
        }

        for read_key in &capabilities.read {
            if read_key.trim().is_empty()
                || read_key != read_key.trim()
                || read_key.chars().any(|character| character.is_ascii_control())
            {
                return Err(anyhow::anyhow!(
                    "Capabilities read key '{read_key}' must be a non-empty value without whitespace"
                ));
            }
        }

        for call in &capabilities.extension_calls {
            if call.target.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "Capabilities extension call target is empty"
                ));
            }
            for action in &call.actions {
                match action.as_str() {
                    "view.open"
                    | "view.toggle"
                    | "command.execute"
                    | "message.send"
                    | "event.emit"
                    | "event.subscribe"
                    | "*" => {}
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Capabilities extension call '{}' references unsupported action '{}'",
                            call.target,
                            action
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// 校验 extension_exports 引用（34 §2.4）。
    fn validate_extension_exports(
        &self,
        manifest: &ExtensionManifest,
        exports: &super::models::ExtensionExports,
    ) -> Result<()> {
        let view_ids: Vec<&str> = manifest
            .contributes
            .views
            .as_ref()
            .map(|views| views.iter().map(|view| view.id.as_str()).collect())
            .unwrap_or_default();
        let command_ids: Vec<&str> = manifest
            .contributes
            .commands
            .as_ref()
            .map(|commands| commands.iter().map(|command| command.id.as_str()).collect())
            .unwrap_or_default();

        for view in &exports.views {
            if !view_ids.contains(&view.as_str()) {
                return Err(anyhow::anyhow!(
                    "Extension exports references non-existent view '{}'",
                    view
                ));
            }
        }
        for command in &exports.commands {
            if !command_ids.contains(&command.as_str()) {
                return Err(anyhow::anyhow!(
                    "Extension exports references non-existent command '{}'",
                    command
                ));
            }
        }
        Ok(())
    }

    /// 校验后端扩展服务声明（35 §3.4）。
    ///
    /// `transport`/`protocol` 是枚举，非法值在反序列化阶段即失败，这里只校验
    /// 服务 ID 唯一性与 entry 路径安全（必须位于扩展 ExtensionBackend/ 目录）。
    fn validate_backend_services(
        &self,
        backend_services: &[BackendServiceRegistration],
    ) -> Result<()> {
        let mut seen_ids = HashSet::new();
        for service in backend_services {
            let id = service.id.trim();
            if id.is_empty() {
                return Err(anyhow::anyhow!("Backend service ID is empty"));
            }
            if id != service.id
                || service
                    .id
                    .chars()
                    .any(|character| character.is_ascii_whitespace())
                || service
                    .id
                    .chars()
                    .any(|character| character.is_ascii_control())
            {
                return Err(anyhow::anyhow!(
                    "Backend service ID '{}' contains whitespace or control characters",
                    service.id
                ));
            }
            if !seen_ids.insert(id) {
                return Err(anyhow::anyhow!(
                    "Duplicate Backend service ID '{}'",
                    service.id
                ));
            }
            self.validate_backend_service_entry(&service.entry, &service.id)?;
        }
        Ok(())
    }

    /// 校验后端服务可执行文件 entry：必须是位于 ExtensionBackend/ 下的相对路径。
    fn validate_backend_service_entry(&self, entry: &str, service_id: &str) -> Result<()> {
        if entry.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Backend service '{service_id}' entry is empty"
            ));
        }
        if entry != entry.trim()
            || entry.contains('\\')
            || entry.contains("://")
            || entry.starts_with('/')
            || entry.starts_with('\0')
            || Path::new(entry).is_absolute()
        {
            return Err(anyhow::anyhow!(
                "Backend service '{service_id}' entry '{entry}' must be a relative path using '/' separators"
            ));
        }

        let segments: Vec<&str> = entry.split('/').collect();
        if segments.first().copied() != Some("ExtensionBackend") || segments.len() < 2 {
            return Err(anyhow::anyhow!(
                "Backend service '{service_id}' entry '{entry}' must be located below the ExtensionBackend directory"
            ));
        }
        for segment in &segments[1..] {
            if segment.is_empty() || *segment == "." || *segment == ".." || segment.contains(':') {
                return Err(anyhow::anyhow!(
                    "Backend service '{service_id}' entry '{entry}' contains an unsafe path segment"
                ));
            }
        }

        Ok(())
    }

    /// 校验 WASM 组件轨声明（37 §5.1）。
    ///
    /// `kind` 是枚举，非法值在反序列化阶段即失败，这里只校验组件 ID 非空且唯一、
    /// entry 为 .wasm 相对路径且必须位于 ExtensionUI/ 或 ExtensionBackend/ 目录下。
    fn validate_components(&self, components: &[ComponentRegistration]) -> Result<()> {
        let mut seen_ids = HashSet::new();
        for component in components {
            let id = component.id.trim();
            if id.is_empty() {
                return Err(anyhow::anyhow!("Component ID is empty"));
            }
            if id != component.id
                || component
                    .id
                    .chars()
                    .any(|character| character.is_ascii_whitespace())
                || component
                    .id
                    .chars()
                    .any(|character| character.is_ascii_control())
            {
                return Err(anyhow::anyhow!(
                    "Component ID '{}' contains whitespace or control characters",
                    component.id
                ));
            }
            if !seen_ids.insert(id) {
                return Err(anyhow::anyhow!(
                    "Duplicate Component ID '{}'",
                    component.id
                ));
            }
            self.validate_component_entry(&component.entry, &component.id)?;
            self.validate_component_run_on(&component.id, &component.run_on)?;
        }
        Ok(())
    }

    /// 校验组件 runOn 触发时机白名单（fail-closed）：只接受 `activation` /
    /// `message`（37 §5.1 / 35 §5.3 语义），未知触发值拒绝，防止声明被静默忽略。
    fn validate_component_run_on(&self, component_id: &str, run_on: &[String]) -> Result<()> {
        for trigger in run_on {
            let trigger = trigger.trim();
            if trigger.is_empty() {
                return Err(anyhow::anyhow!(
                    "Component '{component_id}' has an empty runOn trigger"
                ));
            }
            if !["activation", "message"].contains(&trigger) {
                return Err(anyhow::anyhow!(
                    "Component '{component_id}' declares unsupported runOn trigger '{trigger}' (supported: activation, message)"
                ));
            }
        }
        Ok(())
    }

    /// 校验组件 entry：必须是位于 ExtensionUI/ 或 ExtensionBackend/ 下的 .wasm 相对路径。
    fn validate_component_entry(&self, entry: &str, component_id: &str) -> Result<()> {
        if entry.trim().is_empty() {
            return Err(anyhow::anyhow!("Component '{component_id}' entry is empty"));
        }
        if !entry.ends_with(".wasm") {
            return Err(anyhow::anyhow!(
                "Component '{component_id}' entry '{entry}' must end with '.wasm'"
            ));
        }
        if entry != entry.trim()
            || entry.contains('\\')
            || entry.contains("://")
            || entry.starts_with('/')
            || entry.starts_with('\0')
            || Path::new(entry).is_absolute()
        {
            return Err(anyhow::anyhow!(
                "Component '{component_id}' entry '{entry}' must be a relative path using '/' separators"
            ));
        }

        let segments: Vec<&str> = entry.split('/').collect();
        if segments.len() < 2
            || !matches!(
                segments.first().copied(),
                Some("ExtensionUI") | Some("ExtensionBackend")
            )
        {
            return Err(anyhow::anyhow!(
                "Component '{component_id}' entry '{entry}' must be located below the ExtensionUI or ExtensionBackend directory"
            ));
        }
        for segment in &segments[1..] {
            if segment.is_empty() || *segment == "." || *segment == ".." || segment.contains(':') {
                return Err(anyhow::anyhow!(
                    "Component '{component_id}' entry '{entry}' contains an unsafe path segment"
                ));
            }
        }

        Ok(())
    }
}

fn is_valid_handler_identifier<I>(first: char, mut rest: I) -> bool
where
    I: Iterator<Item = char>,
{
    (first.is_ascii_alphabetic() || matches!(first, '_' | '$'))
        && rest.all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
}

/// 校验 IPC 命令名：允许点分命名空间（如 `file.read`、`host:clipboard.get`），
/// 只含小写字母/数字/点/短横线/下划线/冒号。
fn is_valid_invoke_command_name(command: &str) -> bool {
    command.len() <= 128
        && !command.starts_with('.')
        && !command.ends_with('.')
        && command.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'-' | b'_' | b':')
        })
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::host_view::HOST_PANEL_RENDERER;
    use crate::extension::models::{
        EventHandlerReference, EventSubscriptionRegistration, ExtensionContributes,
        KeybindingRegistration, ResourceLimits, ToolbarItemRegistration,
    };
    use std::fs;

    fn create_valid_manifest(id: &str) -> ExtensionManifest {
        ExtensionManifest {
            id: id.to_string(),
            name: format!("Extension {}", id),
            version: "1.0.0".into(),
            description: "test".into(),
            author: "tester".into(),
            permissions: ExtensionPermissions {
                filesystem: vec![],
                terminal: vec![],
                network: vec![],
                ipc: vec![],
                events: vec![],
                resources: ResourceLimits {
                    max_memory_mb: 512,
                    max_cpu_percent: 50.0,
                    timeout_ms: 30000,
                },
            },
            contributes: ExtensionContributes::default(),
        }
    }

    fn valid_event_subscription(id: &str) -> EventSubscriptionRegistration {
        EventSubscriptionRegistration {
            id: id.into(),
            topic: "session.completed".into(),
            scope_key: Some("session:active".into()),
            handler: EventHandlerReference {
                module: "./runtime/events".into(),
                export: "onSessionCompleted".into(),
            },
        }
    }

    #[test]
    fn test_load_valid_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();

        let manifest_json = serde_json::json!({
            "id": "com.test.valid",
            "name": "Valid Extension",
            "version": "1.0.0",
            "description": "test",
            "author": "tester",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {}
        });

        fs::write(
            temp_dir.path().join("extension.json"),
            manifest_json.to_string(),
        )
        .unwrap();

        let loader = ExtensionLoader::new();
        let manifest = loader.load_manifest(temp_dir.path()).unwrap();

        assert_eq!(manifest.id, "com.test.valid");
        assert_eq!(manifest.name, "Valid Extension");
    }

    #[test]
    fn test_load_missing_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let loader = ExtensionLoader::new();

        let result = loader.load_manifest(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("extension.json"), "{bad json}").unwrap();

        let loader = ExtensionLoader::new();
        let result = loader.load_manifest(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_id() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.id = "".to_string();

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ID is empty"));
    }

    #[test]
    fn test_validate_empty_name() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.name = "".to_string();

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name is empty"));
    }

    #[test]
    fn test_validate_zero_memory() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.permissions.resources.max_memory_mb = 0;

        let result = loader.validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_cpu_percent() {
        let loader = ExtensionLoader::new();

        // CPU > 100
        let mut manifest = create_valid_manifest("test");
        manifest.permissions.resources.max_cpu_percent = 150.0;
        let result = loader.validate(&manifest);
        assert!(result.is_err());

        // CPU = 0
        manifest.permissions.resources.max_cpu_percent = 0.0;
        let result = loader.validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_timeout() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.permissions.resources.timeout_ms = 0;

        let result = loader.validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_event_subscriptions_accepts_valid_declarations() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.event_subscriptions = Some(vec![
            valid_event_subscription("session-completed"),
            valid_event_subscription("project-changed"),
        ]);

        loader.validate(&manifest).unwrap();
    }

    #[test]
    fn test_validate_event_subscriptions_rejects_empty_or_duplicate_ids() {
        let loader = ExtensionLoader::new();

        let mut manifest = create_valid_manifest("test");
        let mut empty_id = valid_event_subscription("valid-id");
        empty_id.id = "  ".into();
        manifest.contributes.event_subscriptions = Some(vec![empty_id]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Event subscription ID is empty"));

        let mut manifest = create_valid_manifest("test");
        manifest.contributes.event_subscriptions = Some(vec![
            valid_event_subscription("duplicate-id"),
            valid_event_subscription("duplicate-id"),
        ]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Duplicate Event subscription ID"));
    }

    #[test]
    fn test_validate_event_subscriptions_rejects_invalid_topics_and_scope_keys() {
        let loader = ExtensionLoader::new();

        for (topic, expected_message) in [
            ("", "topic is empty"),
            ("session..completed", "dot-separated name segments"),
            ("session.*", "dot-separated name segments"),
            ("session completed", "ASCII topic without whitespace"),
        ] {
            let mut manifest = create_valid_manifest("test");
            let mut subscription = valid_event_subscription("invalid-topic");
            subscription.topic = topic.into();
            manifest.contributes.event_subscriptions = Some(vec![subscription]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains(expected_message),
                "unexpected error: {error}"
            );
        }

        let mut manifest = create_valid_manifest("test");
        let mut subscription = valid_event_subscription("invalid-scope");
        subscription.scope_key = Some(" ".into());
        manifest.contributes.event_subscriptions = Some(vec![subscription]);

        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("scopeKey must be a non-empty value"));
    }

    #[test]
    fn test_validate_event_subscriptions_rejects_invalid_handler_module_paths() {
        let loader = ExtensionLoader::new();

        for module in [
            "",
            "/runtime/events",
            "C:/runtime/events",
            "./runtime/../events",
            "./runtime\\events",
            "./runtime//events",
            "./runtime/./events",
        ] {
            let mut manifest = create_valid_manifest("test");
            let mut subscription = valid_event_subscription("invalid-module");
            subscription.handler.module = module.into();
            manifest.contributes.event_subscriptions = Some(vec![subscription]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains("handler module") || error.contains("path segment"),
                "unexpected error for module '{module}': {error}"
            );
        }
    }

    #[test]
    fn test_validate_event_subscriptions_rejects_invalid_handler_exports() {
        let loader = ExtensionLoader::new();

        for export in ["", " ", "on-session-completed", "handler.name", "1handler"] {
            let mut manifest = create_valid_manifest("test");
            let mut subscription = valid_event_subscription("invalid-export");
            subscription.handler.export = export.into();
            manifest.contributes.event_subscriptions = Some(vec![subscription]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains("handler export"),
                "unexpected error: {error}"
            );
        }
    }

    #[test]
    fn test_validate_trigger_prefix_must_start_with_slash() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.triggers = Some(vec![TriggerRegistration {
            prefix: "pr".into(),
            label: "PR".into(),
            description: "desc".into(),
            icon: None,
            placeholder: None,
            search_module: "./s.js".into(),
            select_module: "./s.js".into(),
            scope: super::super::models::TriggerScope::Global,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must start with '/'"));
    }

    #[test]
    fn test_validate_trigger_prefix_uppercase_rejected() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.triggers = Some(vec![TriggerRegistration {
            prefix: "/PR".into(),
            label: "PR".into(),
            description: "desc".into(),
            icon: None,
            placeholder: None,
            search_module: "./s.js".into(),
            select_module: "./s.js".into(),
            scope: super::super::models::TriggerScope::Global,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("lowercase letters, digits, '-' or '.'"));
    }

    #[test]
    fn test_validate_trigger_valid_prefix() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.triggers = Some(vec![TriggerRegistration {
            prefix: "/pr123".into(),
            label: "PR".into(),
            description: "desc".into(),
            icon: None,
            placeholder: None,
            search_module: "./s.js".into(),
            select_module: "./s.js".into(),
            scope: super::super::models::TriggerScope::Global,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_command_without_action_rejected() {
        let loader = ExtensionLoader::new();

        // 声明式 action 可以通过校验
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.views = Some(vec![super::super::models::ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: Some("rightWorkspace".into()),
            renderer: HOST_PANEL_RENDERER.into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);
        manifest.contributes.commands = Some(vec![CommandRegistration {
            id: "test.cmd".into(),
            label: "Test".into(),
            description: None,
            icon: None,
            category: None,
            when: None,
            action: BuiltinAction::OpenView {
                view_id: "test.view".into(),
            },
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_ok());

        // 未声明 action 的 manifest 在反序列化阶段拒绝加载。
        let mut manifest_json = serde_json::to_value(create_valid_manifest("test2")).unwrap();
        manifest_json["contributes"] = serde_json::json!({
            "commands": [{
                "id": "test.cmd",
                "label": "Test"
            }]
        });

        let result = loader.parse_manifest(&manifest_json.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_duplicate_hook_id_rejected() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.hooks = Some(vec![
            super::super::models::HookRegistration {
                id: "guard".into(),
                name: "Guard".into(),
                phase: super::super::models::HookPhase::PreToolUse,
                priority: None,
                module: "./hooks/guard.js".into(),
                when: None,
                action: Default::default(),
            },
            super::super::models::HookRegistration {
                id: "guard".into(),
                name: "Guard Again".into(),
                phase: super::super::models::HookPhase::PostToolUse,
                priority: None,
                module: "./hooks/guard-again.js".into(),
                when: None,
                action: Default::default(),
            },
        ]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate Hook ID"));
    }

    #[test]
    fn test_validate_menu_references_command() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.menus = Some(vec![super::super::models::MenuRegistration {
            id: "test.menu".into(),
            label: "Test Menu".into(),
            target: super::super::models::menu_targets::TOOLS.to_string(),
            command: "nonexistent.cmd".into(),
            group: None,
            when: None,
            icon: None,
            shortcut: None,
            risk: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent command"));
    }

    #[test]
    fn test_validate_builtin_action_open_view_references() {
        let loader = ExtensionLoader::new();

        // OpenView 引用不存在的 view
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.commands = Some(vec![CommandRegistration {
            id: "test.cmd".into(),
            label: "Test".into(),
            description: None,
            icon: None,
            category: None,
            when: None,
            action: BuiltinAction::OpenView {
                view_id: "nonexistent.view".into(),
            },
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent view"));
    }

    #[test]
    fn test_validate_builtin_action_valid_references() {
        let loader = ExtensionLoader::new();

        // 正确引用 view
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.views = Some(vec![super::super::models::ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: Some("rightWorkspace".into()),
            renderer: HOST_PANEL_RENDERER.into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);
        manifest.contributes.commands = Some(vec![CommandRegistration {
            id: "test.cmd".into(),
            label: "Test".into(),
            description: None,
            icon: None,
            category: None,
            when: None,
            action: BuiltinAction::OpenView {
                view_id: "test.view".into(),
            },
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rejects_dynamic_view_renderer() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.views = Some(vec![super::super::models::ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: Some("rightWorkspace".into()),
            renderer: "extension:unsupported-renderer".into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unsupported renderer"));
    }

    #[test]
    fn test_validate_rejects_unsupported_view_placement() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.views = Some(vec![super::super::models::ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: Some("floatingDock".into()),
            renderer: HOST_PANEL_RENDERER.into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid zone"));
    }

    #[test]
    fn test_validate_rejects_view_without_placement() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.views = Some(vec![super::super::models::ViewRegistration {
            id: "test.view".into(),
            name: "Test View".into(),
            icon: None,
            entry: None,
            zone: None,
            placement: None,
            renderer: HOST_PANEL_RENDERER.into(),
            config: None,
            activation_events: vec![],
            allow_close: None,
            default_visible: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must declare a zone"));
    }

    #[test]
    fn test_validate_toolbar_references_command() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.toolbar_items = Some(vec![ToolbarItemRegistration {
            id: "test.toolbar".into(),
            label: "Test".into(),
            icon: "test".into(),
            command: "nonexistent.cmd".into(),
            position: super::super::models::ToolbarPosition::Main,
            group: None,
            when: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent command"));
    }

    #[test]
    fn test_validate_tray_references_command() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.tray_items = Some(vec![super::super::models::TrayItemRegistration {
            id: "test.tray".into(),
            label: "Test".into(),
            icon: None,
            command: "nonexistent.cmd".into(),
            position: super::super::models::TrayPosition::Bottom,
            when: None,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent command"));
    }

    #[test]
    fn test_validate_keybinding_references_command() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.keybindings = Some(vec![KeybindingRegistration {
            command: "nonexistent.cmd".into(),
            key: "Ctrl+Shift+T".into(),
            when: None,
            scope: super::super::models::KeybindingScope::App,
        }]);

        let result = loader.validate(&manifest);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("non-existent command"));
    }

    #[test]
    fn test_parse_manifest_from_string() {
        let loader = ExtensionLoader::new();

        let json_str = serde_json::json!({
            "id": "com.test.valid",
            "name": "Valid Extension",
            "version": "1.0.0",
            "description": "test",
            "author": "tester",
            "permissions": {
                "filesystem": [],
                "terminal": [],
                "network": [],
                "ipc": [],
                "events": [],
                "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
            },
            "contributes": {
                "triggers": [{
                    "prefix": "/pr",
                    "label": "Pull Request",
                    "description": "GitHub PR",
                    "search_module": "./triggers/pr-search.js",
                    "select_module": "./triggers/pr-select.js",
                    "scope": "Global"
                }]
            }
        })
        .to_string();

        let manifest = loader.parse_manifest(&json_str).unwrap();
        assert_eq!(manifest.id, "com.test.valid");
        assert_eq!(manifest.contributes.triggers.as_ref().unwrap().len(), 1);
        assert_eq!(
            manifest.contributes.triggers.as_ref().unwrap()[0].prefix,
            "/pr"
        );
    }

    fn valid_backend_service(id: &str) -> BackendServiceRegistration {
        BackendServiceRegistration {
            id: id.into(),
            entry: "ExtensionBackend/server".into(),
            transport: super::super::models::BackendTransport::Stdio,
            protocol: super::super::models::BackendProtocol::JsonRpc,
            args: vec![],
            env: Default::default(),
            autostart: false,
        }
    }

    #[test]
    fn test_validate_backend_services_accepts_valid_declarations() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.backend_services = Some(vec![
            valid_backend_service("search"),
            valid_backend_service("language-server"),
        ]);

        loader.validate(&manifest).unwrap();
    }

    #[test]
    fn test_validate_backend_services_rejects_entry_outside_extension_backend() {
        let loader = ExtensionLoader::new();
        for entry in ["bin/server", "ExtensionUI/server", "ExtensionBackend"] {
            let mut manifest = create_valid_manifest("test");
            let mut service = valid_backend_service("search");
            service.entry = entry.into();
            manifest.contributes.backend_services = Some(vec![service]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains("ExtensionBackend directory"),
                "unexpected error for entry '{entry}': {error}"
            );
        }
    }

    #[test]
    fn test_validate_backend_services_rejects_empty_entry() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        let mut service = valid_backend_service("search");
        service.entry = "".into();
        manifest.contributes.backend_services = Some(vec![service]);

        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("entry is empty"));
    }

    #[test]
    fn test_validate_backend_services_rejects_unsafe_entries() {
        let loader = ExtensionLoader::new();
        for entry in [
            "/server",
            "C:/server",
            "ExtensionBackend\\server",
            "ExtensionBackend/../server",
            "ExtensionBackend/./server",
            "ExtensionBackend//server",
            "ExtensionBackend/server:8080",
        ] {
            let mut manifest = create_valid_manifest("test");
            let mut service = valid_backend_service("search");
            service.entry = entry.into();
            manifest.contributes.backend_services = Some(vec![service]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains("relative path") || error.contains("unsafe path segment"),
                "unexpected error for entry '{entry}': {error}"
            );
        }
    }

    #[test]
    fn test_validate_backend_services_rejects_empty_or_duplicate_ids() {
        let loader = ExtensionLoader::new();

        let mut manifest = create_valid_manifest("test");
        let mut empty_id = valid_backend_service("search");
        empty_id.id = "  ".into();
        manifest.contributes.backend_services = Some(vec![empty_id]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Backend service ID is empty"));

        let mut manifest = create_valid_manifest("test");
        let mut whitespace_id = valid_backend_service("search");
        whitespace_id.id = "search server".into();
        manifest.contributes.backend_services = Some(vec![whitespace_id]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("whitespace or control characters"));

        let mut manifest = create_valid_manifest("test");
        manifest.contributes.backend_services = Some(vec![
            valid_backend_service("duplicate-id"),
            valid_backend_service("duplicate-id"),
        ]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Duplicate Backend service ID"));
    }

    fn valid_component(id: &str) -> ComponentRegistration {
        ComponentRegistration {
            id: id.into(),
            entry: "ExtensionUI/scripts/app.component.wasm".into(),
            kind: super::super::models::ComponentKind::Logic,
            run_on: vec![],
            capabilities: Default::default(),
            autostart: false,
        }
    }

    #[test]
    fn test_validate_components_accepts_valid_declarations() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        manifest.contributes.components = Some(vec![
            valid_component("app"),
            ComponentRegistration {
                entry: "ExtensionBackend/logic/worker.component.wasm".into(),
                ..valid_component("worker")
            },
        ]);

        loader.validate(&manifest).unwrap();
    }

    #[test]
    fn test_validate_components_rejects_entry_outside_extension_dirs() {
        let loader = ExtensionLoader::new();
        for entry in ["bin/worker.component.wasm", "ui/worker.component.wasm"] {
            let mut manifest = create_valid_manifest("test");
            let mut component = valid_component("worker");
            component.entry = entry.into();
            manifest.contributes.components = Some(vec![component]);

            let error = loader.validate(&manifest).unwrap_err().to_string();
            assert!(
                error.contains("ExtensionUI or ExtensionBackend directory"),
                "unexpected error for entry '{entry}': {error}"
            );
        }
    }

    #[test]
    fn test_validate_components_rejects_empty_or_duplicate_ids() {
        let loader = ExtensionLoader::new();

        let mut manifest = create_valid_manifest("test");
        let mut empty_id = valid_component("worker");
        empty_id.id = "  ".into();
        manifest.contributes.components = Some(vec![empty_id]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Component ID is empty"));

        let mut manifest = create_valid_manifest("test");
        manifest.contributes.components = Some(vec![
            valid_component("duplicate-id"),
            valid_component("duplicate-id"),
        ]);
        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("Duplicate Component ID"));
    }

    #[test]
    fn test_validate_components_rejects_non_wasm_entry() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        let mut component = valid_component("worker");
        component.entry = "ExtensionBackend/logic/worker.component.js".into();
        manifest.contributes.components = Some(vec![component]);

        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("must end with '.wasm'"));
    }

    #[test]
    fn test_validate_components_rejects_unknown_runon_trigger() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        let mut component = valid_component("worker");
        component.run_on = vec!["view-open".into()];
        manifest.contributes.components = Some(vec![component]);

        let error = loader.validate(&manifest).unwrap_err().to_string();
        assert!(error.contains("unsupported runOn trigger"), "unexpected: {error}");
    }

    #[test]
    fn test_validate_components_accepts_known_runon_triggers() {
        let loader = ExtensionLoader::new();
        let mut manifest = create_valid_manifest("test");
        let mut component = valid_component("worker");
        component.run_on = vec!["activation".into(), "message".into()];
        manifest.contributes.components = Some(vec![component]);

        loader.validate(&manifest).unwrap();
    }
}
