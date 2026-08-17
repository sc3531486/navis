//! Extension contribution registration and declarative host conversion.
//!
//! This module converts manifest declarations into existing host DTOs. It does
//! not load extension modules or create runtime adapters.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::extension::types::{CapabilitySet, CustomProtocolConfig};
use crate::extension::types::{ApiProtocol, ModelConfig, ProviderConfig};
use crate::extension::models::{
    ExtensionContributes, ExtensionPermissionConstraint, GatewayAdapterRegistration,
    GatewayContributions, GatewayProviderRegistration, LSPServerConfig, LanguageRegistration,
    LanguageSource, McpToolOverride, ProviderAuthProfile, ProviderCapabilities, ToolRegistration,
};
use crate::extension::provider_validation::{
    ExtensionProviderValidationPort, ExtensionProviderValidationRequest,
};
use crate::extension::types::{
    MCPServerConfig, ToolDefinition, ToolDefinitionOverride, ToolRendererHint, ToolRiskLevel,
    ToolUiHint,
};

use super::{
    ExtensionLifecycle, ExtensionRuntimeHandle, GatewayCapabilityPort, LspCapabilityPort,
    McpCapabilityPort, MCP_OVERRIDE_SERVER_PREFIX,
};

#[derive(Debug, Clone)]
pub(crate) struct GatewayAdapterPlan {
    pub adapter_id: String,
    pub protocol: ApiProtocol,
    pub custom_config: Option<CustomProtocolConfig>,
}

#[derive(Debug)]
pub(crate) struct GatewayProviderPlan {
    pub config: ProviderConfig,
    pub capabilities: CapabilitySet,
    pub validation: ExtensionProviderValidationRequest,
}

#[derive(Debug)]
pub(crate) struct GatewayPlan {
    pub adapters: Vec<GatewayAdapterPlan>,
    pub providers: Vec<GatewayProviderPlan>,
}

/// 将 Extension Gateway 声明规范化为宿主可执行的注册计划。
pub(crate) fn build_gateway_plan(
    extension_id: &str,
    gateway: &GatewayContributions,
) -> Result<GatewayPlan> {
    if gateway.adapters.is_empty() && gateway.providers.is_empty() {
        return Err(anyhow::anyhow!(
            "Gateway contribution must declare an adapter or provider"
        ));
    }

    let mut adapter_ids = std::collections::HashSet::new();
    let mut protocol_ids = std::collections::HashSet::new();
    let mut adapter_plans = Vec::with_capacity(gateway.adapters.len());
    for adapter in &gateway.adapters {
        validate_manifest_id("Gateway adapter", &adapter.id)?;
        if !adapter_ids.insert(adapter.id.clone()) {
            return Err(anyhow::anyhow!(
                "Duplicate Gateway adapter ID '{}'",
                adapter.id
            ));
        }
        let protocol_id = validate_protocol_id(&adapter.protocol_id)?;
        if !protocol_ids.insert(protocol_id.to_string()) {
            return Err(anyhow::anyhow!(
                "Duplicate Gateway protocolId '{}'",
                protocol_id
            ));
        }
        adapter_plans.push(gateway_adapter_plan(adapter)?);
    }

    let adapters = adapter_plans
        .iter()
        .cloned()
        .map(|plan| (plan.adapter_id.clone(), plan))
        .collect::<std::collections::HashMap<_, _>>();

    let mut provider_ids = std::collections::HashSet::new();
    let mut providers = Vec::with_capacity(gateway.providers.len());
    for provider in &gateway.providers {
        validate_manifest_id("Gateway provider", &provider.id)?;
        if !provider_ids.insert(provider.id.clone()) {
            return Err(anyhow::anyhow!(
                "Duplicate Gateway provider ID '{}'",
                provider.id
            ));
        }
        let adapter_plan = adapters.get(&provider.adapter_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Gateway provider '{}' references unknown adapter '{}'",
                provider.id,
                provider.adapter_id
            )
        })?;
        let config = extension_provider_config(extension_id, provider, &adapter_plan.protocol)?;
        let validation = ExtensionProviderValidationRequest::from_manifest(
            extension_id,
            provider,
            super::extension_provider_id(extension_id, &provider.id)?,
        )?;
        providers.push(GatewayProviderPlan {
            config,
            capabilities: provider_capability_set(&provider.capabilities),
            validation,
        });
    }

    Ok(GatewayPlan {
        adapters: adapter_plans,
        providers,
    })
}

/// 提交已经规范化的 Gateway 计划，并把实际注册成功的资源写入统一句柄账本。
///
/// Gateway provider 的依赖顺序是 protocol -> provider -> capability -> validation；
/// 清理由 `ExtensionRuntimeHandle`（fiber disposer / disable）按逆序消费这些句柄。
pub(crate) fn register_gateway_plan(
    extension_id: &str,
    plan: GatewayPlan,
    gateway: &dyn GatewayCapabilityPort,
    provider_validation: Option<&dyn ExtensionProviderValidationPort>,
    handle: &mut ExtensionRuntimeHandle,
) -> Result<()> {
    for adapter in plan.adapters {
        if let Some(config) = adapter.custom_config {
            gateway.register_custom_protocol(extension_id, config)?;
        } else {
            gateway.acquire_protocol(extension_id, &adapter.protocol)?;
        }
        handle.protocols.push(adapter.protocol);
    }

    for provider_plan in plan.providers {
        let provider_id = provider_plan.config.id.clone();
        gateway
            .upsert_provider(provider_plan.config)
            .map_err(|error| {
                anyhow::anyhow!(
                    "Extension '{}' failed to register Gateway provider '{}': {}",
                    extension_id,
                    provider_id,
                    error
                )
            })?;
        handle.provider_ids.push(provider_id.clone());

        gateway
            .set_provider_capabilities(&provider_id, provider_plan.capabilities)
            .map_err(|error| {
                anyhow::anyhow!(
                    "Extension '{}' failed to register Gateway provider capabilities '{}': {}",
                    extension_id,
                    provider_id,
                    error
                )
            })?;
        handle.provider_capability_ids.push(provider_id.clone());

        let validation_port = provider_validation.ok_or_else(|| {
            anyhow::anyhow!(
                "Extension '{}' declares Gateway Provider validation but validation host is not available",
                extension_id
            )
        })?;
        let validation_id = provider_plan.validation.provider_id.clone();
        validation_port
            .register(extension_id, provider_plan.validation)
            .map_err(|error| {
                anyhow::anyhow!(
                    "Extension '{}' failed to register Gateway Provider validation '{}': {}",
                    extension_id,
                    validation_id,
                    error
                )
            })?;
        handle.provider_validation_ids.push(validation_id);
        handle.projection.contribution_counts.providers += 1;
    }

    Ok(())
}

/// 清理 Extension 实际拥有的 Gateway 资源，始终按逆依赖顺序执行。
///
/// 清理失败的句柄保留在 `ExtensionRuntimeHandle` 中，后续 disable/retry 可以继续消费。
pub(crate) fn unregister_gateway_resources(
    extension_id: &str,
    handle: &mut ExtensionRuntimeHandle,
    gateway: Option<&dyn GatewayCapabilityPort>,
    provider_validation: Option<&dyn ExtensionProviderValidationPort>,
    cleanup_errors: &mut Vec<String>,
) {
    if let Some(validation) = provider_validation {
        for provider_id in handle.provider_validation_ids.clone().into_iter().rev() {
            if let Err(error) = validation.unregister(extension_id, &provider_id) {
                cleanup_errors.push(format!("Provider validation '{}': {}", provider_id, error));
            } else {
                handle
                    .provider_validation_ids
                    .retain(|candidate| candidate != &provider_id);
            }
        }
    } else if !handle.provider_validation_ids.is_empty() {
        cleanup_errors.push(
            "Provider validation resources exist but validation host is not available".to_string(),
        );
    }

    if let Some(gateway) = gateway {
        for provider_id in handle.provider_capability_ids.clone().into_iter().rev() {
            if let Err(error) = gateway.remove_provider_capabilities(&provider_id) {
                cleanup_errors.push(format!(
                    "Gateway provider capability source '{}': {}",
                    provider_id, error
                ));
            } else {
                handle
                    .provider_capability_ids
                    .retain(|candidate| candidate != &provider_id);
            }
        }
        for provider_id in handle.provider_ids.clone().into_iter().rev() {
            if let Err(error) = gateway.remove_provider(&provider_id) {
                cleanup_errors.push(format!("Gateway provider '{}': {}", provider_id, error));
            } else {
                handle
                    .provider_ids
                    .retain(|candidate| candidate != &provider_id);
            }
        }
        for protocol in handle.protocols.clone().into_iter().rev() {
            if let Err(error) = gateway.release_protocol(extension_id, &protocol) {
                cleanup_errors.push(format!(
                    "Gateway protocol '{}': {}",
                    protocol.as_str(),
                    error
                ));
            } else {
                handle.protocols.retain(|candidate| candidate != &protocol);
            }
        }
    } else if !handle.provider_ids.is_empty()
        || !handle.provider_capability_ids.is_empty()
        || !handle.protocols.is_empty()
    {
        cleanup_errors.push("Gateway resources exist but Gateway host is not available".to_string());
    }
}

fn provider_capability_set(capabilities: &ProviderCapabilities) -> CapabilitySet {
    CapabilitySet {
        tools: capabilities.supports_tools,
        streaming: capabilities.supports_streaming,
        multimodal: capabilities.supports_multimodal,
        reasoning: capabilities.supports_reasoning_effort,
        structured_output: capabilities.supports_structured_output,
        usage: capabilities.supports_usage,
        model_catalog: true,
        ..CapabilitySet::default()
    }
}

fn validate_manifest_id(kind: &str, id: &str) -> Result<()> {
    let id = id.trim();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.chars().any(char::is_whitespace)
    {
        return Err(anyhow::anyhow!("{} ID '{}' is invalid", kind, id));
    }
    Ok(())
}

fn validate_protocol_id(protocol_id: &str) -> Result<&str> {
    let protocol_id = protocol_id.trim();
    if protocol_id.is_empty() {
        return Err(anyhow::anyhow!(
            "Gateway adapter protocolId cannot be empty"
        ));
    }
    if protocol_id.chars().any(char::is_whitespace) {
        return Err(anyhow::anyhow!(
            "Gateway adapter protocolId '{}' cannot contain whitespace",
            protocol_id
        ));
    }
    if protocol_id.starts_with("custom:") {
        return Err(anyhow::anyhow!(
            "Gateway adapter protocolId '{}' must not use the explicit custom: prefix",
            protocol_id
        ));
    }
    Ok(protocol_id)
}

fn gateway_adapter_plan(adapter: &GatewayAdapterRegistration) -> Result<GatewayAdapterPlan> {
    let protocol_id = validate_protocol_id(&adapter.protocol_id).map_err(|error| {
        anyhow::anyhow!(
            "Gateway adapter '{}' has invalid protocolId: {}",
            adapter.id,
            error
        )
    })?;
    match protocol_id {
        "chat_completions" | "responses" => {
            if adapter.kind != "builtin" {
                return Err(anyhow::anyhow!(
                    "Builtin Gateway protocol '{}' must use adapter kind 'builtin'",
                    protocol_id
                ));
            }
            Ok(GatewayAdapterPlan {
                adapter_id: adapter.id.clone(),
                protocol: ApiProtocol::from_str(protocol_id),

                custom_config: None,
            })
        }
        _ => {
            if adapter.kind != "declarative" {
                return Err(anyhow::anyhow!(
                    "Extension Gateway protocol '{}' must use adapter kind 'declarative'",
                    protocol_id
                ));
            }
            let protocol = ApiProtocol::from_str(protocol_id);
            let config =
                CustomProtocolConfig::from_manifest(protocol.as_str(), adapter.config.clone())?;
            Ok(GatewayAdapterPlan {
                adapter_id: adapter.id.clone(),
                protocol,
                custom_config: Some(config),
            })
        }
    }
}

fn extension_provider_config(
    extension_id: &str,
    provider: &GatewayProviderRegistration,
    protocol: &ApiProtocol,
) -> Result<ProviderConfig> {
    validate_manifest_id("Gateway provider", &provider.id)?;
    if provider.name.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Gateway provider '{}' name cannot be empty",
            provider.id
        ));
    }
    if provider.base_url.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Gateway provider '{}' baseUrl cannot be empty",
            provider.id
        ));
    }
    if provider.models.is_empty() {
        return Err(anyhow::anyhow!(
            "Gateway provider '{}' must declare models",
            provider.id
        ));
    }

    let provider_id = super::extension_provider_id(extension_id, &provider.id)?;
    let mut model_ids = std::collections::HashSet::new();
    let mut models = Vec::with_capacity(provider.models.len());
    for declaration in &provider.models {
        validate_manifest_id("Gateway model", &declaration.id)?;
        if !model_ids.insert(declaration.id.clone()) {
            return Err(anyhow::anyhow!(
                "Gateway provider '{}' contains duplicate model ID '{}',",
                provider.id,
                declaration.id
            ));
        }
        let mut model = ModelConfig::new(declaration.id.clone(), declaration.name.clone());
        model.context_window = declaration.context_window;
        model.max_output_tokens = declaration.max_output_tokens;
        model.supports_tools = declaration.capabilities.supports_tools;
        model.supports_streaming = declaration.capabilities.supports_streaming;
        model.supports_multimodal = declaration.capabilities.supports_multimodal;
        model.supports_reasoning_effort = declaration.capabilities.supports_reasoning_effort;
        model.supports_structured_output = declaration.capabilities.supports_structured_output;
        model.supports_usage = declaration.capabilities.supports_usage;
        model.api_protocol = protocol.clone();
        models.push(model);
    }
    if !model_ids.contains(&provider.default_model) {
        return Err(anyhow::anyhow!(
            "Gateway provider '{}' defaultModel '{}' is not declared",
            provider.id,
            provider.default_model
        ));
    }

    let config = ProviderConfig {
        id: provider_id.clone(),
        provider_type: provider_id,
        name: provider.name.clone(),
        base_url: provider.base_url.clone(),
        secret_ref: provider.auth.secret_ref.clone(),
        auth_profile: Some(ProviderAuthProfile::from_manifest(
            &provider.auth.scheme,
            &provider.auth.header,
        )?),
        models,
        default_model: provider.default_model.clone(),
    };
    config
        .validate()
        .map_err(anyhow::Error::msg)
        .context("Invalid extension Gateway provider declaration")?;
    Ok(config)
}

pub(crate) fn extension_tool_definition(
    extension_id: &str,
    registration: &ToolRegistration,
) -> Result<ToolDefinition> {
    let server_id = super::extension_tool_server_id(extension_id);
    let mut definition = ToolDefinition::new(
        registration.name.clone(),
        registration.description.clone(),
        registration.input_schema.clone(),
        server_id,
    );
    definition.user_visible = registration.user_visible;
    definition.declared_risk = registration
        .declared_risk
        .as_deref()
        .map(parse_tool_risk)
        .transpose()?;
    definition.effective_risk = definition.declared_risk.unwrap_or_default().max(
        crate::extension::types::platform_risk_override(&definition.name).unwrap_or_default(),
    );
    Ok(definition)
}

pub(crate) fn extension_mcp_server_config(
    extension_id: &str,
    declaration: &crate::extension::models::MCPServerConfig,
) -> Result<MCPServerConfig> {
    let mut object = declaration.config.as_object().cloned().ok_or_else(|| {
        anyhow::anyhow!(
            "MCP server '{}' config must be a JSON object",
            declaration.name
        )
    })?;
    object.insert(
        "id".to_string(),
        Value::String(super::extension_mcp_server_id(
            extension_id,
            &declaration.name,
        )),
    );
    object.insert("name".to_string(), Value::String(declaration.name.clone()));
    serde_json::from_value(Value::Object(object)).with_context(|| {
        format!(
            "Invalid MCP server declaration '{}' for extension '{}'",
            declaration.name, extension_id
        )
    })
}

pub(crate) fn mcp_override_server_id(extension_id: &str, server: &str) -> String {
    if server == "builtin" || server.starts_with(MCP_OVERRIDE_SERVER_PREFIX) {
        server.to_string()
    } else {
        format!("{}{}/{}", MCP_OVERRIDE_SERVER_PREFIX, extension_id, server)
    }
}

pub(crate) fn lsp_server_config_from_extension_language(
    language: &LanguageRegistration,
) -> LSPServerConfig {
    LSPServerConfig {
        language_id: language.language_id.clone(),
        language_names: vec![language.display_name.clone()],
        file_extensions: language.extensions.clone(),
        server_command: language.server_command.clone(),
        server_args: language.server_args.clone().unwrap_or_default(),
        initialization_options: language.initialization_options.clone(),
        capabilities_required: Vec::new(),
    }
}

impl ExtensionLifecycle {
    pub(crate) fn register_extension_permission_constraint(
        &self,
        extension_id: &str,
        permissions: &crate::extension::models::ExtensionPermissions,
    ) {
        let Some(policy_engine) = &self.policy_engine else {
            return;
        };
        if let Err(error) = policy_engine.add(ExtensionPermissionConstraint::new(
            extension_id.to_string(),
            permissions.clone(),
        )) {
            tracing::warn!(
                extension_id = %extension_id,
                error = %error,
                "Failed to register extension permission constraint"
            );
        }
    }

    pub(crate) fn unregister_extension_permission_constraint(&self, extension_id: &str) {
        let Some(policy_engine) = &self.policy_engine else {
            return;
        };
        let constraint_id = ExtensionPermissionConstraint::constraint_id_for(extension_id);
        if let Err(error) = policy_engine.remove(&constraint_id) {
            tracing::debug!(
                extension_id = %extension_id,
                error = %error,
                "Extension permission constraint was not present during unregister"
            );
        }
    }

    pub(crate) fn unregister_ui_contributions(&self, extension_id: &str) -> Result<()> {
        let mut registrar = self.ui_contributions.lock().map_err(|error| {
            anyhow::anyhow!(
                "Failed to lock UI contribution registrar during unregister: {}",
                error
            )
        })?;
        registrar.unregister_extension(extension_id);
        Ok(())
    }

    pub(crate) fn register_enabled_hook_declarations(
        &self,
        extension_id: &str,
        contributes: &ExtensionContributes,
    ) -> Result<()> {
        let Some(hooks) = contributes.hooks.as_deref() else {
            return Ok(());
        };
        self.store
            .register_hooks(extension_id, hooks)
            .map_err(|error| anyhow::anyhow!("Failed to register extension hooks: {}", error))
    }
}

pub(crate) fn register_lsp_languages(
    extension_id: &str,
    languages: &[LanguageRegistration],
    lsp: &dyn LspCapabilityPort,
    handle: &mut ExtensionRuntimeHandle,
) -> Result<()> {
    for language in languages {
        lsp.register_language(
            lsp_server_config_from_extension_language(language),
            LanguageSource::Extension {
                owner: extension_id.to_string(),
            },
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "Extension '{}' failed to register LSP language '{}': {}",
                extension_id,
                language.language_id,
                error
            )
        })?;
        handle.languages.push(language.language_id.clone());
    }
    Ok(())
}

pub(crate) fn apply_mcp_tool_overrides(
    mcp: &dyn McpCapabilityPort,
    extension_id: &str,
    overrides: &[McpToolOverride],
    handle: &mut ExtensionRuntimeHandle,
) -> Result<()> {
    for override_ in overrides {
        let server_id = mcp_override_server_id(extension_id, &override_.server);
        let tool_override = tool_definition_override(override_)?;
        mcp.apply_tool_override(extension_id, &server_id, &override_.tool, tool_override)
            .map_err(|error| {
                anyhow::anyhow!(
                    "Failed to apply MCP tool override '{}:{}': {}",
                    server_id,
                    override_.tool,
                    error
                )
            })?;
        handle
            .tool_overrides
            .push((server_id, override_.tool.clone()));
    }
    Ok(())
}

fn tool_definition_override(override_: &McpToolOverride) -> Result<ToolDefinitionOverride> {
    let ui_hint = override_.display_name.clone().map(ToolUiHint::new);
    let renderer_hint = override_.renderer.clone().map(|renderer| {
        let mut hint = ToolRendererHint::new(renderer);
        if let Some(detail_view) = &override_.detail_view {
            hint = hint.with_detail_view(detail_view.clone());
        }
        hint
    });
    Ok(ToolDefinitionOverride {
        model_name: override_.model_name.clone(),
        user_visible: override_.user_visible,
        ui_hint,
        description: override_.description.clone(),
        renderer_hint,
        declared_risk: override_
            .declared_risk
            .as_deref()
            .map(parse_tool_risk)
            .transpose()?,
    })
}

fn parse_tool_risk(value: &str) -> Result<ToolRiskLevel> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(ToolRiskLevel::None),
        "read" => Ok(ToolRiskLevel::Read),
        "write" => Ok(ToolRiskLevel::Write),
        "network" => Ok(ToolRiskLevel::Network),
        "command" => Ok(ToolRiskLevel::Command),
        "destructive" => Ok(ToolRiskLevel::Destructive),
        other => Err(anyhow::anyhow!(
            "Unsupported MCP tool risk level '{}'",
            other
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::models::ProviderCapabilities;
    use serde_json::json;

    fn provider() -> GatewayProviderRegistration {
        GatewayProviderRegistration {
            id: "provider".to_string(),
            name: "Provider".to_string(),
            adapter_id: "adapter".to_string(),
            base_url: "https://example.test/v1".to_string(),
            auth: crate::extension::models::GatewayAuthRegistration {
                scheme: "bearer".to_string(),
                secret_ref: None,
                header: "Authorization".to_string(),
            },
            capabilities: ProviderCapabilities {
                supports_tools: true,
                supports_streaming: true,
                supports_multimodal: false,
                supports_reasoning_effort: false,
                supports_structured_output: false,
                supports_usage: true,
            },
            validation: crate::extension::models::GatewayProviderValidationRegistration {
                endpoint: "models".to_string(),
                valid_status_codes: vec![200],
                invalid_status_codes: vec![401],
                timeout_ms: 5_000,
            },
            models: vec![crate::extension::models::GatewayModelRegistration {
                id: "model".to_string(),
                name: "Model".to_string(),
                capabilities: ProviderCapabilities {
                    supports_tools: true,
                    supports_streaming: true,
                    supports_multimodal: false,
                    supports_reasoning_effort: false,
                    supports_structured_output: false,
                    supports_usage: true,
                },
                context_window: 128000,
                max_output_tokens: 4096,
            }],
            default_model: "model".to_string(),
        }
    }

    fn provider_with(id: &str, adapter_id: &str) -> GatewayProviderRegistration {
        let mut provider = provider();
        provider.id = id.to_string();
        provider.adapter_id = adapter_id.to_string();
        provider
    }

    fn adapter(
        id: &str,
        protocol_id: &str,
        kind: &str,
        config: Value,
    ) -> GatewayAdapterRegistration {
        GatewayAdapterRegistration {
            id: id.to_string(),
            name: id.to_string(),
            protocol_id: protocol_id.to_string(),
            kind: kind.to_string(),
            config,
        }
    }

    fn builtin_adapter(id: &str, protocol_id: &str) -> GatewayAdapterRegistration {
        adapter(id, protocol_id, "builtin", Value::Null)
    }

    fn declarative_adapter(
        id: &str,
        protocol_id: &str,
        supports_tools: bool,
    ) -> GatewayAdapterRegistration {
        let mut config = json!({
            "schemaVersion": 1,
            "configVersion": 1,
            "request": {
                "method": "POST",
                "path": "/v1/chat",
                "body": {}
            },
            "response": {
                "contentPath": "choices.0.message.content",
                "usagePath": "usage",
                "promptTokensPath": "usage.prompt_tokens",
                "completionTokensPath": "usage.completion_tokens"
            },
            "capabilities": {
                "tools": supports_tools,
                "streaming": true,
                "multimodal": false,
                "reasoning": false,
                "structuredOutput": false,
                "usage": true
            }
        });
        config["request"]["body"]["stream"] = json!("{{request.stream}}");
        if supports_tools {
            config["request"]["body"]["tools"] = json!("{{request.tools}}");
        }
        adapter(id, protocol_id, "declarative", config)
    }

    #[test]
    fn provider_runtime_id_is_used_by_conversion() {
        let protocol = ApiProtocol::from_str("responses");
        let config = extension_provider_config("example", &provider(), &protocol).unwrap();
        assert_eq!(config.id, "extension:example/provider");
        assert_eq!(config.provider_type, "extension:example/provider");
        assert_eq!(config.models[0].api_protocol, ApiProtocol::Responses);
    }

    #[test]
    fn protocol_is_resolved_before_provider_conversion() {
        let protocol = ApiProtocol::from_str("custom:extension:example/adapter");
        let config = extension_provider_config("example", &provider(), &protocol).unwrap();
        assert_eq!(config.models[0].api_protocol, protocol);
    }

    #[test]
    fn provider_uses_protocol_and_keeps_model_capabilities_independent_of_adapter() {
        let plan = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![
                    declarative_adapter("adapter-a", "mimo-v1", true),
                    declarative_adapter("adapter-b", "other-v1", false),
                ],
                providers: vec![
                    provider_with("provider-b", "adapter-b"),
                    provider_with("provider-a", "adapter-a"),
                ],
            },
        )
        .unwrap();

        assert_eq!(plan.adapters[0].adapter_id, "adapter-a");
        assert_eq!(plan.adapters[1].adapter_id, "adapter-b");
        assert_eq!(
            plan.providers[0].config.models[0].api_protocol,
            ApiProtocol::Custom("other-v1".to_string())
        );
        assert!(plan.providers[0].config.models[0].supports_tools);
        assert_eq!(
            plan.providers[1].config.models[0].api_protocol,
            ApiProtocol::Custom("mimo-v1".to_string())
        );
        assert!(plan.providers[1].config.models[0].supports_tools);
    }

    #[test]
    fn duplicate_protocol_ids_are_rejected() {
        let error = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![
                    builtin_adapter("adapter-a", "responses"),
                    builtin_adapter("adapter-b", "responses"),
                ],
                providers: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("Duplicate Gateway protocolId 'responses'"));
    }

    #[test]
    fn explicit_custom_protocol_ids_are_rejected() {
        let error = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![declarative_adapter("adapter", "custom:mimo-v1", true)],
                providers: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("must not use the explicit custom: prefix"));
    }

    #[test]
    fn protocol_ids_with_whitespace_are_rejected() {
        let error = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![declarative_adapter("adapter", "mimo v1", true)],
                providers: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("cannot contain whitespace"));
    }

    #[test]
    fn builtin_and_declarative_kinds_are_enforced() {
        let builtin_error = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![adapter("adapter", "responses", "declarative", Value::Null)],
                providers: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(builtin_error.contains("must use adapter kind 'builtin'"));

        let declarative_error = build_gateway_plan(
            "example",
            &GatewayContributions {
                adapters: vec![adapter("adapter", "mimo-v1", "builtin", Value::Null)],
                providers: Vec::new(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(declarative_error.contains("must use adapter kind 'declarative'"));
    }

    #[test]
    fn mcp_override_server_ids_are_stable() {
        assert_eq!(
            mcp_override_server_id("ext", "server"),
            "extension:ext/server"
        );
        assert_eq!(
            mcp_override_server_id("ext", "extension:other/server"),
            "extension:other/server"
        );
    }
}
