//! 密钥、Provider endpoint 和 secret reference 的安全校验。
//!
//! 本模块集中定义认证校验所需的三类边界：本地输入校验、安全 URL 组合和协议级
//! HTTP 验证。网络传输通过 `ValidationTransport` 注入，生产环境使用无重定向的
//! reqwest 实现，测试可以使用完全离线的 fake transport。

use std::net::IpAddr;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, Url};
use zeroize::Zeroize;

use super::{SecretValue, ValidationStatus};

const MAX_BASE_URL_LENGTH: usize = 2048;
const MAX_ENDPOINT_LENGTH: usize = 512;
const MAX_SECRET_REF_LENGTH: usize = 256;
const VALIDATION_TIMEOUT_SECS: u64 = 20;

/// Validator 发出的最小 HTTP 请求合同。
#[derive(Debug, Clone)]
pub struct ValidationHttpRequest {
    pub url: Url,
    pub headers: HeaderMap,
}

/// Validator 只需要 HTTP 状态，不读取响应正文，避免把远端内容带入 secret 校验链路。
#[derive(Debug, Clone, Copy)]
pub struct ValidationHttpResponse {
    pub status: StatusCode,
}

/// 可替换的校验传输端口。
#[async_trait]
pub trait ValidationTransport: Send + Sync {
    async fn send(&self, request: ValidationHttpRequest) -> Result<ValidationHttpResponse>;
}

/// 可替换的 Provider secret validator。
#[async_trait]
pub trait SecretValidator: Send + Sync {
    async fn validate(
        &self,
        provider: &str,
        secret: &SecretValue,
        base_url: Option<&str>,
    ) -> ValidationStatus;
}

/// 使用 reqwest 的生产校验器。
pub struct ReqwestSecretValidator {
    transport: Arc<dyn ValidationTransport>,
}

impl ReqwestSecretValidator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            transport: reqwest_validation_transport()?,
        })
    }
}

/// Create the production validation transport used by security-owned and
/// Extension-owned provider validation adapters.
pub fn reqwest_validation_transport() -> Result<Arc<dyn ValidationTransport>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(VALIDATION_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    Ok(Arc::new(ReqwestValidationTransport { client }))
}

#[async_trait]
impl SecretValidator for ReqwestSecretValidator {
    async fn validate(
        &self,
        provider: &str,
        secret: &SecretValue,
        base_url: Option<&str>,
    ) -> ValidationStatus {
        validate_with_transport(self.transport.as_ref(), provider, secret, base_url).await
    }
}

struct ReqwestValidationTransport {
    client: reqwest::Client,
}

#[async_trait]
impl ValidationTransport for ReqwestValidationTransport {
    async fn send(&self, request: ValidationHttpRequest) -> Result<ValidationHttpResponse> {
        let response = self
            .client
            .get(request.url)
            .headers(request.headers)
            .send()
            .await?;
        Ok(ValidationHttpResponse {
            status: response.status(),
        })
    }
}

/// 使用注入的传输执行真实协议级校验。
pub async fn validate_with_transport(
    transport: &dyn ValidationTransport,
    provider: &str,
    secret: &SecretValue,
    base_url: Option<&str>,
) -> ValidationStatus {
    if !validate_key_format(provider, secret.as_str()) {
        return ValidationStatus::Invalid;
    }

    let Some(request) = build_validation_request(provider, secret, base_url) else {
        return ValidationStatus::Unknown;
    };

    match transport.send(request).await {
        Ok(response) if response.status.is_success() => ValidationStatus::Valid,
        Ok(response)
            if response.status == StatusCode::UNAUTHORIZED
                || response.status == StatusCode::FORBIDDEN =>
        {
            ValidationStatus::Invalid
        }
        Ok(_) => ValidationStatus::Reachable,
        Err(error) => {
            tracing::debug!(error = %error, provider, "Provider secret validation request failed");
            ValidationStatus::Unknown
        }
    }
}

fn build_validation_request(
    provider: &str,
    secret: &SecretValue,
    base_url: Option<&str>,
) -> Option<ValidationHttpRequest> {
    let base = resolve_api_url(provider, base_url)?;
    let url = join_endpoint(&base, "/v1/models").ok()?;
    let mut headers = HeaderMap::new();

    match provider {
        "openai" => {
            let value = format!("Bearer {}", secret.as_str());
            headers.insert("authorization", HeaderValue::from_str(&value).ok()?);
        }
        "anthropic" => {
            headers.insert("x-api-key", HeaderValue::from_str(secret.as_str()).ok()?);
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        }
        _ => return None,
    }

    Some(ValidationHttpRequest { url, headers })
}

/// 校验密钥格式。
pub fn validate_key_format(provider: &str, key: &str) -> bool {
    if key.is_empty() || key.len() > 8192 || key.chars().any(char::is_control) {
        return false;
    }

    match provider {
        "openai" => key.starts_with("sk-") && key.len() >= 10,
        "anthropic" => key.starts_with("sk-ant-") && key.len() >= 15,
        _ => key.len() >= 8,
    }
}

/// 校验 Auth Store 中使用的 opaque secret reference。
pub fn validate_secret_ref(secret_ref: &str) -> bool {
    let value = secret_ref.trim();
    !value.is_empty()
        && value.len() <= MAX_SECRET_REF_LENGTH
        && value == secret_ref
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '_' | '-' | '/')
        })
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("//")
        && !value.contains("..")
}

/// 解析并校验 Provider 的绝对基础 URL。
pub fn parse_base_url(value: &str) -> Result<Url, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_BASE_URL_LENGTH {
        return Err("base URL 为空或超过长度限制".to_string());
    }
    if value.chars().any(char::is_control) || value.contains(['%', '\\']) {
        return Err("base URL 包含控制字符、编码或反斜杠".to_string());
    }

    let url = Url::parse(value).map_err(|_| "base URL 必须是绝对 URL".to_string())?;
    if url.scheme() != "https" {
        return Err("Provider endpoint 只允许 HTTPS".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Provider endpoint 不允许 userinfo".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Provider base URL 不允许 query 或 fragment".to_string());
    }
    if url.host_str().is_none() {
        return Err("Provider base URL 缺少 host".to_string());
    }
    validate_path(url.path(), "base URL")?;
    validate_host(&url)?;
    validate_port(&url)?;
    Ok(url)
}

/// 将受控相对 endpoint 安全地组合到 base URL。
pub fn join_endpoint(base_url: &str, endpoint: &str) -> Result<Url, String> {
    let mut base = parse_base_url(base_url)?;
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Ok(base);
    }
    if endpoint.len() > MAX_ENDPOINT_LENGTH
        || endpoint.chars().any(char::is_control)
        || endpoint.contains(['%', '\\', '?', '#'])
        || endpoint.starts_with("//")
        || endpoint.contains("://")
    {
        return Err("endpoint 必须是有限的相对路径".to_string());
    }

    let endpoint_path = endpoint.trim_start_matches('/');
    validate_path(endpoint_path, "endpoint")?;
    if endpoint_path.is_empty() {
        return Ok(base);
    }

    let base_path = base.path().trim_end_matches('/');
    let base_path = if base_path == "/" { "" } else { base_path };
    let endpoint_path = if base_path.ends_with("/v1") {
        endpoint_path
            .strip_prefix("v1/")
            .or_else(|| endpoint_path.strip_prefix("v1"))
            .unwrap_or(endpoint_path)
    } else {
        endpoint_path
    };
    let joined_path = if endpoint_path.is_empty() {
        base_path.to_string()
    } else if base_path.is_empty() {
        format!("/{endpoint_path}")
    } else {
        format!("{base_path}/{endpoint_path}")
    };
    base.set_path(&joined_path);
    Ok(base)
}

/// 解析 API URL。未知 Provider 没有安全默认 endpoint 时返回 `None`。
pub fn resolve_api_url(provider: &str, base_url: Option<&str>) -> Option<String> {
    if let Some(base_url) = base_url {
        return safe_endpoint_string(base_url, "");
    }

    let default = match provider {
        "openai" => "https://api.openai.com",
        "anthropic" => "https://api.anthropic.com",
        _ => return None,
    };
    safe_endpoint_string(default, "")
}

/// 保持发现/展示调用方需要的字符串形式，但不返回未经校验的 URL。
pub fn safe_endpoint_string(base_url: &str, endpoint: &str) -> Option<String> {
    join_endpoint(base_url, endpoint)
        .ok()
        .map(|url| url.to_string().trim_end_matches('/').to_string())
}

fn validate_path(path: &str, label: &str) -> Result<(), String> {
    if path.len() > MAX_BASE_URL_LENGTH
        || path.contains(['%', '\\', '?', '#'])
        || path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(format!("{label} 包含不安全路径段"));
    }
    Ok(())
}

fn validate_host(url: &Url) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| "Provider endpoint 缺少 host".to_string())?;
    let normalized = host.trim_end_matches('.').to_ascii_lowercase();
    let blocked_names = [
        "localhost",
        "localhost.localdomain",
        "metadata.google.internal",
        "metadata",
        "instance-data",
        "host.docker.internal",
    ];
    if blocked_names
        .iter()
        .any(|blocked| normalized == *blocked || normalized.ends_with(&format!(".{blocked}")))
    {
        return Err("Provider endpoint 不允许指向本机或 metadata host".to_string());
    }

    if let Ok(address) = normalized.parse::<IpAddr>() {
        let blocked = match address {
            IpAddr::V4(address) => {
                address.is_loopback()
                    || address.is_private()
                    || address.is_unspecified()
                    || address.is_link_local()
                    || address.is_multicast()
                    || (address.octets()[0] == 169 && address.octets()[1] == 254)
            }
            IpAddr::V6(address) => {
                address.is_loopback()
                    || address.is_unspecified()
                    || address.is_multicast()
                    || (address.segments()[0] & 0xfe00) == 0xfc00
                    || (address.segments()[0] & 0xffc0) == 0xfe80
            }
        };
        if blocked {
            return Err("Provider endpoint 不允许指向本地或私网地址".to_string());
        }
    }
    Ok(())
}

fn validate_port(url: &Url) -> Result<(), String> {
    let Some(port) = url.port() else {
        return Ok(());
    };
    let dangerous = matches!(
        port,
        1..=22
            | 23
            | 25
            | 53
            | 110
            | 111
            | 135
            | 139
            | 445
            | 502
            | 1433
            | 2049
            | 2375
            | 2376
            | 3000
            | 3306
            | 3389
            | 5432
            | 5900
            | 6379
            | 9200
            | 11211
            | 27017
    );
    if dangerous {
        return Err(format!("Provider endpoint 端口 {port} 被安全策略拒绝"));
    }
    Ok(())
}

/// 使用 zeroize 清除内存中的明文密钥。
pub fn zeroize_string(mut key: String) {
    key.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeTransport {
        response: std::result::Result<ValidationHttpResponse, String>,
        requests: Mutex<Vec<ValidationHttpRequest>>,
    }

    #[async_trait]
    impl ValidationTransport for FakeTransport {
        async fn send(&self, request: ValidationHttpRequest) -> Result<ValidationHttpResponse> {
            self.requests.lock().unwrap().push(request);
            self.response.clone().map_err(anyhow::Error::msg)
        }
    }

    fn secret(value: &str) -> SecretValue {
        SecretValue::new(value.to_string())
    }

    #[tokio::test]
    async fn maps_protocol_status_to_validation_status() {
        for (status, expected) in [
            (StatusCode::OK, ValidationStatus::Valid),
            (StatusCode::UNAUTHORIZED, ValidationStatus::Invalid),
            (StatusCode::FORBIDDEN, ValidationStatus::Invalid),
            (StatusCode::NOT_FOUND, ValidationStatus::Reachable),
            (StatusCode::TOO_MANY_REQUESTS, ValidationStatus::Reachable),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                ValidationStatus::Reachable,
            ),
        ] {
            let transport = FakeTransport {
                response: Ok(ValidationHttpResponse { status }),
                requests: Mutex::new(Vec::new()),
            };
            let actual = validate_with_transport(
                &transport,
                "openai",
                &secret("sk-abcdefghijklmnopqrstuvwxyz"),
                None,
            )
            .await;
            assert_eq!(actual, expected);
        }
    }

    #[tokio::test]
    async fn transport_failure_is_unknown() {
        let transport = FakeTransport {
            response: Err("offline".to_string()),
            requests: Mutex::new(Vec::new()),
        };
        assert_eq!(
            validate_with_transport(
                &transport,
                "openai",
                &secret("sk-abcdefghijklmnopqrstuvwxyz"),
                None,
            )
            .await,
            ValidationStatus::Unknown
        );
    }

    #[tokio::test]
    async fn unknown_provider_and_unsafe_endpoint_never_send() {
        for (provider, base_url) in [
            ("extension-provider", None),
            ("openai", Some("http://evil.test")),
        ] {
            let transport = FakeTransport {
                response: Ok(ValidationHttpResponse {
                    status: StatusCode::OK,
                }),
                requests: Mutex::new(Vec::new()),
            };
            let status = validate_with_transport(
                &transport,
                provider,
                &secret("sk-abcdefghijklmnopqrstuvwxyz"),
                base_url,
            )
            .await;
            assert_eq!(status, ValidationStatus::Unknown);
            assert!(transport.requests.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn builds_provider_specific_validation_headers() {
        let transport = FakeTransport {
            response: Ok(ValidationHttpResponse {
                status: StatusCode::OK,
            }),
            requests: Mutex::new(Vec::new()),
        };
        assert_eq!(
            validate_with_transport(
                &transport,
                "anthropic",
                &secret("sk-ant-1234567890abcdef"),
                None,
            )
            .await,
            ValidationStatus::Valid
        );
        let request = transport.requests.lock().unwrap().pop().unwrap();
        assert_eq!(request.headers["x-api-key"], "sk-ant-1234567890abcdef");
        assert_eq!(request.headers["anthropic-version"], "2023-06-01");
        assert_eq!(request.url.as_str(), "https://api.anthropic.com/v1/models");
    }

    #[test]
    fn validates_known_key_formats() {
        assert!(validate_key_format(
            "openai",
            "sk-abcdefghijklmnopqrstuvwxyz"
        ));
        assert!(validate_key_format("anthropic", "sk-ant-1234567890abcdef"));
        assert!(validate_key_format(
            "extension-provider",
            "some-long-secret"
        ));
        assert!(!validate_key_format("openai", "sk-short"));
        assert!(!validate_key_format("custom", "bad\r\nvalue"));
    }

    #[test]
    fn accepts_safe_public_https_base_urls() {
        let url = parse_base_url("https://api.example.com/v1/").unwrap();
        assert_eq!(url.as_str(), "https://api.example.com/v1/");
        assert_eq!(
            resolve_api_url("openai", Some("https://api.example.com/v1/")),
            Some("https://api.example.com/v1".to_string())
        );
    }

    #[test]
    fn rejects_unsafe_base_urls() {
        for value in [
            "http://api.example.com",
            "https://user:pass@api.example.com",
            "https://api.example.com?token=secret",
            "https://api.example.com/#fragment",
            "https://api.example.com/%2e%2e/private",
            "https://localhost:8443",
            "https://127.0.0.1",
            "https://10.0.0.4",
            "https://169.254.169.254",
            "https://api.example.com:22",
            "https://api.example.com:2375",
        ] {
            assert!(
                parse_base_url(value).is_err(),
                "expected rejection: {value}"
            );
            assert_eq!(resolve_api_url("openai", Some(value)), None);
        }
    }

    #[test]
    fn joins_only_relative_endpoints_and_deduplicates_v1() {
        assert_eq!(
            join_endpoint("https://api.example.com/v1", "/v1/chat/completions")
                .unwrap()
                .as_str(),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://api.example.com/api", "models")
                .unwrap()
                .as_str(),
            "https://api.example.com/api/models"
        );
        for endpoint in [
            "https://evil.example/path",
            "//evil.example/path",
            "/safe/%2e%2e/escape",
            "/safe/../escape",
            "/safe?token=secret",
            "/safe#fragment",
            r"\evil\path",
        ] {
            assert!(join_endpoint("https://api.example.com/v1", endpoint).is_err());
        }
    }

    #[test]
    fn validates_opaque_secret_references() {
        assert!(validate_secret_ref("key:openai/123"));
        assert!(validate_secret_ref("550e8400-e29b-41d4-a716-446655440000"));
        for value in [
            "",
            " key",
            "key ",
            "/key",
            "key/",
            "key//value",
            "key/../value",
            "secret value",
        ] {
            assert!(!validate_secret_ref(value), "expected rejection: {value:?}");
        }
    }

    #[test]
    fn zeroizes_string_without_exposing_content() {
        zeroize_string("sensitive-api-key-12345".to_string());
    }
}
