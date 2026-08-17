// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use crate::extension::types::Gateway;
use crate::foundation::config::Config;
// use [REMOVED: domains reference]
use crate::ui::dto::{SessionIdPayload, UiSessionContextUsage};
use crate::ui::tasks::context_model::{
    compression_threshold_percent, configured_context_window, estimate_session_tokens,
    fallback_context_tokens, route_model_id,
};
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn ui_get_session_context_usage(
    manager: State<'_, Arc<SessionManager>>,
    gateway: State<'_, Arc<Gateway>>,
    config: State<'_, Arc<Mutex<Config>>>,
    payload: SessionIdPayload,
) -> Result<UiSessionContextUsage, String> {
    let manager = manager.inner().as_ref();
    let gateway = gateway.inner().as_ref();
    let config = config.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let provider_id = session.provider_id.clone();
    let model_id = session.model_id.clone();
    let model = provider_id
        .as_deref()
        .zip(model_id.as_deref())
        .map(|(provider_id, model_id)| route_model_id(provider_id, model_id));
    let fallback_tokens = fallback_context_tokens(config);
    let total_tokens = configured_context_window(
        gateway,
        provider_id.as_deref(),
        model_id.as_deref(),
        fallback_tokens,
    );
    let used_tokens = estimate_session_tokens(manager, &payload.session_id, model.as_deref())?;
    let used_percent = if total_tokens == 0 {
        0
    } else {
        ((used_tokens as f64 / total_tokens as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u8
    };

    Ok(UiSessionContextUsage {
        session_id: payload.session_id,
        model,
        used_tokens,
        total_tokens,
        used_percent,
        compression_threshold_percent: compression_threshold_percent(config),
    })
}
