// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

// use [REMOVED: domains reference]
    assistant_visible_content, assistant_visible_delta, has_open_internal_details_block,
    should_buffer_potential_text_tool_call,
};
// use [REMOVED: domains reference]
use crate::extension::types::ChatMessage as GatewayProtocolMessage;
use crate::foundation::stream::{
    send_channel_value, stream_kind, StreamChunk as CoreStreamChunk, StreamIndex, StreamSource,
};
// use [REMOVED: domains reference]
use crate::security::sandbox::permission::ApprovalMode;
// use [REMOVED: domains reference]
    assistant_tool_message, effective_gateway_tool_call, is_supported_gateway_tool,
    parse_text_tool_calls, tool_call_arguments, tool_call_summary, tool_started_event,
    AgentToolEvent, AgentToolPhase, AgentToolStatus,
};
use crate::ui::agent_timeline_part::{agent_text_part_id, publish_agent_timeline_part_delta};
use crate::ui::messages::session_messages;
use crate::ui::runtime::agent_tool_loop::{
    execute_ui_agent_tool_call_with_pipeline, response_text, run_agent_tool_loop, AgentBackend,
    AgentToolLoopOutcome,
};
use crate::ui::runtime::session_change_capture::{
    prepare_session_change_capture, record_completed_session_change,
};
use crate::ui::runtime::session_turn::{
    prepare_stream_session_turn, PreparedStreamSessionTurn, StreamSessionTurnStart,
};
use crate::ui::runtime::tool_approval::ToolApprovalStore;
use crate::ui::runtime::tool_approval_flow::{
    auto_tool_approval_reason, resolve_tool_approval_for_event, tool_approval_request,
    unsupported_gateway_tool_result,
};
use crate::ui::timeline::{
    persist_gateway_retry_agent_timeline_part, persist_text_agent_timeline_part,
    persist_turn_finalizer_step, send_agent_tool_event, ToolTimelineSequencer,
};
use crate::ui::{
    assistant_gateway_metadata, fail_agent_task, persist_terminal_turn_step, session_chat_request,
    session_mode, session_permission_policy, update_assistant_shell, update_task_record,
    SendSessionMessagePayload,
};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tauri::ipc::Channel;
use tokio::time::{timeout, Duration};

struct ManagedAgentStream {
    stream_index: Arc<StreamIndex>,
    stream_id: String,
    tracked: bool,
}

impl ManagedAgentStream {
    fn new(stream_index: Arc<StreamIndex>, stream_id: impl Into<String>) -> Self {
        Self {
            stream_index,
            stream_id: stream_id.into(),
            tracked: false,
        }
    }

    fn mark_tracked(&mut self) {
        self.tracked = true;
    }

    fn record_send(&self) {
        if self.tracked {
            self.stream_index.record_send(&self.stream_id);
        }
    }

    fn untrack(&self) {
        if self.tracked {
            self.stream_index.untrack(&self.stream_id);
        }
    }
}

pub(crate) async fn stream_session_message(
    backend: AgentBackend,
    stream_index: Arc<StreamIndex>,
    approvals: Arc<ToolApprovalStore>,
    payload: SendSessionMessagePayload,
    channel: Channel,
) -> Result<(), String> {
    let skills = backend.skills.clone();
    let manager = backend.manager.as_ref();
    let gateway = backend.gateway.as_ref();
    let mcp = backend.mcp.as_ref();
    let task_manager = backend.task_manager.as_ref();
    let approvals = approvals.as_ref();
    let permission_rules = &backend.permission_rules;
    let PreparedStreamSessionTurn {
        task_id,
        agent_stream_id,
        turn_id,
        assistant_message_id,
        model,
        session,
        gateway_messages,
        mut next_sequence,
        turn_started_at,
    } = match prepare_stream_session_turn(
        manager,
        gateway,
        mcp,
        task_manager,
        approvals,
        &payload,
        &channel,
    )? {
        StreamSessionTurnStart::EarlyReturn => return Ok(()),
        StreamSessionTurnStart::Ready(turn) => turn,
    };

    let mut tool_timeline = ToolTimelineSequencer::default();
    let mut tool_events = Vec::new();
    let mut tool_preludes = Vec::new();
    let gateway_messages = match run_agent_tool_loop(
        Some(manager),
        gateway,
        mcp,
        &session,
        &model,
        gateway_messages,
        Some(&channel),
        &agent_stream_id,
        &mut next_sequence,
        Some(&turn_id),
        &assistant_message_id,
        &mut tool_timeline,
        &mut tool_events,
        Some(approvals),
        permission_rules,
        Some(backend.extension_store.as_ref()),
        Some(&backend),
        Some(task_manager),
        None,
        Some(&mut tool_preludes),
        Some(skills.clone()),
    )
    .await
    {
        Ok(AgentToolLoopOutcome::FinalMessages(messages)) => messages,
        Ok(AgentToolLoopOutcome::Direct(response)) => {
            let assistant_content = response_text(&response);
            let visible_assistant_content =
                assistant_visible_content(&assistant_content, &tool_preludes);
            if let Err(error) = persist_text_agent_timeline_part(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                &channel,
                &visible_assistant_content,
                TimelineStatus::Completed,
            ) {
                let _ = fail_agent_task(task_manager, &task_id, &error);
                let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                return Ok(());
            }
            let metadata = assistant_gateway_metadata(
                &turn_id,
                &agent_stream_id,
                Some(&response.model),
                Some(&response.id),
                Some(&response.finish_reason),
                Some(json!(&response.usage)),
                "completed",
            );
            if let Err(error) = update_assistant_shell(
                manager,
                &payload.session_id,
                &assistant_message_id,
                &visible_assistant_content,
                Some(response.usage.total_tokens as i64),
                metadata,
            ) {
                let message = error.to_string();
                let _ = fail_agent_task(task_manager, &task_id, &message);
                let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                return Ok(());
            }
            if let Err(error) = persist_turn_finalizer_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                &channel,
                &tool_events,
                Some(response.usage.total_tokens as i64),
                Some(turn_started_at.elapsed().as_millis() as u64),
            ) {
                let _ = fail_agent_task(task_manager, &task_id, &error);
                let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                return Ok(());
            }
            update_task_record(task_manager, &task_id, |task| {
                task.add_message(AgentChatMessage::assistant(visible_assistant_content));
                task.mark_completed();
            })?;

            let messages = session_messages(manager, Some(approvals), &payload.session_id)?;
            let final_data = CoreStreamChunk::data(
                &agent_stream_id,
                next_sequence,
                json!({ "type": "messages", "messages": messages.messages, "total": messages.total }),
            );
            let _ = send_channel_value(&channel, final_data.channel_payload());
            let done =
                CoreStreamChunk::final_chunk(&agent_stream_id, next_sequence.saturating_add(1));
            let _ = send_channel_value(&channel, done.channel_payload());
            return Ok(());
        }
        Err(error) => {
            let _ = fail_agent_task(task_manager, &task_id, &error);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &agent_stream_id,
                &mut next_sequence,
                Some(&channel),
                "",
                Some(&model),
                "error",
                "Agent tool loop failed",
                &error,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, error);
            let _ = send_channel_value(&channel, chunk.channel_payload());
            return Ok(());
        }
    };

    let final_gateway_messages = gateway_messages.clone();
    let request = session_chat_request(&session, model.clone(), final_gateway_messages.clone())
        .with_stream(true);
    let mut receiver = match gateway.agent_router_stream(request).await {
        Ok(receiver) => receiver,
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
                Some(&channel),
                "",
                Some(&model),
                "error",
                "Stream failed to start",
                &message,
            );
            let chunk = CoreStreamChunk::error(&agent_stream_id, next_sequence, message);
            let _ = send_channel_value(&channel, chunk.channel_payload());
            return Ok(());
        }
    };

    let stream_id = receiver.stream_id().to_string();
    let mut managed_stream = ManagedAgentStream::new(stream_index.clone(), &stream_id);
    stream_index.track_with_cancel(
        &stream_id,
        StreamSource::new(stream_kind::AGENT, &payload.session_id)
            .with_meta("task_id", &task_id)
            .with_meta("session_id", &payload.session_id)
            .with_meta("turn_id", &turn_id)
            .with_meta("assistant_message_id", &assistant_message_id)
            .with_meta("model", &model),
        Some(receiver.cancel_token()),
    );
    managed_stream.mark_tracked();

    let mut assistant_content = String::new();
    let mut response_id: Option<String> = None;
    let mut finish_reason: Option<String> = None;
    let mut token_count: Option<i64> = None;
    let mut buffered_text_delta = String::new();
    let mut buffering_text_tool_call = false;
    let mut visible_stream_content = String::new();
    let stream_idle_timeout_secs = gateway.request_timeout_secs().saturating_add(120).max(1);
    let text_part_id = agent_text_part_id(&turn_id);
    if let Err(error) = persist_text_agent_timeline_part(
        manager,
        &payload.session_id,
        &turn_id,
        &assistant_message_id,
        &stream_id,
        &mut next_sequence,
        &channel,
        "",
        TimelineStatus::Running,
    ) {
        let _ = fail_agent_task(task_manager, &task_id, &error);
        let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }
    loop {
        let chunk = match timeout(
            Duration::from_secs(stream_idle_timeout_secs),
            receiver.recv(),
        )
        .await
        {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                let message = format!(
                    "流式读取超时：超过 {} 秒未收到新数据",
                    stream_idle_timeout_secs
                );
                let _ = fail_agent_task(task_manager, &task_id, &message);
                persist_terminal_turn_step(
                    manager,
                    &payload.session_id,
                    &turn_id,
                    &assistant_message_id,
                    &stream_id,
                    &mut next_sequence,
                    Some(&channel),
                    &assistant_content,
                    Some(&model),
                    "error",
                    "Stream timed out",
                    &message,
                );
                let metadata = assistant_gateway_metadata(
                    &turn_id,
                    &stream_id,
                    Some(&model),
                    response_id.as_deref(),
                    finish_reason.as_deref(),
                    None,
                    "error",
                );
                let _ = update_assistant_shell(
                    manager,
                    &payload.session_id,
                    &assistant_message_id,
                    &assistant_content,
                    None,
                    metadata,
                );
                let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                managed_stream.record_send();
                managed_stream.untrack();
                return Ok(());
            }
        };
        // 扩展 `extension.stream.subscribeSource`（34 §2.8 / 02b-stream §3.8）：
        // 逐条实时广播 Agent 流 chunk 给按 kind/session 过滤的订阅者。
        // 无订阅者时 publish 零开销（推模型按需订阅）。
        stream_index.publish(stream_kind::AGENT, Some(&payload.session_id), json!(&chunk));
        if chunk.is_error() {
            let message = chunk
                .data
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("流式请求失败")
                .to_string();
            let _ = fail_agent_task(task_manager, &task_id, &message);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &stream_id,
                &mut next_sequence,
                Some(&channel),
                &assistant_content,
                Some(&model),
                "error",
                "Stream failed",
                &message,
            );
            let _ = send_channel_value(&channel, chunk.channel_payload());
            managed_stream.record_send();
            managed_stream.untrack();
            return Ok(());
        }

        if chunk.is_cancelled() {
            let _ = update_task_record(task_manager, &task_id, |task| task.mark_cancelled());
            let reason = chunk
                .data
                .get("reason")
                .and_then(|value| value.as_str())
                .unwrap_or("用户已取消流式请求")
                .to_string();
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &stream_id,
                &mut next_sequence,
                Some(&channel),
                &assistant_content,
                Some(&model),
                "aborted",
                "Stream cancelled",
                &reason,
            );
            let _ = send_channel_value(&channel, chunk.channel_payload());
            managed_stream.record_send();
            managed_stream.untrack();
            return Ok(());
        }

        if chunk.is_done() {
            break;
        }

        if chunk.data.get("type").and_then(|value| value.as_str()) == Some("gatewayRetry") {
            if let Err(error) = persist_gateway_retry_agent_timeline_part(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &stream_id,
                &mut next_sequence,
                &channel,
                &chunk.data,
            ) {
                let _ = fail_agent_task(task_manager, &task_id, &error);
                let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                managed_stream.record_send();
                managed_stream.untrack();
                return Ok(());
            }
            continue;
        }

        if let Some(delta) = chunk.data.get("delta").and_then(|value| value.as_str()) {
            assistant_content.push_str(delta);

            let visible_delta = if should_buffer_potential_text_tool_call(&assistant_content)
                || has_open_internal_details_block(&assistant_content)
            {
                buffering_text_tool_call = true;
                buffered_text_delta.push_str(delta);
                None
            } else if buffering_text_tool_call {
                buffered_text_delta.push_str(delta);
                buffering_text_tool_call = false;
                let _ = std::mem::take(&mut buffered_text_delta);
                let visible_full = assistant_visible_content(&assistant_content, &tool_preludes);
                let next_delta = visible_full
                    .strip_prefix(&visible_stream_content)
                    .map(str::to_string)
                    .unwrap_or_else(|| visible_full.clone());
                visible_stream_content = visible_full;
                if next_delta.is_empty() {
                    None
                } else {
                    Some(next_delta)
                }
            } else {
                let visible_full = assistant_visible_content(&assistant_content, &tool_preludes);
                let next_delta = visible_full
                    .strip_prefix(&visible_stream_content)
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        assistant_visible_delta(&assistant_content, delta, &tool_preludes)
                            .unwrap_or_default()
                    });
                visible_stream_content = visible_full;
                if next_delta.is_empty() {
                    None
                } else {
                    Some(next_delta)
                }
            };

            if let Some(visible_delta) = visible_delta {
                if let Err(error) = publish_agent_timeline_part_delta(
                    &stream_id,
                    &mut next_sequence,
                    &channel,
                    &assistant_message_id,
                    &turn_id,
                    &text_part_id,
                    "text",
                    &visible_delta,
                ) {
                    let _ = fail_agent_task(task_manager, &task_id, &error);
                    let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
                    let _ = send_channel_value(&channel, chunk.channel_payload());
                    managed_stream.record_send();
                    managed_stream.untrack();
                    return Ok(());
                }
            }
        }
        if response_id.is_none() {
            response_id = chunk
                .data
                .get("id")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        if finish_reason.is_none() {
            finish_reason = chunk
                .data
                .get("finishReason")
                .or_else(|| chunk.data.get("finish_reason"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }

        // Text-form tool calls are held in the backend until the full block can
        // be classified. Tool requests must become tool AgentTimelineParts, never text deltas.
    }

    if receiver.is_cancelled() {
        let _ = update_task_record(task_manager, &task_id, |task| task.mark_cancelled());
        persist_terminal_turn_step(
            manager,
            &payload.session_id,
            &turn_id,
            &assistant_message_id,
            &stream_id,
            &mut next_sequence,
            Some(&channel),
            &assistant_content,
            Some(&model),
            "aborted",
            "Stream cancelled",
            "用户已取消流式请求",
        );
        let chunk = CoreStreamChunk::cancelled(&stream_id, next_sequence, "用户已取消流式请求");
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }

    let mut assistant_content = if assistant_content.trim().is_empty() {
        String::new()
    } else {
        assistant_content
    };

    let held_tool_call_candidate = buffering_text_tool_call && !buffered_text_delta.is_empty();
    let mut parsed_text_tool_calls = parse_text_tool_calls(&assistant_content);
    if held_tool_call_candidate && parsed_text_tool_calls.is_empty() {
        let message = "Model emitted an incomplete text-form tool call".to_string();
        let _ = fail_agent_task(task_manager, &task_id, &message);
        persist_terminal_turn_step(
            manager,
            &payload.session_id,
            &turn_id,
            &assistant_message_id,
            &stream_id,
            &mut next_sequence,
            Some(&channel),
            "",
            Some(&model),
            "error",
            "Malformed tool call",
            &message,
        );
        let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }

    let recovery_mode_config = mode_config_from_key(session_mode(&session).as_deref());
    let mut recovery_messages = final_gateway_messages.clone();
    let mut text_tool_recovery_count = 0usize;
    while let Some(original_text_tool_call) = parsed_text_tool_calls.first().cloned() {
        parsed_text_tool_calls.remove(0);
        let mut tool_call = original_text_tool_call.clone();
        text_tool_recovery_count += 1;
        if text_tool_recovery_count > 8 {
            let message =
                "Model kept emitting text-form tool calls after the recovery limit".to_string();
            let _ = fail_agent_task(task_manager, &task_id, &message);
            persist_terminal_turn_step(
                manager,
                &payload.session_id,
                &turn_id,
                &assistant_message_id,
                &stream_id,
                &mut next_sequence,
                Some(&channel),
                "",
                Some(&model),
                "error",
                "Tool recovery limit reached",
                &message,
            );
            let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
            let _ = send_channel_value(&channel, chunk.channel_payload());
            managed_stream.record_send();
            managed_stream.untrack();
            return Ok(());
        }

        if !is_supported_gateway_tool(mcp, &recovery_mode_config, &tool_call.function.name) {
            recovery_messages.push(assistant_tool_message(vec![tool_call.clone()]));
            recovery_messages.push(unsupported_gateway_tool_result(&tool_call));
        } else {
            match effective_gateway_tool_call(mcp, &recovery_mode_config, &tool_call) {
                Ok(effective_tool_call) => {
                    tool_call = effective_tool_call;
                    recovery_messages.push(assistant_tool_message(vec![tool_call.clone()]));
                }
                Err(error) => {
                    recovery_messages.push(assistant_tool_message(vec![original_text_tool_call]));
                    recovery_messages.push(GatewayProtocolMessage::tool_result(
                        &tool_call.id,
                        serde_json::to_string_pretty(&json!({
                            "callId": tool_call.id,
                            "tool": tool_call.function.name,
                            "isError": true,
                            "error": error.to_string(),
                        }))
                        .unwrap_or_else(|_| error.to_string()),
                    ));
                    continue;
                }
            }
            let mut started_event = None;
            if let Ok(event) = tool_started_event(mcp, &recovery_mode_config, &tool_call) {
                started_event = Some(event.clone());
                tool_events.push(event.clone());
                send_agent_tool_event(
                    Some(manager),
                    &payload.session_id,
                    Some(&turn_id),
                    &assistant_message_id,
                    &mut tool_timeline,
                    &tool_events,
                    Some(&channel),
                    &stream_id,
                    &mut next_sequence,
                    event,
                )?;
            }

            let event = started_event.clone().unwrap_or_else(|| {
                AgentToolEvent::new(
                    AgentToolPhase::Completed,
                    AgentToolStatus::Completed,
                    tool_call.id.clone(),
                    tool_call.function.name.clone(),
                    tool_call.function.name.clone(),
                    tool_call.function.name.clone(),
                    tool_call_summary(&tool_call),
                    Some(auto_tool_approval_reason(&session).to_string()),
                    Some(tool_call_arguments(&tool_call)),
                    None,
                    None,
                    None,
                    Some(Utc::now().to_rfc3339()),
                    Some(Utc::now().to_rfc3339()),
                    Some(0),
                )
            });
            let approval_request = tool_approval_request(&session, &tool_call, &event);
            let policy = session_permission_policy(&session)
                .as_deref()
                .and_then(ApprovalMode::from_str)
                .unwrap_or_default();
            let decision = resolve_tool_approval_for_event(
                Some(manager),
                &session,
                policy,
                permission_rules,
                Some(&turn_id),
                &assistant_message_id,
                Some(approvals),
                Some(&channel),
                &stream_id,
                &mut next_sequence,
                &mut tool_timeline,
                &mut tool_events,
                &event,
                &mut tool_call,
                approval_request,
            )
            .await?;
            if !decision.is_allowed() {
                let message = "Tool call was not approved".to_string();
                let rejected_event = AgentToolEvent {
                    summary: Some(message.clone()),
                    detail: Some(message.clone()),
                    output: Some(json!({ "error": message })),
                    metadata: Some(json!({ "isError": true })),
                    progress: None,
                    completed_at: Some(Utc::now().to_rfc3339()),
                    duration_ms: None,
                    ..event.with_lifecycle(AgentToolPhase::Completed, AgentToolStatus::Error)
                };
                tool_events.push(rejected_event.clone());
                send_agent_tool_event(
                    Some(manager),
                    &payload.session_id,
                    Some(&turn_id),
                    &assistant_message_id,
                    &mut tool_timeline,
                    &tool_events,
                    Some(&channel),
                    &stream_id,
                    &mut next_sequence,
                    rejected_event,
                )?;
                persist_terminal_turn_step(
                    manager,
                    &payload.session_id,
                    &turn_id,
                    &assistant_message_id,
                    &stream_id,
                    &mut next_sequence,
                    Some(&channel),
                    "",
                    Some(&model),
                    "error",
                    "Tool rejected",
                    &message,
                );
                let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                managed_stream.record_send();
                managed_stream.untrack();
                return Ok(());
            }
            let pending_session_change = prepare_session_change_capture(
                &session,
                Some(&turn_id),
                &assistant_message_id,
                &tool_call,
            )?;
            let extension_hooks = backend.extension_store.list_hooks();
            let execution_result = execute_ui_agent_tool_call_with_pipeline(
                Some(manager),
                Some(&backend),
                Some(task_manager),
                mcp,
                &session,
                &recovery_mode_config,
                &tool_call,
                &recovery_messages,
                Some(&mut |event: AgentToolEvent| {
                    tool_events.push(event.clone());
                    send_agent_tool_event(
                        Some(manager),
                        &session.id,
                        Some(&turn_id),
                        &assistant_message_id,
                        &mut tool_timeline,
                        &tool_events,
                        Some(&channel),
                        &stream_id,
                        &mut next_sequence,
                        event,
                    )
                    .map(|_| ())
                }),
                &extension_hooks,
                Some(skills.clone()),
            )
            .await;

            match execution_result {
                Ok(execution) => {
                    tool_events.push(execution.event.clone());
                    let part_id = send_agent_tool_event(
                        Some(manager),
                        &payload.session_id,
                        Some(&turn_id),
                        &assistant_message_id,
                        &mut tool_timeline,
                        &tool_events,
                        Some(&channel),
                        &stream_id,
                        &mut next_sequence,
                        execution.event.clone(),
                    )?;
                    record_completed_session_change(
                        Some(manager),
                        &payload.session_id,
                        pending_session_change,
                        part_id,
                        &execution,
                    )?;
                    recovery_messages.push(execution.result_message);
                }
                Err(error) => {
                    let message = error.to_string();
                    let _ = fail_agent_task(task_manager, &task_id, &message);
                    persist_terminal_turn_step(
                        manager,
                        &payload.session_id,
                        &turn_id,
                        &assistant_message_id,
                        &stream_id,
                        &mut next_sequence,
                        Some(&channel),
                        "",
                        Some(&model),
                        "error",
                        "Tool execution failed",
                        &message,
                    );
                    let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
                    let _ = send_channel_value(&channel, chunk.channel_payload());
                    managed_stream.record_send();
                    managed_stream.untrack();
                    return Ok(());
                }
            }
        }

        if let Err(error) = persist_text_agent_timeline_part(
            manager,
            &payload.session_id,
            &turn_id,
            &assistant_message_id,
            &stream_id,
            &mut next_sequence,
            &channel,
            "",
            TimelineStatus::Running,
        ) {
            let _ = fail_agent_task(task_manager, &task_id, &error);
            let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
            let _ = send_channel_value(&channel, chunk.channel_payload());
            managed_stream.record_send();
            managed_stream.untrack();
            return Ok(());
        }

        let response = gateway
            .router(session_chat_request(
                &session,
                model.clone(),
                recovery_messages.clone(),
            ))
            .await
            .map_err(|error| error.to_string());
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let _ = fail_agent_task(task_manager, &task_id, &error);
                persist_terminal_turn_step(
                    manager,
                    &payload.session_id,
                    &turn_id,
                    &assistant_message_id,
                    &stream_id,
                    &mut next_sequence,
                    Some(&channel),
                    "",
                    Some(&model),
                    "error",
                    "Tool recovery request failed",
                    &error,
                );
                let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
                let _ = send_channel_value(&channel, chunk.channel_payload());
                managed_stream.record_send();
                managed_stream.untrack();
                return Ok(());
            }
        };
        assistant_content = response_text(&response);
        response_id = Some(response.id.clone());
        finish_reason = Some(response.finish_reason.clone());
        token_count = Some(response.usage.total_tokens as i64);
        parsed_text_tool_calls = parse_text_tool_calls(&assistant_content);
    }
    let visible_assistant_content = assistant_visible_content(&assistant_content, &tool_preludes);
    if let Err(error) = persist_text_agent_timeline_part(
        manager,
        &payload.session_id,
        &turn_id,
        &assistant_message_id,
        &stream_id,
        &mut next_sequence,
        &channel,
        &visible_assistant_content,
        TimelineStatus::Completed,
    ) {
        let _ = fail_agent_task(task_manager, &task_id, &error);
        let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }
    let metadata = assistant_gateway_metadata(
        &turn_id,
        &stream_id,
        Some(&model),
        response_id.as_deref(),
        finish_reason.as_deref(),
        None,
        "completed",
    );
    if let Err(error) = update_assistant_shell(
        manager,
        &payload.session_id,
        &assistant_message_id,
        &visible_assistant_content,
        token_count,
        metadata,
    ) {
        let message = error.to_string();
        let _ = fail_agent_task(task_manager, &task_id, &message);
        let chunk = CoreStreamChunk::error(&stream_id, next_sequence, message);
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }
    if let Err(error) = persist_turn_finalizer_step(
        manager,
        &payload.session_id,
        &turn_id,
        &assistant_message_id,
        &stream_id,
        &mut next_sequence,
        &channel,
        &tool_events,
        token_count,
        Some(turn_started_at.elapsed().as_millis() as u64),
    ) {
        let _ = fail_agent_task(task_manager, &task_id, &error);
        let chunk = CoreStreamChunk::error(&stream_id, next_sequence, error);
        let _ = send_channel_value(&channel, chunk.channel_payload());
        managed_stream.record_send();
        managed_stream.untrack();
        return Ok(());
    }
    update_task_record(task_manager, &task_id, |task| {
        task.add_message(AgentChatMessage::assistant(visible_assistant_content));
        task.mark_completed();
    })?;

    let messages = session_messages(manager, Some(approvals), &payload.session_id)?;
    let final_data = CoreStreamChunk::data(
        &stream_id,
        next_sequence,
        json!({ "type": "messages", "messages": messages.messages, "total": messages.total }),
    );
    if let Err(error) = send_channel_value(&channel, final_data.channel_payload()) {
        managed_stream.untrack();
        return Err(format!("会话流推送失败: {}", error));
    }
    managed_stream.record_send();
    let done = CoreStreamChunk::final_chunk(&stream_id, next_sequence.saturating_add(1));
    if let Err(error) = send_channel_value(&channel, done.channel_payload()) {
        managed_stream.untrack();
        return Err(format!("会话流完成信号推送失败: {}", error));
    }
    managed_stream.record_send();
    managed_stream.untrack();

    Ok(())
}
