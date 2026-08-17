// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use crate::foundation::status::{StatusClassify, StatusPresentation};
// use [REMOVED: domains reference]
use crate::foundation::stream::{send_channel_value, StreamChunk as CoreStreamChunk};
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use tauri::ipc::Channel;

pub const ASSISTANT_PRELUDE_STEP_SEQUENCE: i64 = -100;
pub const ASSISTANT_PERMISSION_STEP_SEQUENCE_BASE: i64 = 5_000;
pub const ASSISTANT_RETRY_STEP_SEQUENCE_BASE: i64 = 9_000;
pub const ASSISTANT_TEXT_STEP_SEQUENCE: i64 = 10_000;
pub const ASSISTANT_ERROR_STEP_SEQUENCE: i64 = 10_001;
pub const ASSISTANT_FINALIZER_STEP_SEQUENCE: i64 = 10_002;

pub fn agent_text_part_id(turn_id: &str) -> String {
    format!("text:{}", turn_id)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAgentTimelinePart {
    pub part_id: String,
    pub message_id: String,
    pub turn_id: String,
    pub sequence: i64,
    pub kind: String,
    pub call_id: Option<String>,
    pub tool: Option<String>,
    pub gateway_tool: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub status_presentation: StatusPresentation,
    pub summary: Option<String>,
    pub detail: Option<String>,
    pub text: Option<String>,
    pub source: Option<String>,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub metadata: Option<Value>,
    pub progress: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
}

fn is_active_status(status: &str) -> bool {
    TimelineStatus::parse(status).is_live()
}

fn millis_between(started_at: &str, completed_at: &str) -> Option<u64> {
    let started_at = DateTime::parse_from_rfc3339(started_at).ok()?;
    let completed_at = DateTime::parse_from_rfc3339(completed_at).ok()?;
    let elapsed_ms = completed_at
        .timestamp_millis()
        .saturating_sub(started_at.timestamp_millis());
    Some(elapsed_ms.max(0) as u64)
}

fn duration_from_event(
    event: &AgentToolEvent,
    started_at: Option<&str>,
    completed_at: Option<&str>,
) -> Option<u64> {
    event
        .duration_ms
        .or_else(|| millis_between(started_at?, completed_at?))
}

pub fn agent_timeline_parts_from_tool_events(
    events: &[AgentToolEvent],
    turn_id: String,
    message_id: String,
    tool_sequences: &HashMap<String, i64>,
) -> Vec<UiAgentTimelinePart> {
    let mut steps = Vec::new();
    let mut by_call_id = HashMap::new();

    for event in events {
        let now = Utc::now().to_rfc3339();
        if let Some(index) = by_call_id.get(&event.call_id).copied() {
            let step: &mut UiAgentTimelinePart = &mut steps[index];
            let completed_at = event.completed_at.clone().or_else(|| {
                if is_active_status(&event.status) {
                    None
                } else {
                    Some(now.clone())
                }
            });
            step.tool = Some(event.tool.clone());
            step.gateway_tool = Some(event.gateway_tool.clone());
            step.title = Some(event.title.clone());
            step.status = Some(event.status.clone());
            step.status_presentation = TimelineStatus::parse(&event.status).status_presentation();
            step.summary = event.summary.clone();
            step.detail = event.detail.clone();
            if event.input.is_some() {
                step.input = event.input.clone();
            }
            step.output = event.output.clone().or_else(|| step.output.clone());
            step.metadata = event.metadata.clone().or_else(|| step.metadata.clone());
            step.progress = event.progress.clone().or_else(|| step.progress.clone());
            if step.started_at.is_none() {
                step.started_at = event.started_at.clone();
            }
            if completed_at.is_some() {
                step.completed_at = completed_at;
            }
            step.duration_ms = duration_from_event(
                event,
                step.started_at.as_deref(),
                step.completed_at.as_deref(),
            )
            .or(step.duration_ms);
            step.updated_at = now;
            continue;
        }

        by_call_id.insert(event.call_id.clone(), steps.len());
        let part_id = format!("tool:{}:{}", event.call_id, steps.len());
        let started_at = event.started_at.clone().or_else(|| {
            if is_active_status(&event.status) {
                Some(now.clone())
            } else {
                None
            }
        });
        let completed_at = event.completed_at.clone().or_else(|| {
            if is_active_status(&event.status) {
                None
            } else {
                Some(now.clone())
            }
        });
        let duration_ms =
            duration_from_event(event, started_at.as_deref(), completed_at.as_deref());
        steps.push(UiAgentTimelinePart {
            part_id,
            message_id: message_id.clone(),
            turn_id: turn_id.clone(),
            sequence: *tool_sequences
                .get(&event.call_id)
                .expect("tool sequence must be assigned before rendering AgentTimelinePart"),
            kind: "tool".to_string(),
            call_id: Some(event.call_id.clone()),
            tool: Some(event.tool.clone()),
            gateway_tool: Some(event.gateway_tool.clone()),
            title: Some(event.title.clone()),
            status: Some(event.status.clone()),
            status_presentation: TimelineStatus::parse(&event.status).status_presentation(),
            summary: event.summary.clone(),
            detail: event.detail.clone(),
            text: None,
            source: Some("tool_agent_runtime".to_string()),
            input: event.input.clone(),
            output: event.output.clone(),
            metadata: event.metadata.clone(),
            progress: event.progress.clone(),
            created_at: started_at.clone().unwrap_or_else(|| now.clone()),
            updated_at: now,
            started_at,
            completed_at,
            duration_ms,
        });
    }

    steps
}

pub fn ui_agent_timeline_part(step: AgentTimelinePart) -> UiAgentTimelinePart {
    let part_id = step.id.clone();
    let status_presentation = step
        .status
        .as_ref()
        .map(StatusClassify::status_presentation)
        .unwrap_or_else(StatusPresentation::unknown);
    UiAgentTimelinePart {
        part_id,
        message_id: step.message_id,
        turn_id: step.turn_id,
        sequence: step.sequence,
        kind: step.kind,
        call_id: step.call_id,
        tool: step
            .data
            .get("tool")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        gateway_tool: step
            .data
            .get("gatewayTool")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        title: step
            .data
            .get("title")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        status: step.status.map(|status| status.as_str().to_string()),
        status_presentation,
        summary: step
            .data
            .get("summary")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        detail: step
            .data
            .get("detail")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        text: step
            .data
            .get("text")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        source: step
            .data
            .get("source")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        input: step.data.get("input").cloned(),
        output: step.data.get("output").cloned(),
        metadata: step.data.get("metadata").cloned(),
        progress: step.data.get("progress").cloned(),
        created_at: step.created_at,
        updated_at: step.updated_at,
        started_at: step
            .data
            .get("startedAt")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        completed_at: step
            .data
            .get("completedAt")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        duration_ms: step.data.get("durationMs").and_then(|value| value.as_u64()),
    }
}

pub fn agent_timeline_part_from_ui(
    session_id: &str,
    step: &UiAgentTimelinePart,
) -> AgentTimelinePart {
    let updated_at = if step.updated_at.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        step.updated_at.clone()
    };
    AgentTimelinePart {
        id: step.part_id.clone(),
        session_id: session_id.to_string(),
        turn_id: step.turn_id.clone(),
        message_id: step.message_id.clone(),
        sequence: step.sequence,
        kind: step.kind.clone(),
        status: step.status.as_deref().map(TimelineStatus::parse),
        call_id: step.call_id.clone(),
        data: json!({
            "tool": step.tool.clone(),
            "gatewayTool": step.gateway_tool.clone(),
            "title": step.title.clone(),
            "summary": step.summary.clone(),
            "detail": step.detail.clone(),
            "text": step.text.clone(),
            "source": step.source.clone(),
            "input": step.input.clone(),
            "output": step.output.clone(),
            "metadata": step.metadata.clone(),
            "progress": step.progress.clone(),
            "startedAt": step.started_at.clone(),
            "completedAt": step.completed_at.clone(),
            "durationMs": step.duration_ms,
        }),
        created_at: step.created_at.clone(),
        updated_at,
        metadata: None,
    }
}

pub fn record_agent_timeline_part(
    manager: &SessionManager,
    stream_id: &str,
    stream_sequence: &mut u64,
    channel: Option<&Channel>,
    step: AgentTimelinePart,
) -> Result<UiAgentTimelinePart, String> {
    let stored_step = manager
        .upsert_agent_timeline_part(&step)
        .map_err(|error| format!("AgentTimelinePart 保存失败: {}", error))?;
    let agent_timeline_part = ui_agent_timeline_part(stored_step);
    if let Some(channel) = channel {
        publish_agent_timeline_part(
            stream_id,
            stream_sequence,
            channel,
            agent_timeline_part.clone(),
        )?;
    }
    Ok(agent_timeline_part)
}

pub fn publish_agent_timeline_part(
    stream_id: &str,
    stream_sequence: &mut u64,
    channel: &Channel,
    part: UiAgentTimelinePart,
) -> Result<(), String> {
    let chunk = CoreStreamChunk::data(
        stream_id,
        *stream_sequence,
        json!({
            "type": "agentTimelinePart",
            "part": part,
        }),
    );
    *stream_sequence = (*stream_sequence).saturating_add(1);
    send_channel_value(channel, chunk.channel_payload())
        .map_err(|error| format!("AgentTimelinePart 推送失败: {}", error))
}

pub fn publish_agent_timeline_part_delta(
    stream_id: &str,
    stream_sequence: &mut u64,
    channel: &Channel,
    message_id: &str,
    turn_id: &str,
    part_id: &str,
    field: &str,
    delta: &str,
) -> Result<(), String> {
    let chunk = CoreStreamChunk::data(
        stream_id,
        *stream_sequence,
        json!({
            "type": "agentTimelinePartDelta",
            "messageId": message_id,
            "turnId": turn_id,
            "partId": part_id,
            "field": field,
            "delta": delta,
        }),
    );
    *stream_sequence = (*stream_sequence).saturating_add(1);
    send_channel_value(channel, chunk.channel_payload())
        .map_err(|error| format!("AgentTimelinePart delta 推送失败: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;
//     use [REMOVED: domains reference]
    use serde_json::json;

    fn tool_event(call_id: &str, status: AgentToolStatus) -> AgentToolEvent {
        let phase = match status {
            AgentToolStatus::Running | AgentToolStatus::WaitingPermission => {
                AgentToolPhase::Started
            }
            AgentToolStatus::Completed
            | AgentToolStatus::Error
            | AgentToolStatus::Denied
            | AgentToolStatus::Aborted => AgentToolPhase::Completed,
        };
        AgentToolEvent::new(
            phase,
            status,
            call_id,
            "bash",
            "terminal.run_command",
            "bash",
            Some("cargo check".to_string()),
            None,
            Some(json!({ "command": "cargo check" })),
            None,
            None,
            None,
            Some("2026-06-05T00:00:00Z".to_string()),
            None,
            None,
        )
    }

    #[test]
    fn tool_steps_use_assigned_event_order_sequences() {
        let steps = agent_timeline_parts_from_tool_events(
            &[
                tool_event("call-1", AgentToolStatus::Running),
                tool_event("call-2", AgentToolStatus::Running),
            ],
            "turn-1".to_string(),
            "assistant-1".to_string(),
            &HashMap::from([("call-1".to_string(), 1), ("call-2".to_string(), 2)]),
        );

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].sequence, 1);
        assert_eq!(steps[1].sequence, 2);
    }

    #[test]
    fn tool_step_updates_keep_original_sequence() {
        let steps = agent_timeline_parts_from_tool_events(
            &[
                tool_event("call-1", AgentToolStatus::Running),
                tool_event("call-1", AgentToolStatus::Completed),
                tool_event("call-2", AgentToolStatus::Running),
            ],
            "turn-1".to_string(),
            "assistant-1".to_string(),
            &HashMap::from([("call-1".to_string(), 1), ("call-2".to_string(), 2)]),
        );

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].call_id.as_deref(), Some("call-1"));
        assert_eq!(steps[0].sequence, 1);
        assert_eq!(steps[0].status.as_deref(), Some("completed"));
        assert_eq!(steps[1].sequence, 2);
    }
}
