//! UI Framework IPC commands.
//! 模块归属标注:
//! 🏠 框架层: dto, extension_bridge, extension_network, extension_router, extension_storage,
//!   extension_stream, extensions, host_view, tauri_events, permissions, menus
//! 🔌 navis-ai-platform: gateway, lsp
//! 🔌 navis-session: messages, sessions, session_metadata
//! 🔌 navis-agent-core: timeline, agent_timeline_part, composer_projection, composer_run_state, runtime/
//! 🔌 navis-task: tasks/
//! 🔌 navis-settings: settings
//! 🔌 navis-terminal: terminal
//! 🔌 navis-project: worktree

mod dto;
pub mod extension_bridge;
pub mod extension_network;
pub mod extension_router;
pub mod extension_storage;
pub mod extension_stream;
pub mod extensions;
pub(crate) mod host_view;
mod permissions;
pub mod menus;
pub mod tauri_events;
pub mod gateway;
pub mod lsp;
pub mod messages;
pub mod sessions;
mod session_metadata;
mod agent_timeline_part;
mod composer_projection;
mod composer_run_state;
mod runtime;
mod timeline;
pub mod tasks;
pub mod settings;
pub mod terminal;
pub mod worktree;

use crate::extension::ExtensionStore;
use crate::foundation::config::Config;
use crate::foundation::stream::StreamIndex;
#[cfg(test)]
use crate::security::sandbox::permission::ApprovalMode;
use crate::domains::agent_core::agent::turn_output::assistant_visible_content;
use crate::domains::agent_core::agent::{mode_config_from_key, AgentTurnContext, TaskManager, TaskRecord};
use crate::domains::agent_core::tool_runtime::{agent_tool_definitions, sidechain_agent_tool_definitions};
use crate::domains::ai_platform::gateway::{
    ChatMessage as GatewayProtocolMessage, ChatRequest, ContentPart as GatewayContentPart,
    Gateway, ImageContent as GatewayImageContent,
    ImageMediaType as GatewayImageMediaType, ImageSourceType as GatewayImageSourceType,
    MessageRole as GatewayMessageRole,
    multimodal::{FileContent as GatewayFileContent, TextContent as GatewayTextContent},
};
use crate::domains::ai_platform::mcp::MCP;
use crate::domains::session::session::{
    Message, MessageContent as StorageMessageContent, MessageRole as StorageMessageRole, Session,
    SessionManager, TimelineStatus,
};
use agent_timeline_part::record_agent_timeline_part;
pub(crate) use composer_run_state::*;
pub use dto::*;
use messages::{gateway_protocol_message_from_session_message, is_running_assistant_shell};
use runtime::agent_tool_loop::AgentBackend;
use runtime::session_message_stream::stream_session_message;
pub use runtime::tool_approval::ToolApprovalStore;
use serde_json::{json, Value};
pub(crate) use session_metadata::*;
use settings::tool_permission_rules_from_config;
use std::sync::{Arc, Mutex};
use tauri::{ipc::Channel, State};
use timeline::{error_agent_timeline_part, text_agent_timeline_part};

fn update_task_record(task_manager: &Mutex<TaskManager>, task_id: &str, update: impl FnOnce(&mut TaskRecord)) -> Result<(), String> {
    let mut tasks = task_manager.lock().map_err(|_| "后台任务状态不可用".to_string())?;
    let task = tasks.get_task_mut(task_id).ok_or_else(|| format!("后台任务不存在: {}", task_id))?;
    update(task);
    Ok(())
}

fn fail_agent_task(task_manager: &Mutex<TaskManager>, task_id: &str, error: &str) -> Result<(), String> {
    update_task_record(task_manager, task_id, |task| task.mark_failed(error))
}

fn session_mode(session: &Session) -> Option<String> {
    session.metadata.as_ref().and_then(|m| m.get("ui").and_then(|ui| ui.get("mode")).and_then(|v| v.as_str())).map(|s| s.to_string())
}

fn session_permission_policy(session: &Session) -> Option<String> {
    session.metadata.as_ref().and_then(|m| m.get("permissionPolicy").and_then(|v| v.as_str())).map(|s| s.to_string())
}

fn build_gateway_protocol_messages(mcp: &MCP, session: &Session, storage_messages: &[Message], allow_sidechain_tools: bool) -> Vec<GatewayProtocolMessage> {
    let mode_config = mode_config_from_key(session_mode(session).as_deref());
    let runtime_tools = if allow_sidechain_tools { agent_tool_definitions(mcp, &mode_config) } else { sidechain_agent_tool_definitions(mcp, &mode_config) };
    let turn_context = AgentTurnContext::new(&session.id, mode_config, session.worktree_root.as_deref(), session.system_prompt.as_deref()).with_runtime_tools(runtime_tools);
    let mut messages = Vec::with_capacity(storage_messages.len() + 1);
    messages.push(GatewayProtocolMessage::system(turn_context.system_prompt()));
    messages.extend(storage_messages.iter().filter(|m| !is_running_assistant_shell(m)).map(gateway_protocol_message_from_session_message));
    messages
}

fn build_gateway_protocol_messages_for_turn(mcp: &MCP, session: &Session, storage_messages: &[Message], turn_id: &str, execution_message: GatewayProtocolMessage, allow_sidechain_tools: bool) -> Vec<GatewayProtocolMessage> {
    let mut messages = build_gateway_protocol_messages(mcp, session, storage_messages, allow_sidechain_tools);
    let matches = storage_messages.iter().any(|m| m.id == turn_id && m.role == StorageMessageRole::User);
    if matches { if let Some(msg) = messages.iter_mut().rev().find(|m| m.role == GatewayMessageRole::User) { *msg = execution_message; } } else { messages.push(execution_message); }
    messages
}

pub(crate) fn session_chat_request(_session: &Session, model: String, messages: Vec<GatewayProtocolMessage>) -> ChatRequest {
    ChatRequest::new(model, messages)
}

pub(crate) fn gateway_user_message_for_payload(content: &str, attachments: &[UiChatMessageAttachment]) -> GatewayProtocolMessage {
    let mut parts: Vec<GatewayContentPart> = Vec::new();
    if !content.is_empty() { parts.push(GatewayContentPart::Text(GatewayTextContent { text: content.to_string() })); }
    for att in attachments {
        if att.kind == "image" {
            if let Some(ref data) = att.data_base64 {
                let mt = match att.mime_type.as_deref() { Some("image/jpeg") | Some("image/jpg") => GatewayImageMediaType::Jpeg, Some("image/gif") => GatewayImageMediaType::Gif, Some("image/webp") => GatewayImageMediaType::WebP, _ => GatewayImageMediaType::Png };
                parts.push(GatewayContentPart::Image(GatewayImageContent { media_type: mt, data: data.clone(), source_type: GatewayImageSourceType::Base64, url: None, width: None, height: None, size_bytes: 0 }));
            }
        } else if att.kind == "file" {
            if let Some(ref text) = att.text_content {
                parts.push(GatewayContentPart::File(GatewayFileContent { file_name: att.name.clone(), file_type: "document".to_string(), text_content: text.clone(), is_truncated: att.is_truncated.unwrap_or(false), mime_type: att.mime_type.clone() }));
            }
        }
    }
    if parts.len() == 1 { if let Some(GatewayContentPart::Text(text)) = parts.pop() { return GatewayProtocolMessage::user(text.text); } }
    GatewayProtocolMessage::user_parts(parts)
}

pub(crate) fn assistant_gateway_metadata(turn_id: &str, agent_stream_id: &str, model: Option<&str>, response_id: Option<&str>, finish_reason: Option<&str>, usage: Option<Value>, status: &str) -> Value {
    let mut meta = json!({"turnId": turn_id, "agentStreamId": agent_stream_id, "status": status});
    if let Some(m) = model { meta["model"] = json!(m); }
    if let Some(id) = response_id { meta["responseId"] = json!(id); }
    if let Some(r) = finish_reason { meta["finishReason"] = json!(r); }
    if let Some(u) = usage { meta["usage"] = u; }
    meta
}

pub(crate) fn persist_terminal_turn_step(manager: &SessionManager, session_id: &str, turn_id: &str, assistant_message_id: &str, agent_stream_id: &str, next_sequence: &mut u64, channel: Option<&Channel>, _content: &str, _model: Option<&str>, status: &str, description: &str, _error: &str) -> Result<(), String> {
    let ts = match status { "completed" => TimelineStatus::Completed, "error" => TimelineStatus::Error, "running" => TimelineStatus::Running, _ => TimelineStatus::Unknown(status.to_string()) };
    let step = timeline::text_agent_timeline_part(session_id, turn_id, assistant_message_id, description, ts);
    agent_timeline_part::record_agent_timeline_part(manager, agent_stream_id, next_sequence, channel, step).map(|_| ())
}

pub(crate) fn update_assistant_shell(manager: &SessionManager, session_id: &str, message_id: &str, content: &str, tokens: Option<i64>, metadata: Value) -> Result<(), String> {
    let msg_content = StorageMessageContent::Text(content.to_string());
    manager.update_message_content(session_id, message_id, msg_content, tokens, Some(metadata)).map_err(|e| e.to_string())
}

pub(crate) fn merge_ui_metadata(manager: &SessionManager, session_id: &str, pairs: Vec<(&str, Value)>) -> Result<(), String> {
    let mut meta = json!({});
    for (k, v) in pairs { meta[k] = v; }
    manager.update_metadata(session_id, Some(meta)).map_err(|e| e.to_string())
}

pub(crate) fn rename_placeholder_session(manager: &SessionManager, session_id: &str, display_content: &str) -> Result<(), String> {
    let title = display_content.chars().take(60).collect::<String>();
    if title.trim().is_empty() { return Ok(()); }
    manager.update(session_id, Some(&title), None, None).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ui_stream_session_message(
    backend: State<'_, AgentBackend>,
    stream_index: State<'_, StreamIndex>,
    approvals: State<'_, Arc<ToolApprovalStore>>,
    payload: SendSessionMessagePayload,
    channel: Channel,
) -> Result<(), String> {
    stream_session_message(backend.inner().clone(), Arc::new(stream_index.inner().clone()), approvals.inner().clone(), payload, channel).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder() {}
}
