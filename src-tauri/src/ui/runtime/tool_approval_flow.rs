// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

//! Agent 工具审批运行流。
//!
//! 这里负责把工具调用转成 UI 审批请求、应用会话/项目级审批缓存、
//! 写入 permission AgentTimelinePart，并等待前端决策。UI command 层只调用
//! 这些 helper，不直接承载审批状态机。

use crate::extension::types::{ChatMessage as GatewayProtocolMessage, ToolCall};
// use [REMOVED: domains reference]
use crate::foundation::stream::{send_channel_value, StreamChunk as CoreStreamChunk};
// use [REMOVED: domains reference]
use crate::security::sandbox::permission::ApprovalMode;
// use [REMOVED: domains reference]
use crate::ui::agent_timeline_part::ASSISTANT_PERMISSION_STEP_SEQUENCE_BASE;
use crate::ui::dto::{UiToolApprovalRequest, UiToolPermissionRule};
use crate::ui::permissions::{
    approval_prompt_action, hardline_block_reason, is_risky_tool_permission,
    tool_permission_from_names, ApprovalPromptAction,
};
use crate::ui::runtime::tool_approval::{ToolApprovalDecision, ToolApprovalStore};
use crate::ui::session_permission_policy;
use crate::ui::timeline::{
    persist_permission_agent_timeline_part, send_agent_tool_event, ToolTimelineSequencer,
};
use serde_json::{json, Value};
use tauri::ipc::Channel;
use tokio::time::{timeout, Duration};

const NAVIS_PERMISSION_GRANTED_KEY: &str = "_navis_permission_granted";

fn tool_permission_pattern(tool_call: &ToolCall) -> String {
    tool_call_summary(tool_call).unwrap_or_else(|| tool_call_arguments(tool_call).to_string())
}

fn is_interactive_tool_approval_available(
    approvals: Option<&ToolApprovalStore>,
    channel: Option<&Channel>,
) -> bool {
    approvals.is_some() && channel.is_some()
}

pub(crate) fn unsupported_gateway_tool_result(tool_call: &ToolCall) -> GatewayProtocolMessage {
    GatewayProtocolMessage::tool_result(
        &tool_call.id,
        format!(
            "Unsupported Agent tool: {}. Use only the exact callable Gateway function tools from the system prompt.",
            tool_call.function.name
        ),
    )
}

pub(crate) fn auto_tool_approval_reason(session: &Session) -> &'static str {
    match session_permission_policy(session).as_deref() {
        Some(value) if value == ApprovalMode::AutoEdit.as_str() => {
            "Approved by Review risk only prompt policy"
        }
        Some(value) if value == ApprovalMode::FullAuto.as_str() => {
            "Approved by Full access prompt policy"
        }
        _ => "Approved by current prompt policy",
    }
}

fn tool_approval_message(tool_call: &ToolCall) -> String {
    match tool_call.function.name.as_str() {
        "edit" => "Navis Go wants to edit this file.".to_string(),
        "write" => "Navis Go wants to write this file.".to_string(),
        "bash" => "Navis Go wants to run this command.".to_string(),
        name if name.starts_with("terminal_") => "Navis Go wants to run this command.".to_string(),
        _ => "Navis Go wants to run this action.".to_string(),
    }
}

pub(crate) fn tool_approval_request(
    session: &Session,
    tool_call: &ToolCall,
    event: &AgentToolEvent,
) -> UiToolApprovalRequest {
    let permission = tool_permission_from_names(&tool_call.function.name, &event.tool);
    UiToolApprovalRequest {
        request_id: format!("approval-{}", uuid::Uuid::new_v4().simple()),
        session_id: session.id.clone(),
        worktree_root: session.worktree_root.clone(),
        call_id: tool_call.id.clone(),
        permission,
        tool: event.tool.clone(),
        gateway_tool: tool_call.function.name.clone(),
        pattern: tool_permission_pattern(tool_call),
        title: event.title.clone(),
        summary: tool_call_summary(tool_call),
        message: tool_approval_message(tool_call),
        risk_level: if matches!(tool_call.function.name.as_str(), "bash")
            || tool_call.function.name.starts_with("terminal_")
        {
            "high".to_string()
        } else {
            "medium".to_string()
        },
        args: tool_call_arguments(tool_call),
    }
}

fn attach_tool_approval_evidence(tool_call: &mut ToolCall) -> Result<(), String> {
    let mut arguments = serde_json::from_str::<Value>(&tool_call.function.arguments)
        .map_err(|error| format!("Invalid tool arguments for approval evidence: {}", error))?;
    let object = arguments
        .as_object_mut()
        .ok_or_else(|| "Tool arguments must be a JSON object".to_string())?;
    object.insert(NAVIS_PERMISSION_GRANTED_KEY.to_string(), json!(true));
    tool_call.function.arguments = serde_json::to_string(&arguments)
        .map_err(|error| format!("Failed to encode approved tool arguments: {}", error))?;
    Ok(())
}

pub(crate) fn attach_allowed_tool_approval_evidence(
    prompt_action: ApprovalPromptAction,
    decision: Option<ToolApprovalDecision>,
    tool_call: &mut ToolCall,
) -> Result<(), String> {
    let allowed_by_prompt_policy = prompt_action == ApprovalPromptAction::Allow;
    let allowed_by_decision = decision
        .map(ToolApprovalDecision::is_allowed)
        .unwrap_or(false);
    if allowed_by_prompt_policy || allowed_by_decision {
        attach_tool_approval_evidence(tool_call)?;
    }
    Ok(())
}

pub(crate) async fn request_tool_approval(
    manager: Option<&SessionManager>,
    session_id: &str,
    policy: ApprovalMode,
    permission_rules: &[UiToolPermissionRule],
    turn_id: Option<&str>,
    assistant_message_id: &str,
    approvals: Option<&ToolApprovalStore>,
    channel: Option<&Channel>,
    stream_id: &str,
    sequence: &mut u64,
    request: UiToolApprovalRequest,
) -> Result<ToolApprovalDecision, String> {
    if let Some(reason) = hardline_block_reason(&request.permission, &request.pattern) {
        if let (Some(manager), Some(turn_id), Some(channel)) = (manager, turn_id, channel) {
            persist_permission_agent_timeline_part(
                manager,
                session_id,
                turn_id,
                assistant_message_id,
                stream_id,
                sequence,
                channel,
                &request,
                TimelineStatus::Denied,
                Some(ToolApprovalDecision::DenyAlways),
                Some(&reason),
                ASSISTANT_PERMISSION_STEP_SEQUENCE_BASE + (*sequence as i64),
            )?;
        }
        return Ok(ToolApprovalDecision::DenyAlways);
    }

    let prompt_action = approval_prompt_action(
        permission_rules,
        policy,
        &request.permission,
        &request.pattern,
    )?;

    if !is_interactive_tool_approval_available(approvals, channel) {
        return Ok(if prompt_action == ApprovalPromptAction::Allow {
            if is_risky_tool_permission(&request.permission) {
                ToolApprovalDecision::DenyAlways
            } else {
                ToolApprovalDecision::AllowOnce
            }
        } else {
            ToolApprovalDecision::DenyAlways
        });
    }
    let approvals = approvals.expect("checked interactive approval registry");
    let channel = channel.expect("checked interactive approval channel");

    let step_sequence = ASSISTANT_PERMISSION_STEP_SEQUENCE_BASE + (*sequence as i64);
    if approvals.is_project_denied(
        request.worktree_root.as_deref(),
        &request.permission,
        &request.pattern,
    )? {
        if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
            persist_permission_agent_timeline_part(
                manager,
                session_id,
                turn_id,
                assistant_message_id,
                stream_id,
                sequence,
                channel,
                &request,
                TimelineStatus::Denied,
                Some(ToolApprovalDecision::DenyAlways),
                Some("Denied by this project's permission rule"),
                step_sequence,
            )?;
        }
        return Ok(ToolApprovalDecision::DenyAlways);
    }

    if prompt_action == ApprovalPromptAction::Deny {
        if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
            persist_permission_agent_timeline_part(
                manager,
                session_id,
                turn_id,
                assistant_message_id,
                stream_id,
                sequence,
                channel,
                &request,
                TimelineStatus::Denied,
                Some(ToolApprovalDecision::DenyAlways),
                Some("Denied by the UI approval prompt policy"),
                step_sequence,
            )?;
        }
        return Ok(ToolApprovalDecision::DenyAlways);
    }

    if approvals.is_project_allowed(
        request.worktree_root.as_deref(),
        &request.permission,
        &request.pattern,
    )? {
        if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
            persist_permission_agent_timeline_part(
                manager,
                session_id,
                turn_id,
                assistant_message_id,
                stream_id,
                sequence,
                channel,
                &request,
                TimelineStatus::Completed,
                Some(ToolApprovalDecision::AllowProject),
                Some("Allowed by this project's permission rule"),
                step_sequence,
            )?;
        }
        return Ok(ToolApprovalDecision::AllowProject);
    }

    if approvals.is_session_allowed(session_id, &request.permission, &request.pattern)? {
        if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
            persist_permission_agent_timeline_part(
                manager,
                session_id,
                turn_id,
                assistant_message_id,
                stream_id,
                sequence,
                channel,
                &request,
                TimelineStatus::Completed,
                Some(ToolApprovalDecision::AllowSession),
                Some("Approved by this session's cached approval evidence"),
                step_sequence,
            )?;
        }
        return Ok(ToolApprovalDecision::AllowSession);
    }

    if prompt_action == ApprovalPromptAction::Allow {
        if is_risky_tool_permission(&request.permission) {
            if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
                persist_permission_agent_timeline_part(
                    manager,
                    session_id,
                    turn_id,
                    assistant_message_id,
                    stream_id,
                    sequence,
                    channel,
                    &request,
                    TimelineStatus::Completed,
                    Some(ToolApprovalDecision::AllowOnce),
                    Some("Approved by the UI approval prompt policy"),
                    step_sequence,
                )?;
            }
        }
        return Ok(ToolApprovalDecision::AllowOnce);
    }

    if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
        persist_permission_agent_timeline_part(
            manager,
            session_id,
            turn_id,
            assistant_message_id,
            stream_id,
            sequence,
            channel,
            &request,
            TimelineStatus::WaitingPermission,
            None,
            Some("Waiting for user permission"),
            step_sequence,
        )?;
    }

    let receiver = approvals.register(session_id, &request.request_id)?;
    let chunk = CoreStreamChunk::data(
        stream_id,
        *sequence,
        json!({
            "type": "toolApproval",
            "request": request,
        }),
    );
    *sequence = (*sequence).saturating_add(1);
    if let Err(error) = send_channel_value(channel, chunk.channel_payload()) {
        let _ = approvals.remove_pending(&request.request_id);
        return Err(format!("工具授权请求推送失败: {}", error));
    }

    let decision = match timeout(Duration::from_secs(300), receiver).await {
        Ok(Ok(decision)) => decision,
        Ok(Err(_)) => {
            let _ = approvals.remove_pending(&request.request_id);
            return Err("Tool approval was cancelled".to_string());
        }
        Err(_) => {
            let _ = approvals.remove_pending(&request.request_id);
            return Err("Tool approval timed out".to_string());
        }
    };

    match decision {
        ToolApprovalDecision::AllowSession => {
            approvals.remember_session_allow(session_id, &request.permission, &request.pattern)?;
        }
        ToolApprovalDecision::AllowProject => {
            approvals.remember_project_allow(
                request.worktree_root.as_deref(),
                &request.permission,
                &request.pattern,
            )?;
        }
        ToolApprovalDecision::DenyAlways => {
            approvals.remember_project_deny(
                request.worktree_root.as_deref(),
                &request.permission,
                &request.pattern,
            )?;
        }
        ToolApprovalDecision::AllowOnce => {}
    }

    if let (Some(manager), Some(turn_id)) = (manager, turn_id) {
        let status = if decision.is_allowed() {
            "completed"
        } else {
            "denied"
        };
        persist_permission_agent_timeline_part(
            manager,
            session_id,
            turn_id,
            assistant_message_id,
            stream_id,
            sequence,
            channel,
            &request,
            TimelineStatus::parse(status),
            Some(decision),
            Some(if decision.is_allowed() {
                "Approved by user"
            } else {
                "Denied by user"
            }),
            step_sequence,
        )?;
    }

    Ok(decision)
}

pub(crate) async fn resolve_tool_approval_for_event(
    manager: Option<&SessionManager>,
    session: &Session,
    policy: ApprovalMode,
    permission_rules: &[UiToolPermissionRule],
    turn_id: Option<&str>,
    assistant_message_id: &str,
    approvals: Option<&ToolApprovalStore>,
    channel: Option<&Channel>,
    stream_id: &str,
    sequence: &mut u64,
    tool_timeline: &mut ToolTimelineSequencer,
    tool_events: &mut Vec<AgentToolEvent>,
    event: &AgentToolEvent,
    tool_call: &mut ToolCall,
    request: UiToolApprovalRequest,
) -> Result<ToolApprovalDecision, String> {
    let prompt_action = approval_prompt_action(
        permission_rules,
        policy,
        &request.permission,
        &request.pattern,
    )?;
    if prompt_action == ApprovalPromptAction::Ask {
        let waiting_event = AgentToolEvent {
            detail: Some("Waiting for approval".to_string()),
            output: None,
            progress: None,
            completed_at: None,
            duration_ms: None,
            ..event.clone().with_lifecycle(
//                 [REMOVED: domains reference]
//                 [REMOVED: domains reference]
            )
        };
        tool_events.push(waiting_event.clone());
        send_agent_tool_event(
            manager,
            &session.id,
            turn_id,
            assistant_message_id,
            tool_timeline,
            tool_events,
            channel,
            stream_id,
            sequence,
            waiting_event,
        )?;
    }

    let decision = request_tool_approval(
        manager,
        &session.id,
        policy,
        permission_rules,
        turn_id,
        assistant_message_id,
        approvals,
        channel,
        stream_id,
        sequence,
        request,
    )
    .await?;
    attach_allowed_tool_approval_evidence(prompt_action, Some(decision), tool_call)?;
    Ok(decision)
}
