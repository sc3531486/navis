use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

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
}

pub trait NavisBackendPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn activate(&self, app: &AppHandle, registry: &ExtensionRegistry) -> Result<(), String>;
}
