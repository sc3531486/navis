pub mod kernel;

use kernel::{ExtensionRegistry, manifest::ExtensionManifest};
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
fn navis_dispatch_rpc(
    app: AppHandle,
    registry: State<'_, ExtensionRegistry>,
    route: String,
    payload: Value,
) -> Result<Value, String> {
    registry.dispatch(&app, &route, payload)
}

#[tauri::command]
fn navis_list_extensions(
    state: State<'_, Vec<ExtensionManifest>>,
) -> Result<Vec<Value>, String> {
    Ok(state.iter().map(|m| {
        serde_json::json!({
            "name": m.name,
            "version": m.version,
            "slots": m.contributes.slots.len(),
            "commands": m.contributes.commands.len(),
        })
    }).collect())
}

#[tauri::command]
fn navis_list_routes(
    registry: State<'_, ExtensionRegistry>,
) -> Result<Vec<String>, String> {
    Ok(registry.list_routes())
}

pub fn run() {
    let registry = ExtensionRegistry::new();

    // 扫描扩展
    let app_data = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("navis");
    let manifests = kernel::scan_extensions(&app_data);

    // 激活扩展
    // 注意：setup 里才能拿到 AppHandle，所以先克隆 registry
    let registry_clone = registry.clone();
    let manifests_clone = manifests.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry)
        .manage(manifests)
        .invoke_handler(tauri::generate_handler![
            navis_dispatch_rpc,
            navis_list_extensions,
            navis_list_routes,
        ])
        .setup(move |app| {
            // 激活扩展
            kernel::activate_extensions(
                app.handle(),
                &registry_clone,
                &manifests_clone,
            );
            println!("[Navis Kernel] Whiteboard host microkernel initialized with {} extensions", manifests_clone.len());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running navis core app");
}
