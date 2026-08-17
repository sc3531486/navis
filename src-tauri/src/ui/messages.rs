// ── 归属扩展：navis-session ──
// 迁移目标：extensions/navis-session/ExtensionBackend/src/

use super::agent_timeline_part::{ui_agent_timeline_part, UiAgentTimelinePart};
use super::dto::*;
use super::ToolApprovalStore;
use crate::domains::ai_platform::gateway::{
    ChatMessage as GatewayProtocolMessage, ContentPart as GatewayContentPart,
    FileContent as GatewayFileContent, ImageContent as GatewayImageContent,
    ImageMediaType as GatewayImageMediaType, ImageSourceType as GatewayImageSourceType,
    MessageContent as GatewayMessageContent, MessageRole as GatewayMessageRole,
    TextContent as GatewayTextContent,
};
use crate::domains::session::session::{
    Message, MessageContent as StorageMessageContent, MessageRole as StorageMessageRole,
    SessionChange,
};
use crate::domains::session::session::SessionManager;
use crate::domains::session::session::TimelineStatus;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn ui_list_session_messages(
    manager: State<'_, Arc<SessionManager>>,
    approvals: State<'_, Arc<ToolApprovalStore>>,
    payload: ListSessionMessagesPayload,
) -> Result<UiSessionMessages, String> {
    let manager = manager.inner().as_ref();
    let approvals = approvals.inner().as_ref();
    let limit = payload.limit.unwrap_or(100).clamp(1, 500);
    let total = manager
        .get_message_count(&payload.session_id)
        .map_err(|error| error.to_string())?;
    let offset = if payload.latest {
        total.saturating_sub(limit)
    } else {
        payload.offset.unwrap_or(0)
    };
    let storage_messages = manager
        .get_messages(&payload.session_id, Some(limit), Some(offset))
        .map_err(|error| error.to_string())?;
    let messages = ui_chat_messages_for_storage(
        manager,
        Some(approvals),
        &payload.session_id,
        storage_messages,
    )?;
    Ok(UiSessionMessages { messages, total })
}

pub(crate) fn session_messages(
    manager: &SessionManager,
    approvals: Option<&ToolApprovalStore>,
    session_id: &str,
) -> Result<UiSessionMessages, String> {
    let total = manager
        .get_message_count(session_id)
        .map_err(|error| error.to_string())?;
    let limit = 100;
    let offset = total.saturating_sub(limit);
    let storage_messages = manager
        .get_messages(session_id, Some(limit), Some(offset))
        .map_err(|error| error.to_string())?;
    let messages = ui_chat_messages_for_storage(manager, approvals, session_id, storage_messages)?;
    Ok(UiSessionMessages { messages, total })
}

pub(crate) fn ui_chat_messages_for_storage(
    manager: &SessionManager,
    approvals: Option<&ToolApprovalStore>,
    session_id: &str,
    storage_messages: Vec<Message>,
) -> Result<Vec<UiChatMessage>, String> {
    abort_stale_running_agent_timeline_parts(manager, approvals, session_id)?;

    let mut parts_by_message: HashMap<String, Vec<UiAgentTimelinePart>> = HashMap::new();
    for part in manager
        .get_agent_timeline_parts(session_id)
        .map_err(|error| error.to_string())?
    {
        parts_by_message
            .entry(part.message_id.clone())
            .or_default()
            .push(ui_agent_timeline_part(part));
    }

    let messages = storage_messages
        .into_iter()
        .filter_map(|message| {
            let agent_timeline_parts = parts_by_message
                .get(&message.id)
                .cloned()
                .unwrap_or_default();
            let has_visible_agent_timeline_part = agent_timeline_parts
                .iter()
                .any(ui_agent_timeline_part_visible_in_history);
            if is_running_assistant_shell(&message) && !has_visible_agent_timeline_part {
                return None;
            }
            Some(ui_chat_message(message, agent_timeline_parts))
        })
        .collect();

    Ok(messages)
}

fn ui_chat_message(
    message: Message,
    agent_timeline_parts: Vec<UiAgentTimelinePart>,
) -> UiChatMessage {
    let content = message_content_text(&message.content);
    let attachments = message_attachments(&message);

    UiChatMessage {
        id: message.id,
        session_id: message.session_id,
        role: message.role.as_str().to_string(),
        content,
        attachments,
        token_count: message.token_count,
        created_at: message.created_at,
        agent_timeline_parts,
    }
}

fn message_attachments(message: &Message) -> Vec<UiChatMessageAttachment> {
    message
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("ui"))
        .and_then(|ui| ui.get("attachments"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_else(|| message_content_attachments(&message.content))
}

pub(crate) fn message_content_text(content: &StorageMessageContent) -> String {
    match content {
        StorageMessageContent::Text(text) => text.clone(),
        StorageMessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                crate::domains::session::session::store::ContentPart::Text(text) => {
                    Some(text.text.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn message_content_attachments(content: &StorageMessageContent) -> Vec<UiChatMessageAttachment> {
    match content {
        StorageMessageContent::Text(_) => Vec::new(),
        StorageMessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                crate::domains::session::session::store::ContentPart::Image(image) => {
                    Some(UiChatMessageAttachment {
                        kind: "image".to_string(),
                        name: "Image".to_string(),
                        mime_type: Some(image.media_type.clone()),
                        size_bytes: Some(image.data.len() as u64),
                        data_base64: Some(image.data.clone()),
                        text_content: None,
                        is_truncated: Some(false),
                        model_readable: Some(true),
                    })
                }
                crate::domains::session::session::store::ContentPart::File(file) => {
                    Some(UiChatMessageAttachment {
                        kind: "file".to_string(),
                        name: file.name.clone(),
                        mime_type: None,
                        size_bytes: Some(file.content.len() as u64),
                        data_base64: None,
                        text_content: Some(file.content.clone()),
                        is_truncated: Some(false),
                        model_readable: Some(true),
                    })
                }
                crate::domains::session::session::store::ContentPart::Text(_) => None,
            })
            .collect(),
    }
}

pub(crate) fn storage_text_message(
    session_id: &str,
    role: StorageMessageRole,
    content: impl Into<String>,
    token_count: Option<i64>,
    metadata: Option<Value>,
) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        role,
        content: StorageMessageContent::Text(content.into()),
        token_count,
        tool_calls: None,
        tool_result: None,
        created_at: Utc::now().to_rfc3339(),
        metadata,
    }
}

pub(crate) fn storage_user_message_with_attachments(
    session_id: &str,
    display_content: &str,
    attachments: &[UiChatMessageAttachment],
) -> Message {
    let mut parts = Vec::new();
    if !display_content.is_empty() {
        parts.push(
            crate::domains::session::session::store::ContentPart::Text(
                crate::domains::session::session::store::TextContent {
                    text: display_content.to_string(),
                },
            ),
        );
    }

    for attachment in attachments {
        match attachment.kind.as_str() {
            "image" => {
                if let Some(data) = attachment
                    .data_base64
                    .as_deref()
                    .filter(|data| !data.is_empty())
                {
                    parts.push(
                        crate::domains::session::session::store::ContentPart::Image(
                            crate::domains::session::session::store::ImageContent {
                                media_type: attachment
                                    .mime_type
                                    .clone()
                                    .unwrap_or_else(|| "image/png".to_string()),
                                data: data.to_string(),
                            },
                        ),
                    );
                }
            }
            "file" => {
                if let Some(text) = attachment
                    .text_content
                    .as_deref()
                    .filter(|text| !text.is_empty())
                {
                    parts.push(
                        crate::domains::session::session::store::ContentPart::File(
                            crate::domains::session::session::store::FileContent {
                                name: attachment.name.clone(),
                                content: text.to_string(),
                            },
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        role: StorageMessageRole::User,
        content: StorageMessageContent::Parts(parts),
        token_count: None,
        tool_calls: None,
        tool_result: None,
        created_at: Utc::now().to_rfc3339(),
        metadata: Some(json!({
            "ui": {
                "attachments": attachments,
            }
        })),
    }
}

pub(crate) fn storage_assistant_message(
    session_id: &str,
    content: impl Into<String>,
    token_count: Option<i64>,
    metadata: Option<Value>,
) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        role: StorageMessageRole::Assistant,
        content: StorageMessageContent::Text(content.into()),
        token_count,
        tool_calls: None,
        tool_result: None,
        created_at: Utc::now().to_rfc3339(),
        metadata,
    }
}

pub(crate) fn storage_assistant_shell_message(
    session_id: &str,
    turn_id: &str,
    stream_id: &str,
    model: Option<&str>,
) -> Message {
    storage_assistant_message(
        session_id,
        "",
        None,
        Some(json!({
            "gateway": {
                "turnId": turn_id,
                "streamId": stream_id,
                "model": model,
                "status": "running",
            }
        })),
    )
}

pub(crate) fn gateway_protocol_message_from_session_message(
    message: &Message,
) -> GatewayProtocolMessage {
    match message.role {
        StorageMessageRole::System => {
            GatewayProtocolMessage::system(message_content_text(&message.content))
        }
        StorageMessageRole::Assistant => {
            GatewayProtocolMessage::assistant(message_content_text(&message.content))
        }
        StorageMessageRole::Tool => GatewayProtocolMessage {
            role: GatewayMessageRole::Tool,
            content: GatewayMessageContent::Text(message_content_text(&message.content)),
            tool_calls: None,
            tool_call_id: None,
        },
        StorageMessageRole::User => gateway_user_message_from_storage_content(&message.content),
    }
}

fn gateway_user_message_from_storage_content(
    content: &StorageMessageContent,
) -> GatewayProtocolMessage {
    match content {
        StorageMessageContent::Text(text) => GatewayProtocolMessage::user(text.clone()),
        StorageMessageContent::Parts(parts) => {
            let gateway_parts: Vec<GatewayContentPart> = parts
                .iter()
                .filter_map(|part| match part {
                    crate::domains::session::session::store::ContentPart::Text(text) => {
                        Some(GatewayContentPart::Text(GatewayTextContent {
                            text: text.text.clone(),
                        }))
                    }
                    crate::domains::session::session::store::ContentPart::Image(image) => {
                        let media_type =
                            GatewayImageMediaType::from_mime_str(&image.media_type).ok()?;
                        Some(GatewayContentPart::Image(GatewayImageContent {
                            media_type,
                            data: image.data.clone(),
                            source_type: GatewayImageSourceType::Base64,
                            url: None,
                            width: None,
                            height: None,
                            size_bytes: image.data.len() as u64,
                        }))
                    }
                    crate::domains::session::session::store::ContentPart::File(file) => {
                        Some(GatewayContentPart::File(GatewayFileContent {
                            file_name: file.name.clone(),
                            file_type: "document".to_string(),
                            text_content: file.content.clone(),
                            is_truncated: false,
                            mime_type: None,
                        }))
                    }
                })
                .collect();
            if gateway_parts.is_empty() {
                GatewayProtocolMessage::user(String::new())
            } else {
                GatewayProtocolMessage::user_parts(gateway_parts)
            }
        }
    }
}

pub(crate) fn is_running_assistant_shell(message: &Message) -> bool {
    message.role == StorageMessageRole::Assistant
        && message_content_text(&message.content).trim().is_empty()
        && message
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("gateway"))
            .and_then(|gateway| gateway.get("status"))
            .and_then(|status| status.as_str())
            == Some("running")
}

fn ui_agent_timeline_part_visible_in_history(step: &UiAgentTimelinePart) -> bool {
    let is_inactive_prelude = step.source.as_deref() == Some("turn_prelude")
        && !step
            .status
            .as_deref()
            .is_some_and(|status| TimelineStatus::parse(status).is_live());
    !is_inactive_prelude
}

fn abort_stale_running_agent_timeline_parts(
    manager: &SessionManager,
    approvals: Option<&ToolApprovalStore>,
    session_id: &str,
) -> Result<(), String> {
    let mut turns = HashSet::new();
    let mut has_aborted_turns = false;
    for step in manager
        .get_agent_timeline_parts(session_id)
        .map_err(|error| error.to_string())?
    {
        let is_running = step.status.as_ref().is_some_and(TimelineStatus::is_live);
        if !is_running {
            continue;
        }
        if turns.insert((step.turn_id.clone(), step.message_id.clone())) {
            manager
                .abort_running_agent_timeline_parts(session_id, &step.turn_id, &step.message_id)
                .map_err(|error| error.to_string())?;
            has_aborted_turns = true;
        }
    }
    if has_aborted_turns {
        if let Some(approvals) = approvals {
            let aborted = approvals.abort_session_pending(session_id)?;
            if aborted > 0 {
                tracing::debug!(
                    session_id = session_id,
                    approvals = aborted,
                    "Aborted stale pending tool approvals"
                );
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn ui_list_session_changes(
    manager: State<'_, Arc<SessionManager>>,
    payload: ListSessionChangesPayload,
) -> Result<Vec<SessionChange>, String> {
    manager
        .inner()
        .as_ref()
        .list_session_changes(&payload.session_id, payload.turn_id.as_deref())
        .map_err(|error| error.to_string())
}
