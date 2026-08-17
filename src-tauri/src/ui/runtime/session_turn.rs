// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

// use [REMOVED: domains reference]
use crate::extension::types::{ChatMessage as GatewayProtocolMessage, Gateway};
// use [REMOVED: domains reference]
use crate::foundation::stream::{send_channel_value, StreamChunk as CoreStreamChunk};
// use [REMOVED: domains reference]
// use [REMOVED: MCP reference]
use crate::ui::messages::{
    session_messages, storage_assistant_shell_message, storage_text_message,
    storage_user_message_with_attachments,
};
use crate::ui::SendSessionMessagePayload;
use serde_json::json;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tokio::time::Instant;

use super::super::tasks::context_model::current_or_default_model;
use super::super::timeline::persist_turn_prelude_step;
use super::super::{
    build_gateway_protocol_messages_for_turn, fail_agent_task, gateway_user_message_for_payload,
    persist_terminal_turn_step, rename_placeholder_session,
};
use super::tool_approval::ToolApprovalStore;

pub(crate) enum StreamSessionTurnStart {
    EarlyReturn,
    Ready(PreparedStreamSessionTurn),
}

pub(crate) struct PreparedStreamSessionTurn {
    pub(crate) task_id: String,
    pub(crate) agent_stream_id: String,
    pub(crate) turn_id: String,
    pub(crate) assistant_message_id: String,
    pub(crate) model: String,
    pub(crate) session: Session,
    pub(crate) gateway_messages: Vec<GatewayProtocolMessage>,
    pub(crate) next_sequence: u64,
    pub(crate) turn_started_at: Instant,
}

pub(crate) fn prepare_stream_session_turn(
    manager: &SessionManager,
    gateway: &Gateway,
    mcp: &MCP,
    task_manager: &Mutex<TaskManager>,
    approvals: &ToolApprovalStore,
    payload: &SendSessionMessagePayload,
    channel: &Channel,
) -> Result<StreamSessionTurnStart, String> {
    let content = payload.content.trim();
    let has_attachments = !payload.attachments.is_empty();
    if content.is_empty() && !has_attachments {
        let messages = session_messages(manager, Some(approvals), &payload.session_id)?;
        let chunk = CoreStreamChunk::data(
            "ui:empty",
            0,
            json!({ "type": "messages", "messages": messages.messages, "total": messages.total }),
        );
        let _ = send_channel_value(channel, chunk.channel_payload());
        let done = CoreStreamChunk::final_chunk("ui:empty", 1);
        let _ = send_channel_value(channel, done.channel_payload());
        return Ok(StreamSessionTurnStart::EarlyReturn);
    }

    let display_content = payload
        .display_content
        .as_deref()
        .map(str::trim)
        .unwrap_or(content);

    let task_id = {
        let mut tasks = task_manager
            .lock()
            .map_err(|_| "后台任务状态不可用".to_string())?;
        let task_id = tasks.create_task(&payload.session_id);
        if let Some(task) = tasks.get_task_mut(&task_id) {
            task.add_message(AgentChatMessage::user(if content.is_empty() {
                "[Attachments]".to_string()
            } else {
                content.to_string()
            }));
            task.mark_running();
        }
        task_id
    };
    let agent_stream_id = format!("agent:{}", task_id);
    let turn_started_at = Instant::now();
    let mut next_sequence = 0_u64;

    let user_message = if has_attachments {
        storage_user_message_with_attachments(
            &payload.session_id,
            display_content,
            &payload.attachments,
        )
    } else {
        storage_text_message(
            &payload.session_id,
//             [REMOVED: domains reference]
            display_content,
            None,
            None,
        )
    };
    let turn_id = user_message.id.clone();
    if let Err(error) = manager.add_message(&payload.session_id, user_message) {
        let message = error.to_string();
        let _ = fail_agent_task(task_manager, &task_id, &message);
        let chunk = CoreStreamChunk::error("ui:session", 0, message);
        let _ = send_channel_value(channel, chunk.channel_payload());
        return Ok(StreamSessionTurnStart::EarlyReturn);
    }

    let assistant_message =
        storage_assistant_shell_message(&payload.session_id, &turn_id, &agent_stream_id, None);
    let assistant_message_id = assistant_message.id.clone();
    if let Err(error) = manager.add_message(&payload.session_id, assistant_message) {
        let message = error.to_string();
        let _ = fail_agent_task(task_manager, &task_id, &message);
        let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
        let _ = send_channel_value(channel, chunk.channel_payload());
        return Ok(StreamSessionTurnStart::EarlyReturn);
    }

    if let Err(error) = persist_turn_prelude_step(
        manager,
        &payload.session_id,
        &turn_id,
        &assistant_message_id,
        &agent_stream_id,
        &mut next_sequence,
        channel,
        display_content,
    ) {
        let _ = fail_agent_task(task_manager, &task_id, &error);
        let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
        let _ = send_channel_value(channel, chunk.channel_payload());
        return Ok(StreamSessionTurnStart::EarlyReturn);
    }

    if let Err(error) = rename_placeholder_session(manager, &payload.session_id, display_content) {
        let _ = fail_agent_task(task_manager, &task_id, &error);
        persist_terminal_turn_step(
            manager,
            &payload.session_id,
            &turn_id,
            &assistant_message_id,
            &agent_stream_id,
            &mut next_sequence,
            Some(channel),
            "",
            None,
            "error",
            "Session rename failed",
            &error,
        );
        let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
        let _ = send_channel_value(channel, chunk.channel_payload());
        return Ok(StreamSessionTurnStart::EarlyReturn);
    }

    let model = match current_or_default_model(manager, gateway, &payload.session_id) {
        Ok(model) => model,
        Err(error) => {
            let _ = fail_agent_task(task_manager, &task_id, &error);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                Some(channel),
                "",
                None,
                "error",
                "Model unavailable",
                &error,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
            let _ = send_channel_value(channel, chunk.channel_payload());
            return Ok(StreamSessionTurnStart::EarlyReturn);
        }
    };

    let session = match manager.get(&payload.session_id) {
        Ok(Some(session)) => session,
        Ok(None) => {
            let message = format!("会话不存在: {}", payload.session_id);
            let _ = fail_agent_task(task_manager, &task_id, &message);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                Some(channel),
                "",
                Some(&model),
                "error",
                "Session not found",
                &message,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
            let _ = send_channel_value(channel, chunk.channel_payload());
            return Ok(StreamSessionTurnStart::EarlyReturn);
        }
        Err(error) => {
            let message = error.to_string();
            let _ = fail_agent_task(task_manager, &task_id, &message);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                Some(channel),
                "",
                Some(&model),
                "error",
                "Session load failed",
                &message,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
            let _ = send_channel_value(channel, chunk.channel_payload());
            return Ok(StreamSessionTurnStart::EarlyReturn);
        }
    };

    let gateway_messages = match manager.get_messages(&payload.session_id, Some(100), Some(0)) {
        Ok(messages) => {
            let execution_message = gateway_user_message_for_payload(content, &payload.attachments);
            build_gateway_protocol_messages_for_turn(
                mcp,
                &session,
                &messages,
                &turn_id,
                execution_message,
                true,
            )
        }
        Err(error) => {
            let message = error.to_string();
            let _ = fail_agent_task(task_manager, &task_id, &message);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                Some(channel),
                "",
                Some(&model),
                "error",
                "History load failed",
                &message,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
            let _ = send_channel_value(channel, chunk.channel_payload());
            return Ok(StreamSessionTurnStart::EarlyReturn);
        }
    };

    Ok(StreamSessionTurnStart::Ready(PreparedStreamSessionTurn {
        task_id,
        agent_stream_id,
        turn_id,
        assistant_message_id,
        model,
        session,
        gateway_messages,
        next_sequence,
        turn_started_at,
    }))
}
