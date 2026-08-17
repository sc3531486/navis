//! Auth 身份认证模块
//!
//! 基于设计文档 §05 实现，管理所有认证凭据，包括 LLM API Key、Git 凭证、第三方服务 Token。
//! 提供加密存储、有效性校验、过期提醒能力。
//!
//! # 子模块
//! - `key_store` - API Key 存储（加密）
//! - `key_validator` - 密钥有效性校验
//! - `credential` - Git/第三方凭证
//! - `provider_keys` - Provider 分组管理
//! - `schema_check` - Auth root schema verification

pub mod credential;
pub mod key_store;
pub mod key_validator;
pub mod provider_keys;
pub mod schema_check;

mod store;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::app::infra::Encryption;
use crate::kernel::{EventBus, EventEnvelope, KernelContext, KernelScope};
use store::AuthStore;
use triomphe::Arc as SharedArc;
use zeroize::Zeroize;

// ============================================================================
// 数据模型
// ============================================================================

/// API Key 信息
///
/// 存储 LLM Provider 的 API Key 元数据，密钥本身使用 AES-256-GCM 加密存储。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// 唯一 ID
    pub id: String,
    /// Provider 类型（openai / anthropic / custom）
    pub provider: String,
    /// 用户自定义名称
    pub name: String,
    /// AES-256-GCM 加密后的密钥（base64 编码）
    #[serde(skip_serializing)]
    pub key_encrypted: Vec<u8>,
    /// 自定义 API 地址
    pub base_url: Option<String>,
    /// 最近一次校验结果
    pub is_valid: ValidationStatus,
    /// 最后校验时间
    pub last_validated: Option<DateTime<Utc>>,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// Git 凭证信息
///
/// 存储 Git 仓库的认证凭证，支持用户名/密码、SSH Key、Token 三种方式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCredential {
    /// 唯一 ID
    pub id: String,
    /// 匹配模式（如 "github.com/*"）
    pub repo_pattern: String,
    /// 凭证类型
    pub credential_type: CredentialType,
    /// 用户名（可选）
    pub username: Option<String>,
    /// 加密后的密钥/密码/token（base64 编码）
    #[serde(skip_serializing)]
    pub secret_encrypted: Vec<u8>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 凭证类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    /// 用户名/密码
    UsernamePassword,
    /// SSH Key
    SshKey,
    /// Token
    Token,
}

impl CredentialType {
    /// 从字符串解析凭证类型
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "username_password" | "password" => Ok(Self::UsernamePassword),
            "ssh_key" | "ssh" => Ok(Self::SshKey),
            "token" => Ok(Self::Token),
            _ => anyhow::bail!("不支持的凭证类型: {}", s),
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UsernamePassword => "username_password",
            Self::SshKey => "ssh_key",
            Self::Token => "token",
        }
    }
}

/// 密钥校验状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    /// 未校验或校验超时
    Unknown,
    /// endpoint 可达，但尚未完成协议级鉴权
    Reachable,
    /// 校验通过
    Valid,
    /// 校验失败（密钥无效或已过期）
    Invalid,
}

impl ValidationStatus {
    /// 从字符串解析校验状态
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "reachable" => Self::Reachable,
            "valid" => Self::Valid,
            "invalid" => Self::Invalid,
            _ => Self::Unknown,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Reachable => "reachable",
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 生成唯一 ID
fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// 生成掩码后的密钥显示文本
///
/// 显示前 3 个字符和后 3 个字符，中间用 `***` 替代。
/// 例如: "sk-abc123xyz" -> "sk-***xyz"
///
/// # Arguments
/// * `key` - 原始密钥
///
/// # Returns
/// 掩码后的字符串
pub fn mask_key(key: &str) -> String {
    if key.len() <= 6 {
        return "***".to_string();
    }
    format!("{}***{}", &key[..3], &key[key.len() - 3..])
}

// ============================================================================
// Auth 主结构体
// ============================================================================

/// Auth 认证管理器
///
/// 管理所有认证凭据，提供 API Key 和 Git 凭证的 CRUD 操作，
/// 支持密钥加密存储、有效性校验、过期提醒和密钥轮转。
pub struct Auth {
    /// Security 域专属认证存储能力。
    store: AuthStore,
    /// 事件总线
    event_bus: Arc<dyn EventBus>,
}

/// Gateway 等宿主能力读取密钥时使用的最小端口。
///
/// 业务模块只接触 opaque secret_ref，不能依赖 AuthStore 或数据库实现。
pub trait SecretResolver: Send + Sync {
    fn resolve_secret(&self, secret_ref: &str) -> Result<Option<SecretValue>>;
}

/// 只在请求执行的最小范围内存在的明文 secret。
///
/// 该类型不提供序列化或日志接口，并在离开作用域时清除底层字符串。
pub struct SecretValue(String);

impl SecretValue {
    pub(crate) fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretValue(REDACTED)")
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl SecretResolver for Auth {
    fn resolve_secret(&self, secret_ref: &str) -> Result<Option<SecretValue>> {
        let secret_ref = secret_ref.trim();
        if !key_validator::validate_secret_ref(secret_ref) {
            return Ok(None);
        }
        self.store
            .get_decrypted_key(secret_ref)
            .map(|secret| secret.map(SecretValue::new))
    }
}

/// 仅供不需要认证的测试 Gateway 使用，生产装配必须注入 Auth。
#[cfg(test)]
#[derive(Debug, Default)]
pub struct NoopSecretResolver;

#[cfg(test)]
impl SecretResolver for NoopSecretResolver {
    fn resolve_secret(&self, _secret_ref: &str) -> Result<Option<SecretValue>> {
        Ok(None)
    }
}

impl Auth {
    /// 打开 Auth 专属存储并创建认证管理器。
    pub fn open(
        db_path: &Path,
        encryption: Option<Encryption>,
        event_bus: Arc<dyn EventBus>,
    ) -> Result<Self> {
        let store = AuthStore::open(db_path, encryption)?;
        Ok(Self::from_store(store, event_bus))
    }

    fn from_store(store: AuthStore, event_bus: Arc<dyn EventBus>) -> Self {
        tracing::info!("Auth module initialized");
        Self { store, event_bus }
    }

    // ----------------------------------------------------------------
    // API Key 管理
    // ----------------------------------------------------------------

    /// 添加 API Key
    ///
    /// # Arguments
    /// * `provider` - Provider 类型
    /// * `name` - 用户自定义名称
    /// * `key` - 明文密钥（base64 解码后的值）
    /// * `base_url` - 自定义 API 地址（可选）
    ///
    /// # Returns
    /// 创建的 ApiKey 信息
    pub fn add_key(
        &self,
        provider: &str,
        name: &str,
        key: &str,
        base_url: Option<&str>,
    ) -> Result<ApiKey> {
        if let Some(base_url) = base_url {
            key_validator::parse_base_url(base_url).map_err(anyhow::Error::msg)?;
        }
        let api_key = self.store.insert_key(provider, name, key, base_url)?;

        // 设置为该 Provider 的活跃密钥
        self.store.set_active_key(provider, &api_key.id)?;

        // 发布事件
        let event = EventEnvelope::new(
            "auth.key.added",
            KernelContext::new("auth", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "id": api_key.id,
                "provider": provider,
                "name": name,
            }))),
        );
        if let Err(error) = self.event_bus.emit(event) {
            tracing::warn!(error = %error, "Failed to emit auth.key.added event");
        }

        tracing::info!(
            key_id = %api_key.id,
            provider = provider,
            name = name,
            "API key added"
        );

        Ok(api_key)
    }

    /// 获取解密后的 API Key
    ///
    /// # Arguments
    /// * `id` - Key ID
    ///
    /// # Returns
    /// 解密后的明文密钥，如果解密失败返回 None
    pub fn get_key(&self, id: &str) -> Result<Option<SecretValue>> {
        self.store
            .get_decrypted_key(id)
            .map(|secret| secret.map(SecretValue::new))
    }

    /// 获取指定 Provider 的当前活跃密钥
    ///
    /// Gateway 按 provider 获取当前活跃密钥。
    ///
    /// # Arguments
    /// * `provider` - Provider 类型
    ///
    /// # Returns
    /// 解密后的明文密钥
    pub fn get_active_key(&self, provider: &str) -> Result<Option<SecretValue>> {
        self.store
            .get_active_decrypted_key(provider)
            .map(|secret| secret.map(SecretValue::new))
    }

    /// 获取指定 Provider 的所有 API Keys
    ///
    /// # Arguments
    /// * `provider` - Provider 类型
    pub fn get_keys_by_provider(&self, provider: &str) -> Result<Vec<ApiKey>> {
        self.store.get_keys_by_provider(provider)
    }

    /// 列出所有 API Keys
    pub fn list_keys(&self) -> Result<Vec<ApiKey>> {
        self.store.list_keys()
    }

    /// 删除 API Key
    ///
    /// # Arguments
    /// * `id` - Key ID
    pub fn remove_key(&self, id: &str) -> Result<()> {
        // 获取 key 信息用于事件
        let key_info = self.store.get_key(id)?;
        let (provider, name) = match &key_info {
            Some(k) => (k.provider.clone(), k.name.clone()),
            None => (String::new(), String::new()),
        };

        self.store.delete_key(id)?;

        // 发布事件
        let event = EventEnvelope::new(
            "auth.key.removed",
            KernelContext::new("auth", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "id": id,
                "provider": provider,
            }))),
        );
        if let Err(error) = self.event_bus.emit(event) {
            tracing::warn!(error = %error, "Failed to emit auth.key.removed event");
        }

        tracing::info!(
            key_id = id,
            provider = %provider,
            name = %name,
            "API key removed"
        );

        Ok(())
    }

    /// 校验 API Key 有效性
    ///
    /// # Arguments
    /// * `id` - Key ID
    ///
    /// # Returns
    /// 校验状态
    pub async fn validate_key(&self, id: &str) -> Result<ValidationStatus> {
        let validator = key_validator::ReqwestSecretValidator::new()?;
        self.validate_key_with_validator(id, &validator).await
    }

    pub async fn validate_key_with_validator(
        &self,
        id: &str,
        validator: &dyn key_validator::SecretValidator,
    ) -> Result<ValidationStatus> {
        let key_info = self
            .store
            .get_key(id)?
            .ok_or_else(|| anyhow::anyhow!("API Key 不存在: {}", id))?;

        let secret = self.store.get_decrypted_key(id)?.map(SecretValue::new);

        let status = match secret {
            Some(secret) => {
                validator
                    .validate(&key_info.provider, &secret, key_info.base_url.as_deref())
                    .await
            }
            None => {
                tracing::warn!(key_id = id, "Failed to decrypt key for validation");
                ValidationStatus::Unknown
            }
        };

        self.store.update_validation_status(id, &status)?;

        let event = EventEnvelope::new(
            "auth.key.validated",
            KernelContext::new("auth", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "id": id,
                "status": status.as_str(),
            }))),
        );
        if let Err(error) = self.event_bus.emit(event) {
            tracing::warn!(error = %error, "Failed to emit auth.key.validated event");
        }

        if status == ValidationStatus::Invalid {
            let event = EventEnvelope::new(
                "auth.key.invalid",
                KernelContext::new("auth", KernelScope::global()),
                Some(SharedArc::new(serde_json::json!({
                    "id": id,
                    "provider": key_info.provider,
                    "error": "Key validation failed",
                }))),
            );
            if let Err(error) = self.event_bus.emit(event) {
                tracing::warn!(error = %error, "Failed to emit auth.key.invalid event");
            }
        }

        tracing::info!(
            key_id = id,
            status = ?status,
            "API key validated"
        );

        Ok(status)
    }
    /// 密钥轮转：更新密钥并重新加密存储
    ///
    /// # Arguments
    /// * `id` - Key ID
    /// * `new_key` - 新的明文密钥
    pub fn rotate_key(&self, id: &str, new_key: &str) -> Result<()> {
        self.store.rotate_key(id, new_key)?;

        tracing::info!(key_id = id, "API key rotated");
        Ok(())
    }

    // ----------------------------------------------------------------
    // Git 凭证管理
    // ----------------------------------------------------------------

    /// 添加 Git 凭证
    ///
    /// # Arguments
    /// * `pattern` - 仓库匹配模式
    /// * `cred` - Git 凭证信息
    pub fn add_git_credential(&self, pattern: &str, cred: GitCredential) -> Result<()> {
        self.store.insert_credential(pattern, &cred)?;

        // 发布事件
        let event = EventEnvelope::new(
            "auth.git.cred.added",
            KernelContext::new("auth", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "id": cred.id,
                "pattern": pattern,
            }))),
        );
        if let Err(error) = self.event_bus.emit(event) {
            tracing::warn!(error = %error, "Failed to emit auth.git.cred.added event");
        }

        tracing::info!(
            cred_id = %cred.id,
            pattern = pattern,
            "Git credential added"
        );

        Ok(())
    }

    /// 根据仓库 URL 获取匹配的 Git 凭证
    ///
    /// # Arguments
    /// * `repo_url` - 仓库 URL
    ///
    /// # Returns
    /// 匹配的凭证（解密后的密钥已填充到返回值中）
    pub fn get_git_credential(&self, repo_url: &str) -> Result<Option<GitCredential>> {
        self.store.find_matching_credential(repo_url)
    }

    /// 列出所有 Git 凭证
    pub fn list_git_credentials(&self) -> Result<Vec<GitCredential>> {
        self.store.list_credentials()
    }

    /// 删除 Git 凭证
    ///
    /// # Arguments
    /// * `id` - 凭证 ID
    pub fn remove_git_credential(&self, id: &str) -> Result<()> {
        self.store.delete_credential(id)?;

        // 发布事件
        let event = EventEnvelope::new(
            "auth.git.cred.removed",
            KernelContext::new("auth", KernelScope::global()),
            Some(SharedArc::new(serde_json::json!({
                "id": id,
            }))),
        );
        if let Err(error) = self.event_bus.emit(event) {
            tracing::warn!(error = %error, "Failed to emit auth.git.cred.removed event");
        }

        tracing::info!(cred_id = id, "Git credential removed");

        Ok(())
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::infra::db::Database;
use crate::app::infra::Encryption;

    fn test_event_bus() -> Arc<dyn crate::kernel::EventBus> {
        static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
        let runtime = RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
        });
        Arc::new(crate::kernel::InMemoryEventBus::new(
            1000,
            runtime.handle().clone(),
        ))
    }

    /// 创建测试用 Auth 实例
    fn create_test_auth() -> Auth {
        let conn = Database::open_memory().unwrap();
        let conn = Arc::new(std::sync::Mutex::new(conn));
        let event_bus = test_event_bus();
        let encryption = Encryption::new(&vec![0u8; 32]).unwrap();
        Auth::from_store(
            AuthStore::from_connection(conn, Some(encryption)),
            event_bus,
        )
    }

    #[test]
    fn test_auth_add_and_get_key() {
        let auth = create_test_auth();

        let api_key = auth
            .add_key("openai", "My GPT Key", "sk-test123456789", None)
            .unwrap();

        assert_eq!(api_key.provider, "openai");
        assert_eq!(api_key.name, "My GPT Key");
        assert!(api_key.base_url.is_none());
        assert_eq!(api_key.is_valid, ValidationStatus::Unknown);

        // 获取解密后的密钥
        let decrypted = auth.get_key(&api_key.id).unwrap();
        assert_eq!(
            decrypted.as_ref().map(SecretValue::as_str),
            Some("sk-test123456789")
        );
    }

    #[test]
    fn test_auth_add_key_with_base_url() {
        let auth = create_test_auth();

        let api_key = auth
            .add_key(
                "custom",
                "Custom LLM",
                "my-key-123",
                Some("https://api.example.com"),
            )
            .unwrap();

        assert_eq!(
            api_key.base_url,
            Some("https://api.example.com".to_string())
        );
    }

    #[test]
    fn test_auth_list_keys() {
        let auth = create_test_auth();

        auth.add_key("openai", "Key 1", "sk-key1", None).unwrap();
        auth.add_key("anthropic", "Key 2", "sk-ant-key2", None)
            .unwrap();
        auth.add_key("openai", "Key 3", "sk-key3", None).unwrap();

        let all_keys = auth.list_keys().unwrap();
        assert_eq!(all_keys.len(), 3);
    }

    #[test]
    fn test_auth_get_keys_by_provider() {
        let auth = create_test_auth();

        auth.add_key("openai", "Key 1", "sk-key1", None).unwrap();
        auth.add_key("anthropic", "Key 2", "sk-ant-key2", None)
            .unwrap();
        auth.add_key("openai", "Key 3", "sk-key3", None).unwrap();

        let openai_keys = auth.get_keys_by_provider("openai").unwrap();
        assert_eq!(openai_keys.len(), 2);
        assert!(openai_keys.iter().all(|k| k.provider == "openai"));

        let anthropic_keys = auth.get_keys_by_provider("anthropic").unwrap();
        assert_eq!(anthropic_keys.len(), 1);
    }

    #[test]
    fn test_auth_remove_key() {
        let auth = create_test_auth();

        let api_key = auth
            .add_key("openai", "To Remove", "sk-remove-me", None)
            .unwrap();
        assert_eq!(auth.list_keys().unwrap().len(), 1);

        auth.remove_key(&api_key.id).unwrap();
        assert_eq!(auth.list_keys().unwrap().len(), 0);
    }

    #[test]
    fn test_auth_get_active_key() {
        let auth = create_test_auth();

        let k1 = auth.add_key("openai", "Key 1", "sk-key1", None).unwrap();
        auth.add_key("openai", "Key 2", "sk-key2", None).unwrap();

        // 活跃密钥应该是最新的
        let active = auth.get_active_key("openai").unwrap();
        assert!(active.is_some());
        // 最新的 key 应该是最后添加的
        assert_eq!(active.unwrap().as_str(), "sk-key2");

        // 验证 k1 仍然是某个 provider 的 key
        let all_openai = auth.get_keys_by_provider("openai").unwrap();
        assert!(all_openai.iter().any(|k| k.id == k1.id));
    }

    #[test]
    fn test_auth_rotate_key() {
        let auth = create_test_auth();

        let api_key = auth
            .add_key("openai", "Rotate Me", "sk-old-key", None)
            .unwrap();

        // 轮转密钥
        auth.rotate_key(&api_key.id, "sk-new-key").unwrap();

        // 获取密钥应返回新值
        let decrypted = auth.get_key(&api_key.id).unwrap();
        assert_eq!(
            decrypted.as_ref().map(SecretValue::as_str),
            Some("sk-new-key")
        );

        // 校验状态应为 Unknown
        let key_info = auth.store.get_key(&api_key.id).unwrap().unwrap();
        assert_eq!(key_info.is_valid, ValidationStatus::Unknown);
    }

    struct FakeValidator {
        status: ValidationStatus,
    }

    #[async_trait::async_trait]
    impl key_validator::SecretValidator for FakeValidator {
        async fn validate(
            &self,
            _provider: &str,
            _secret: &SecretValue,
            _base_url: Option<&str>,
        ) -> ValidationStatus {
            self.status
        }
    }

    #[tokio::test]
    async fn test_auth_validate_key_with_injected_validator() {
        let auth = create_test_auth();
        let api_key = auth
            .add_key("openai", "Validate Me", "sk-test123", None)
            .unwrap();

        let status = auth
            .validate_key_with_validator(
                &api_key.id,
                &FakeValidator {
                    status: ValidationStatus::Reachable,
                },
            )
            .await
            .unwrap();
        assert_eq!(status, ValidationStatus::Reachable);
        assert_eq!(
            auth.store.get_key(&api_key.id).unwrap().unwrap().is_valid,
            status
        );
    }

    #[test]
    fn test_secret_value_debug_is_redacted() {
        let secret = SecretValue::new("super-secret-value".to_string());
        let debug = format!("{secret:?}");
        assert_eq!(debug, "SecretValue(REDACTED)");
        assert!(!debug.contains("super-secret-value"));
    }
    #[test]
    fn test_auth_add_and_get_git_credential() {
        let auth = create_test_auth();

        let cred = GitCredential {
            id: generate_id(),
            repo_pattern: "github.com/*".to_string(),
            credential_type: CredentialType::Token,
            username: None,
            secret_encrypted: Vec::new(),
            created_at: Utc::now(),
        };

        auth.add_git_credential("github.com/*", cred).unwrap();

        // 通过 URL 匹配获取凭证
        let found = auth.get_git_credential("github.com/user/repo.git").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.credential_type, CredentialType::Token);
    }

    #[test]
    fn test_auth_list_git_credentials() {
        let auth = create_test_auth();

        for i in 0..3 {
            let cred = GitCredential {
                id: generate_id(),
                repo_pattern: format!("host{}.com/*", i),
                credential_type: CredentialType::Token,
                username: None,
                secret_encrypted: Vec::new(),
                created_at: Utc::now(),
            };
            auth.add_git_credential(&cred.repo_pattern.clone(), cred)
                .unwrap();
        }

        let all_creds = auth.list_git_credentials().unwrap();
        assert_eq!(all_creds.len(), 3);
    }

    #[test]
    fn test_auth_remove_git_credential() {
        let auth = create_test_auth();

        let cred = GitCredential {
            id: generate_id(),
            repo_pattern: "github.com/*".to_string(),
            credential_type: CredentialType::SshKey,
            username: Some("git".to_string()),
            secret_encrypted: Vec::new(),
            created_at: Utc::now(),
        };
        let cred_id = cred.id.clone();
        auth.add_git_credential("github.com/*", cred).unwrap();

        assert_eq!(auth.list_git_credentials().unwrap().len(), 1);

        auth.remove_git_credential(&cred_id).unwrap();
        assert_eq!(auth.list_git_credentials().unwrap().len(), 0);
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("sk-abc123xyz"), "sk-***xyz");
        assert_eq!(mask_key("short"), "***");
        assert_eq!(mask_key("123456"), "***");
        assert_eq!(mask_key("1234567"), "123***567");
    }

    #[test]
    fn test_validation_status_serializes_as_lowercase_contract() {
        for (status, expected) in [
            (ValidationStatus::Unknown, "unknown"),
            (ValidationStatus::Reachable, "reachable"),
            (ValidationStatus::Valid, "valid"),
            (ValidationStatus::Invalid, "invalid"),
        ] {
            assert_eq!(
                serde_json::to_string(&status).unwrap(),
                format!("\"{expected}\"")
            );
            let decoded = serde_json::from_str::<ValidationStatus>(&format!("\"{expected}\""))
                .expect("validation status JSON should decode");
            assert_eq!(decoded, status);
        }
    }
    #[test]
    fn test_validation_status_from_str() {
        assert_eq!(ValidationStatus::from_str("valid"), ValidationStatus::Valid);
        assert_eq!(
            ValidationStatus::from_str("invalid"),
            ValidationStatus::Invalid
        );
        assert_eq!(
            ValidationStatus::from_str("unknown"),
            ValidationStatus::Unknown
        );
        assert_eq!(
            ValidationStatus::from_str("other"),
            ValidationStatus::Unknown
        );
    }

    #[test]
    fn test_credential_type_from_str() {
        assert_eq!(
            CredentialType::from_str("token").unwrap(),
            CredentialType::Token
        );
        assert_eq!(
            CredentialType::from_str("ssh_key").unwrap(),
            CredentialType::SshKey
        );
        assert_eq!(
            CredentialType::from_str("username_password").unwrap(),
            CredentialType::UsernamePassword
        );
        assert!(CredentialType::from_str("unknown").is_err());
    }
}
