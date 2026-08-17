//! Config 配置管理模块
//!
//! 基于设计文档 §2 实现，提供多层级配置管理、配置读写、热更新、校验、导入导出能力
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）
//!
//! # 依赖
//! - `tracing`（日志）
//! - `crate::kernel::EventBus`（配置变更通知）

pub mod exporter;
pub mod loader;
pub mod root_version;
pub mod schema;
pub mod store;
pub mod validator;

pub use exporter::ConfigExporter;
pub use loader::ConfigLoader;
pub use root_version::{ensure_current_config_version, CURRENT_CONFIG_VERSION};
pub use schema::{ConfigSchema, ConfigValueType};
pub use store::ConfigStore;
pub use validator::ConfigValidator;

use anyhow::Result;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::kernel::{EventBus, EventEnvelope, KernelContext, KernelScope, Topic};
use triomphe::Arc as SharedArc;

/// 配置来源
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigSource {
    /// 系统默认
    System,
    /// 用户配置
    User,
    /// 项目配置
    Project,
    /// 模式配置
    Mode,
    /// 运行时
    Runtime,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::System => write!(f, "system"),
            ConfigSource::User => write!(f, "user"),
            ConfigSource::Project => write!(f, "project"),
            ConfigSource::Mode => write!(f, "mode"),
            ConfigSource::Runtime => write!(f, "runtime"),
        }
    }
}

/// 配置条目
#[derive(Debug, Clone)]
pub struct ConfigEntry {
    /// 配置键（点号路径）
    pub key: String,
    /// 配置值
    pub value: Value,
    /// 来源层级
    pub source: ConfigSource,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 配置变更信息
#[derive(Debug, Clone)]
pub struct ConfigChange {
    /// 配置键
    pub key: String,
    /// 新值
    pub new_value: Value,
    /// 旧值
    pub old_value: Option<Value>,
    /// 来源
    pub source: ConfigSource,
}

/// 配置校验错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// 配置键
    pub key: String,
    /// 错误信息
    pub message: String,
}

/// 导出格式
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    /// JSON 格式
    Json,
    /// TOML 格式
    Toml,
    /// YAML 格式
    Yaml,
}

impl ExportFormat {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "toml" => Some(ExportFormat::Toml),
            "yaml" | "yml" => Some(ExportFormat::Yaml),
            _ => None,
        }
    }
}

/// 配置管理器
pub struct Config {
    /// 配置存储
    store: ConfigStore,
    /// 配置校验器
    validator: ConfigValidator,
    /// 配置加载器
    loader: ConfigLoader,
    /// 配置导出器
    exporter: ConfigExporter,
    /// 事件总线
    event_bus: Arc<dyn EventBus>,
    /// 用户配置路径
    user_config_path: Option<PathBuf>,
    /// 项目配置路径
    project_config_path: Option<PathBuf>,
}

impl Config {
    /// 创建新的配置管理器
    ///
    /// # Arguments
    /// * `event_bus` - 事件总线
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        tracing::info!("Creating new Config manager");

        Self {
            store: ConfigStore::new(),
            validator: ConfigValidator::new(),
            loader: ConfigLoader::new(),
            exporter: ConfigExporter::new(),
            event_bus,
            user_config_path: None,
            project_config_path: None,
        }
    }

    /// 加载用户配置
    ///
    /// # Arguments
    /// * `path` - 用户配置文件路径
    pub fn load_user_config(&mut self, path: &Path) -> Result<()> {
        tracing::info!(path = %path.display(), "Loading user config");

        self.user_config_path = Some(path.to_path_buf());

        if path.exists() {
            let config = self.loader.load(path)?;
            ensure_current_config_version(&config, path)?;
            self.store.merge_config(config, ConfigSource::User);

            tracing::info!("User config loaded successfully");
        } else {
            tracing::info!("User config file not found, using defaults");
        }

        Ok(())
    }

    /// 加载项目配置
    ///
    /// # Arguments
    /// * `path` - 项目配置文件路径
    pub fn load_project_config(&mut self, path: &Path) -> Result<()> {
        tracing::info!(path = %path.display(), "Loading project config");

        self.project_config_path = Some(path.to_path_buf());

        if path.exists() {
            let config = self.loader.load(path)?;
            ensure_current_config_version(&config, path)?;
            self.store.merge_config(config, ConfigSource::Project);

            tracing::info!("Project config loaded successfully");
        } else {
            tracing::info!("Project config file not found, using defaults");
        }

        Ok(())
    }

    /// 获取配置值
    ///
    /// # Arguments
    /// * `key` - 配置键（点号路径，如 "gateway.defaultModel"）
    pub fn get(&self, key: &str) -> Option<Value> {
        self.store.get(key)
    }

    /// 获取配置值（带默认值）
    ///
    /// # Arguments
    /// * `key` - 配置键
    /// * `default` - 默认值
    pub fn get_or(&self, key: &str, default: Value) -> Value {
        self.store.get(key).unwrap_or(default)
    }

    /// 获取配置条目（含来源信息）
    ///
    /// # Arguments
    /// * `key` - 配置键
    pub fn get_source(&self, key: &str) -> Option<ConfigEntry> {
        self.store.get_entry(key)
    }

    /// 设置配置值
    ///
    /// # Arguments
    /// * `key` - 配置键
    /// * `value` - 配置值
    pub fn set(&mut self, key: &str, value: Value) -> Result<()> {
        tracing::debug!(key = %key, "Setting config value");

        // 校验配置
        self.validator
            .validate_key_value(key, &value)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // 获取旧值
        let old_value = self.store.get(key);

        // 设置新值
        self.store.set(key, value.clone(), ConfigSource::User);

        // 发送变更事件
        let change = ConfigChange {
            key: key.to_string(),
            new_value: value,
            old_value,
            source: ConfigSource::User,
        };

        self.emit_change_event(change);

        tracing::info!(key = %key, "Config value set successfully");
        Ok(())
    }

    /// 保存用户配置到最近一次 load_user_config 指定的路径。
    ///
    /// 该方法只写 User 来源配置，保持 System / Project / Runtime 分层不混写。
    pub fn save_user_config(&self) -> Result<()> {
        let path = self
            .user_config_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("User config path is not initialized"))?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = self.export(ConfigSource::User, ExportFormat::Json)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 设置配置值（指定来源）
    ///
    /// # Arguments
    /// * `key` - 配置键
    /// * `value` - 配置值
    /// * `source` - 配置来源
    pub fn set_to(&mut self, key: &str, value: Value, source: ConfigSource) -> Result<()> {
        tracing::debug!(key = %key, source = %source, "Setting config value with source");

        // 校验配置
        self.validator
            .validate_key_value(key, &value)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        // 获取旧值
        let old_value = self.store.get(key);

        // 设置新值
        self.store.set(key, value.clone(), source.clone());

        // 发送变更事件
        let change = ConfigChange {
            key: key.to_string(),
            new_value: value,
            old_value,
            source,
        };

        self.emit_change_event(change);

        tracing::info!(key = %key, "Config value set successfully");
        Ok(())
    }

    /// 删除配置值
    ///
    /// # Arguments
    /// * `key` - 配置键
    pub fn unset(&mut self, key: &str) -> Result<()> {
        tracing::debug!(key = %key, "Unsetting config value");

        // 获取旧值
        let old_value = self.store.get(key);

        // 删除配置
        self.store.unset(key);

        // 发送变更事件
        if let Some(old_value) = old_value {
            let change = ConfigChange {
                key: key.to_string(),
                new_value: Value::Null,
                old_value: Some(old_value),
                source: ConfigSource::User,
            };

            self.emit_change_event(change);
        }

        tracing::info!(key = %key, "Config value unset successfully");
        Ok(())
    }

    /// 订阅配置变更
    ///
    /// # Arguments
    /// * `key_pattern` - 配置键模式（支持通配符）
    /// * `handler` - 变更处理器
    pub fn on_change(
        &self,
        key_pattern: &str,
        handler: impl Fn(&ConfigChange) + Send + Sync + 'static,
    ) -> String {
        let pattern = key_pattern.to_string();

        let subscription = self.event_bus.subscribe(
            Some(Topic::from("config.changed")),
            None,
            Arc::new(move |event| {
                let payload_ref = event.payload.as_ref().map(|p| p.as_ref());
                if let Some(payload) = payload_ref {
                    if let Some(key) = payload.get("key").and_then(|v| v.as_str()) {
                        if Self::matches_pattern(key, &pattern) {
                            let change = ConfigChange {
                                key: key.to_string(),
                                new_value: payload.get("value").cloned().unwrap_or(Value::Null),
                                old_value: payload.get("oldValue").cloned(),
                                source: ConfigSource::User,
                            };
                            handler(&change);
                        }
                    }
                }
            }),
        );

        subscription
            .map(|id| id.into_string())
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "Failed to subscribe config changes");
                String::new()
            })
    }

    /// 校验所有配置
    pub fn validate(&self) -> Vec<ValidationError> {
        self.validator.validate_all(&self.store)
    }

    /// 校验指定配置键
    ///
    /// # Arguments
    /// * `key` - 配置键
    pub fn validate_key(&self, key: &str) -> Result<()> {
        if let Some(value) = self.store.get(key) {
            self.validator
                .validate_key_value(key, &value)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Ok(())
    }

    /// 导出配置
    ///
    /// # Arguments
    /// * `source` - 配置来源
    /// * `format` - 导出格式
    pub fn export(&self, source: ConfigSource, format: ExportFormat) -> Result<String> {
        tracing::info!(source = %source, format = ?format, "Exporting config");

        let mut config = self.store.get_source_config(source);
        Self::stamp_root_config_version(&mut config);
        self.exporter.export(&config, format)
    }

    /// 导入配置
    ///
    /// # Arguments
    /// * `data` - 配置数据
    /// * `format` - 数据格式
    /// * `target` - 目标来源
    pub fn import(&mut self, data: &str, format: ExportFormat, target: ConfigSource) -> Result<()> {
        tracing::info!(format = ?format, target = %target, "Importing config");

        let config = self.loader.parse(data, format)?;
        ensure_current_config_version(&config, Path::new("<import>"))?;
        self.store.merge_config(config, target);

        tracing::info!("Config imported successfully");
        Ok(())
    }

    /// 发送配置变更事件
    fn emit_change_event(&self, change: ConfigChange) {
        let payload = serde_json::json!({
            "key": change.key,
            "value": change.new_value,
            "oldValue": change.old_value,
            "source": change.source.to_string(),
        });

        if let Err(error) = self.event_bus.emit(EventEnvelope::new(
            "config.changed",
            KernelContext::new("config", KernelScope::global()),
            Some(SharedArc::new(payload)),
        )) {
            tracing::warn!(error = %error, "Failed to emit config.changed event");
        }
    }

    fn stamp_root_config_version(config: &mut Value) {
        if let Some(obj) = config.as_object_mut() {
            obj.insert(
                "configVersion".to_string(),
                Value::Number(serde_json::Number::from(CURRENT_CONFIG_VERSION)),
            );
        }
    }

    /// 通配符匹配
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        let regex_pattern = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("?", ".");

        if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            re.is_match(text)
        } else {
            text == pattern
        }
    }
}

// ============================================================================
// ConfigPolicyBridge：配置变更自动同步到 PolicyEngine
// ============================================================================

/// 配置变更与 PolicyEngine 约束同步的桥接器。
///
/// 在构造时注册 Sandbox 相关 Constraint 到 PolicyEngine，
/// 并订阅 `config.changed` 事件；当 sandbox 相关配置键变更时，
/// 自动调用 `PolicyEngine.replace()` 更新对应 Constraint，
/// 使策略引擎始终反映最新配置。
///
/// # 映射关系
///
/// | Config Key                    | Constraint ID        |
/// |-------------------------------|----------------------|
/// | `sandbox.command_rules`       | `sandbox.command`    |
/// | `sandbox.network_blacklist`   | `sandbox.network`    |
/// | `sandbox.approval_mode`       | `sandbox.path`       |
///
/// # 设计意图
///
/// ConfigPolicyBridge 是纯粹的应用层胶水代码，不引入新内核概念。
/// 它只做一件事：**监听 EventBus 事件 → 调用 PolicyEngine.replace() 更新 Constraint**。
pub struct ConfigPolicyBridge {
    /// 策略引擎（持有 Constraint 注册表）
    policy_engine: Arc<crate::kernel::PolicyEngine>,
    /// Sandbox 实例（Constraint 内部持有的共享引用）
    sandbox: Arc<crate::security::sandbox::Sandbox>,
}

impl ConfigPolicyBridge {
    /// 创建 ConfigPolicyBridge 并完成初始化：
    /// 1. 从 Config 读取 sandbox 相关配置，同步到 Sandbox 内部状态
    /// 2. 注册初始 Constraint 到 PolicyEngine
    /// 3. 订阅 `config.changed` 事件，监听 sandbox 相关键变更
    ///
    /// # Arguments
    /// * `policy_engine` - 全局 PolicyEngine 实例
    /// * `sandbox` - Sandbox 实例
    /// * `config` - Config 管理器（用于读取初始配置和订阅事件）
    pub fn new(
        policy_engine: Arc<crate::kernel::PolicyEngine>,
        sandbox: Arc<crate::security::sandbox::Sandbox>,
        config: &Config,
    ) -> Self {
        tracing::info!("ConfigPolicyBridge: initializing");

        // ── 1. 从 Config 读取 sandbox 相关配置，同步到 Sandbox 内部状态 ──

        if let Some(mode_value) = config.get("sandbox.approval_mode") {
            if let Some(mode_str) = mode_value.as_str() {
                match mode_str {
                    "suggest" => {
                        let _ = sandbox
                            .set_approval_mode(crate::security::sandbox::ApprovalMode::Suggest);
                    }
                    "auto_edit" | "autoedit" => {
                        let _ = sandbox
                            .set_approval_mode(crate::security::sandbox::ApprovalMode::AutoEdit);
                    }
                    "full_auto" | "fullauto" => {
                        let _ = sandbox
                            .set_approval_mode(crate::security::sandbox::ApprovalMode::FullAuto);
                    }
                    other => {
                        tracing::warn!(
                            value = other,
                            "ConfigPolicyBridge: unknown sandbox.approval_mode value, skipping"
                        );
                    }
                }
            }
        }

        if let Some(domains) = config.get("sandbox.network_blacklist") {
            if let Some(domain_arr) = domains.as_array() {
                let blocked: Vec<String> = domain_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !blocked.is_empty() {
                    let policy = crate::security::sandbox::NetworkPolicy::with_blocked(blocked);
                    let _ = sandbox.set_network_policy(policy);
                }
            }
        }

        // ── 2. 注册初始 Constraint 到 PolicyEngine ──

        if let Err(err) = crate::security::sandbox::constraint::register_sandbox_constraints(
            &policy_engine,
            Arc::clone(&sandbox),
        ) {
            tracing::warn!(error = %err, "ConfigPolicyBridge: initial constraint registration failed (may already be registered)");
        }

        // ── 3. 订阅 config.changed 事件 ──

        let pe_for_cmd = Arc::clone(&policy_engine);
        let sb_for_cmd = Arc::clone(&sandbox);
        config.on_change("sandbox.command_rules", move |_change| {
            tracing::info!("ConfigPolicyBridge: sandbox.command_rules changed, re-registering CommandConstraint");
            let constraint =
                crate::security::sandbox::constraint::CommandConstraint::new(Arc::clone(&sb_for_cmd));
            if let Err(err) = pe_for_cmd.replace(constraint) {
                tracing::warn!(error = %err, "ConfigPolicyBridge: failed to replace CommandConstraint");
            }
        });

        let pe_for_net = Arc::clone(&policy_engine);
        let sb_for_net = Arc::clone(&sandbox);
        config.on_change("sandbox.network_blacklist", move |_change| {
            tracing::info!("ConfigPolicyBridge: sandbox.network_blacklist changed, re-registering NetworkConstraint");
            let constraint =
                crate::security::sandbox::constraint::NetworkConstraint::new(Arc::clone(&sb_for_net));
            if let Err(err) = pe_for_net.replace(constraint) {
                tracing::warn!(error = %err, "ConfigPolicyBridge: failed to replace NetworkConstraint");
            }
        });

        let pe_for_mode = Arc::clone(&policy_engine);
        let sb_for_mode = Arc::clone(&sandbox);
        config.on_change("sandbox.approval_mode", move |change| {
            tracing::info!(
                new_value = %change.new_value,
                "ConfigPolicyBridge: sandbox.approval_mode changed, updating Sandbox and re-registering PathAccessConstraint"
            );
            // 同步 approval_mode 到 Sandbox 内部状态
            if let Some(mode_str) = change.new_value.as_str() {
                let mode = match mode_str {
                    "suggest" => Some(crate::security::sandbox::ApprovalMode::Suggest),
                    "auto_edit" | "autoedit" => Some(crate::security::sandbox::ApprovalMode::AutoEdit),
                    "full_auto" | "fullauto" => Some(crate::security::sandbox::ApprovalMode::FullAuto),
                    _ => None,
                };
                if let Some(m) = mode {
                    let _ = sb_for_mode.set_approval_mode(m);
                }
            }
            // 替换 PathAccessConstraint，确保 PolicyEngine 使用最新约束
            let constraint =
                crate::security::sandbox::constraint::PathAccessConstraint::new(Arc::clone(&sb_for_mode));
            if let Err(err) = pe_for_mode.replace(constraint) {
                tracing::warn!(error = %err, "ConfigPolicyBridge: failed to replace PathAccessConstraint");
            }
        });

        tracing::info!("ConfigPolicyBridge: initialized, subscribed to sandbox config changes");

        Self {
            policy_engine,
            sandbox,
        }
    }

    /// 获取内部 PolicyEngine 引用
    pub fn policy_engine(&self) -> &Arc<crate::kernel::PolicyEngine> {
        &self.policy_engine
    }

    /// 获取内部 Sandbox 引用
    pub fn sandbox(&self) -> &Arc<crate::security::sandbox::Sandbox> {
        &self.sandbox
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_config() -> Config {
        let event_bus = Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            tokio::runtime::Handle::current(),
        ));
        Config::new(event_bus)
    }

    #[tokio::test]
    async fn test_config_get_set() {
        let mut config = create_test_config();

        // 设置配置
        config
            .set("gateway.defaultModel", json!("claude-sonnet-4-6"))
            .unwrap();

        // 获取配置
        let value = config.get("gateway.defaultModel");
        assert_eq!(value, Some(json!("claude-sonnet-4-6")));
    }

    #[tokio::test]
    async fn test_config_get_or() {
        let config = create_test_config();

        // 获取不存在的配置，返回默认值
        let value = config.get_or("nonexistent.key", json!("default"));
        assert_eq!(value, json!("default"));
    }

    #[tokio::test]
    async fn test_config_unset() {
        let mut config = create_test_config();

        // 设置配置
        config.set("test.key", json!("value")).unwrap();
        assert!(config.get("test.key").is_some());

        // 删除配置
        config.unset("test.key").unwrap();
        assert!(config.get("test.key").is_none());
    }

    #[tokio::test]
    async fn test_config_export_import() {
        let mut config = create_test_config();

        // 设置配置
        config
            .set("gateway.defaultModel", json!("test-model"))
            .unwrap();

        // 导出
        let exported = config
            .export(ConfigSource::User, ExportFormat::Json)
            .unwrap();
        assert!(exported.contains("test-model"));
        assert!(exported.contains("configVersion"));

        // 导入到新的配置管理器
        let mut config2 = create_test_config();
        config2
            .import(&exported, ExportFormat::Json, ConfigSource::User)
            .unwrap();

        // 验证导入的配置
        let value = config2.get("gateway.defaultModel");
        assert_eq!(value, Some(json!("test-model")));
    }

    #[tokio::test]
    async fn test_config_import_rejects_mismatched_root_version() {
        let mut config = create_test_config();

        let err = config
            .import(
                r#"{"configVersion": 99, "gateway": {"defaultModel": "bad"}}"#,
                ExportFormat::Json,
                ConfigSource::User,
            )
            .unwrap_err();

        assert!(err.to_string().contains("配置文件根版本不匹配"));
    }
}
