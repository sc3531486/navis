//! 配置加载器
//!
//! 基于设计文档 §2.1 实现，支持从 JSON/TOML/YAML 文件加载配置
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::ExportFormat;

/// 配置加载器
pub struct ConfigLoader;

impl ConfigLoader {
    /// 创建新的配置加载器
    pub fn new() -> Self {
        Self
    }

    /// 从文件加载配置
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    pub fn load(&self, path: &Path) -> Result<Value> {
        tracing::info!(path = %path.display(), "Loading config from file");

        // 读取文件内容
        let content = fs::read_to_string(path)?;

        // 根据文件扩展名确定格式
        let format = self.detect_format(path)?;

        // 解析配置
        let config = self.parse(&content, format)?;

        tracing::info!(
            path = %path.display(),
            keys_count = config.as_object().map(|o| o.len()).unwrap_or(0),
            "Config loaded successfully"
        );

        Ok(config)
    }

    /// 解析配置内容
    ///
    /// # Arguments
    /// * `content` - 配置内容
    /// * `format` - 配置格式
    pub fn parse(&self, content: &str, format: ExportFormat) -> Result<Value> {
        tracing::debug!(format = ?format, "Parsing config content");

        match format {
            ExportFormat::Json => {
                let config: Value = serde_json::from_str(content)?;
                Ok(config)
            }
            ExportFormat::Toml => {
                let config: toml::Value = toml::from_str(content)?;
                let json_value = self.toml_to_json(config)?;
                Ok(json_value)
            }
            ExportFormat::Yaml => {
                let config: serde_yaml::Value = serde_yaml::from_str(content)?;
                let json_value = self.yaml_to_json(config)?;
                Ok(json_value)
            }
        }
    }

    /// 检测文件格式
    fn detect_format(&self, path: &Path) -> Result<ExportFormat> {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "toml" => Ok(ExportFormat::Toml),
            "yaml" | "yml" => Ok(ExportFormat::Yaml),
            _ => {
                tracing::warn!(
                    extension = %extension,
                    "Unknown config file format, defaulting to JSON"
                );
                Ok(ExportFormat::Json)
            }
        }
    }

    /// TOML 转 JSON
    fn toml_to_json(&self, toml_value: toml::Value) -> Result<Value> {
        let json_str = serde_json::to_string(&toml_value)?;
        let json_value: Value = serde_json::from_str(&json_str)?;
        Ok(json_value)
    }

    /// YAML 转 JSON
    fn yaml_to_json(&self, yaml_value: serde_yaml::Value) -> Result<Value> {
        let json_str = serde_json::to_string(&yaml_value)?;
        let json_value: Value = serde_json::from_str(&json_str)?;
        Ok(json_value)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_json_config() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"{{
            "gateway": {{
                "defaultModel": "test-model",
                "timeout": 60000
            }}
        }}"#
        )
        .unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load(file.path()).unwrap();

        assert_eq!(config["gateway"]["defaultModel"], "test-model");
        assert_eq!(config["gateway"]["timeout"], 60000);
    }

    #[test]
    fn test_load_toml_config() {
        let mut file = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(
            file,
            r#"
[gateway]
defaultModel = "test-model"
timeout = 60000
"#
        )
        .unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load(file.path()).unwrap();

        assert_eq!(config["gateway"]["defaultModel"], "test-model");
        assert_eq!(config["gateway"]["timeout"], 60000);
    }

    #[test]
    fn test_load_yaml_config() {
        let mut file = NamedTempFile::with_suffix(".yaml").unwrap();
        writeln!(
            file,
            r#"
gateway:
  defaultModel: test-model
  timeout: 60000
"#
        )
        .unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load(file.path()).unwrap();

        assert_eq!(config["gateway"]["defaultModel"], "test-model");
        assert_eq!(config["gateway"]["timeout"], 60000);
    }

    #[test]
    fn test_detect_format() {
        let loader = ConfigLoader::new();

        assert_eq!(
            loader.detect_format(Path::new("config.json")).unwrap(),
            ExportFormat::Json
        );
        assert_eq!(
            loader.detect_format(Path::new("config.toml")).unwrap(),
            ExportFormat::Toml
        );
        assert_eq!(
            loader.detect_format(Path::new("config.yaml")).unwrap(),
            ExportFormat::Yaml
        );
        assert_eq!(
            loader.detect_format(Path::new("config.yml")).unwrap(),
            ExportFormat::Yaml
        );
    }

    #[test]
    fn test_parse_json() {
        let loader = ConfigLoader::new();
        let content = r#"{"key": "value"}"#;

        let config = loader.parse(content, ExportFormat::Json).unwrap();
        assert_eq!(config["key"], "value");
    }

    #[test]
    fn test_parse_toml() {
        let loader = ConfigLoader::new();
        let content = r#"key = "value""#;

        let config = loader.parse(content, ExportFormat::Toml).unwrap();
        assert_eq!(config["key"], "value");
    }

    #[test]
    fn test_parse_yaml() {
        let loader = ConfigLoader::new();
        let content = "key: value";

        let config = loader.parse(content, ExportFormat::Yaml).unwrap();
        assert_eq!(config["key"], "value");
    }
}
