//! 配置存储
//!
//! 基于设计文档 §2.2 实现，提供多层配置合并和读写能力
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

use super::{ConfigEntry, ConfigSource};

/// 配置存储
///
/// 管理多层配置，支持配置合并和读写
pub struct ConfigStore {
    /// 各层配置存储（source -> (key -> value)）
    layers: HashMap<ConfigSource, HashMap<String, Value>>,
    /// 配置变更时间（key -> updated_at）
    updated_at: HashMap<String, DateTime<Utc>>,
}

impl ConfigStore {
    /// 创建新的配置存储
    pub fn new() -> Self {
        tracing::debug!("Creating new ConfigStore");

        Self {
            layers: HashMap::new(),
            updated_at: HashMap::new(),
        }
    }

    /// 合并配置
    ///
    /// # Arguments
    /// * `config` - 配置数据（JSON 对象）
    /// * `source` - 配置来源
    pub fn merge_config(&mut self, config: Value, source: ConfigSource) {
        tracing::debug!(source = %source, "Merging config");

        if let Some(obj) = config.as_object() {
            let layer = self
                .layers
                .entry(source.clone())
                .or_insert_with(HashMap::new);

            for (key, value) in obj {
                // 展平点号路径
                ConfigStore::flatten_and_store(layer, key, value, "");
            }

            tracing::info!(
                source = %source,
                keys_count = layer.len(),
                "Config merged successfully"
            );
        }
    }

    /// 展平并存储配置
    fn flatten_and_store(
        layer: &mut HashMap<String, Value>,
        key: &str,
        value: &Value,
        prefix: &str,
    ) {
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", prefix, key)
        };

        if value.is_object() {
            // 递归展平嵌套对象
            if let Some(obj) = value.as_object() {
                for (nested_key, nested_value) in obj {
                    Self::flatten_and_store(layer, nested_key, nested_value, &full_key);
                }
            }
        } else {
            // 存储叶子节点
            layer.insert(full_key, value.clone());
        }
    }

    /// 获取配置值（自动合并多层）
    ///
    /// # Arguments
    /// * `key` - 配置键（点号路径）
    pub fn get(&self, key: &str) -> Option<Value> {
        // 按优先级从高到低查找：运行时 > 模式 > 项目 > 用户 > 系统
        let priority_order = [
            ConfigSource::Runtime,
            ConfigSource::Mode,
            ConfigSource::Project,
            ConfigSource::User,
            ConfigSource::System,
        ];

        for source in &priority_order {
            if let Some(layer) = self.layers.get(source) {
                if let Some(value) = layer.get(key) {
                    return Some(value.clone());
                }
            }
        }

        None
    }

    /// 获取配置条目（含来源信息）
    ///
    /// # Arguments
    /// * `key` - 配置键
    pub fn get_entry(&self, key: &str) -> Option<ConfigEntry> {
        let priority_order = [
            ConfigSource::Runtime,
            ConfigSource::Mode,
            ConfigSource::Project,
            ConfigSource::User,
            ConfigSource::System,
        ];

        for source in &priority_order {
            if let Some(layer) = self.layers.get(source) {
                if let Some(value) = layer.get(key) {
                    return Some(ConfigEntry {
                        key: key.to_string(),
                        value: value.clone(),
                        source: source.clone(),
                        updated_at: self.updated_at.get(key).cloned().unwrap_or_else(Utc::now),
                    });
                }
            }
        }

        None
    }

    /// 设置配置值
    ///
    /// # Arguments
    /// * `key` - 配置键
    /// * `value` - 配置值
    /// * `source` - 配置来源
    pub fn set(&mut self, key: &str, value: Value, source: ConfigSource) {
        tracing::debug!(key = %key, source = %source, "Setting config value");

        let layer = self.layers.entry(source).or_insert_with(HashMap::new);
        layer.insert(key.to_string(), value);
        self.updated_at.insert(key.to_string(), Utc::now());
    }

    /// 删除配置值
    ///
    /// # Arguments
    /// * `key` - 配置键
    pub fn unset(&mut self, key: &str) {
        tracing::debug!(key = %key, "Unsetting config value");

        // 从所有层中删除
        for (_, layer) in self.layers.iter_mut() {
            layer.remove(key);
        }

        self.updated_at.remove(key);
    }

    /// 获取指定来源的配置
    ///
    /// # Arguments
    /// * `source` - 配置来源
    pub fn get_source_config(&self, source: ConfigSource) -> Value {
        if let Some(layer) = self.layers.get(&source) {
            let mut obj = serde_json::Map::new();

            for (key, value) in layer {
                // 将点号路径转换为嵌套对象
                let parts: Vec<&str> = key.split('.').collect();
                Self::insert_nested(&mut obj, &parts, value);
            }

            Value::Object(obj)
        } else {
            Value::Object(serde_json::Map::new())
        }
    }

    /// 插入嵌套值
    fn insert_nested(obj: &mut serde_json::Map<String, Value>, parts: &[&str], value: &Value) {
        if parts.len() == 1 {
            obj.insert(parts[0].to_string(), value.clone());
        } else {
            let key = parts[0].to_string();
            let entry = obj
                .entry(key)
                .or_insert_with(|| Value::Object(serde_json::Map::new()));

            if let Value::Object(ref mut nested_obj) = entry {
                Self::insert_nested(nested_obj, &parts[1..], value);
            }
        }
    }

    /// 获取所有配置键
    pub fn get_all_keys(&self) -> Vec<String> {
        let mut keys = std::collections::HashSet::new();

        for (_, layer) in self.layers.iter() {
            for key in layer.keys() {
                keys.insert(key.clone());
            }
        }

        keys.into_iter().collect()
    }

    /// 获取所有配置（合并后）
    pub fn get_all(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        let keys = self.get_all_keys();

        for key in keys {
            if let Some(value) = self.get(&key) {
                result.insert(key, value);
            }
        }

        result
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_store_merge_and_get() {
        let mut store = ConfigStore::new();

        // 合并用户配置
        store.merge_config(
            json!({
                "gateway": {
                    "defaultModel": "user-model",
                    "timeout": 60000
                }
            }),
            ConfigSource::User,
        );

        // 合并项目配置（覆盖部分）
        store.merge_config(
            json!({
                "gateway": {
                    "defaultModel": "project-model"
                }
            }),
            ConfigSource::Project,
        );

        // 获取配置（项目配置优先级更高）
        let model = store.get("gateway.defaultModel");
        assert_eq!(model, Some(json!("project-model")));

        // 获取用户配置独有的
        let timeout = store.get("gateway.timeout");
        assert_eq!(timeout, Some(json!(60000)));
    }

    #[test]
    fn test_config_store_set_and_unset() {
        let mut store = ConfigStore::new();

        // 设置配置
        store.set("test.key", json!("value"), ConfigSource::User);
        assert_eq!(store.get("test.key"), Some(json!("value")));

        // 删除配置
        store.unset("test.key");
        assert_eq!(store.get("test.key"), None);
    }

    #[test]
    fn test_config_store_priority() {
        let mut store = ConfigStore::new();

        // 设置系统默认
        store.set("key", json!("system"), ConfigSource::System);

        // 设置用户配置
        store.set("key", json!("user"), ConfigSource::User);

        // 设置运行时配置
        store.set("key", json!("runtime"), ConfigSource::Runtime);

        // 运行时优先级最高
        assert_eq!(store.get("key"), Some(json!("runtime")));
    }

    #[test]
    fn test_config_store_get_entry() {
        let mut store = ConfigStore::new();

        store.set("test.key", json!("value"), ConfigSource::Project);

        let entry = store.get_entry("test.key").unwrap();
        assert_eq!(entry.key, "test.key");
        assert_eq!(entry.value, json!("value"));
        assert_eq!(entry.source, ConfigSource::Project);
    }

    #[test]
    fn test_config_store_get_source_config() {
        let mut store = ConfigStore::new();

        store.set(
            "gateway.defaultModel",
            json!("test-model"),
            ConfigSource::User,
        );
        store.set("gateway.timeout", json!(60000), ConfigSource::User);
        store.set("editor.fontSize", json!(14), ConfigSource::User);

        let config = store.get_source_config(ConfigSource::User);
        assert!(config.is_object());

        let obj = config.as_object().unwrap();
        assert!(obj.contains_key("gateway"));
        assert!(obj.contains_key("editor"));
    }

    #[test]
    fn test_config_store_get_all() {
        let mut store = ConfigStore::new();

        store.set("key1", json!("value1"), ConfigSource::System);
        store.set("key2", json!("value2"), ConfigSource::User);
        store.set("key3", json!("value3"), ConfigSource::Project);

        let all = store.get_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all.get("key1"), Some(&json!("value1")));
        assert_eq!(all.get("key2"), Some(&json!("value2")));
        assert_eq!(all.get("key3"), Some(&json!("value3")));
    }
}
