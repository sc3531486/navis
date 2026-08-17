//! 配置校验器
//!
//! 基于设计文档 §3.1 实现，提供配置校验能力
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde_json::Value;

use super::schema::SchemaIndex;
use super::{ConfigStore, ValidationError};

/// 配置校验器
pub struct ConfigValidator {
    /// Schema 索引
    schema_index: SchemaIndex,
}

impl ConfigValidator {
    /// 创建新的配置校验器
    pub fn new() -> Self {
        tracing::debug!("Creating new ConfigValidator");

        Self {
            schema_index: SchemaIndex::default(),
        }
    }

    /// 校验指定配置键的值
    ///
    /// # Arguments
    /// * `key` - 配置键
    /// * `value` - 配置值
    pub fn validate_key_value(&self, key: &str, value: &Value) -> Result<(), String> {
        tracing::debug!(key = %key, "Validating config key-value");

        self.schema_index.validate(key, value)
    }

    /// 校验所有配置
    ///
    /// # Arguments
    /// * `store` - 配置存储
    pub fn validate_all(&self, store: &ConfigStore) -> Vec<ValidationError> {
        tracing::debug!("Validating all config");

        let mut errors = Vec::new();
        let all_config = store.get_all();

        for (key, value) in all_config {
            if let Err(message) = self.validate_key_value(&key, &value) {
                errors.push(ValidationError {
                    key: key.clone(),
                    message,
                });
            }
        }

        // 检查必填配置
        for schema in self.schema_index.get_all() {
            if schema.required {
                if store.get(&schema.key).is_none() {
                    errors.push(ValidationError {
                        key: schema.key.clone(),
                        message: format!("Required config '{}' is missing", schema.key),
                    });
                }
            }
        }

        tracing::debug!(errors_count = errors.len(), "Config validation completed");

        errors
    }

    /// 注册自定义 Schema
    ///
    /// # Arguments
    /// * `schema` - 配置 Schema
    pub fn register_schema(&mut self, schema: super::schema::ConfigSchema) {
        self.schema_index.add(schema);
    }

    /// 获取 Schema 索引
    pub fn schema_index(&self) -> &SchemaIndex {
        &self.schema_index
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::config::ConfigSource;
    use serde_json::json;

    #[test]
    fn test_validate_valid_config() {
        let validator = ConfigValidator::new();

        // 有效配置
        assert!(validator
            .validate_key_value("gateway.defaultModel", &json!("claude-sonnet-4-6"))
            .is_ok());

        assert!(validator
            .validate_key_value("gateway.temperature", &json!(0.5))
            .is_ok());

        assert!(validator
            .validate_key_value("ui.theme", &json!("dark"))
            .is_ok());
    }

    #[test]
    fn test_validate_invalid_config() {
        let validator = ConfigValidator::new();

        // 无效类型
        assert!(validator
            .validate_key_value("gateway.defaultModel", &json!(42))
            .is_err());

        // 超出范围
        assert!(validator
            .validate_key_value("gateway.temperature", &json!(3.0))
            .is_err());

        // 无效枚举值
        assert!(validator
            .validate_key_value("ui.theme", &json!("invalid"))
            .is_err());
    }

    #[test]
    fn test_validate_all() {
        let validator = ConfigValidator::new();
        let mut store = ConfigStore::new();

        // 设置有效配置
        store.set(
            "gateway.defaultModel",
            json!("test-model"),
            ConfigSource::User,
        );
        store.set("gateway.temperature", json!(0.5), ConfigSource::User);

        let errors = validator.validate_all(&store);
        assert!(errors.is_empty());

        // 设置无效配置
        store.set("gateway.temperature", json!(3.0), ConfigSource::User);

        let errors = validator.validate_all(&store);
        assert!(!errors.is_empty());
    }
}
