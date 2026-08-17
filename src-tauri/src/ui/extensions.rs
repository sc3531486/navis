use super::dto::{
    UiExtensionConfiguration, UiExtensionConfigurationUpdate, UiExtensionContributionCounts, UiExtensionDiscoveryResult, UiExtensionLocale,
    UiExtensionPointRegistration, UiExtensionScript, UiExtensionState, UiExtensionView, UiRegisteredWorkMode,
    UiWorkModeModelPreferences, UiWorkModeRegistration, UiZone,
};
use super::host_view::ui_extension_view_descriptor;
use crate::extension::host_view::HTML_SANDBOX_RENDERER;
use crate::extension::resource::resolve_extension_manifest_entry;
use super::ExtensionViewPayload;
use crate::extension::lifecycle::ExtensionRuntimeProjection;
use crate::extension::models::{
    ExtensionStatus, NetworkPolicy, WorkModeModelPreferences, WorkModeRegistration as WorkModeModel,
};
use crate::extension::store::RegisteredWorkMode;
use crate::extension::{ExtensionInstaller, ExtensionLifecycle, ExtensionStore};
use crate::foundation::config::Config;
use crate::foundation::status::StatusClassify;
use crate::security::sandbox::permission::{OperationRequest, OperationType};
// use [REMOVED: MCP reference]
use crate::ui::extension_storage::{
    clear_extension_ephemeral, ExtensionEphemeralStorage, ExtensionStorage,
};
use serde_json::Value;
use std::sync::Mutex;
use tauri::Emitter;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub(crate) fn contribution_counts(
    projection: Option<&ExtensionRuntimeProjection>,
) -> UiExtensionContributionCounts {
    let counts = projection
        .map(|projection| &projection.contribution_counts)
        .cloned()
        .unwrap_or_default();
    UiExtensionContributionCounts {
        work_modes: counts.work_modes,
        views: counts.views,
        menus: counts.menus,
        commands: counts.commands,
        keybindings: counts.keybindings,
        triggers: counts.triggers,
        mcp_servers: counts.mcp_servers,
        providers: counts.providers,
        zones: projection
            .map(|projection| projection.zones.len())
            .unwrap_or_default(),
        scripts: projection
            .map(|projection| projection.scripts.len())
            .unwrap_or_default(),
        toolbar_items: projection
            .map(|projection| projection.toolbar_items.len())
            .unwrap_or_default(),
        statusbar_items: projection
            .map(|projection| projection.statusbar_items.len())
            .unwrap_or_default(),
        inline_extensions: projection
            .map(|projection| projection.inline_extensions.len())
            .unwrap_or_default(),
        configuration: projection
            .and_then(|projection| projection.configuration.as_ref())
            .map_or(0, |_| 1),
    }
}

fn ui_extension(
    state: crate::extension::models::ExtensionState,
    projection: Option<&ExtensionRuntimeProjection>,
) -> UiExtensionState {
    let contribution_counts = contribution_counts(projection);
    let status_presentation = state.status.status_presentation();
    UiExtensionState {
        id: state.id,
        status: state.status.to_string(),
        status_presentation,
        name: state.manifest.name,
        version: state.manifest.version,
        description: state.manifest.description,
        author: state.manifest.author,
        install_path: state.install_path.display().to_string(),
        installed_at: state.installed_at.to_rfc3339(),
        enabled_at: state.enabled_at.map(|enabled_at| enabled_at.to_rfc3339()),
        error: state.error,
        permissions: state.manifest.permissions,
        contribution_counts,
        provides: state
            .manifest
            .contributes
            .provides
            .clone()
            .unwrap_or_default(),
    }
}

fn runtime_projection(
    lifecycle: &ExtensionLifecycle,
    extension_id: &str,
) -> Option<ExtensionRuntimeProjection> {
    match lifecycle.runtime_projection(extension_id) {
        Ok(projection) => projection,
        Err(error) => {
            tracing::warn!(
                extension_id = %extension_id,
                error = %error,
                "Failed to read Extension runtime projection"
            );
            None
        }
    }
}

fn sorted_extensions(
    extension_store: &ExtensionStore,
    lifecycle: &ExtensionLifecycle,
) -> Vec<UiExtensionState> {
    let mut extensions = extension_store.list();
    extensions.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    extensions
        .into_iter()
        .map(|state| {
            let projection = runtime_projection(lifecycle, &state.id);
            ui_extension(state, projection.as_ref())
        })
        .collect()
}

fn ui_model_preferences(preferences: WorkModeModelPreferences) -> UiWorkModeModelPreferences {
    UiWorkModeModelPreferences {
        temperature: preferences.temperature,
        max_tokens: preferences.max_tokens,
        extended_thinking: preferences.extended_thinking,
        language_quality_emphasis: preferences.language_quality_emphasis,
    }
}

fn ui_work_mode(mode: WorkModeModel) -> UiWorkModeRegistration {
    UiWorkModeRegistration {
        id: mode.id,
        name: mode.name,
        description: mode.description,
        icon: mode.icon,
        role: mode.role,
        available_tools: mode.available_tools,
        skills: mode.skills,
        commands: mode.commands,
        context_policy: mode.context_policy,
        behavior_rules: mode.behavior_rules,
        entry_view: mode.entry_view,
        default_views: mode.default_views,
        default_model: mode.default_model,
        model_preferences: mode.model_preferences.map(ui_model_preferences),
        capabilities: mode.capabilities,
    }
}

fn ui_registered_work_mode(work_mode: RegisteredWorkMode) -> UiRegisteredWorkMode {
    UiRegisteredWorkMode {
        extension_id: work_mode.extension_id,
        extension_name: work_mode.extension_name,
        mode_id: work_mode.mode_id,
        runtime_id: work_mode.runtime_id,
        mode: ui_work_mode(work_mode.mode),
    }
}

#[tauri::command]
pub fn ui_list_extensions(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiExtensionState> {
    sorted_extensions(extension_store.inner().as_ref(), lifecycle.inner().as_ref())
}

fn ui_extension_view(
    state: crate::extension::models::ExtensionState,
    projection: &ExtensionRuntimeProjection,
    view: crate::extension::models::ViewRegistration,
) -> Result<UiExtensionView, String> {
    let view = ui_extension_view_descriptor(&view, &state.install_path)?;
    let contribution_counts = contribution_counts(Some(projection));

    Ok(UiExtensionView {
        extension_id: state.id,
        extension_name: state.manifest.name,
        extension_description: state.manifest.description,
        view,
        contribution_counts,
    })
}

fn project_extension_views(
    extensions: Vec<(
        crate::extension::models::ExtensionState,
        ExtensionRuntimeProjection,
    )>,
) -> Vec<UiExtensionView> {
    let mut views = extensions
        .into_iter()
        .flat_map(|(extension, projection)| {
            let views = projection.views.clone();
            views.into_iter().filter_map(move |view| {
                ui_extension_view(extension.clone(), &projection, view).ok()
            })
        })
        .collect::<Vec<_>>();

    views.sort_by(|left, right| {
        left.extension_name
            .to_lowercase()
            .cmp(&right.extension_name.to_lowercase())
            .then_with(|| left.extension_id.cmp(&right.extension_id))
            .then_with(|| {
                left.view
                    .name
                    .to_lowercase()
                    .cmp(&right.view.name.to_lowercase())
            })
            .then_with(|| left.view.view_id.cmp(&right.view.view_id))
    });
    views
}

#[tauri::command]
pub fn ui_list_extension_views(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiExtensionView> {
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    let extensions = extension_store
        .inner()
        .as_ref()
        .list()
        .into_iter()
        .filter(|extension| extension.status == ExtensionStatus::Enabled)
        .filter_map(|extension| {
            projections
                .get(&extension.id)
                .cloned()
                .map(|projection| (extension, projection))
        })
        .collect();
    project_extension_views(extensions)
}

#[tauri::command]
pub fn ui_get_extension_view(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    payload: ExtensionViewPayload,
) -> Result<UiExtensionView, String> {
    let extension = extension_store
        .inner()
        .as_ref()
        .get(&payload.extension_id)
        .ok_or_else(|| format!("Extension '{}' was not found", payload.extension_id))?;

    if extension.status != ExtensionStatus::Enabled {
        return Err(format!(
            "Extension '{}' is not enabled",
            payload.extension_id
        ));
    }

    let projection = runtime_projection(lifecycle.inner().as_ref(), &payload.extension_id)
        .ok_or_else(|| {
            format!(
                "Extension '{}' has no registered UI projection",
                payload.extension_id
            )
        })?;
    let view = projection
        .views
        .iter()
        .find(|view| view.id == payload.view_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "Extension '{}' does not have registered view '{}'",
                payload.extension_id, payload.view_id
            )
        })?;

    ui_extension_view(extension, &projection, view)
}

/// 解析扩展生效的 network 策略：优先 contributes.network，其次 capabilities.network，
/// 均未声明时 fail-closed 为 `None`。
fn network_policy_for_state(state: &crate::extension::models::ExtensionState) -> NetworkPolicy {
    state
        .manifest
        .contributes
        .network
        .clone()
        .or_else(|| {
            state
                .manifest
                .contributes
                .capabilities
                .as_ref()
                .and_then(|caps| caps.network.clone())
        })
        .unwrap_or(NetworkPolicy::None)
}

/// 按扩展 network 策略生成 iframe CSP（设计 34 §2.6 纵深防御）。
///
/// 核心拦截在宿主网络代理层；CSP 只防止扩展通过 img/font/connect 等被动加载
/// 绕过主动拦截。srcdoc 的 origin 是 opaque，宿主垫片以内联 `<script>` 注入，
/// 因此 `script-src 'unsafe-inline'` / `style-src 'unsafe-inline'` 是沙箱内自约束
/// 而非安全退化（iframe 本就 allow-scripts）。
fn csp_for_network(policy: &NetworkPolicy) -> String {
    match policy {
        NetworkPolicy::None => {
            "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; \
             img-src 'none'; connect-src 'none'; font-src 'none'; frame-src 'none';"
                .to_string()
        }
        NetworkPolicy::Allowlist { hosts } => {
            // 白名单 hosts 生成 img/font/connect 域名源；allow_subdomains 加 `*.host` 前缀通配。
            let sources: Vec<String> = hosts
                .iter()
                .flat_map(|host| {
                    let base = host.host.clone();
                    let mut entries = vec![format!("https://{base}")];
                    if host.allow_subdomains {
                        entries.push(format!("https://*.{base}"));
                    }
                    entries
                })
                .collect();
            let sources = if sources.is_empty() {
                "'none'".to_string()
            } else {
                sources.join(" ")
            };
            format!(
                "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; \
                 img-src {sources}; connect-src {sources}; font-src {sources}; frame-src 'none';"
            )
        }
        NetworkPolicy::Proxy => {
            // fetch 走宿主代理而非 iframe 直连，connect-src 保守留空（default-src 'none' 兜底）。
            "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; \
             img-src 'none'; connect-src 'none'; font-src 'none'; frame-src 'none';"
                .to_string()
        }
    }
}

/// 在 HTML `<head>` 注入 CSP meta 标签；无 head 时前置兜底。
fn inject_csp(html: &str, csp: &str) -> String {
    let meta = format!(r#"<meta http-equiv="Content-Security-Policy" content="{csp}" />"#);
    let lower = html.to_ascii_lowercase();
    if let Some(head) = lower.find("<head") {
        if let Some(after) = html[head..].find('>') {
            let insert_at = head + after + 1;
            return format!("{}{}{}", &html[..insert_at], meta, &html[insert_at..]);
        }
    }
    format!("{meta}{html}")
}

/// 组合：按扩展 network 策略给 entry HTML 注入 CSP。
fn inject_entry_csp(html: &str, policy: &NetworkPolicy) -> String {
    inject_csp(html, &csp_for_network(policy))
}

/// 宿主渲染命令：读取 html:sandbox 视图的入口 HTML 文本。
///
/// 这是宿主渲染管线的一部分（宿主读取自己投影的扩展静态资源来渲染 iframe），
/// 不属于扩展运行时能力桥。流程：Enabled 校验 → 视图须为 html:sandbox 且有 entry →
/// `resolve_extension_manifest_entry` 归一化到扩展安装目录 → Sandbox FileRead 门禁（host actor）
/// → `spawn_blocking` 读取文本 → 按扩展 network 策略注入 CSP meta（纵深防御）。
#[tauri::command]
pub async fn ui_read_extension_entry_html(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    mcp: State<'_, Arc<MCP>>,
    payload: ExtensionViewPayload,
) -> Result<String, String> {
    let extension = extension_store
        .inner()
        .as_ref()
        .get(&payload.extension_id)
        .ok_or_else(|| format!("Extension '{}' was not found", payload.extension_id))?;

    if extension.status != ExtensionStatus::Enabled {
        return Err(format!(
            "Extension '{}' is not enabled",
            payload.extension_id
        ));
    }

    let projection = runtime_projection(lifecycle.inner().as_ref(), &payload.extension_id)
        .ok_or_else(|| {
            format!(
                "Extension '{}' has no registered UI projection",
                payload.extension_id
            )
        })?;
    let view = projection
        .views
        .iter()
        .find(|view| view.id == payload.view_id)
        .ok_or_else(|| {
            format!(
                "Extension '{}' does not have registered view '{}'",
                payload.extension_id, payload.view_id
            )
        })?;

    if view.renderer != HTML_SANDBOX_RENDERER {
        return Err(format!(
            "View '{}' is not an html:sandbox view",
            payload.view_id
        ));
    }
    let entry = view
        .entry
        .as_deref()
        .ok_or_else(|| format!("View '{}' is missing entry", payload.view_id))?;

    let resource =
        resolve_extension_manifest_entry(&extension.install_path, entry).map_err(|error| {
            format!(
                "Failed to resolve extension resource entry '{}': {error}",
                entry
            )
        })?;

    let display = resource.display().to_string();
    let request = OperationRequest::new(
        OperationType::FileRead,
        display.clone(),
        "host".to_string(),
    );
    let sandbox = mcp.sandbox();
    let result = sandbox
        .check(&request)
        .map_err(|error| format!("Sandbox check failed: {error}"))?;
    if !result.allowed || result.require_confirm {
        return Err(format!(
            "Host entry read denied by sandbox: {}",
            result.reason.as_deref().unwrap_or("denied")
        ));
    }

    let read_path = resource.clone();
    let html = tokio::task::spawn_blocking(move || std::fs::read(&read_path))
        .await
        .map_err(|join| format!("Failed to join entry read task: {join}"))?
        .map_err(|error| format!("Failed to read '{}': {error}", display))?;
    let html = String::from_utf8(html)
        .map_err(|error| format!("Entry '{}' is not valid UTF-8: {error}", display))?;
    // 按扩展 network 策略注入 CSP meta，防止 img/font/connect 被动加载绕过宿主拦截。
    Ok(inject_entry_csp(&html, &network_policy_for_state(&extension)))
}

#[tauri::command]
pub fn ui_set_extension_enabled(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,
    payload: super::ExtensionEnabledPayload,
) -> Result<Vec<UiExtensionState>, String> {
    let lifecycle = lifecycle.inner().as_ref();
    if payload.enabled {
        lifecycle
            .enable(&payload.extension_id)
            .map_err(|error| error.to_string())?;
    } else {
        lifecycle
            .disable(&payload.extension_id)
            .map_err(|error| error.to_string())?;
        // 禁用后清理扩展 ephemeral 存储（`extension:{id}:` 前缀）。
        clear_extension_ephemeral(ephemeral.inner().as_ref(), &payload.extension_id)
            .map_err(|error| error.to_string())?;
    }

    Ok(sorted_extensions(
        extension_store.inner().as_ref(),
        lifecycle,
    ))
}

#[tauri::command]
pub fn ui_install_extension(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    installer: State<'_, Arc<ExtensionInstaller>>,
    payload: super::InstallExtensionPayload,
) -> Result<Vec<UiExtensionState>, String> {
    let source_path = PathBuf::from(payload.source_path);
    installer
        .install(&source_path)
        .map_err(|error| error.to_string())?;
    Ok(sorted_extensions(
        extension_store.inner().as_ref(),
        lifecycle.inner().as_ref(),
    ))
}

#[tauri::command]
pub fn ui_uninstall_extension(
    extension_store: State<'_, Arc<ExtensionStore>>,
    installer: State<'_, Arc<ExtensionInstaller>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    ephemeral: State<'_, Arc<ExtensionEphemeralStorage>>,
    payload: super::ExtensionIdPayload,
) -> Result<Vec<UiExtensionState>, String> {
    lifecycle
        .prepare_uninstall(&payload.extension_id)
        .map_err(|error| error.to_string())?;
    // 卸载前清理扩展 ephemeral 存储（`extension:{id}:` 前缀）。
    clear_extension_ephemeral(ephemeral.inner().as_ref(), &payload.extension_id)
        .map_err(|error| error.to_string())?;
    installer
        .uninstall(&payload.extension_id)
        .map_err(|error| error.to_string())?;
    // 卸载后删除该扩展的文件存储目录（`{extensions_dir}/{id}/storage`）。
    // 设计（35 C0-5）：扩展数据自包含，卸载删目录即干净；目录不存在时幂等返回。
    let extension_storage = ExtensionStorage::new(installer.extensions_dir().to_path_buf());
    extension_storage
        .clear_extension(&payload.extension_id)
        .map_err(|error| error.to_string())?;
    Ok(sorted_extensions(
        extension_store.inner().as_ref(),
        lifecycle.inner().as_ref(),
    ))
}

#[tauri::command]
pub fn ui_list_custom_work_modes(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiRegisteredWorkMode> {
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    let mut modes = extension_store
        .inner()
        .as_ref()
        .list()
        .into_iter()
        .filter(|state| state.status == ExtensionStatus::Enabled)
        .flat_map(|state| {
            let extension_id = state.id.clone();
            let extension_name = state.manifest.name.clone();
            projections
                .get(&extension_id)
                .into_iter()
                .flat_map(move |projection| {
                    projection.work_modes.clone().into_iter().map({
                        let extension_id = extension_id.clone();
                        let extension_name = extension_name.clone();
                        move |mode| {
                            let mode_id = mode.id.clone();
                            RegisteredWorkMode {
                                extension_id: extension_id.clone(),
                                extension_name: extension_name.clone(),
                                runtime_id: format!("{}/{}", extension_id, mode_id),
                                mode_id,
                                mode,
                            }
                        }
                    })
                })
        })
        .collect::<Vec<_>>();
    modes.sort_by(|a, b| {
        let a_name = a.mode.name.as_ref().unwrap_or(&a.extension_name);
        let b_name = b.mode.name.as_ref().unwrap_or(&b.extension_name);
        a_name
            .to_lowercase()
            .cmp(&b_name.to_lowercase())
            .then_with(|| a.runtime_id.cmp(&b.runtime_id))
    });
    modes.into_iter().map(ui_registered_work_mode).collect()
}

#[tauri::command]
pub fn ui_list_zones(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiZone> {
    let mut zones = vec![
        UiZone { id: "rightWorkspace".into(), name: "Right workspace".into(), kind: "builtin".into(), extension_id: None, anchor_parent: None, anchor_position: None, available: true },
        UiZone { id: "chatAside".into(), name: "Chat aside".into(), kind: "builtin".into(), extension_id: None, anchor_parent: None, anchor_position: None, available: true },
        UiZone { id: "bottomDrawer".into(), name: "Bottom drawer".into(), kind: "builtin".into(), extension_id: None, anchor_parent: None, anchor_position: None, available: true },
        UiZone { id: "settingsSection".into(), name: "Settings section".into(), kind: "builtin".into(), extension_id: None, anchor_parent: None, anchor_position: None, available: true },
        UiZone { id: "dialog".into(), name: "Dialog".into(), kind: "builtin".into(), extension_id: None, anchor_parent: None, anchor_position: None, available: true },
    ];
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    for extension in extension_store.inner().as_ref().list() {
        if extension.status != ExtensionStatus::Enabled { continue; }
        let Some(projection) = projections.get(&extension.id) else { continue; };
        for zone in &projection.zones {
            zones.push(UiZone {
                id: format!("{}:{}", extension.id, zone.id),
                name: zone.name.clone(),
                kind: "extension".into(),
                extension_id: Some(extension.id.clone()),
                anchor_parent: Some(zone.anchor.parent.clone()),
                anchor_position: zone.anchor.position.clone(),
                available: true,
            });
        }
    }
    zones
}

#[tauri::command]
pub fn ui_list_extension_scripts(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiExtensionScript> {
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    extension_store.inner().as_ref().list().into_iter()
        .filter(|extension| extension.status == ExtensionStatus::Enabled)
        .flat_map(|extension| {
            let extension_id = extension.id.clone();
            projections.get(&extension_id).cloned().into_iter().flat_map(move |projection| {
                let extension_id = extension_id.clone();
                let install_path = extension.install_path.clone();
                projection.scripts.into_iter().map(move |script| {
                    let resource_path = crate::ui::host_view::extension_resource_path(&install_path, &script.entry)
                        .ok()
                        .map(|path| path.display().to_string());
                    UiExtensionScript {
                        extension_id: extension_id.clone(),
                        script_id: script.id,
                        entry: script.entry,
                        resource_path,
                        run_on: script.run_on.unwrap_or_default(),
                    }
                })
            })
        })
        .collect()
}

#[tauri::command]
pub fn ui_list_extension_locales(
    extension_store: State<'_, Arc<ExtensionStore>>,
) -> Vec<UiExtensionLocale> {
    extension_store.inner().as_ref().list().into_iter()
        .filter(|extension| extension.status == ExtensionStatus::Enabled)
        .flat_map(|extension| {
            extension.manifest.contributes.i18n.clone().unwrap_or_default().into_iter().map(move |locale| {
                let resource_path = resolve_extension_manifest_entry(&extension.install_path, &locale.entry)
                    .ok()
                    .map(|path| path.display().to_string());
                UiExtensionLocale {
                    extension_id: extension.id.clone(),
                    lang: locale.lang,
                    entry: locale.entry,
                    resource_path,
                }
            })
        })
        .collect()
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDiscoveryQuery {
    pub capability: Option<String>,
    pub provides: Option<String>,
}

#[tauri::command]
pub fn ui_extension_discovery_query(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    query: ExtensionDiscoveryQuery,
) -> Vec<UiExtensionDiscoveryResult> {
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    extension_store.inner().as_ref().list().into_iter()
        .filter(|extension| extension.status == ExtensionStatus::Enabled)
        .filter_map(|extension| {
            let projection = projections.get(&extension.id)?;
            let provides = extension.manifest.contributes.provides.clone()
                .unwrap_or_else(|| extension.manifest.contributes.capabilities.as_ref().map(|caps| caps.provides.clone()).unwrap_or_default());
            if let Some(required) = query.provides.as_deref() {
                if !provides.iter().any(|item| item == required) { return None; }
            }
            if let Some(capability) = query.capability.as_deref() {
                let has_capability = match capability {
                    "view" => !projection.views.is_empty(),
                    "command" => projection.contribution_counts.commands > 0,
                    "script" => !projection.scripts.is_empty(),
                    "storage" => extension.manifest.contributes.storage.is_some(),
                    "network" => extension.manifest.contributes.network.is_some() || extension.manifest.contributes.capabilities.as_ref().and_then(|caps| caps.network.as_ref()).is_some(),
                    _ => false,
                };
                if !has_capability { return None; }
            }
            Some(UiExtensionDiscoveryResult {
                extension_id: extension.id,
                extension_name: extension.manifest.name,
                provides,
                views: projection.views.iter().map(|view| view.id.clone()).collect(),
                commands: extension.manifest.contributes.commands.unwrap_or_default().into_iter().map(|cmd| cmd.id).collect(),
                scripts: projection.scripts.iter().map(|script| script.id.clone()).collect(),
            })
        })
        .collect()
}

#[tauri::command]
pub fn ui_list_extension_points(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
) -> Vec<UiExtensionPointRegistration> {
    let projections = lifecycle.inner().runtime_projections().unwrap_or_default();
    let mut points = Vec::new();
    for extension in extension_store.inner().as_ref().list() {
        if extension.status != ExtensionStatus::Enabled { continue; }
        let Some(projection) = projections.get(&extension.id) else { continue; };
        for item in &projection.toolbar_items {
            points.push(UiExtensionPointRegistration { extension_id: extension.id.clone(), kind: "toolbar".into(), id: item.id.clone(), label: Some(item.label.clone()), command: Some(item.command.clone()), target: Some(format!("{:?}", item.position)), group: item.group.clone(), when: item.when.clone(), data: serde_json::to_value(item).unwrap_or_default() });
        }
        for item in &projection.statusbar_items {
            points.push(UiExtensionPointRegistration { extension_id: extension.id.clone(), kind: "statusbar".into(), id: item.id.clone(), label: Some(item.label.clone()), command: item.command.clone(), target: Some(format!("{:?}", item.position)), group: None, when: item.when.clone(), data: serde_json::to_value(item).unwrap_or_default() });
        }
        for item in &projection.inline_extensions {
            points.push(UiExtensionPointRegistration { extension_id: extension.id.clone(), kind: "inline".into(), id: item.id.clone(), label: Some(item.name.clone()), command: None, target: Some(format!("{:?}", item.target)), group: None, when: item.when.clone(), data: serde_json::to_value(item).unwrap_or_default() });
        }
        if let Some(configuration) = &projection.configuration {
            points.push(UiExtensionPointRegistration { extension_id: extension.id.clone(), kind: "configuration".into(), id: format!("{}:configuration", extension.id), label: Some(extension.manifest.name.clone()), command: None, target: Some("settingsSection".into()), group: None, when: None, data: configuration.clone() });
        }
    }
    points
}

fn extension_configuration_schema(
    lifecycle: &ExtensionLifecycle,
    extension_id: &str,
) -> Result<Value, String> {
    let projection = runtime_projection(lifecycle, extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' runtime projection is unavailable"))?;
    projection
        .configuration
        .ok_or_else(|| format!("Extension '{extension_id}' did not declare configuration"))
}

fn schema_default(schema: &Value) -> Value {
    if let Some(default) = schema.get("default") {
        return default.clone();
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let mut object = serde_json::Map::new();
            if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                for (key, property) in properties {
                    let value = schema_default(property);
                    if !value.is_null() {
                        object.insert(key.clone(), value);
                    }
                }
            }
            Value::Object(object)
        }
        Some("array") => Value::Array(Vec::new()),
        _ => Value::Null,
    }
}

fn validate_configuration_value(schema: &Value, value: &Value, path: &str, depth: usize) -> Result<(), String> {
    if depth > 12 {
        return Err(format!("Configuration value is too deeply nested at {path}"));
    }
    if serde_json::to_vec(value).map_err(|error| error.to_string())?.len() > 64 * 1024 {
        return Err("Extension configuration exceeds the 64 KiB limit".to_string());
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
        if !options.iter().any(|option| option == value) {
            return Err(format!("Configuration value at {path} is not in the declared enum"));
        }
    }
    if let Some(expected) = schema.get("type").and_then(Value::as_str) {
        let valid = match expected {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        };
        if !valid {
            return Err(format!("Configuration value at {path} must have type {expected}"));
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    return Err(format!("Missing required configuration property {path}.{key}"));
                }
            }
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, child) in properties {
                if let Some(child_value) = object.get(key) {
                    validate_configuration_value(child, child_value, &format!("{path}.{key}"), depth + 1)?;
                }
            }
        }
    }
    if let Some(items) = schema.get("items") {
        if let Some(array) = value.as_array() {
            for (index, child) in array.iter().enumerate() {
                validate_configuration_value(items, child, &format!("{path}[{index}]"), depth + 1)?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn ui_get_extension_config(
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    config: State<'_, Arc<Mutex<Config>>>,
    extension_id: String,
) -> Result<UiExtensionConfiguration, String> {
    let state = extension_store
        .inner()
        .as_ref()
        .get(&extension_id)
        .ok_or_else(|| format!("Extension '{extension_id}' is not installed"))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{extension_id}' is not enabled"));
    }
    let schema = extension_configuration_schema(lifecycle.inner().as_ref(), &extension_id)?;
    let key = format!("extensions.{extension_id}.configuration");
    let value = config
        .lock()
        .map_err(|error| error.to_string())?
        .get(&key)
        .unwrap_or_else(|| schema_default(&schema));
    validate_configuration_value(&schema, &value, "$", 0)?;
    Ok(UiExtensionConfiguration { extension_id, schema, value })
}

#[tauri::command]
pub fn ui_set_extension_config(
    app: tauri::AppHandle,
    extension_store: State<'_, Arc<ExtensionStore>>,
    lifecycle: State<'_, Arc<ExtensionLifecycle>>,
    config: State<'_, Arc<Mutex<Config>>>,
    update: UiExtensionConfigurationUpdate,
) -> Result<UiExtensionConfiguration, String> {
    let state = extension_store
        .inner()
        .as_ref()
        .get(&update.extension_id)
        .ok_or_else(|| format!("Extension '{}' is not installed", update.extension_id))?;
    if state.status != ExtensionStatus::Enabled {
        return Err(format!("Extension '{}' is not enabled", update.extension_id));
    }
    let schema = extension_configuration_schema(lifecycle.inner().as_ref(), &update.extension_id)?;
    validate_configuration_value(&schema, &update.value, "$", 0)?;
    let key = format!("extensions.{}.configuration", update.extension_id);
    config
        .lock()
        .map_err(|error| error.to_string())?
        .set(&key, update.value.clone())
        .map_err(|error| error.to_string())?;
    config
        .lock()
        .map_err(|error| error.to_string())?
        .save_user_config()
        .map_err(|error| error.to_string())?;
    let response = UiExtensionConfiguration {
        extension_id: update.extension_id.clone(),
        schema,
        value: update.value,
    };
    app.emit("extension.config.updated", &response)
        .map_err(|error| error.to_string())?;
    Ok(response)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::host_view::{HOST_PANEL_RENDERER, HTML_SANDBOX_RENDERER};
    use crate::extension::lifecycle::ExtensionRuntimeProjection;
    use crate::extension::models::{
        ExtensionContributes, ExtensionManifest, ExtensionPermissions, ExtensionState,
        ViewRegistration,
    };
    use chrono::Utc;
    use std::fs;

    fn test_view(
        id: &str,
        name: &str,
        renderer: &str,
        placement: &str,
        entry: Option<&str>,
    ) -> ViewRegistration {
        ViewRegistration {
            id: id.to_string(),
            name: name.to_string(),
            icon: None,
            entry: entry.map(str::to_string),
            zone: Some(placement.to_string()),
            placement: None,
            renderer: renderer.to_string(),
            config: None,
            activation_events: Vec::new(),
            allow_close: None,
            default_visible: None,
        }
    }

    fn test_extension(
        id: &str,
        name: &str,
        status: ExtensionStatus,
        install_path: &std::path::Path,
        views: Vec<ViewRegistration>,
    ) -> (ExtensionState, ExtensionRuntimeProjection) {
        let projection = ExtensionRuntimeProjection {
            views: views.clone(),
            ..ExtensionRuntimeProjection::default()
        };
        let state = ExtensionState {
            id: id.to_string(),
            status,
            manifest: ExtensionManifest {
                id: id.to_string(),
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: format!("{name} description"),
                author: "test".to_string(),
                permissions: ExtensionPermissions::default(),
                contributes: ExtensionContributes {
                    views: Some(views),
                    ..ExtensionContributes::default()
                },
            },
            install_path: install_path.to_path_buf(),
            installed_at: Utc::now(),
            enabled_at: None,
            error: None,
        };
        (state, projection)
    }

    #[test]
    fn projects_enabled_views_fail_closed_and_in_stable_order() {
        let alpha_root = tempfile::tempdir().unwrap();
        fs::create_dir_all(alpha_root.path().join("ExtensionUI")).unwrap();
        fs::write(alpha_root.path().join("ExtensionUI/index.html"), "<main />").unwrap();

        let disabled_root = tempfile::tempdir().unwrap();
        let zeta_root = tempfile::tempdir().unwrap();

        let views = project_extension_views(vec![
            test_extension(
                "ext.zeta",
                "Zeta",
                ExtensionStatus::Enabled,
                zeta_root.path(),
                vec![test_view(
                    "zeta.view",
                    "Zeta View",
                    HOST_PANEL_RENDERER,
                    "rightWorkspace",
                    None,
                )],
            ),
            test_extension(
                "ext.alpha",
                "Alpha",
                ExtensionStatus::Enabled,
                alpha_root.path(),
                vec![
                    test_view(
                        "alpha.html",
                        "HTML View",
                        HTML_SANDBOX_RENDERER,
                        "chatAside",
                        Some("ExtensionUI/index.html"),
                    ),
                    test_view(
                        "alpha.host",
                        "Host View",
                        HOST_PANEL_RENDERER,
                        "rightWorkspace",
                        None,
                    ),
                    test_view(
                        "alpha.renderer",
                        "Invalid Renderer",
                        "extension:custom",
                        "rightWorkspace",
                        None,
                    ),
                    test_view(
                        "alpha.placement",
                        "Invalid Placement",
                        HOST_PANEL_RENDERER,
                        "floatingDock",
                        None,
                    ),
                    test_view(
                        "alpha.entry",
                        "Missing HTML",
                        HTML_SANDBOX_RENDERER,
                        "bottomDrawer",
                        Some("ExtensionUI/missing.html"),
                    ),
                ],
            ),
            test_extension(
                "ext.disabled",
                "Aardvark",
                ExtensionStatus::Disabled,
                disabled_root.path(),
                vec![test_view(
                    "disabled.view",
                    "Disabled View",
                    "extension:custom",
                    "floatingDock",
                    None,
                )],
            ),
        ]);

        assert_eq!(
            views
                .iter()
                .map(|view| view.view.view_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.host", "alpha.html", "zeta.view"]
        );
        assert_eq!(views[0].extension_id, "ext.alpha");
        assert_eq!(views[1].view.zone, "chatAside");
        let resource_path = views[1].view.resource_path.as_deref().unwrap();
        assert!(
            std::path::Path::new(resource_path)
                .ends_with(std::path::Path::new("ExtensionUI").join("index.html")),
            "unexpected resource path: {resource_path}"
        );
    }

    #[test]
    fn entry_html_gets_csp_meta_from_allowlist_network_manifest() {
        let root = tempfile::tempdir().unwrap();
        let mut state = test_extension(
            "ext.csp",
            "CSP",
            ExtensionStatus::Enabled,
            root.path(),
            vec![],
        )
        .0;
        state.manifest.contributes.network = Some(NetworkPolicy::Allowlist {
            hosts: vec![crate::extension::models::NetworkHost {
                host: "api.example.com".into(),
                allow_subdomains: true,
                protocols: vec!["https".into()],
            }],
        });

        let html = inject_entry_csp(
            "<html><head><title>hi</title></head><body></body></html>",
            &network_policy_for_state(&state),
        );

        assert!(html.contains(r#"<meta http-equiv="Content-Security-Policy""#));
        assert!(html.contains("script-src 'unsafe-inline'"));
        assert!(html.contains("connect-src https://api.example.com https://*.api.example.com"));
        assert!(html.contains("img-src https://api.example.com"));
    }

    #[test]
    fn csp_for_none_network_blocks_all_passive_directives() {
        let csp = csp_for_network(&NetworkPolicy::None);
        assert!(csp.contains("script-src 'unsafe-inline'"));
        assert!(csp.contains("style-src 'unsafe-inline'"));
        assert!(csp.contains("img-src 'none'"));
        assert!(csp.contains("connect-src 'none'"));
        assert!(csp.contains("font-src 'none'"));
        assert!(csp.contains("frame-src 'none'"));
    }

    #[test]
    fn csp_injection_falls_back_when_html_has_no_head() {
        let html = inject_entry_csp("<html><body>plain</body></html>", &NetworkPolicy::None);
        assert!(html.starts_with(r#"<meta http-equiv="Content-Security-Policy""#));
    }
}


