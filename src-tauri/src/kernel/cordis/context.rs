//! Context 依赖注入容器
//! 参考 DeepSeek Harness Cordis Context，通用框架，不绑定业务领域。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Cordis 风格的 Context 容器
#[derive(Clone)]
pub struct CordisContext {
    services: Arc<RwLock<HashMap<TypeId, (String, Arc<dyn Any + Send + Sync>)>>>,
    names: Arc<RwLock<HashMap<String, TypeId>>>,
    parent: Option<Box<CordisContext>>,
    isolate_label: Option<String>,
}

impl CordisContext {
    pub fn root() -> Self {
        Self { services: Arc::new(RwLock::new(HashMap::new())), names: Arc::new(RwLock::new(HashMap::new())), parent: None, isolate_label: None }
    }
    pub fn provide<T: Send + Sync + 'static>(&self, name: impl Into<String>, service: Arc<T>) {
        let name = name.into();
        let tid = TypeId::of::<T>();
        self.services.write().unwrap().insert(tid, (name.clone(), service));
        self.names.write().unwrap().insert(name, tid);
    }
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let tid = TypeId::of::<T>();
        if let Some((_, s)) = self.services.read().unwrap().get(&tid) {
            s.clone().downcast::<T>().ok()
        } else {
            self.parent.as_ref().and_then(|p| p.get::<T>())
        }
    }
    pub fn extend(&self) -> Self {
        Self { services: Arc::new(RwLock::new(HashMap::new())), names: Arc::new(RwLock::new(HashMap::new())), parent: Some(Box::new(self.clone())), isolate_label: self.isolate_label.clone() }
    }
    pub fn isolate(&self, label: impl Into<String>) -> Self {
        Self { services: Arc::new(RwLock::new(HashMap::new())), names: Arc::new(RwLock::new(HashMap::new())), parent: None, isolate_label: Some(label.into()) }
    }
    pub fn isolate_label(&self) -> Option<&str> { self.isolate_label.as_deref() }
    pub fn has<T: Send + Sync + 'static>(&self) -> bool { self.get::<T>().is_some() }
}
