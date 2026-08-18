//! 作用域存储：NamedEntries + ScopedLayers
//! 参考 DeepSeek Harness dsh-scope 设计，通用框架，不绑定业务领域。

use std::collections::HashMap;

/// 命名条目表（插入顺序，幂等卸载）
pub struct NamedEntries<V> {
    data: HashMap<String, V>,
    order: Vec<String>,
}

impl<V> NamedEntries<V> {
    pub fn new() -> Self { Self { data: HashMap::new(), order: Vec::new() } }
    pub fn insert(&mut self, name: String, value: V) -> Result<(), String> {
        if self.data.contains_key(&name) { return Err(format!("Duplicate: '{name}'")); }
        self.order.push(name.clone());
        self.data.insert(name, value);
        Ok(())
    }
    pub fn remove(&mut self, name: &str) -> Option<V> {
        self.order.retain(|n| n != name);
        self.data.remove(name)
    }
    pub fn get(&self, name: &str) -> Option<&V> { self.data.get(name) }
    pub fn has(&self, name: &str) -> bool { self.data.contains_key(name) }
    pub fn keys(&self) -> impl Iterator<Item = &str> { self.order.iter().map(|s| s.as_str()) }
    pub fn is_empty(&self) -> bool { self.data.is_empty() }
    pub fn len(&self) -> usize { self.data.len() }
}

impl<V> Default for NamedEntries<V> { fn default() -> Self { Self::new() } }

/// 作用域层级（子层继承父层条目）
pub struct ScopedLayers<V> {
    local: NamedEntries<V>,
    parent: Option<Box<ScopedLayers<V>>>,
}

impl<V> ScopedLayers<V> {
    pub fn root() -> Self { Self { local: NamedEntries::new(), parent: None } }
    pub fn child(parent: ScopedLayers<V>) -> Self { Self { local: NamedEntries::new(), parent: Some(Box::new(parent)) } }
    pub fn insert(&mut self, name: String, value: V) -> Result<(), String> { self.local.insert(name, value) }
    pub fn get(&self, name: &str) -> Option<&V> {
        self.local.get(name).or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }
    pub fn get_local(&self, name: &str) -> Option<&V> { self.local.get(name) }
}

impl<V> Default for ScopedLayers<V> { fn default() -> Self { Self::root() } }
