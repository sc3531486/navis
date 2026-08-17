// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

//! Context usage and model selection helpers for UI commands.

use std::sync::Mutex;

use serde_json::Value;

// use [REMOVED: domains reference]
use crate::extension::types::Gateway;
use crate::foundation::config::Config;
// use [REMOVED: domains reference]

pub(crate) fn route_model_id(provider_id: &str, model_id: &str) -> String {
    format!("{}/{}", provider_id.trim(), model_id.trim())
}

pub(crate) fn configured_context_window(
    gateway: &Gateway,
    provider_id: Option<&str>,
    model_id: Option<&str>,
    fallback: usize,
) -> usize {
    let models = gateway.list_models_with_provider();
    let preferred_selection = provider_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .zip(model_id.map(str::trim).filter(|id| !id.is_empty()))
        .map(|(provider, model)| (provider.to_string(), model.to_string()))
        .or_else(|| gateway.preferred_default_model());

    preferred_selection
        .as_ref()
        .and_then(|(preferred_provider_id, preferred_model_id)| {
            models.iter().find(|(provider_id, model)| {
                provider_id == preferred_provider_id && model.id == *preferred_model_id
            })
        })
        .map(|(_, model)| model.context_window as usize)
        .filter(|tokens| *tokens > 0)
        .unwrap_or(fallback)
}

pub(crate) fn compression_threshold_percent(config: &Mutex<Config>) -> u8 {
    let ratio = context_config_value(config, "context.autoCompressThreshold")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.8)
        .clamp(0.0, 1.0);
    (ratio * 100.0).round() as u8
}

pub(crate) fn fallback_context_tokens(config: &Mutex<Config>) -> usize {
    context_config_value(config, "context.maxTokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(128_000)
}

pub(crate) fn estimate_session_tokens(
    manager: &SessionManager,
    session_id: &str,
    model_id: Option<&str>,
) -> Result<usize, String> {
    let total = manager
        .get_message_count(session_id)
        .map_err(|error| error.to_string())?;
    let counter = TokenCounter::new(tokenizer_for_model(model_id));
    let mut used_tokens = 0usize;
    let page_size = 500usize;
    let mut offset = 0usize;

    while offset < total {
        let messages = manager
            .get_messages(session_id, Some(page_size), Some(offset))
            .map_err(|error| error.to_string())?;
        if messages.is_empty() {
            break;
        }

        for message in messages {
            let message_tokens = message
                .token_count
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value > 0)
                .unwrap_or_else(|| counter.count_tokens(&message.content.to_string()));
            used_tokens = used_tokens.saturating_add(message_tokens.saturating_add(4));
        }

        offset += page_size;
    }

    Ok(used_tokens)
}

pub(crate) fn current_or_default_model(
    manager: &SessionManager,
    gateway: &Gateway,
    session_id: &str,
) -> Result<String, String> {
    let session = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;

    if let Some((provider_id, model_id)) = session
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .zip(
            session
                .model_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty()),
        )
    {
        return Ok(route_model_id(provider_id, model_id));
    }

    let (provider_id, model_id) = gateway.preferred_default_model().ok_or_else(|| {
        "Gateway 尚未配置可用模型，请先在 Settings > Gateway 添加模型".to_string()
    })?;

    manager
        .update_model_selection(session_id, &provider_id, &model_id)
        .map_err(|error| error.to_string())?;
    Ok(route_model_id(&provider_id, &model_id))
}

fn context_config_value(config: &Mutex<Config>, key: &str) -> Option<Value> {
    config.lock().ok()?.get(key)
}

fn tokenizer_for_model(model_id: Option<&str>) -> TokenizerType {
    let normalized = model_id.unwrap_or_default().to_ascii_lowercase();
    if normalized.contains("claude") {
        TokenizerType::Claude
    } else if normalized.contains("llama") {
        TokenizerType::Llama
    } else if normalized.contains("gpt-4o")
        || normalized.starts_with('o')
        || normalized.contains("o200k")
    {
        TokenizerType::O200K
    } else {
        TokenizerType::CL100K
    }
}
