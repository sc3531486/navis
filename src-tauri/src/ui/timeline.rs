// ── 归属扩展：navis-session ──
// 迁移目标：extensions/navis-session/ExtensionBackend/src/

use super::agent_timeline_part::{
    agent_text_part_id, agent_timeline_part_from_ui, agent_timeline_parts_from_tool_events,
    publish_agent_timeline_part, record_agent_timeline_part, ui_agent_timeline_part,
    ASSISTANT_ERROR_STEP_SEQUENCE, ASSISTANT_FINALIZER_STEP_SEQUENCE,
    ASSISTANT_PRELUDE_STEP_SEQUENCE, ASSISTANT_RETRY_STEP_SEQUENCE_BASE,
    ASSISTANT_TEXT_STEP_SEQUENCE,
};
use super::dto::UiToolApprovalRequest;
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use crate::ui::runtime::tool_approval::ToolApprovalDecision;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tauri::ipc::Channel;

// ─── ToolTimelineSequencer ───

#[derive(Debug, Default)]
pub(crate) struct ToolTimelineSequencer {
    pub(crate) next_sequence: i64,
    next_prelude_index: usize,
    pub(crate) tool_sequences: HashMap<String, i64>,
}

impl ToolTimelineSequencer {
    pub(crate) fn next_tool_prelude(&mut self) -> (usize, i64) {
        let index = self.next_prelude_index;
        self.next_prelude_index = self.next_prelude_index.saturating_add(1);
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        (index, sequence)
    }

    pub(crate) fn sequence_for_tool_call(&mut self, call_id: &str) -> i64 {
        if let Some(sequence) = self.tool_sequences.get(call_id).copied() {
            return sequence;
        }
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.tool_sequences.insert(call_id.to_string(), sequence);
        sequence
    }
}

// ─── Utility functions ───

pub(crate) fn duration_label(elapsed_ms: Option<u64>) -> Option<String> {
    let elapsed_ms = elapsed_ms?;
    let total_seconds = elapsed_ms / 1_000;
    if total_seconds < 60 {
        return Some(format!("{}s", total_seconds.max(1)));
    }
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    Some(format!("{}m {}s", minutes, seconds))
}

pub(crate) fn compact_timeline_summary(title: &str, detail: &str) -> String {
    let first_line = detail
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(title)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = first_line.chars();
    let preview: String = chars.by_ref().take(96).collect();
    if chars.next().is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}

// ─── Timeline part builders ───

pub(crate) fn turn_prelude_text() -> (String, String) {
    (
        "Thinking".to_string(),
        "Waiting for assistant text, thinking, or the next tool event.".to_string(),
    )
}

pub(crate) fn prelude_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    _content: &str,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    let (summary, detail) = turn_prelude_text();
    AgentTimelinePart {
        id: format!("prelude:{}", turn_id),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: ASSISTANT_PRELUDE_STEP_SEQUENCE,
        kind: "reasoning".to_string(),
        status: Some(TimelineStatus::Running),
        call_id: None,
        data: json!({
            "title": "Thinking",
            "summary": summary,
            "detail": detail,
            "source": "turn_prelude",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn text_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    text: &str,
    status: TimelineStatus,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    AgentTimelinePart {
        id: agent_text_part_id(turn_id),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: ASSISTANT_TEXT_STEP_SEQUENCE,
        kind: "text".to_string(),
        status: Some(status.clone()),
        call_id: None,
        data: json!({
            "title": "Assistant response",
            "summary": (if text.trim().is_empty() { "Waiting for response" } else { "Streaming response" }),
            "detail": text,
            "text": text,
            "source": "gateway",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn tool_prelude_text_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    prelude_index: usize,
    step_sequence: i64,
    text: &str,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    AgentTimelinePart {
        id: format!("text:{}:tool-prelude:{}", turn_id, prelude_index),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: step_sequence,
        kind: "text".to_string(),
        status: Some(TimelineStatus::Completed),
        call_id: None,
        data: json!({
            "title": "Assistant update",
            "summary": compact_timeline_summary("Assistant update", text),
            "detail": text,
            "text": text,
            "source": "gateway_tool_prelude",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn error_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    status: TimelineStatus,
    title: &str,
    detail: &str,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    let summary = compact_timeline_summary(title, detail);
    AgentTimelinePart {
        id: format!("error:{}", turn_id),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: ASSISTANT_ERROR_STEP_SEQUENCE,
        kind: "error".to_string(),
        status: Some(status.clone()),
        call_id: None,
        data: json!({
            "title": title,
            "summary": summary,
            "detail": detail,
            "text": detail,
            "reason": status.as_str(),
            "source": "gateway",
            "startedAt": now,
            "completedAt": now,
            "durationMs": 0,
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn gateway_retry_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    attempt: u32,
    max_retries: u32,
    delay_ms: u64,
    detail: &str,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    let title = format!("Retrying {}/{}", attempt, max_retries);
    AgentTimelinePart {
        id: format!("retry:{}:{}", turn_id, attempt),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: ASSISTANT_RETRY_STEP_SEQUENCE_BASE + i64::from(attempt),
        kind: "error".to_string(),
        status: Some(TimelineStatus::Retrying),
        call_id: None,
        data: json!({
            "title": title,
            "summary": title,
            "detail": detail,
            "text": detail,
            "attempt": attempt,
            "maxRetries": max_retries,
            "delayMs": delay_ms,
            "source": "gateway_retry",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn permission_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    request: &UiToolApprovalRequest,
    status: TimelineStatus,
    decision: Option<ToolApprovalDecision>,
    reason: Option<&str>,
    sequence: i64,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    let decision_text = decision.map(ToolApprovalDecision::as_str);
    let summary = decision_text
        .map(|value| format!("{} · {}", request.title, value))
        .unwrap_or_else(|| request.title.clone());
    AgentTimelinePart {
        id: format!("permission:{}", request.request_id),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence,
        kind: "permission".to_string(),
        status: Some(status),
        call_id: Some(request.call_id.clone()),
        data: json!({
            "requestId": request.request_id,
            "permission": request.permission,
            "tool": request.tool,
            "gatewayTool": request.gateway_tool,
            "worktreeRoot": request.worktree_root,
            "pattern": request.pattern,
            "title": request.title,
            "summary": summary,
            "detail": reason.unwrap_or(request.message.as_str()),
            "message": request.message,
            "riskLevel": request.risk_level,
            "args": request.args,
            "decision": decision_text,
            "reason": reason,
            "source": "permission_runtime",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

pub(crate) fn finalizer_summary(
    tool_events: &[AgentToolEvent],
    token_count: Option<i64>,
    elapsed_ms: Option<u64>,
) -> String {
    let tool_count = tool_events
        .iter()
        .map(|event| event.call_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let mut parts = vec!["Finished response".to_string()];
    if tool_count > 0 {
        parts.push(format!(
            "{} tool {}",
            tool_count,
            if tool_count == 1 { "call" } else { "calls" }
        ));
    }
    if let Some(tokens) = token_count {
        parts.push(format!("{} tokens", tokens));
    }
    if let Some(duration) = duration_label(elapsed_ms) {
        parts.push(duration);
    }
    parts.join(" · ")
}

pub(crate) fn finalizer_agent_timeline_part(
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    tool_events: &[AgentToolEvent],
    token_count: Option<i64>,
    elapsed_ms: Option<u64>,
) -> AgentTimelinePart {
    let now = Utc::now().to_rfc3339();
    let summary = finalizer_summary(tool_events, token_count, elapsed_ms);
    AgentTimelinePart {
        id: format!("finalizer:{}", turn_id),
        session_id: session_id.to_string(),
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        sequence: ASSISTANT_FINALIZER_STEP_SEQUENCE,
        kind: "summary".to_string(),
        status: Some(TimelineStatus::Completed),
        call_id: None,
        data: json!({
            "title": "Turn complete",
            "summary": summary,
            "detail": "The assistant response, tool results, and turn metadata have been persisted.",
            "durationMs": elapsed_ms,
            "source": "turn_finalizer",
        }),
        created_at: now.clone(),
        updated_at: now,
        metadata: Some(json!({ "schemaVersion": 1 })),
    }
}

// ─── Timeline persistence helpers ───

pub(crate) fn persist_turn_prelude_step(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
    content: &str,
) -> Result<(), String> {
    let step = prelude_agent_timeline_part(session_id, turn_id, assistant_message_id, content);
    record_agent_timeline_part(manager, stream_id, sequence, Some(channel), step).map(|_| ())
}

pub(crate) fn complete_turn_prelude_step(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
) -> Result<(), String> {
    let mut step = prelude_agent_timeline_part(session_id, turn_id, assistant_message_id, "");
    step.status = Some(TimelineStatus::Completed);
    step.updated_at = Utc::now().to_rfc3339();
    step.data = json!({
        "title": "Thinking",
        "summary": "Thinking complete",
        "detail": "The assistant has emitted text, tool events, or a final turn summary.",
        "source": "turn_prelude",
    });
    record_agent_timeline_part(manager, stream_id, sequence, Some(channel), step).map(|_| ())
}

fn is_active_agent_timeline_status(part: &AgentTimelinePart) -> bool {
    part.status.as_ref().is_some_and(TimelineStatus::is_live)
}

fn close_active_agent_timeline_parts_and_publish(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
    status: TimelineStatus,
) -> Result<(), String> {
    let active_part_ids = manager
        .get_agent_timeline_parts(session_id)
        .map_err(|error| format!("AgentTimelinePart 查询失败: {}", error))?
        .into_iter()
        .filter(|part| part.turn_id == turn_id && is_active_agent_timeline_status(part))
        .map(|part| part.id)
        .collect::<HashSet<_>>();

    if active_part_ids.is_empty() {
        return Ok(());
    }

    manager
        .close_active_agent_timeline_parts(session_id, turn_id, assistant_message_id, status)
        .map_err(|error| format!("AgentTimelinePart 状态收口失败: {}", error))?;

    for part in manager
        .get_agent_timeline_parts(session_id)
        .map_err(|error| format!("AgentTimelinePart 查询失败: {}", error))?
        .into_iter()
        .filter(|part| active_part_ids.contains(&part.id))
    {
        publish_agent_timeline_part(stream_id, sequence, channel, ui_agent_timeline_part(part))?;
    }

    Ok(())
}

pub(crate) fn persist_text_agent_timeline_part(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
    text: &str,
    status: TimelineStatus,
) -> Result<(), String> {
    let step = text_agent_timeline_part(session_id, turn_id, assistant_message_id, text, status);
    record_agent_timeline_part(manager, stream_id, sequence, Some(channel), step).map(|_| ())
}

pub(crate) fn persist_tool_prelude_text_step(
//     manager: Option<&[REMOVED: domains reference]
    session_id: &str,
    turn_id: Option<&str>,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: Option<&Channel>,
    tool_timeline: &mut ToolTimelineSequencer,
    text: &str,
) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    let (Some(manager), Some(turn_id)) = (manager, turn_id) else {
        return Ok(());
    };
    let (prelude_index, step_sequence) = tool_timeline.next_tool_prelude();
    let step = tool_prelude_text_agent_timeline_part(
        session_id,
        turn_id,
        assistant_message_id,
        prelude_index,
        step_sequence,
        text,
    );
    record_agent_timeline_part(manager, stream_id, sequence, channel, step).map(|_| ())
}

pub(crate) fn persist_gateway_retry_agent_timeline_part(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
    retry: &Value,
) -> Result<(), String> {
    let attempt = retry
        .get("attempt")
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        .max(1) as u32;
    let max_retries = retry
        .get("maxRetries")
        .and_then(|value| value.as_u64())
        .unwrap_or(u64::from(attempt))
        .max(1) as u32;
    let delay_ms = retry
        .get("delayMs")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);
    let detail = retry
        .get("message")
        .and_then(|value| value.as_str())
        .unwrap_or("Gateway request failed; retrying");
    let step = gateway_retry_agent_timeline_part(
        session_id,
        turn_id,
        assistant_message_id,
        attempt,
        max_retries,
        delay_ms,
        detail,
    );
    record_agent_timeline_part(manager, stream_id, sequence, Some(channel), step).map(|_| ())
}

pub(crate) fn persist_permission_agent_timeline_part(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    stream_sequence: &mut u64,
    channel: &Channel,
    request: &UiToolApprovalRequest,
    status: TimelineStatus,
    decision: Option<ToolApprovalDecision>,
    reason: Option<&str>,
    step_sequence: i64,
) -> Result<(), String> {
    let step = permission_agent_timeline_part(
        session_id,
        turn_id,
        assistant_message_id,
        request,
        status,
        decision,
        reason,
        step_sequence,
    );
    record_agent_timeline_part(manager, stream_id, stream_sequence, Some(channel), step).map(|_| ())
}

pub(crate) fn persist_turn_finalizer_step(
//     manager: &[REMOVED: domains reference]
    session_id: &str,
    turn_id: &str,
    assistant_message_id: &str,
    stream_id: &str,
    sequence: &mut u64,
    channel: &Channel,
    tool_events: &[AgentToolEvent],
    token_count: Option<i64>,
    elapsed_ms: Option<u64>,
) -> Result<(), String> {
    complete_turn_prelude_step(
        manager,
        session_id,
        turn_id,
        assistant_message_id,
        stream_id,
        sequence,
        channel,
    )?;
    close_active_agent_timeline_parts_and_publish(
        manager,
        session_id,
        turn_id,
        assistant_message_id,
        stream_id,
        sequence,
        channel,
        TimelineStatus::Completed,
    )?;
    let step = finalizer_agent_timeline_part(
        session_id,
        turn_id,
        assistant_message_id,
        tool_events,
        token_count,
        elapsed_ms,
    );
    record_agent_timeline_part(manager, stream_id, sequence, Some(channel), step).map(|_| ())
}

// ─── Tool event sending ───

pub(crate) fn send_agent_tool_event(
//     manager: Option<&[REMOVED: domains reference]
    session_id: &str,
    turn_id: Option<&str>,
    assistant_message_id: &str,
    tool_timeline: &mut ToolTimelineSequencer,
    tool_events: &[AgentToolEvent],
    channel: Option<&Channel>,
    stream_id: &str,
    sequence: &mut u64,
    event: AgentToolEvent,
) -> Result<Option<String>, String> {
    tool_timeline.sequence_for_tool_call(&event.call_id);
    let agent_timeline_part = turn_id.and_then(|turn_id| {
        agent_timeline_parts_from_tool_events(
            tool_events,
            turn_id.to_string(),
            assistant_message_id.to_string(),
            &tool_timeline.tool_sequences,
        )
        .into_iter()
        .find(|step| step.call_id.as_deref() == Some(event.call_id.as_str()))
    });

    let Some(agent_timeline_part) = agent_timeline_part else {
        return Ok(None);
    };
    let part_id = agent_timeline_part.part_id.clone();
    if let Some(manager) = manager {
        record_agent_timeline_part(
            manager,
            stream_id,
            sequence,
            channel,
            agent_timeline_part_from_ui(session_id, &agent_timeline_part),
        )?;
    }
    Ok(Some(part_id))
}
