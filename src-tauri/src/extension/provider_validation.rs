//! Extension-owned Gateway Provider validation.
//!
//! This module owns the validation contract for extension providers. It is a
//! deliberately small host-side port: extensions declare a bounded relative
//! endpoint and status mapping, while the host supplies the already-approved
//! auth profile, secret resolver, and HTTP transport. No provider name is
//! interpreted here, in Auth, or in the Gateway request path.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;

use crate::extension::models::ProviderAuthProfile;
use crate::security::auth::key_validator::{
    join_endpoint, parse_base_url, validate_secret_ref, ValidationHttpRequest, ValidationTransport,
};
use crate::security::auth::{SecretResolver, ValidationStatus};

use super::models::{GatewayProviderRegistration, GatewayProviderValidationRegistration};

const MAX_VALIDATION_TIMEOUT_MS: u64 = 30_000;
const MIN_VALIDATION_TIMEOUT_MS: u64 = 1;

/// Normalized, host-executable validation contract for one Extension Provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionProviderValidationRequest {
    /// Extension owner ID.
    pub owner: String,
    /// Canonical runtime Provider ID (`extension:{owner}/{provider}`).
    pub provider_id: String,
    /// Provider base URL after manifest validation.
    pub base_url: String,
    /// Opaque Auth Store reference. The secret itself never enters this DTO.
    pub secret_ref: Option<String>,
    /// Generic auth profile supplied by the existing Gateway contract.
    pub auth_profile: ProviderAuthProfile,
    /// Relative validation endpoint.
    pub endpoint: String,
    /// Status codes that prove successful credential validation.
    pub valid_status_codes: Vec<u16>,
    /// Status codes that prove credential rejection.
    pub invalid_status_codes: Vec<u16>,
    /// Maximum time allowed for this validation request.
    pub timeout_ms: u64,
}

impl ExtensionProviderValidationRequest {
    /// Build a normalized request from an Extension manifest declaration.
    pub fn from_manifest(
        owner: &str,
        provider: &GatewayProviderRegistration,
        provider_id: String,
    ) -> Result<Self> {
        validate_owner(owner)?;
        validate_provider_id(&provider_id)?;
        if provider_id != format!("extension:{owner}/{}", provider.id) {
            bail!("Gateway validation provider ID does not match its Extension owner");
        }

        let base_url = parse_base_url(&provider.base_url)
            .map_err(anyhow::Error::msg)
            .context("Invalid Extension Provider validation baseUrl")?
            .to_string();
        let contract = NormalizedValidationContract::from_manifest(&provider.validation)?;
        let auth_profile =
            ProviderAuthProfile::from_manifest(&provider.auth.scheme, &provider.auth.header)
                .context("Invalid Extension Provider validation auth profile")?;

        let secret_ref = provider.auth.secret_ref.clone();
        if let Some(secret_ref) = secret_ref.as_deref() {
            if !validate_secret_ref(secret_ref) {
                bail!("Gateway Provider '{}' secretRef is invalid", provider.id);
            }
        }

        Ok(Self {
            owner: owner.to_string(),
            provider_id,
            base_url,
            secret_ref,
            auth_profile,
            endpoint: contract.endpoint,
            valid_status_codes: contract.valid_status_codes,
            invalid_status_codes: contract.invalid_status_codes,
            timeout_ms: contract.timeout_ms,
        })
    }

    fn validate(&self) -> Result<()> {
        validate_owner(&self.owner)?;
        validate_provider_id(&self.provider_id)?;
        parse_base_url(&self.base_url)
            .map_err(anyhow::Error::msg)
            .context("Invalid Extension Provider validation baseUrl")?;
        let endpoint = join_endpoint(&self.base_url, &self.endpoint)
            .map_err(anyhow::Error::msg)
            .context("Invalid Extension Provider validation endpoint")?;
        if endpoint.as_str().is_empty() {
            bail!("Gateway Provider validation endpoint is empty");
        }
        if self.valid_status_codes.is_empty() {
            bail!("Gateway Provider validation must declare validStatusCodes");
        }
        validate_status_codes(&self.valid_status_codes, "validStatusCodes")?;
        validate_status_codes(&self.invalid_status_codes, "invalidStatusCodes")?;
        if self
            .valid_status_codes
            .iter()
            .any(|status| self.invalid_status_codes.contains(status))
        {
            bail!("Gateway Provider validation status mappings overlap");
        }
        validate_timeout(self.timeout_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedValidationContract {
    endpoint: String,
    valid_status_codes: Vec<u16>,
    invalid_status_codes: Vec<u16>,
    timeout_ms: u64,
}

impl NormalizedValidationContract {
    fn from_manifest(registration: &GatewayProviderValidationRegistration) -> Result<Self> {
        let endpoint = registration.endpoint.trim();
        if endpoint.is_empty() {
            bail!("Gateway Provider validation endpoint cannot be empty");
        }
        if endpoint != registration.endpoint {
            bail!("Gateway Provider validation endpoint must not contain surrounding whitespace");
        }
        // The base URL is checked when the complete Provider request is built;
        // this check keeps the contract itself constrained to a relative path.
        if endpoint.starts_with('/')
            || endpoint.starts_with("//")
            || endpoint.contains("://")
            || endpoint.contains("..")
            || endpoint.contains(['?', '#', '%', '\\'])
            || endpoint.chars().any(char::is_control)
        {
            bail!("Gateway Provider validation endpoint must be a safe relative path");
        }
        if registration.valid_status_codes.is_empty() {
            bail!("Gateway Provider validation must declare validStatusCodes");
        }
        validate_status_codes(&registration.valid_status_codes, "validStatusCodes")?;
        validate_status_codes(&registration.invalid_status_codes, "invalidStatusCodes")?;
        if registration
            .valid_status_codes
            .iter()
            .any(|status| registration.invalid_status_codes.contains(status))
        {
            bail!("Gateway Provider validation status mappings overlap");
        }
        validate_timeout(registration.timeout_ms)?;

        Ok(Self {
            endpoint: endpoint.to_string(),
            valid_status_codes: registration.valid_status_codes.clone(),
            invalid_status_codes: registration.invalid_status_codes.clone(),
            timeout_ms: registration.timeout_ms,
        })
    }
}

/// Lower-level adapter contract. Implementations must never return Valid from
/// reachability alone; only the declared successful status mapping may do so.
#[async_trait]
pub trait ExtensionProviderValidationAdapter: Send + Sync {
    async fn validate(&self, request: &ExtensionProviderValidationRequest) -> ValidationStatus;
}

/// Lifecycle-facing port for Extension-owned validation registrations.
#[async_trait]
pub trait ExtensionProviderValidationPort: Send + Sync {
    fn register(&self, owner: &str, request: ExtensionProviderValidationRequest) -> Result<()>;
    fn unregister(&self, owner: &str, provider_id: &str) -> Result<()>;
    fn unregister_owner(&self, owner: &str) -> Result<usize>;
    async fn validate(&self, provider_id: &str) -> ValidationStatus;
}

#[derive(Clone)]
struct ValidationRecord {
    owner: String,
    request: ExtensionProviderValidationRequest,
}

/// In-memory Extension-owned validation registry.
///
/// The registry stores only normalized contracts and opaque secret references.
/// It does not expose or retain decrypted secrets; the adapter resolves one
/// only for the duration of a single validation request.
pub struct ExtensionProviderValidationRegistry {
    records: RwLock<HashMap<String, ValidationRecord>>,
    adapter: Arc<dyn ExtensionProviderValidationAdapter>,
}

impl ExtensionProviderValidationRegistry {
    pub fn new(adapter: Arc<dyn ExtensionProviderValidationAdapter>) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            adapter,
        }
    }
}

#[async_trait]
impl ExtensionProviderValidationPort for ExtensionProviderValidationRegistry {
    fn register(&self, owner: &str, request: ExtensionProviderValidationRequest) -> Result<()> {
        validate_owner(owner)?;
        if request.owner != owner {
            bail!("Extension Provider validation owner mismatch");
        }
        request.validate()?;

        let mut records = self.records.write().map_err(|error| {
            anyhow::anyhow!("Failed to lock Provider validation registry: {error}")
        })?;
        if records.contains_key(&request.provider_id) {
            bail!(
                "Extension Provider validation already registered: {}",
                request.provider_id
            );
        }
        records.insert(
            request.provider_id.clone(),
            ValidationRecord {
                owner: owner.to_string(),
                request,
            },
        );
        Ok(())
    }

    fn unregister(&self, owner: &str, provider_id: &str) -> Result<()> {
        let mut records = self.records.write().map_err(|error| {
            anyhow::anyhow!("Failed to lock Provider validation registry: {error}")
        })?;
        let Some(record) = records.get(provider_id) else {
            return Ok(());
        };
        if record.owner != owner {
            bail!("Extension Provider validation owner mismatch");
        }
        records.remove(provider_id);
        Ok(())
    }

    fn unregister_owner(&self, owner: &str) -> Result<usize> {
        let mut records = self.records.write().map_err(|error| {
            anyhow::anyhow!("Failed to lock Provider validation registry: {error}")
        })?;
        let before = records.len();
        records.retain(|_, record| record.owner != owner);
        Ok(before - records.len())
    }

    async fn validate(&self, provider_id: &str) -> ValidationStatus {
        let request = match self.records.read() {
            Ok(records) => records
                .get(provider_id)
                .map(|record| record.request.clone()),
            Err(error) => {
                tracing::warn!(error = %error, "Failed to lock Provider validation registry");
                None
            }
        };
        let Some(request) = request else {
            return ValidationStatus::Unknown;
        };
        self.adapter.validate(&request).await
    }
}

/// Generic HTTP adapter for declarative Extension validation contracts.
pub struct HttpExtensionProviderValidationAdapter {
    transport: Arc<dyn ValidationTransport>,
    secrets: Arc<dyn SecretResolver>,
}

impl HttpExtensionProviderValidationAdapter {
    pub fn new(transport: Arc<dyn ValidationTransport>, secrets: Arc<dyn SecretResolver>) -> Self {
        Self { transport, secrets }
    }
}

#[async_trait]
impl ExtensionProviderValidationAdapter for HttpExtensionProviderValidationAdapter {
    async fn validate(&self, request: &ExtensionProviderValidationRequest) -> ValidationStatus {
        let secret = match request.secret_ref.as_deref() {
            Some(secret_ref) if validate_secret_ref(secret_ref) => {
                match self.secrets.resolve_secret(secret_ref) {
                    Ok(secret) => secret,
                    Err(error) => {
                        tracing::debug!(
                            provider_id = %request.provider_id,
                            error = %error,
                            "Extension Provider secret resolution failed"
                        );
                        return ValidationStatus::Unknown;
                    }
                }
            }
            Some(_) => return ValidationStatus::Unknown,
            None => None,
        };

        if request.auth_profile.requires_secret() && secret.is_none() {
            return ValidationStatus::Unknown;
        }

        let url = match join_endpoint(&request.base_url, &request.endpoint) {
            Ok(url) => url,
            Err(error) => {
                tracing::debug!(
                    provider_id = %request.provider_id,
                    error,
                    "Extension Provider validation endpoint rejected"
                );
                return ValidationStatus::Unknown;
            }
        };
        let headers = match request.auth_profile.auth_headers(secret.as_ref()) {
            Ok(headers) => headers,
            Err(error) => {
                tracing::debug!(
                    provider_id = %request.provider_id,
                    error = %error,
                    "Extension Provider validation auth rejected"
                );
                return ValidationStatus::Unknown;
            }
        };

        let timeout_ms = request.timeout_ms;
        let valid_status_codes = request.valid_status_codes.clone();
        let invalid_status_codes = request.invalid_status_codes.clone();
        let provider_id = request.provider_id.clone();
        let http_request = ValidationHttpRequest { url, headers };
        let result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            self.transport.send(http_request),
        )
        .await;

        match result {
            Ok(Ok(response)) if valid_status_codes.contains(&response.status.as_u16()) => {
                ValidationStatus::Valid
            }
            Ok(Ok(response)) if invalid_status_codes.contains(&response.status.as_u16()) => {
                ValidationStatus::Invalid
            }
            Ok(Ok(_)) => ValidationStatus::Reachable,
            Ok(Err(error)) => {
                tracing::debug!(
                    provider_id = %provider_id,
                    error = %error,
                    "Extension Provider validation request failed"
                );
                ValidationStatus::Unknown
            }
            Err(_) => {
                tracing::debug!(
                    provider_id = %provider_id,
                    "Extension Provider validation request timed out"
                );
                ValidationStatus::Unknown
            }
        }
    }
}

fn validate_owner(owner: &str) -> Result<()> {
    if owner.trim().is_empty()
        || owner != owner.trim()
        || owner.contains('/')
        || owner.contains('\\')
        || owner.chars().any(char::is_whitespace)
    {
        bail!("Extension Provider validation owner is invalid");
    }
    Ok(())
}

fn validate_provider_id(provider_id: &str) -> Result<()> {
    if provider_id.trim().is_empty()
        || provider_id != provider_id.trim()
        || provider_id.contains('\\')
        || provider_id.chars().any(char::is_whitespace)
    {
        bail!("Extension Provider validation provider ID is invalid");
    }
    Ok(())
}

fn validate_status_codes(status_codes: &[u16], field: &str) -> Result<()> {
    if status_codes
        .iter()
        .any(|status| !(100..=599).contains(status))
    {
        bail!("Gateway Provider validation {field} contains an invalid HTTP status code");
    }
    Ok(())
}

fn validate_timeout(timeout_ms: u64) -> Result<()> {
    if !(MIN_VALIDATION_TIMEOUT_MS..=MAX_VALIDATION_TIMEOUT_MS).contains(&timeout_ms) {
        bail!(
            "Gateway Provider validation timeoutMs must be between {} and {}",
            MIN_VALIDATION_TIMEOUT_MS,
            MAX_VALIDATION_TIMEOUT_MS
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::{
        GatewayAuthRegistration, GatewayModelRegistration, ProviderCapabilities,
    };
    use crate::security::auth::key_validator::{ValidationHttpResponse, ValidationTransport};
    use crate::security::auth::{SecretResolver, SecretValue};
    use async_trait::async_trait;
    use reqwest::StatusCode;
    use std::time::Duration;

    #[derive(Clone)]
    struct StaticTransport {
        response: std::result::Result<ValidationHttpResponse, String>,
        delay: Option<Duration>,
    }

    #[async_trait]
    impl ValidationTransport for StaticTransport {
        async fn send(&self, _request: ValidationHttpRequest) -> Result<ValidationHttpResponse> {
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            self.response.clone().map_err(anyhow::Error::msg)
        }
    }

    #[derive(Debug, Default)]
    struct EmptySecretResolver;

    impl SecretResolver for EmptySecretResolver {
        fn resolve_secret(&self, _secret_ref: &str) -> Result<Option<SecretValue>> {
            Ok(None)
        }
    }

    fn provider() -> GatewayProviderRegistration {
        GatewayProviderRegistration {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            adapter_id: "adapter".to_string(),
            base_url: "https://api.example.test/v1".to_string(),
            auth: GatewayAuthRegistration {
                scheme: "none".to_string(),
                secret_ref: None,
                header: String::new(),
            },
            capabilities: ProviderCapabilities::default(),
            validation: GatewayProviderValidationRegistration {
                endpoint: "models".to_string(),
                valid_status_codes: vec![200],
                invalid_status_codes: vec![401],
                timeout_ms: 1_000,
            },
            models: vec![GatewayModelRegistration {
                id: "model".to_string(),
                name: "Model".to_string(),
                capabilities: ProviderCapabilities::default(),
                context_window: 128_000,
                max_output_tokens: 4_096,
            }],
            default_model: "model".to_string(),
        }
    }

    fn request() -> ExtensionProviderValidationRequest {
        ExtensionProviderValidationRequest::from_manifest(
            "owner",
            &provider(),
            "extension:owner/provider".to_string(),
        )
        .unwrap()
    }

    fn adapter(
        status: StatusCode,
        delay: Option<Duration>,
    ) -> HttpExtensionProviderValidationAdapter {
        HttpExtensionProviderValidationAdapter::new(
            Arc::new(StaticTransport {
                response: Ok(ValidationHttpResponse { status }),
                delay,
            }),
            Arc::new(EmptySecretResolver),
        )
    }

    #[tokio::test]
    async fn registry_is_unknown_for_unregistered_provider_and_after_unregister() {
        let registry =
            ExtensionProviderValidationRegistry::new(Arc::new(adapter(StatusCode::OK, None)));
        assert_eq!(
            registry.validate("extension:owner/provider").await,
            ValidationStatus::Unknown
        );

        registry.register("owner", request()).unwrap();
        assert_eq!(
            registry.validate("extension:owner/provider").await,
            ValidationStatus::Valid
        );

        assert!(registry
            .unregister("other-owner", "extension:owner/provider")
            .is_err());
        registry
            .unregister("owner", "extension:owner/provider")
            .unwrap();
        assert_eq!(
            registry.validate("extension:owner/provider").await,
            ValidationStatus::Unknown
        );
    }

    #[tokio::test]
    async fn adapter_maps_declared_statuses_and_reachability_without_validating_by_name() {
        let request = request();

        assert_eq!(
            adapter(StatusCode::OK, None).validate(&request).await,
            ValidationStatus::Valid
        );
        assert_eq!(
            adapter(StatusCode::UNAUTHORIZED, None)
                .validate(&request)
                .await,
            ValidationStatus::Invalid
        );
        assert_eq!(
            adapter(StatusCode::TOO_MANY_REQUESTS, None)
                .validate(&request)
                .await,
            ValidationStatus::Reachable
        );
    }

    #[tokio::test]
    async fn adapter_fails_closed_on_transport_error_and_timeout() {
        let request = request();
        let transport_error_adapter = HttpExtensionProviderValidationAdapter::new(
            Arc::new(StaticTransport {
                response: Err("transport failed".to_string()),
                delay: None,
            }),
            Arc::new(EmptySecretResolver),
        );
        assert_eq!(
            transport_error_adapter.validate(&request).await,
            ValidationStatus::Unknown
        );

        let mut timeout_request = request;
        timeout_request.timeout_ms = 1;
        assert_eq!(
            adapter(StatusCode::OK, Some(Duration::from_millis(50)))
                .validate(&timeout_request)
                .await,
            ValidationStatus::Unknown
        );
    }

    #[test]
    fn manifest_contract_rejects_unsafe_endpoint_and_invalid_timeout() {
        let mut unsafe_provider = provider();
        unsafe_provider.validation.endpoint = "../models".to_string();
        assert!(ExtensionProviderValidationRequest::from_manifest(
            "owner",
            &unsafe_provider,
            "extension:owner/provider".to_string(),
        )
        .is_err());

        let mut invalid_timeout_provider = provider();
        invalid_timeout_provider.validation.timeout_ms = 30_001;
        assert!(ExtensionProviderValidationRequest::from_manifest(
            "owner",
            &invalid_timeout_provider,
            "extension:owner/provider".to_string(),
        )
        .is_err());
    }
}
