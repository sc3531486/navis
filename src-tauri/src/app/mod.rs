//! Tauri 应用启动壳
//!
//! 框架层：Tauri bootstrap + 扩展加载 + 状态装配
//! 业务代码通过扩展机制加载，不在这里硬编码

pub mod infra;

use crate::app::infra::Storage;
use crate::extension;
use crate::extension::types::MCP;
use crate::foundation::stream::StreamIndex;
use crate::kernel;
use crate::ui::extension_storage::ExtensionEphemeralStorage;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Manager;

/// 扫描扩展源目录
fn scan_extension_source(
    extension_loader: &extension::ExtensionLoader,
    extension_store: &extension::ExtensionStore,
    source_dir: &Path,
    source_label: &str,
) -> anyhow::Result<()> {
    if !source_dir.exists() {
        tracing::debug!(
            source = %source_dir.display(),
            label = source_label,
            "扩展源目录不存在，跳过扫描"
        );
        return Ok(());
    }

    if !source_dir.is_dir() {
        anyhow::bail!(
            "扩展源路径不是目录: {} ({})",
            source_dir.display(),
            source_label
        );
    }

    // 使用显式目录栈而不是假设固定的一级目录结构，支持：
    // extensions/<product>/<extension>/extension.json
    let mut pending = vec![source_dir.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)?.collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                pending.push(path);
                continue;
            }

            if !file_type.is_file()
                || path.file_name().and_then(|name| name.to_str()) != Some("extension.json")
            {
                continue;
            }

            let Some(extension_dir) = path.parent() else {
                continue;
            };

            let manifest = match extension_loader.load_manifest(extension_dir) {
                Ok(manifest) => manifest,
                Err(error) => {
                    tracing::warn!(
                        manifest = %path.display(),
                        source = source_label,
                        error = %error,
                        "扩展清单加载失败，跳过该扩展"
                    );
                    continue;
                }
            };

            let extension_id = manifest.id.clone();
            if extension_store.contains(&extension_id) {
                tracing::debug!(
                    extension_id = %extension_id,
                    source = source_label,
                    "扩展已注册，跳过重复清单"
                );
                continue;
            }

            let state = extension::ExtensionState {
                id: extension_id.clone(),
                status: extension::ExtensionStatus::Installed,
                manifest,
                install_path: extension_dir.to_path_buf(),
                installed_at: Utc::now(),
                enabled_at: None,
                error: None,
            };

            extension_store.register(state)?;
            tracing::info!(
                extension_id = %extension_id,
                path = %extension_dir.display(),
                source = source_label,
                "发现并注册扩展"
            );
        }
    }

    Ok(())
}

/// 获取源码树中的开发期扩展目录。
///
/// 源码扩展采用 `extensions/<product>/<extension>` 布局，宿主只关心
/// `extensions` 根目录，因此不会把任何具体产品名称写入框架启动链路。
fn development_extension_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("extensions")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 框架启动：EventBus → ExtensionStore → ExtensionLifecycle → ExtensionInstaller。
    // 业务代码通过扩展加载，不在这里硬编码。
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            crate::ui::extension_network::ui_extension_network_proxy,
            crate::ui::extension_stream::ui_extension_stream_subscribe,
            crate::ui::extension_stream::ui_extension_stream_unsubscribe,
            crate::ui::extension_bridge::ui_extension_bridge_authorize_event,
            crate::ui::extension_bridge::ui_extension_bridge_invoke,
            crate::ui::extensions::ui_list_extensions,
            crate::ui::extensions::ui_list_extension_views,
            crate::ui::extensions::ui_get_extension_view,
            crate::ui::extensions::ui_read_extension_entry_html,
            crate::ui::extensions::ui_set_extension_enabled,
            crate::ui::extensions::ui_install_extension,
            crate::ui::extensions::ui_uninstall_extension,
            crate::ui::extensions::ui_list_custom_work_modes,
            crate::ui::extensions::ui_list_zones,
            crate::ui::extensions::ui_list_extension_scripts,
            crate::ui::extensions::ui_list_extension_locales,
            crate::ui::extensions::ui_extension_discovery_query,
            crate::ui::extensions::ui_list_extension_points,
            crate::ui::extensions::ui_get_extension_config,
            crate::ui::extensions::ui_set_extension_config,
            crate::ui::extension_storage::ui_extension_storage_get,
            crate::ui::extension_storage::ui_extension_storage_set,
            crate::ui::extension_storage::ui_extension_storage_delete,
            crate::ui::extension_storage::ui_extension_storage_clear,
            crate::ui::menus::ui_list_menus,
            crate::ui::menus::ui_list_extension_commands,
            crate::ui::menus::ui_list_extension_keybindings,
            crate::ui::menus::ui_list_slash_commands,
            crate::ui::extension_router::ui_extension_route_call,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let runtime = tokio::runtime::Handle::try_current()
                .map_err(|error| anyhow::anyhow!("无法获取 Tauri Tokio runtime: {error}"))?;
            let event_bus: Arc<dyn kernel::EventBus> =
                Arc::new(kernel::InMemoryEventBus::new(1024, runtime));
            let extension_store = Arc::new(extension::ExtensionStore::new(Arc::clone(&event_bus)));
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let storage = Arc::new(Storage::new(
                &app_data_dir.join("navis.db"),
                None,
                Arc::clone(&event_bus),
            )?);
            let mcp = Arc::new(MCP::new());
            let stream_index = Arc::new(StreamIndex::new());
            let ephemeral_storage = Arc::new(ExtensionEphemeralStorage::default());

            // 开发源优先于用户目录中的旧安装，便于直接调试源码扩展。
            let development_dir = development_extension_dir();
            scan_extension_source(
                &extension::ExtensionLoader::new(),
                extension_store.as_ref(),
                &development_dir,
                "development",
            )?;

            let installed_dir = app.path().app_data_dir()?.join("extensions");
            scan_extension_source(
                &extension::ExtensionLoader::new(),
                extension_store.as_ref(),
                &installed_dir,
                "installed",
            )?;

            let lifecycle = Arc::new(extension::ExtensionLifecycle::new_without_skills(
                Arc::clone(&extension_store),
                Arc::clone(&event_bus),
            ));
            let installer = Arc::new(extension::ExtensionInstaller::new(
                installed_dir,
                Arc::clone(&extension_store),
                Arc::clone(&event_bus),
            ));

            // 扩展发现后统一交给生命周期启用；单个扩展失败不阻断宿主启动。
            for state in extension_store.list() {
                if let Err(error) = lifecycle.enable(&state.id) {
                    tracing::warn!(
                        extension_id = %state.id,
                        error = %error,
                        "扩展启用失败，保持已安装状态"
                    );
                }
            }

            app.manage(event_bus);
            app.manage(storage);
            app.manage(mcp);
            app.manage(stream_index);
            app.manage(ephemeral_storage);
            app.manage(Arc::clone(&extension_store));
            app.manage(lifecycle);
            app.manage(installer);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
