// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use super::agent_control::build_agent_control_host;
use super::session_change_capture::{
    prepare_session_change_capture, record_completed_session_change,
};
use super::tool_approval::ToolApprovalStore;
use super::tool_approval_flow::{
    auto_tool_approval_reason, resolve_tool_approval_for_event, tool_approval_request,
    unsupported_gateway_tool_result,
};
// use [REMOVED: domains reference]
    mode_config_from_key, sidechain_stop_requested, update_sidechain_progress, TaskManager,
};
use crate::extension::types::{ChatMessage as GatewayProtocolMessage, ChatResponse, Gateway, ToolCall};
use crate::extension::skills::Skills;
use crate::extension::ExtensionStore;
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use crate::security::sandbox::permission::ApprovalMode;
// use [REMOVED: domains reference]
    agent_tool_definitions, assistant_tool_message, assistant_tool_message_with_content,
    effective_gateway_tool_call, execute_agent_tool_call_async, is_supported_gateway_tool,
    is_supported_sidechain_gateway_tool, parse_text_tool_call, sidechain_agent_tool_definitions,
    tool_call_arguments, tool_call_summary, tool_started_event, AgentToolEvent, AgentToolExecution,
    AgentToolPhase, AgentToolStatus, ToolAvailability,
};
// use [REMOVED: MCP reference]
use crate::ui::timeline::{
    persist_tool_prelude_text_step, send_agent_tool_event, ToolTimelineSequencer,
};
use crate::ui::{
    session_chat_request, session_mode, session_permission_policy, UiToolPermissionRule,
};
use chrono::Utc;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

const AGENT_TOOL_LOOP_MAX_ROUNDS: usize = 24;

pub(crate) enum AgentToolLoopOutcome {
    Direct(ChatResponse),
    FinalMessages(Vec<GatewayProtocolMessage>),
}

pub(crate) fn response_text(response: &ChatResponse) -> String {
    response
        .content
        .clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| {
            response
                .output_items
                .as_ref()
                .map(|items| serde_json::to_string_pretty(items).unwrap_or_default())
                .filter(|text| !text.trim().is_empty())
                .unwrap_or_default()
        })
}

#[derive(Clone)]
pub(crate) struct AgentBackend {
    pub(crate) manager: Arc<SessionManager>,
    pub(crate) gateway: Arc<Gateway>,
    pub(crate) mcp: Arc<MCP>,
    pub(crate) extension_store: Arc<ExtensionStore>,
    pub(crate) skills: Arc<Mutex<Skills>>,
    pub(crate) task_manager: Arc<Mutex<TaskManager>>,
    pub(crate) permission_rules: Vec<UiToolPermissionRule>,
}

pub(crate) async fn execute_ui_agent_tool_call_with_pipeline(
    _manager: Option<&SessionManager>,
    backend: Option<&AgentBackend>,
    _task_manager: Option<&Mutex<TaskManager>>,
    mcp: &MCP,
    session: &Session,
    tool_availability: &impl ToolAvailability,
    tool_call: &ToolCall,
    prior_messages: &[GatewayProtocolMessage],
//     on_progress: Option<&mut [REMOVED: domains reference]
    extension_hooks: &[crate::extension::store::RegisteredHook],
    skills: Option<Arc<Mutex<Skills>>>,
) -> std::result::Result<AgentToolExecution, anyhow::Error> {
    let agent_control_host = backend.map(build_agent_control_host);
//     let execution_context = [REMOVED: domains reference]
        session.id.clone(),
        session.worktree_root.clone(),
    );
    execute_agent_tool_call_async(
        mcp,
        &execution_context,
        tool_availability,
        tool_call,
        prior_messages,
        on_progress,
        extension_hooks,
        agent_control_host,
        skills,
        mcp.policy_engine(),
    )
    .await
}

pub(crate) async fn run_agent_tool_loop(
    manager: Option<&SessionManager>,
    gateway: &Gateway,
    mcp: &MCP,
    session: &Session,
    model: &str,
    mut messages: Vec<GatewayProtocolMessage>,
    channel: Option<&Channel>,
    stream_id: &str,
    next_sequence: &mut u64,
    turn_id: Option<&str>,
    assistant_message_id: &str,
    tool_timeline: &mut ToolTimelineSequencer,
    tool_events: &mut Vec<AgentToolEvent>,
    approvals: Option<&ToolApprovalStore>,
    permission_rules: &[UiToolPermissionRule],
    extension_store: Option<&ExtensionStore>,
    backend: Option<&AgentBackend>,
    task_manager: Option<&Mutex<TaskManager>>,
    sidechain_task_id: Option<&str>,
    mut tool_preludes: Option<&mut Vec<String>>,
    skills: Option<Arc<Mutex<Skills>>>,
) -> Result<AgentToolLoopOutcome, String> {
    let mode_config = mode_config_from_key(session_mode(session).as_deref());
    let allow_sidechain_tools = sidechain_task_id.is_none();
    let tools = if allow_sidechain_tools {
        agent_tool_definitions(mcp, &mode_config)
    } else {
        sidechain_agent_tool_definitions(mcp, &mode_config)
    };
    if tools.is_empty()
        || session
            .worktree_root
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    {
        return Ok(AgentToolLoopOutcome::FinalMessages(messages));
    }

    let mut used_tools = false;
    for _ in 0..AGENT_TOOL_LOOP_MAX_ROUNDS {
        if sidechain_stop_requested(task_manager, sidechain_task_id) {
            return Err("Sidechain task stopped".to_string());
        }
        let request = session_chat_request(session, model.to_string(), messages.clone())
            .with_tools(tools.clone());
        let response = gateway
            .router(request)
            .await
            .map_err(|error| error.to_string())?;
        let assistant_response_text = response_text(&response);
        let tool_calls = response
            .tool_calls
            .clone()
            .filter(|calls| !calls.is_empty());

        let tool_calls = match tool_calls {
            Some(tool_calls) => tool_calls,
            None => {
                if let Some(tool_call) = parse_text_tool_call(&response_text(&response)) {
                    let is_supported = if allow_sidechain_tools {
                        is_supported_gateway_tool(mcp, &mode_config, &tool_call.function.name)
                    } else {
                        is_supported_sidechain_gateway_tool(
                            mcp,
                            &mode_config,
                            &tool_call.function.name,
                        )
                    };
                    if is_supported {
                        vec![tool_call]
                    } else {
                        messages.push(assistant_tool_message(vec![tool_call.clone()]));
                        messages.push(unsupported_gateway_tool_result(&tool_call));
                        continue;
                    }
                } else if used_tools {
                    return Ok(AgentToolLoopOutcome::FinalMessages(messages));
                } else if channel.is_some() {
                    return Ok(AgentToolLoopOutcome::FinalMessages(messages));
                } else {
                    return Ok(AgentToolLoopOutcome::Direct(response));
                }
            }
        };

        used_tools = true;
        let unsupported_tool_call = tool_calls.iter().find(|tool_call| {
            if allow_sidechain_tools {
                !is_supported_gateway_tool(mcp, &mode_config, &tool_call.function.name)
            } else {
                !is_supported_sidechain_gateway_tool(mcp, &mode_config, &tool_call.function.name)
            }
        });
        if let Some(tool_call) = unsupported_tool_call {
            messages.push(assistant_tool_message(tool_calls.clone()));
            messages.push(unsupported_gateway_tool_result(tool_call));
            continue;
        }
        let tool_prelude_text = assistant_response_text.trim();
        if !tool_prelude_text.is_empty() {
            if let Some(tool_preludes) = tool_preludes.as_deref_mut() {
                tool_preludes.push(tool_prelude_text.to_string());
            }
        }
        persist_tool_prelude_text_step(
            manager,
            &session.id,
            turn_id,
            assistant_message_id,
            stream_id,
            next_sequence,
            channel,
            tool_timeline,
            tool_prelude_text,
        )?;
        let resolved_tool_calls = tool_calls
            .into_iter()
            .map(|tool_call| {
                let resolved = effective_gateway_tool_call(mcp, &mode_config, &tool_call);
                (tool_call, resolved)
            })
            .collect::<Vec<_>>();
        let assistant_tool_calls = resolved_tool_calls
            .iter()
            .map(|(original, resolved)| {
                resolved
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|_| original.clone())
            })
            .collect::<Vec<_>>();
        messages.push(assistant_tool_message_with_content(
            assistant_tool_calls,
            assistant_response_text,
        ));
        for (original_tool_call, resolved_tool_call) in resolved_tool_calls {
            let mut tool_call = match resolved_tool_call {
                Ok(effective_tool_call) => effective_tool_call,
                Err(error) => {
                    messages.push(GatewayProtocolMessage::tool_result(
                        &original_tool_call.id,
                        serde_json::to_string_pretty(&json!({
                            "callId": original_tool_call.id,
                            "tool": original_tool_call.function.name,
                            "isError": true,
                            "error": error.to_string(),
                        }))
                        .unwrap_or_else(|_| error.to_string()),
                    ));
                    continue;
                }
            };
            let mut started_event = None;
            if let Ok(event) = tool_started_event(mcp, &mode_config, &tool_call) {
                started_event = Some(event.clone());
                tool_events.push(event.clone());
                update_sidechain_progress(task_manager, sidechain_task_id, tool_events, &event);
                send_agent_tool_event(
                    manager,
                    &session.id,
                    turn_id,
                    assistant_message_id,
                    tool_timeline,
                    tool_events,
                    channel,
                    stream_id,
                    next_sequence,
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
                    Some(auto_tool_approval_reason(session).to_string()),
                    Some(tool_call_arguments(&tool_call)),
                    None,
                    None,
                    None,
                    Some(Utc::now().to_rfc3339()),
                    Some(Utc::now().to_rfc3339()),
                    Some(0),
                )
            });
            let approval_request = tool_approval_request(session, &tool_call, &event);
            let policy = session_permission_policy(session)
                .as_deref()
                .and_then(ApprovalMode::from_str)
                .unwrap_or_default();
            let approval_event_count = tool_events.len();
            let decision = resolve_tool_approval_for_event(
                manager,
                session,
                policy,
                permission_rules,
                turn_id,
                assistant_message_id,
                approvals,
                channel,
                stream_id,
                next_sequence,
                tool_timeline,
                tool_events,
                &event,
                &mut tool_call,
                approval_request,
            )
            .await?;
            for approval_event in tool_events[approval_event_count..].to_vec() {
                update_sidechain_progress(
                    task_manager,
                    sidechain_task_id,
                    tool_events,
                    &approval_event,
                );
            }
            if !decision.is_allowed() {
                let rejected_text = "Tool call was not approved".to_string();
                let rejected_event = AgentToolEvent {
                    detail: Some(rejected_text.clone()),
                    summary: Some(rejected_text.clone()),
                    output: Some(json!({ "error": rejected_text })),
                    metadata: Some(json!({ "isError": true })),
                    progress: None,
                    completed_at: Some(Utc::now().to_rfc3339()),
                    duration_ms: None,
                    ..event.with_lifecycle(AgentToolPhase::Completed, AgentToolStatus::Error)
                };
                tool_events.push(rejected_event.clone());
                update_sidechain_progress(
                    task_manager,
                    sidechain_task_id,
                    tool_events,
                    &rejected_event,
                );
                send_agent_tool_event(
                    manager,
                    &session.id,
                    turn_id,
                    assistant_message_id,
                    tool_timeline,
                    tool_events,
                    channel,
                    stream_id,
                    next_sequence,
                    rejected_event,
                )?;
                messages.push(GatewayProtocolMessage::tool_result(
                    &tool_call.id,
                    serde_json::to_string_pretty(&json!({
                        "callId": tool_call.id,
                        "tool": tool_call.function.name,
                        "isError": true,
                        "error": rejected_text,
                    }))
                    .unwrap_or_else(|_| rejected_text),
                ));
                continue;
            }
            let pending_session_change =
                prepare_session_change_capture(session, turn_id, assistant_message_id, &tool_call)?;
            let extension_hooks = extension_store
                .map(ExtensionStore::list_hooks)
                .unwrap_or_default();
            let execution_result = execute_ui_agent_tool_call_with_pipeline(
                manager,
                backend,
                task_manager,
                mcp,
                session,
                &mode_config,
                &tool_call,
                &messages,
                Some(&mut |event: AgentToolEvent| {
                    tool_events.push(event.clone());
                    update_sidechain_progress(task_manager, sidechain_task_id, tool_events, &event);
                    send_agent_tool_event(
                        manager,
                        &session.id,
                        turn_id,
                        assistant_message_id,
                        tool_timeline,
                        tool_events,
                        channel,
                        stream_id,
                        next_sequence,
                        event,
                    )
                    .map(|_| ())
                }),
                &extension_hooks,
                skills.clone(),
            )
            .await;

            match execution_result {
                Ok(execution) => {
                    tool_events.push(execution.event.clone());
                    update_sidechain_progress(
                        task_manager,
                        sidechain_task_id,
                        tool_events,
                        &execution.event,
                    );
                    let part_id = send_agent_tool_event(
                        manager,
                        &session.id,
                        turn_id,
                        assistant_message_id,
                        tool_timeline,
                        tool_events,
                        channel,
                        stream_id,
                        next_sequence,
                        execution.event.clone(),
                    )?;
                    record_completed_session_change(
                        manager,
                        &session.id,
                        pending_session_change,
                        part_id,
                        &execution,
                    )?;
                    messages.push(execution.result_message);
                }
                Err(error) => {
                    let error_text = error.to_string();
                    let event = AgentToolEvent::new(
                        AgentToolPhase::Completed,
                        AgentToolStatus::Error,
                        tool_call.id.clone(),
                        tool_call.function.name.clone(),
                        tool_call.function.name.clone(),
                        tool_call.function.name.clone(),
                        Some(error_text.clone()),
                        Some(error_text.clone()),
                        Some(tool_call_arguments(&tool_call)),
                        Some(json!({ "error": error_text })),
                        Some(json!({ "isError": true })),
                        None,
                        started_event
                            .as_ref()
                            .and_then(|event| event.started_at.clone()),
                        Some(Utc::now().to_rfc3339()),
                        None,
                    );
                    tool_events.push(event.clone());
                    update_sidechain_progress(task_manager, sidechain_task_id, tool_events, &event);
                    send_agent_tool_event(
                        manager,
                        &session.id,
                        turn_id,
                        assistant_message_id,
                        tool_timeline,
                        tool_events,
                        channel,
                        stream_id,
                        next_sequence,
                        event,
                    )?;
                    if error_text.contains("Unsupported Agent tool") {
                        return Err(error_text);
                    }
                    messages.push(GatewayProtocolMessage::tool_result(
                        &tool_call.id,
                        serde_json::to_string_pretty(&json!({
                            "callId": tool_call.id,
                            "tool": tool_call.function.name,
                            "isError": true,
                            "error": error_text.clone(),
                        }))
                        .unwrap_or_else(|_| error_text.clone()),
                    ));
                }
            }
        }
    }

    Ok(AgentToolLoopOutcome::FinalMessages(messages))
}
