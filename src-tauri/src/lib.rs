pub mod kernel;

use kernel::ExtensionRegistry;
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

pub fn run() {
    let registry = ExtensionRegistry::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry.clone())
        .invoke_handler(tauri::generate_handler![navis_dispatch_rpc])
        .setup(|_app| {
            println!("[Navis Kernel] Whiteboard host microkernel initialized.");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running navis core app");
}
