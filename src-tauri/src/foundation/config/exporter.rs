//! 配置导出器
//!
//! 基于设计文档 §4.1 实现，支持将配置导出为 JSON/TOML/YAML 格式
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use anyhow::Result;
use serde_json::Value;

use super::ExportFormat;

/// 配置导出器
pub struct ConfigExporter;

impl ConfigExporter {
    /// 创建新的配置导出器
    pub fn new() -> Self {
        Self
    }

    /// 导出配置
    ///
    /// # Arguments
    /// * `config` - 配置数据
    /// * `format` - 导出格式
    pub fn export(&self, config: &Value, format: ExportFormat) -> Result<String> {
        tracing::debug!(format = ?format, "Exporting config");

        let content = match format {
            ExportFormat::Json => serde_json::to_string_pretty(config)?,
            ExportFormat::Toml => {
                let toml_value: toml::Value = self.json_to_toml(config)?;
                toml::to_string_pretty(&toml_value)?
            }
            ExportFormat::Yaml => {
                let yaml_value: serde_yaml::Value = self.json_to_yaml(config)?;
                serde_yaml::to_string(&yaml_value)?
            }
        };

        tracing::debug!(
            format = ?format,
            content_len = content.len(),
            "Config exported successfully"
        );

        Ok(content)
    }

    /// JSON 转 TOML
    fn json_to_toml(&self, json_value: &Value) -> Result<toml::Value> {
        let json_str = serde_json::to_string(json_value)?;
        let toml_value: toml::Value = serde_json::from_str(&json_str)?;
        Ok(toml_value)
    }

    /// JSON 转 YAML
    fn json_to_yaml(&self, json_value: &Value) -> Result<serde_yaml::Value> {
        let json_str = serde_json::to_string(json_value)?;
        let yaml_value: serde_yaml::Value = serde_json::from_str(&json_str)?;
        Ok(yaml_value)
    }
}

impl Default for ConfigExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_export_json() {
        let exporter = ConfigExporter::new();
        let config = json!({
            "gateway": {
                "defaultModel": "test-model",
                "timeout": 60000
            }
        });

        let content = exporter.export(&config, ExportFormat::Json).unwrap();

        assert!(content.contains("test-model"));
        assert!(content.contains("60000"));

        // 验证是有效的 JSON
        let parsed: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["gateway"]["defaultModel"], "test-model");
    }

    #[test]
    fn test_export_toml() {
        let exporter = ConfigExporter::new();
        let config = json!({
            "gateway": {
                "defaultModel": "test-model",
                "timeout": 60000
            }
        });

        let content = exporter.export(&config, ExportFormat::Toml).unwrap();

        assert!(content.contains("test-model"));
        assert!(content.contains("60000"));

        // 验证是有效的 TOML
        let parsed: toml::Value = toml::from_str(&content).unwrap();
        assert_eq!(
            parsed["gateway"]["defaultModel"].as_str().unwrap(),
            "test-model"
        );
    }

    #[test]
    fn test_export_yaml() {
        let exporter = ConfigExporter::new();
        let config = json!({
            "gateway": {
                "defaultModel": "test-model",
                "timeout": 60000
            }
        });

        let content = exporter.export(&config, ExportFormat::Yaml).unwrap();

        assert!(content.contains("test-model"));
        assert!(content.contains("60000"));

        // 验证是有效的 YAML
        let parsed: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        assert_eq!(
            parsed["gateway"]["defaultModel"].as_str().unwrap(),
            "test-model"
        );
    }

    #[test]
    fn test_export_empty_config() {
        let exporter = ConfigExporter::new();
        let config = json!({});

        let content = exporter.export(&config, ExportFormat::Json).unwrap();
        assert_eq!(content, "{}");
    }
}
