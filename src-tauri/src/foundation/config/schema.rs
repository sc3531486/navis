//! 配置 Schema 定义
//!
//! 基于设计文档 §3.1 实现，定义配置的 Schema 结构和类型
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 配置值类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigValueType {
    /// 字符串
    String,
    /// 数字
    Number,
    /// 布尔值
    Boolean,
    /// 数组
    Array,
    /// 对象
    Object,
    /// 枚举（可选值列表）
    Enum(Vec<String>),
}

impl std::fmt::Display for ConfigValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValueType::String => write!(f, "string"),
            ConfigValueType::Number => write!(f, "number"),
            ConfigValueType::Boolean => write!(f, "boolean"),
            ConfigValueType::Array => write!(f, "array"),
            ConfigValueType::Object => write!(f, "object"),
            ConfigValueType::Enum(values) => write!(f, "enum({})", values.join(", ")),
        }
    }
}

/// 校验器类型
#[derive(Debug, Clone)]
pub enum Validator {
    /// 最小值（数值）
    Min(f64),
    /// 最大值（数值）
    Max(f64),
    /// 最小长度（字符串）
    MinLength(usize),
    /// 最大长度（字符串）
    MaxLength(usize),
    /// 正则表达式
    Regex(String),
    /// 自定义校验函数名称
    Custom(String),
}

/// 配置 Schema
#[derive(Debug, Clone)]
pub struct ConfigSchema {
    /// 配置键（点号路径，如 "gateway.defaultModel"）
    pub key: String,
    /// 值类型
    pub value_type: ConfigValueType,
    /// 默认值
    pub default: Option<Value>,
    /// 描述
    pub description: String,
    /// 是否必填
    pub required: bool,
    /// 校验规则列表
    pub validators: Vec<Validator>,
    /// 是否敏感（脱敏显示）
    pub sensitive: bool,
    /// 是否支持热更新
    pub hot_reload: bool,
}

impl ConfigSchema {
    /// 创建新的配置 Schema
    pub fn new(
        key: impl Into<String>,
        value_type: ConfigValueType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value_type,
            default: None,
            description: description.into(),
            required: false,
            validators: Vec::new(),
            sensitive: false,
            hot_reload: true,
        }
    }

    /// 设置默认值
    pub fn with_default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }

    /// 设置为必填
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// 设置校验规则（可多次调用添加多个）
    pub fn with_validator(mut self, validator: Validator) -> Self {
        self.validators.push(validator);
        self
    }

    /// 设置为敏感信息
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }

    /// 设置不支持热更新
    pub fn no_hot_reload(mut self) -> Self {
        self.hot_reload = false;
        self
    }

    /// 校验值是否符合 Schema
    pub fn validate_value(&self, value: &Value) -> Result<(), String> {
        // 检查类型
        match &self.value_type {
            ConfigValueType::String => {
                if !value.is_string() {
                    return Err(format!("Expected string, got {}", value));
                }
            }
            ConfigValueType::Number => {
                if !value.is_number() {
                    return Err(format!("Expected number, got {}", value));
                }
            }
            ConfigValueType::Boolean => {
                if !value.is_boolean() {
                    return Err(format!("Expected boolean, got {}", value));
                }
            }
            ConfigValueType::Array => {
                if !value.is_array() {
                    return Err(format!("Expected array, got {}", value));
                }
            }
            ConfigValueType::Object => {
                if !value.is_object() {
                    return Err(format!("Expected object, got {}", value));
                }
            }
            ConfigValueType::Enum(allowed_values) => {
                if let Some(str_value) = value.as_str() {
                    if !allowed_values.contains(&str_value.to_string()) {
                        return Err(format!(
                            "Expected one of [{}], got '{}'",
                            allowed_values.join(", "),
                            str_value
                        ));
                    }
                } else {
                    return Err(format!("Expected string for enum, got {}", value));
                }
            }
        }

        // 检查校验规则
        for validator in &self.validators {
            match validator {
                Validator::Min(min) => {
                    if let Some(num) = value.as_f64().or_else(|| value.as_i64().map(|n| n as f64)) {
                        if num < *min {
                            return Err(format!("Value {} is less than minimum {}", num, min));
                        }
                    }
                }
                Validator::Max(max) => {
                    if let Some(num) = value.as_f64().or_else(|| value.as_i64().map(|n| n as f64)) {
                        if num > *max {
                            return Err(format!("Value {} is greater than maximum {}", num, max));
                        }
                    }
                }
                Validator::MinLength(min_len) => {
                    if let Some(str_value) = value.as_str() {
                        if str_value.len() < *min_len {
                            return Err(format!(
                                "String length {} is less than minimum {}",
                                str_value.len(),
                                min_len
                            ));
                        }
                    }
                }
                Validator::MaxLength(max_len) => {
                    if let Some(str_value) = value.as_str() {
                        if str_value.len() > *max_len {
                            return Err(format!(
                                "String length {} is greater than maximum {}",
                                str_value.len(),
                                max_len
                            ));
                        }
                    }
                }
                Validator::Regex(pattern) => {
                    if let Some(str_value) = value.as_str() {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if !re.is_match(str_value) {
                                return Err(format!(
                                    "String '{}' does not match pattern '{}'",
                                    str_value, pattern
                                ));
                            }
                        }
                    }
                }
                Validator::Custom(_) => {
                    // 自定义校验需要在外部实现
                }
            }
        }

        Ok(())
    }
}

/// 内置配置 Schema 索引
///
/// 这是 Config 领域内部的 schema DTO 索引，不是 Kernel Registry：
/// 不声明运行能力，不参与生命周期，也不作为跨模块事实源。
pub struct SchemaIndex {
    schemas: Vec<ConfigSchema>,
}

impl SchemaIndex {
    /// 创建新的 Schema 索引
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    /// 将配置 Schema 加入索引
    pub fn add(&mut self, schema: ConfigSchema) {
        tracing::debug!(key = %schema.key, "Adding config schema to index");
        self.schemas.push(schema);
    }

    /// 获取配置 Schema
    pub fn get(&self, key: &str) -> Option<&ConfigSchema> {
        self.schemas.iter().find(|s| s.key == key)
    }

    /// 获取所有 Schema
    pub fn get_all(&self) -> &[ConfigSchema] {
        &self.schemas
    }

    /// 校验值
    pub fn validate(&self, key: &str, value: &Value) -> Result<(), String> {
        if let Some(schema) = self.get(key) {
            schema.validate_value(value)
        } else {
            // 没有 Schema 的配置键允许任意值
            Ok(())
        }
    }
}

impl Default for SchemaIndex {
    fn default() -> Self {
        let mut index = Self::new();

        // 加入内置配置 Schema
        index.add_builtin_schemas();

        index
    }
}

impl SchemaIndex {
    /// 加入内置配置 Schema
    fn add_builtin_schemas(&mut self) {
        // Gateway 配置
        self.add(
            ConfigSchema::new("gateway.defaultModel", ConfigValueType::String, "默认模型")
                .with_default(Value::String("claude-sonnet-4-6".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "gateway.offlineMode",
                ConfigValueType::Enum(vec![
                    "auto".to_string(),
                    "always".to_string(),
                    "never".to_string(),
                ]),
                "离线模式",
            )
            .with_default(Value::String("auto".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "gateway.maxRetries",
                ConfigValueType::Number,
                "最大重试次数",
            )
            .with_default(Value::Number(serde_json::Number::from(3)))
            .with_validator(Validator::Min(0.0))
            .with_validator(Validator::Max(10.0)),
        );

        self.add(
            ConfigSchema::new(
                "gateway.timeout",
                ConfigValueType::Number,
                "超时时间（毫秒）",
            )
            .with_default(Value::Number(serde_json::Number::from(30000)))
            .with_validator(Validator::Min(1000.0))
            .with_validator(Validator::Max(300000.0)),
        );

        self.add(
            ConfigSchema::new("gateway.temperature", ConfigValueType::Number, "温度参数")
                .with_default(Value::Number(serde_json::Number::from_f64(0.3).unwrap()))
                .with_validator(Validator::Min(0.0))
                .with_validator(Validator::Max(2.0)),
        );

        // Agent 配置
        self.add(
            ConfigSchema::new(
                "agent.maxHistoryMessages",
                ConfigValueType::Number,
                "最大历史消息数",
            )
            .with_default(Value::Number(serde_json::Number::from(50)))
            .with_validator(Validator::Min(10.0))
            .with_validator(Validator::Max(500.0)),
        );

        self.add(
            ConfigSchema::new(
                "agent.compressStrategy",
                ConfigValueType::Enum(vec![
                    "sliding_window".to_string(),
                    "summary".to_string(),
                    "smart".to_string(),
                ]),
                "压缩策略",
            )
            .with_default(Value::String("sliding_window".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "agent.maxExecutionTime",
                ConfigValueType::Number,
                "最大执行时间（秒）",
            )
            .with_default(Value::Number(serde_json::Number::from(300)))
            .with_validator(Validator::Min(30.0))
            .with_validator(Validator::Max(3600.0)),
        );

        // Editor 配置
        self.add(
            ConfigSchema::new("editor.fontSize", ConfigValueType::Number, "字体大小")
                .with_default(Value::Number(serde_json::Number::from(14)))
                .with_validator(Validator::Min(8.0))
                .with_validator(Validator::Max(32.0)),
        );

        self.add(
            ConfigSchema::new("editor.tabSize", ConfigValueType::Number, "Tab 大小")
                .with_default(Value::Number(serde_json::Number::from(2)))
                .with_validator(Validator::Min(1.0))
                .with_validator(Validator::Max(8.0)),
        );

        self.add(
            ConfigSchema::new(
                "editor.wordWrap",
                ConfigValueType::Enum(vec![
                    "on".to_string(),
                    "off".to_string(),
                    "wordWrapColumn".to_string(),
                    "bounded".to_string(),
                ]),
                "自动换行",
            )
            .with_default(Value::String("on".to_string())),
        );

        self.add(
            ConfigSchema::new("editor.minimap", ConfigValueType::Boolean, "显示小地图")
                .with_default(Value::Bool(true)),
        );

        self.add(
            ConfigSchema::new(
                "editor.formatOnSave",
                ConfigValueType::Boolean,
                "保存时格式化",
            )
            .with_default(Value::Bool(true)),
        );

        self.add(
            ConfigSchema::new(
                "editor.externalEditors",
                ConfigValueType::Array,
                "外部编程工具列表",
            )
            .with_default(Value::Array(Vec::new())),
        );

        self.add(
            ConfigSchema::new(
                "editor.defaultExternalEditorId",
                ConfigValueType::String,
                "默认外部编程工具 ID",
            )
            .with_default(Value::Null),
        );

        // Terminal 配置
        self.add(
            ConfigSchema::new(
                "terminal.defaultShell",
                ConfigValueType::String,
                "默认 Shell",
            )
            .with_default(Value::String("auto".to_string())),
        );

        self.add(
            ConfigSchema::new("terminal.fontSize", ConfigValueType::Number, "终端字体大小")
                .with_default(Value::Number(serde_json::Number::from(14)))
                .with_validator(Validator::Min(8.0))
                .with_validator(Validator::Max(32.0)),
        );

        self.add(
            ConfigSchema::new(
                "terminal.maxHistory",
                ConfigValueType::Number,
                "终端历史记录数",
            )
            .with_default(Value::Number(serde_json::Number::from(1000)))
            .with_validator(Validator::Min(100.0))
            .with_validator(Validator::Max(10000.0)),
        );

        // UI 配置
        self.add(
            ConfigSchema::new(
                "ui.theme",
                ConfigValueType::Enum(vec![
                    "light".to_string(),
                    "dark".to_string(),
                    "system".to_string(),
                ]),
                "主题",
            )
            .with_default(Value::String("system".to_string())),
        );

        self.add(
            ConfigSchema::new("ui.language", ConfigValueType::String, "语言")
                .with_default(Value::String("zh-CN".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "ui.sidebarPosition",
                ConfigValueType::Enum(vec!["left".to_string(), "right".to_string()]),
                "侧边栏位置",
            )
            .with_default(Value::String("left".to_string())),
        );

        self.add(
            ConfigSchema::new("ui.showStatusBar", ConfigValueType::Boolean, "显示状态栏")
                .with_default(Value::Bool(true)),
        );

        // Security 配置
        self.add(
            ConfigSchema::new(
                "security.sandboxMode",
                ConfigValueType::Enum(vec![
                    "strict".to_string(),
                    "normal".to_string(),
                    "permissive".to_string(),
                ]),
                "沙箱模式",
            )
            .with_default(Value::String("strict".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "security.requireWorktreeTrust",
                ConfigValueType::Boolean,
                "要求worktree 信任",
            )
            .with_default(Value::Bool(true)),
        );

        // RAG 配置
        self.add(
            ConfigSchema::new("rag.enabled", ConfigValueType::Boolean, "启用 RAG")
                .with_default(Value::Bool(true)),
        );

        self.add(
            ConfigSchema::new("rag.autoIndex", ConfigValueType::Boolean, "自动索引")
                .with_default(Value::Bool(true)),
        );

        self.add(
            ConfigSchema::new("rag.topK", ConfigValueType::Number, "Top K 结果数")
                .with_default(Value::Number(serde_json::Number::from(5)))
                .with_validator(Validator::Min(1.0))
                .with_validator(Validator::Max(20.0)),
        );

        // Logger 配置
        self.add(
            ConfigSchema::new(
                "logger.level",
                ConfigValueType::Enum(vec![
                    "debug".to_string(),
                    "info".to_string(),
                    "warn".to_string(),
                    "error".to_string(),
                ]),
                "日志级别",
            )
            .with_default(Value::String("info".to_string())),
        );

        self.add(
            ConfigSchema::new(
                "logger.fileEnabled",
                ConfigValueType::Boolean,
                "启用文件日志",
            )
            .with_default(Value::Bool(true)),
        );

        self.add(
            ConfigSchema::new(
                "logger.consoleEnabled",
                ConfigValueType::Boolean,
                "启用控制台日志",
            )
            .with_default(Value::Bool(true)),
        );

        tracing::info!(
            count = self.schemas.len(),
            "Built-in config schemas registered"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_schema_validate_string() {
        let schema = ConfigSchema::new("test.key", ConfigValueType::String, "Test");

        assert!(schema
            .validate_value(&Value::String("hello".to_string()))
            .is_ok());
        assert!(schema
            .validate_value(&Value::Number(serde_json::Number::from(42)))
            .is_err());
    }

    #[test]
    fn test_config_schema_validate_number() {
        let schema = ConfigSchema::new("test.key", ConfigValueType::Number, "Test")
            .with_validator(Validator::Min(0.0))
            .with_validator(Validator::Max(100.0));

        assert!(schema
            .validate_value(&Value::Number(serde_json::Number::from(50)))
            .is_ok());
        assert!(schema
            .validate_value(&Value::Number(serde_json::Number::from(-1i64)))
            .is_err());
        assert!(schema
            .validate_value(&Value::Number(serde_json::Number::from(101)))
            .is_err());
    }

    #[test]
    fn test_config_schema_validate_enum() {
        let schema = ConfigSchema::new(
            "test.key",
            ConfigValueType::Enum(vec!["option1".to_string(), "option2".to_string()]),
            "Test",
        );

        assert!(schema
            .validate_value(&Value::String("option1".to_string()))
            .is_ok());
        assert!(schema
            .validate_value(&Value::String("option3".to_string()))
            .is_err());
    }

    #[test]
    fn test_schema_index() {
        let index = SchemaIndex::default();

        // 检查内置 Schema 是否注册
        assert!(index.get("gateway.defaultModel").is_some());
        assert!(index.get("editor.fontSize").is_some());
        assert!(index.get("nonexistent.key").is_none());
    }

    #[test]
    fn test_schema_index_validate() {
        let index = SchemaIndex::default();

        // 有效值
        assert!(index
            .validate("gateway.defaultModel", &Value::String("test".to_string()))
            .is_ok());

        // 无效值
        assert!(index
            .validate(
                "gateway.defaultModel",
                &Value::Number(serde_json::Number::from(42))
            )
            .is_err());
    }
}
