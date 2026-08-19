// 通用宿主入口：只暴露通用命令（发现 + 通用路由），
// 所有业务能力通过扩展的进程/插件实现，宿主不出现 Git/Terminal/LSP/File 专有命令。
pub mod core;
pub mod kernel;

use core::ipc_bridge::TransportRouter;
use core::sandbox::Sandbox;
use kernel::{ExtensionRegistry, manifest::ExtensionManifest};
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, State};

// 在进程内注册的动态 RPC 路由（Rust 原生扩展插件用）
#[tauri::command]
fn navis_dispatch_rpc(
    app: AppHandle,
    registry: State<'_, ExtensionRegistry>,
    route: String,
    payload: Value,
) -> Result<Value, String> {
    registry.dispatch(&app, &route, payload)
}

// 通用 IPC 路由：同步请求 -> 插件进程
#[tauri::command]
async fn core_route_ipc(
    transport: State<'_, Arc<TransportRouter>>,
    plugin_id: String,
    method: String,
    params: Value,
) -> Result<Value, String> {
    transport.send_rpc(&plugin_id, &method, params).await
}

// 通用 IPC 路由：流式请求 -> 插件进程 + Channel 推送
#[tauri::command]
async fn core_route_stream(
    transport: State<'_, Arc<TransportRouter>>,
    plugin_id: String,
    method: String,
    params: Value,
    on_event: tauri::ipc::Channel<Value>,
) -> Result<(), String> {
    transport.stream_rpc(&plugin_id, &method, params, on_event).await
}

// 扩展发现：返回完整清单（含 contributes 全量结构）
#[tauri::command]
fn navis_list_extensions(
    state: State<'_, Vec<ExtensionManifest>>,
) -> Result<Vec<Value>, String> {
    Ok(state.iter().map(|m| serde_json::to_value(m).unwrap_or_default()).collect())
}

// 动态路由清单（进程内注册的 route）
#[tauri::command]
fn navis_list_routes(
    registry: State<'_, ExtensionRegistry>,
) -> Result<Vec<String>, String> {
    Ok(registry.list_routes())
}

// 运行中的插件进程清单
#[tauri::command]
async fn navis_list_processes(
    transport: State<'_, Arc<TransportRouter>>,
) -> Result<Vec<String>, String> {
    Ok(transport.list_running().await)
}

// 权限审计日志
#[tauri::command]
fn navis_audit_log(
    sandbox: State<'_, Arc<Sandbox>>,
) -> Result<Vec<core::sandbox::AuditEntry>, String> {
    Ok(sandbox.audit_log())
}

/// 扩展发现目录：开发期扫描仓库 extensions/，运行期扫描应用数据目录
fn discover_extension_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    // 开发目录：CARGO_MANIFEST_DIR 的上层 extensions/
    let dev_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("extensions"));
    if let Some(d) = dev_dir {
        if d.exists() {
            dirs.push(d);
        }
    }
    // 运行期安装目录
    let app_data = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("navis")
        .join("extensions");
    dirs.push(app_data);
    dirs
}

pub fn run() {
    let registry = ExtensionRegistry::new();
    let transport = Arc::new(TransportRouter::new());
    let sandbox = Arc::new(Sandbox::new());

    // 扫描扩展（开发 + 运行期两个目录）
    let manifests: Vec<ExtensionManifest> = discover_extension_dirs()
        .iter()
        .flat_map(|d| kernel::scan_extensions(d))
        .collect();

    // 注册扩展声明的命令为进程内 RPC 路由
    let registry_clone = registry.clone();
    for manifest in &manifests {
        let plugin_id = manifest.plugin_id();
        for cmd in &manifest.contributes.commands {
            let route = format!("{plugin_id}:{}", cmd.id);
            let cmd_name = cmd.title.clone();
            registry_clone.register_route(
                &route,
                Arc::new(move |_app, _payload| {
                    println!("[Navis Kernel] Command '{}' invoked", cmd_name);
                    Ok(serde_json::json!({"status": "ok", "command": cmd_name}))
                }),
            );
        }
        // 依据清单 permissions 授权沙箱
        sandbox.grant_from_manifest(&plugin_id, &manifest.permissions);
    }

    let setup_transport = transport.clone();
    let app_result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry)
        .manage(manifests.clone())
        .manage(transport.clone())
        .manage(sandbox.clone())
        .invoke_handler(tauri::generate_handler![
            navis_dispatch_rpc,
            core_route_ipc,
            core_route_stream,
            navis_list_extensions,
            navis_list_routes,
            navis_list_processes,
            navis_audit_log,
        ])
        .setup(move |_app| {
            let ext_dirs = discover_extension_dirs();
            // 按清单 main 声明拉起插件后端进程
            for manifest in &manifests {
                if let Some(main) = &manifest.main {
                    let plugin_id = manifest.plugin_id();
                    let cwd = manifest_dir_for(&plugin_id, &ext_dirs);
                    if let Err(e) = setup_transport.ensure_plugin_process(&plugin_id, main, cwd.as_deref()) {
                        println!("[Navis Kernel] failed to start backend for '{plugin_id}': {e}");
                    }
                }
            }
            println!(
                "[Navis Kernel] Generic runtime shell initialized with {} extensions",
                manifests.len()
            );
            Ok(())
        })
        .run(tauri::generate_context!());
    app_result.expect("error while running navis core app");

    // 宿主退出：回收全部插件进程
    tauri::async_runtime::block_on(transport.shutdown());
}

/// 定位扩展目录：优先开发目录（仓库 extensions/），否则运行期安装目录
fn manifest_dir_for(plugin_id: &str, dirs: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    for dir in dirs {
        let candidate = dir.join(plugin_id);
        if candidate.join("extension.json").exists() {
            return Some(candidate);
        }
    }
    None
}