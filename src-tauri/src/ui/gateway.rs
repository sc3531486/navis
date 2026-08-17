// ── 归属扩展：navis-ai-platform ──
// 迁移目标：extensions/navis-ai-platform/ExtensionBackend/src/

use super::dto::{
    UiGatewayCatalog, UiGatewayConfig, UiGatewayDiscoveredModel, UiGatewayModel,
    UiGatewayModelConfig, UiGatewayProtocolCatalog, UiGatewayProvider, UiGatewayProviderCatalog,
    UiGatewayProviderConfig,
};
use crate::domains::ai_platform::gateway::provider::builtin_provider_profile;
use crate::domains::ai_platform::gateway::{
    Gateway, GatewayCapabilityCatalogProjection, GatewayConfig, GatewayModelProjection,
    GatewayProviderProjection, ModelConfig, ProtocolAdapterInfo, ProviderConfig,
};
use crate::foundation::config::Config;
use crate::security::auth::key_validator::{join_endpoint, parse_base_url, validate_secret_ref};
use crate::security::auth::{Auth, SecretResolver};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn ui_list_gateway_providers(gateway: State<'_, Arc<Gateway>>) -> Vec<UiGatewayProvider> {
    gateway
        .inner()
        .capability_projection()
        .providers
        .into_iter()
        .filter(|provider| provider.configured)
        .map(ui_gateway_provider)
        .collect()
}
#[tauri::command]
pub fn ui_list_gateway_models(gateway: State<'_, Arc<Gateway>>) -> Vec<UiGatewayModel> {
    gateway
        .inner()
        .capability_projection()
        .models
        .into_iter()
        .map(ui_gateway_model)
        .collect()
}

#[tauri::command]
pub fn ui_get_gateway_catalog(gateway: State<'_, Arc<Gateway>>) -> UiGatewayCatalog {
    ui_gateway_catalog(gateway.inner().capability_projection())
}

fn ui_gateway_provider(projection: GatewayProviderProjection) -> UiGatewayProvider {
    UiGatewayProvider {
        id: projection.id,
        label: projection.label,
        description: projection.description,
        default_base_url: projection.default_base_url,
        default_protocol: projection.default_protocol,
        protocols: projection.protocols,
        requires_secret: projection.requires_secret,
        capabilities: projection.capabilities,
        capability_version: projection.capability_version,
        diagnostics: projection.diagnostics,
        configured: projection.configured,
        model_count: projection.model_count,
        available_model_count: projection.available_model_count,
        status: projection.status,
    }
}
fn ui_gateway_protocol_catalog(info: ProtocolAdapterInfo) -> UiGatewayProtocolCatalog {
    let label = if info.name.trim().is_empty() {
        info.id.clone()
    } else {
        info.name
    };

    UiGatewayProtocolCatalog {
        id: crate::domains::ai_platform::gateway::ApiProtocol::from_str(&info.id),
        runtime_id: info.id,
        description: if info.description.trim().is_empty() {
            format!("{label} Gateway protocol adapter.")
        } else {
            info.description
        },
        label,
        supports_tools: info.capabilities.supports_tools,
        supports_streaming: info.capabilities.supports_streaming,
        supports_multimodal: info.capabilities.supports_multimodal,
        supports_reasoning_effort: info.capabilities.supports_reasoning_effort,
        supports_structured_output: info.capabilities.supports_structured_output,
        supports_usage: info.capabilities.supports_usage,
    }
}

fn ui_gateway_model(projection: GatewayModelProjection) -> UiGatewayModel {
    let model = projection.model;
    let capabilities = projection.capability.capabilities;
    UiGatewayModel {
        id: model.id,
        provider_id: projection.provider_id,
        name: model.name,
        context_window: model.context_window,
        max_output_tokens: model.max_output_tokens,
        supports_tools: capabilities.tools,
        supports_streaming: capabilities.streaming,
        supports_multimodal: capabilities.multimodal,
        supports_reasoning_effort: capabilities.reasoning,
        supports_structured_output: capabilities.structured_output,
        supports_usage: capabilities.usage,
        default_reasoning_effort: model.default_reasoning_effort,
        api_protocol: model.api_protocol,
        cost_per_1k_input: model.cost_per_1k_input,
        cost_per_1k_output: model.cost_per_1k_output,
    }
}

fn ui_gateway_provider_catalog(projection: GatewayProviderProjection) -> UiGatewayProviderCatalog {
    UiGatewayProviderCatalog {
        id: projection.id,
        label: projection.label,
        description: projection.description,
        default_base_url: projection.default_base_url,
        default_protocol: projection.default_protocol,
        protocols: projection.protocols,
        requires_secret: projection.requires_secret,
        supports_tools: projection.capabilities.tools,
        supports_streaming: projection.capabilities.streaming,
        supports_multimodal: projection.capabilities.multimodal,
        supports_reasoning_effort: projection.capabilities.reasoning,
        supports_structured_output: projection.capabilities.structured_output,
        supports_usage: projection.capabilities.usage,
        capabilities: projection.capabilities,
        capability_version: projection.capability_version,
        diagnostics: projection.diagnostics,
        configured: projection.configured,
        model_count: projection.model_count,
        available_model_count: projection.available_model_count,
        status: projection.status,
    }
}

fn ui_gateway_catalog(projection: GatewayCapabilityCatalogProjection) -> UiGatewayCatalog {
    UiGatewayCatalog {
        protocols: projection
            .protocols
            .into_iter()
            .map(ui_gateway_protocol_catalog)
            .collect(),
        providers: projection
            .providers
            .into_iter()
            .map(ui_gateway_provider_catalog)
            .collect(),
        models: projection
            .models
            .into_iter()
            .map(ui_gateway_model)
            .collect(),
    }
}
fn ui_gateway_model_config(model: ModelConfig) -> UiGatewayModelConfig {
    UiGatewayModelConfig {
        id: model.id,
        name: model.name,
        context_window: model.context_window,
        max_output_tokens: model.max_output_tokens,
        supports_tools: model.supports_tools,
        supports_streaming: model.supports_streaming,
        supports_multimodal: model.supports_multimodal,
        supports_reasoning_effort: model.supports_reasoning_effort,
        supports_structured_output: model.supports_structured_output,
        supports_usage: model.supports_usage,
        default_reasoning_effort: model.default_reasoning_effort,
        api_protocol: model.api_protocol,
        cost_per_1k_input: model.cost_per_1k_input,
        cost_per_1k_output: model.cost_per_1k_output,
    }
}

fn model_config_from_ui(model: UiGatewayModelConfig) -> ModelConfig {
    ModelConfig {
        id: model.id,
        name: model.name,
        context_window: model.context_window,
        max_output_tokens: model.max_output_tokens,
        supports_tools: model.supports_tools,
        supports_streaming: model.supports_streaming,
        supports_multimodal: model.supports_multimodal,
        supports_reasoning_effort: model.supports_reasoning_effort,
        supports_structured_output: model.supports_structured_output,
        supports_usage: model.supports_usage,
        default_reasoning_effort: model.default_reasoning_effort,
        supported_image_formats: None,
        max_image_size: None,
        api_protocol: model.api_protocol,
        required_request_fields: None,
        required_response_fields: None,
        custom_headers: None,
        cost_per_1k_input: model.cost_per_1k_input,
        cost_per_1k_output: model.cost_per_1k_output,
    }
}

fn ui_gateway_provider_config(provider: ProviderConfig) -> UiGatewayProviderConfig {
    UiGatewayProviderConfig {
        id: provider.id,
        provider_type: provider.provider_type,
        name: provider.name,
        base_url: provider.base_url,
        secret_ref: provider.secret_ref,
        models: provider
            .models
            .into_iter()
            .map(ui_gateway_model_config)
            .collect(),
        default_model: provider.default_model,
    }
}

fn provider_config_from_ui(provider: UiGatewayProviderConfig) -> ProviderConfig {
    ProviderConfig {
        id: provider.id,
        provider_type: provider.provider_type,
        name: provider.name,
        base_url: provider.base_url,
        secret_ref: provider.secret_ref,
        auth_profile: None,
        models: provider
            .models
            .into_iter()
            .map(model_config_from_ui)
            .collect(),
        default_model: provider.default_model,
    }
}

fn ui_gateway_config(config: GatewayConfig) -> UiGatewayConfig {
    UiGatewayConfig {
        providers: config
            .providers
            .into_iter()
            .map(ui_gateway_provider_config)
            .collect(),
        default_provider: config.default_provider,
        offline_fallback_model: config.offline_fallback_model,
        request_timeout_secs: config.request_timeout_secs,
        max_retries: config.max_retries,
    }
}

fn gateway_config_from_ui(config: UiGatewayConfig) -> GatewayConfig {
    GatewayConfig {
        providers: config
            .providers
            .into_iter()
            .map(provider_config_from_ui)
            .collect(),
        default_provider: config.default_provider,
        offline_fallback_model: config.offline_fallback_model,
        request_timeout_secs: config.request_timeout_secs.max(1),
        max_retries: config.max_retries,
    }
}

pub fn gateway_config_from_config(config: &Config) -> Result<GatewayConfig, String> {
    let mut gateway_config = GatewayConfig::default();

    if let Some(value) = config.get("gateway.providers") {
        let providers = serde_json::from_value::<Vec<UiGatewayProviderConfig>>(value)
            .map_err(|error| format!("Gateway provider 配置无效: {error}"))?;
        gateway_config.providers = providers.into_iter().map(provider_config_from_ui).collect();
    }
    gateway_config.default_provider = config
        .get("gateway.defaultProvider")
        .and_then(|value| value.as_str().map(str::to_string));
    gateway_config.offline_fallback_model = config
        .get("gateway.offlineFallbackModel")
        .and_then(|value| value.as_str().map(str::to_string));
    gateway_config.request_timeout_secs = config
        .get("gateway.requestTimeoutSecs")
        .and_then(|value| value.as_u64())
        .unwrap_or(gateway_config.request_timeout_secs)
        .max(1);
    gateway_config.max_retries = config
        .get("gateway.maxRetries")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32)
        .unwrap_or(gateway_config.max_retries);

    Ok(gateway_config)
}

fn gateway_origin_url(base_url: &str) -> Result<reqwest::Url, String> {
    let mut origin = parse_base_url(base_url)?;
    origin.set_path("");
    Ok(origin)
}

fn gateway_discovery_urls(provider: &UiGatewayProviderConfig) -> Result<Vec<reqwest::Url>, String> {
    let base = parse_base_url(&provider.base_url)?;
    let profile = builtin_provider_profile(&provider.provider_type);
    let endpoints = profile
        .map(|profile| profile.model_catalog.endpoints)
        .unwrap_or(&["/v1/models"]);
    let root = gateway_origin_url(&provider.base_url)?;
    let mut urls = Vec::with_capacity(endpoints.len());

    for endpoint in endpoints {
        let endpoint = endpoint.trim();
        let target_base = if matches!(endpoint, "/models" | "/api/tags") {
            &root
        } else {
            &base
        };
        let url = join_endpoint(target_base.as_str(), endpoint)?;
        if !urls.contains(&url) {
            urls.push(url);
        }
    }

    Ok(urls)
}
fn gateway_discovered_models(value: Value) -> Vec<UiGatewayDiscoveredModel> {
    let candidates = value
        .get("data")
        .and_then(Value::as_array)
        .or_else(|| value.get("models").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default();

    let mut models = Vec::new();
    for item in candidates {
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| item.get("name").and_then(Value::as_str))
            .unwrap_or_default()
            .trim()
            .to_string();
        if id.is_empty()
            || models
                .iter()
                .any(|model: &UiGatewayDiscoveredModel| model.id == id)
        {
            continue;
        }
        let name = item
            .get("display_name")
            .and_then(Value::as_str)
            .or_else(|| item.get("name").and_then(Value::as_str))
            .or_else(|| item.get("id").and_then(Value::as_str))
            .unwrap_or(&id)
            .trim()
            .to_string();
        models.push(UiGatewayDiscoveredModel { id, name });
    }
    models
}

fn gateway_payload_secret_ref(provider: &UiGatewayProviderConfig) -> Result<Option<&str>, String> {
    match provider.secret_ref.as_deref() {
        None => Ok(None),
        Some(secret_ref) if validate_secret_ref(secret_ref) => Ok(Some(secret_ref)),
        Some(_) => Err("secret_ref 无效".to_string()),
    }
}
#[tauri::command]
pub async fn ui_discover_gateway_models(
    provider: UiGatewayProviderConfig,
    auth: State<'_, Arc<Auth>>,
) -> Result<Vec<UiGatewayDiscoveredModel>, String> {
    if provider.base_url.trim().is_empty() {
        return Err("请先填写 Base URL".to_string());
    }

    let secret_ref = gateway_payload_secret_ref(&provider).map_err(|error| error.to_string())?;
    let secret = secret_ref
        .map(|secret_ref| auth.resolve_secret(secret_ref))
        .transpose()
        .map_err(|error| error.to_string())?
        .flatten();

    if builtin_provider_profile(&provider.provider_type)
        .map(|profile| profile.quirks.requires_secret)
        .unwrap_or(false)
        && secret.is_none()
    {
        return Err("请先配置该 Connection 的 secret_ref".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;
    let mut errors = Vec::new();

    for url in gateway_discovery_urls(&provider)? {
        let mut request = client.get(url.clone());
        if let Some(profile) = builtin_provider_profile(&provider.provider_type) {
            request = request.headers(
                profile
                    .auth_headers(secret.as_ref())
                    .map_err(|error| error.to_string())?,
            );
        }
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    errors.push(format!("{url} 返回 {status}"));
                    continue;
                }
                let body = response
                    .json::<Value>()
                    .await
                    .map_err(|error| error.to_string())?;
                let models = gateway_discovered_models(body);
                if models.is_empty() {
                    errors.push(format!("{url} 未返回可识别的模型列表"));
                    continue;
                }
                return Ok(models);
            }
            Err(error) => {
                errors.push(format!("{url} 请求失败：{error}"));
            }
        }
    }

    Err(if errors.is_empty() {
        "未获取到模型列表".to_string()
    } else {
        format!("未获取到模型列表。已尝试：{}", errors.join("；"))
    })
}

#[tauri::command]
pub fn ui_get_gateway_config(
    config: State<'_, Arc<Mutex<Config>>>,
) -> Result<UiGatewayConfig, String> {
    let config = config.lock().map_err(|error| error.to_string())?;
    Ok(ui_gateway_config(gateway_config_from_config(&config)?))
}

#[tauri::command]
pub fn ui_save_gateway_config(
    gateway: State<'_, Arc<Gateway>>,
    config: State<'_, Arc<Mutex<Config>>>,
    payload: UiGatewayConfig,
) -> Result<UiGatewayConfig, String> {
    let gateway_config = gateway_config_from_ui(payload);
    for provider in &gateway_config.providers {
        provider.validate().map_err(|error| error.to_string())?;
    }

    gateway
        .inner()
        .apply_config(gateway_config.clone())
        .map_err(|error| error.to_string())?;

    {
        let mut config = config.lock().map_err(|error| error.to_string())?;
        config
            .set(
                "gateway.providers",
                json!(ui_gateway_config(gateway_config.clone()).providers),
            )
            .map_err(|error| error.to_string())?;
        config
            .set(
                "gateway.defaultProvider",
                json!(gateway_config.default_provider),
            )
            .map_err(|error| error.to_string())?;
        config
            .set(
                "gateway.offlineFallbackModel",
                json!(gateway_config.offline_fallback_model),
            )
            .map_err(|error| error.to_string())?;
        config
            .set(
                "gateway.requestTimeoutSecs",
                json!(gateway_config.request_timeout_secs),
            )
            .map_err(|error| error.to_string())?;
        config
            .set("gateway.maxRetries", json!(gateway_config.max_retries))
            .map_err(|error| error.to_string())?;
        config
            .save_user_config()
            .map_err(|error| error.to_string())?;
    }

    Ok(ui_gateway_config(gateway_config))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::ai_platform::gateway::{ApiProtocol, CapabilitySet};

    fn provider_projection(id: &str) -> GatewayProviderProjection {
        GatewayProviderProjection {
            id: id.to_string(),
            label: id.to_string(),
            description: "projection".to_string(),
            default_base_url: "https://example.test".to_string(),
            default_protocol: ApiProtocol::ChatCompletions,
            protocols: Vec::new(),
            requires_secret: false,
            capabilities: CapabilitySet::default(),
            capability_version: crate::domains::ai_platform::gateway::GATEWAY_CAPABILITY_PROJECTION_VERSION,
            diagnostics: Vec::new(),
            configured: true,
            model_count: 1,
            available_model_count: 1,
            status: crate::domains::ai_platform::gateway::GatewayProviderStatus::Available,
        }
    }

    #[test]
    fn provider_ui_projection_preserves_capability_contract() {
        let provider = ui_gateway_provider(provider_projection("custom"));

        assert_eq!(provider.id, "custom");
        assert_eq!(
            provider.status,
            crate::domains::ai_platform::gateway::GatewayProviderStatus::Available
        );
        assert_eq!(
            provider.capability_version,
            crate::domains::ai_platform::gateway::GATEWAY_CAPABILITY_PROJECTION_VERSION
        );
        assert_eq!(provider.model_count, 1);
        assert_eq!(provider.available_model_count, 1);
        assert!(provider.diagnostics.is_empty());
    }

    #[test]
    fn catalog_keeps_extension_provider_id_separate_from_builtin_profile() {
        let extension_id = "extension:acme/openai";
        let catalog = ui_gateway_catalog(GatewayCapabilityCatalogProjection {
            protocols: Vec::new(),
            providers: vec![
                provider_projection("openai"),
                provider_projection(extension_id),
            ],
            models: Vec::new(),
        });

        let extension_entry = catalog
            .providers
            .iter()
            .find(|item| item.id == extension_id)
            .expect("extension provider catalog entry");

        assert!(catalog.providers.iter().any(|item| item.id == "openai"));
        assert_eq!(extension_entry.id, extension_id);
        assert_eq!(
            extension_entry.default_protocol,
            ApiProtocol::ChatCompletions
        );
    }
    fn ui_provider_config(base_url: &str, secret_ref: Option<&str>) -> UiGatewayProviderConfig {
        UiGatewayProviderConfig {
            id: "custom".to_string(),
            provider_type: "custom".to_string(),
            name: "Custom".to_string(),
            base_url: base_url.to_string(),
            secret_ref: secret_ref.map(str::to_string),
            models: Vec::new(),
            default_model: String::new(),
        }
    }

    #[test]
    fn discovery_urls_use_validated_url_joining() {
        let urls = gateway_discovery_urls(&ui_provider_config("https://api.example.com/v1/", None))
            .unwrap();
        let urls = urls.iter().map(reqwest::Url::as_str).collect::<Vec<_>>();
        assert_eq!(
            urls,
            vec![
                "https://api.example.com/v1/models",
                "https://api.example.com/models",
                "https://api.example.com/api/tags",
            ]
        );
    }

    #[test]
    fn discovery_urls_reject_unsafe_base_url() {
        let error = gateway_discovery_urls(&ui_provider_config("https://localhost:8443", None))
            .unwrap_err();
        assert!(error.contains("本机") || error.contains("私网"));
    }

    #[test]
    fn payload_secret_ref_requires_an_opaque_reference() {
        assert_eq!(
            gateway_payload_secret_ref(&ui_provider_config(
                "https://api.example.com",
                Some("key:openai/123"),
            ))
            .unwrap(),
            Some("key:openai/123")
        );
        assert_eq!(
            gateway_payload_secret_ref(&ui_provider_config("https://api.example.com", None))
                .unwrap(),
            None
        );
        assert!(gateway_payload_secret_ref(&ui_provider_config(
            "https://api.example.com",
            Some(" key"),
        ))
        .is_err());
        assert!(gateway_payload_secret_ref(&ui_provider_config(
            "https://api.example.com",
            Some(""),
        ))
        .is_err());
    }
}
