// ── 归属扩展：navis-settings ──
// 迁移目标：extensions/navis-settings/ExtensionBackend/src/

use crate::foundation::config::Config;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::State;

use super::dto::{
    UiEditorSettings, UiExternalEditorConfig, UiLanguageOption, UiLanguageState,
    UiToolPermissionRule,
};
use super::permissions::default_tool_permission_rules;

const BUILTIN_UI_LANGUAGES: [&str; 2] = ["zh-CN", "en-US"];

fn builtin_ui_language_options() -> Vec<UiLanguageOption> {
    vec![
        UiLanguageOption {
            value: "zh-CN".to_string(),
            label: "中文（简体）".to_string(),
        },
        UiLanguageOption {
            value: "en-US".to_string(),
            label: "English".to_string(),
        },
    ]
}

fn normalize_editor_word_wrap(value: Option<&str>) -> String {
    match value {
        Some("off") => "off".to_string(),
        _ => "on".to_string(),
    }
}

fn external_editors_from_config(config: &Config) -> (Vec<UiExternalEditorConfig>, Option<String>) {
    let mut editors = config
        .get("editor.externalEditors")
        .and_then(|value| serde_json::from_value::<Vec<UiExternalEditorConfig>>(value).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|editor| {
            let id = editor.id.trim().to_string();
            let name = editor.name.trim().to_string();
            let path = editor.path.trim().to_string();
            if id.is_empty() || name.is_empty() || path.is_empty() {
                return None;
            }
            Some(UiExternalEditorConfig {
                id,
                name,
                path,
                is_default: editor.is_default,
            })
        })
        .collect::<Vec<_>>();

    let requested_default = config
        .get("editor.defaultExternalEditorId")
        .and_then(|value| value.as_str().map(str::to_string))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            editors
                .iter()
                .find(|editor| editor.is_default)
                .map(|editor| editor.id.clone())
        });

    let default_external_editor_id = requested_default.filter(|default_id| {
        editors
            .iter()
            .any(|editor| editor.id.as_str() == default_id.as_str())
    });

    for editor in &mut editors {
        editor.is_default = default_external_editor_id
            .as_deref()
            .is_some_and(|default_id| default_id == editor.id);
    }

    (editors, default_external_editor_id)
}

pub(crate) fn tool_permission_rules_from_config(_config: &Config) -> Vec<UiToolPermissionRule> {
    default_tool_permission_rules()
}

pub(crate) fn editor_settings_from_config(config: &Config) -> UiEditorSettings {
    let font_size = config
        .get("editor.fontSize")
        .and_then(|value| value.as_u64())
        .map(|value| value.clamp(8, 32) as u32)
        .unwrap_or(14);
    let tab_size = config
        .get("editor.tabSize")
        .and_then(|value| value.as_u64())
        .map(|value| value.clamp(1, 8) as u32)
        .unwrap_or(2);
    let word_wrap_value = config
        .get("editor.wordWrap")
        .and_then(|value| value.as_str().map(str::to_string));
    let word_wrap = normalize_editor_word_wrap(word_wrap_value.as_deref());
    let minimap = config
        .get("editor.minimap")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let format_on_save = config
        .get("editor.formatOnSave")
        .and_then(|value| value.as_bool())
        .unwrap_or(true);
    let (external_editors, default_external_editor_id) = external_editors_from_config(config);
    let tool_permissions = tool_permission_rules_from_config(config);

    UiEditorSettings {
        font_size,
        tab_size,
        word_wrap,
        minimap,
        format_on_save,
        external_editors,
        default_external_editor_id,
        tool_permissions,
    }
}

#[tauri::command]
pub fn ui_get_editor_settings(
    config: State<'_, Arc<Mutex<Config>>>,
) -> Result<UiEditorSettings, String> {
    let config = config.lock().map_err(|error| error.to_string())?;
    Ok(editor_settings_from_config(&config))
}

#[tauri::command]
pub fn ui_save_editor_settings(
    config: State<'_, Arc<Mutex<Config>>>,
    payload: UiEditorSettings,
) -> Result<UiEditorSettings, String> {
    if !(8..=32).contains(&payload.font_size) {
        return Err("Coding Editor font size must be between 8 and 32".to_string());
    }
    if !(1..=8).contains(&payload.tab_size) {
        return Err("Coding Editor tab size must be between 1 and 8".to_string());
    }
    if !matches!(payload.word_wrap.as_str(), "off" | "on") {
        return Err("Coding Editor word wrap must be either 'off' or 'on'".to_string());
    }
    let source_external_editors = payload.external_editors;
    let requested_default_external_editor_id = payload
        .default_external_editor_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            source_external_editors
                .iter()
                .find(|editor| editor.is_default)
                .map(|editor| editor.id.trim().to_string())
        });

    let mut seen_editor_ids = HashSet::new();
    let mut external_editors = Vec::new();
    for editor in source_external_editors {
        let id = editor.id.trim().to_string();
        let name = editor.name.trim().to_string();
        let path = editor.path.trim().to_string();
        if id.is_empty() || name.is_empty() || path.is_empty() {
            return Err(
                "External programming tool id, name and absolute path are required".to_string(),
            );
        }
        if !Path::new(&path).is_absolute() {
            return Err(format!(
                "External programming tool path must be absolute: {}",
                path
            ));
        }
        if !seen_editor_ids.insert(id.clone()) {
            return Err(format!(
                "External programming tool id is duplicated: {}",
                id
            ));
        }
        external_editors.push(UiExternalEditorConfig {
            id,
            name,
            path,
            is_default: false,
        });
    }
    let default_external_editor_id = requested_default_external_editor_id.filter(|default_id| {
        external_editors
            .iter()
            .any(|editor| editor.id.as_str() == default_id.as_str())
    });
    for editor in &mut external_editors {
        editor.is_default = default_external_editor_id
            .as_deref()
            .is_some_and(|default_id| default_id == editor.id);
    }
    let _ = payload.tool_permissions;

    let mut config = config.lock().map_err(|error| error.to_string())?;
    config
        .set("editor.fontSize", json!(payload.font_size))
        .map_err(|error| error.to_string())?;
    config
        .set("editor.tabSize", json!(payload.tab_size))
        .map_err(|error| error.to_string())?;
    config
        .set("editor.wordWrap", json!(payload.word_wrap))
        .map_err(|error| error.to_string())?;
    config
        .set("editor.minimap", json!(payload.minimap))
        .map_err(|error| error.to_string())?;
    config
        .set("editor.formatOnSave", json!(payload.format_on_save))
        .map_err(|error| error.to_string())?;
    config
        .set("editor.externalEditors", json!(external_editors))
        .map_err(|error| error.to_string())?;
    config
        .set(
            "editor.defaultExternalEditorId",
            json!(default_external_editor_id),
        )
        .map_err(|error| error.to_string())?;
    config
        .save_user_config()
        .map_err(|error| error.to_string())?;

    Ok(editor_settings_from_config(&config))
}

use serde_json::json;

#[tauri::command]
pub fn ui_get_language(config: State<'_, Arc<Mutex<Config>>>) -> Result<UiLanguageState, String> {
    let config = config.lock().map_err(|error| error.to_string())?;
    let language = config
        .get_or("ui.language", json!("zh-CN"))
        .as_str()
        .unwrap_or("zh-CN")
        .to_string();

    Ok(UiLanguageState {
        language,
        builtin_languages: builtin_ui_language_options(),
    })
}

#[tauri::command]
pub fn ui_set_language(
    config: State<'_, Arc<Mutex<Config>>>,
    payload: super::LanguagePayload,
) -> Result<UiLanguageState, String> {
    let next_language = payload.language.trim();
    if !BUILTIN_UI_LANGUAGES.contains(&next_language) {
        return Err(format!("Unsupported built-in language: {}", next_language));
    }

    let mut config = config.lock().map_err(|error| error.to_string())?;
    config
        .set("ui.language", json!(next_language))
        .map_err(|error| error.to_string())?;
    config
        .save_user_config()
        .map_err(|error| error.to_string())?;

    Ok(UiLanguageState {
        language: next_language.to_string(),
        builtin_languages: builtin_ui_language_options(),
    })
}

#[tauri::command]
pub fn ui_open_session_external_editor(
    manager: State<'_, std::sync::Arc<crate::domains::session::session::SessionManager>>,
    config: State<'_, Arc<Mutex<Config>>>,
    payload: super::OpenSessionExternalEditorPayload,
) -> Result<(), String> {
    let manager = manager.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let worktree_root = session
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .ok_or_else(|| "当前会话未绑定 worktree，无法用外部编程工具打开".to_string())?;
    let worktree_root_path = std::path::PathBuf::from(worktree_root);
    if !worktree_root_path.exists() {
        return Err(format!("worktree 不存在: {}", worktree_root));
    }

    let settings = {
        let config = config.lock().map_err(|error| error.to_string())?;
        editor_settings_from_config(&config)
    };
    let editor = settings
        .external_editors
        .into_iter()
        .find(|editor| editor.id == payload.editor_id)
        .ok_or_else(|| format!("未找到外部编程工具配置: {}", payload.editor_id))?;
    let editor_path = std::path::PathBuf::from(&editor.path);
    if !editor_path.exists() {
        return Err(format!("外部编程工具不存在: {}", editor.path));
    }

    Command::new(&editor_path)
        .arg(&worktree_root_path)
        .current_dir(&worktree_root_path)
        .spawn()
        .map_err(|error| format!("启动 {} 失败: {}", editor.name, error))?;

    Ok(())
}
