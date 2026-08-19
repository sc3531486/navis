use crate::kernel::manifest::ExtensionManifest;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

pub mod manifest;

pub type DynamicRpcHandler = Arc<dyn Fn(&AppHandle, Value) -> Result<Value, String> + Send + Sync>;

#[derive(Default, Clone)]
pub struct ExtensionRegistry {
    routes: Arc<RwLock<HashMap<String, DynamicRpcHandler>>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_route(&self, route: &str, handler: DynamicRpcHandler) {
        let mut map = self.routes.write().unwrap();
        map.insert(route.to_string(), handler);
        println!("[Navis Kernel] Dynamic route registered: {}", route);
    }

    pub fn dispatch(&self, app: &AppHandle, route: &str, payload: Value) -> Result<Value, String> {
        let map = self.routes.read().unwrap();
        if let Some(handler) = map.get(route) {
            handler(app, payload)
        } else {
            Err(format!("[Navis Kernel] Route '{}' not found in registry", route))
        }
    }

    pub fn list_routes(&self) -> Vec<String> {
        let map = self.routes.read().unwrap();
        map.keys().cloned().collect()
    }
}

pub trait NavisBackendPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String>;
}

/// 扫描扩展目录并返回所有清单
pub fn scan_extensions(app_data_dir: &std::path::Path) -> Vec<ExtensionManifest> {
    let extensions_dir = app_data_dir.join("extensions");
    ExtensionManifest::load_from_dir(&extensions_dir)
}

/// 后端扩展激活入口
pub fn activate_extensions(
    _app: &AppHandle,
    registry: &ExtensionRegistry,
    manifests: &[ExtensionManifest],
) {
    for manifest in manifests {
        println!("[Navis Kernel] Extension '{}' v{} found", manifest.name, manifest.version);
        // 注册扩展声明的命令为 RPC 路由
        for cmd in &manifest.contributes.commands {
            let route = format!("{}:{}", manifest.name, cmd.id);
            let cmd_name = cmd.title.clone();
            registry.register_route(
                &route,
                Arc::new(move |_app, _payload| {
                    println!("[Navis Kernel] Command '{}' invoked", cmd_name);
                    Ok(serde_json::json!({"status": "ok", "command": cmd_name}))
                }),
            );
        }
    }
}
