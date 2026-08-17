// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use crate::domains::agent_core::agent::turn_output::assistant_visible_content;
use crate::domains::agent_core::agent::{
    mark_sidechain_failed_and_notify, notify_parent_sidechain_task_best_effort,
    sidechain_outcome_from_assistant_content, sidechain_stop_requested, TaskManager,
};
use crate::domains::agent_core::application::runtime::{SidechainStartRequest, SidechainStarted};
use crate::domains::session::session::store::MessageRole as StorageMessageRole;
use crate::domains::session::session::{SessionManager, TimelineStatus};
use crate::ui::agent_timeline_part::record_agent_timeline_part;
use crate::ui::messages::{storage_assistant_shell_message, storage_text_message};
use crate::ui::runtime::agent_tool_loop::{
    response_text, run_agent_tool_loop, AgentBackend, AgentToolLoopOutcome,
};
use crate::ui::tasks::context_model::current_or_default_model;
use crate::ui::timeline::{
    finalizer_agent_timeline_part, prelude_agent_timeline_part, text_agent_timeline_part,
    ToolTimelineSequencer,
};
use crate::ui::{
    assistant_gateway_metadata, build_gateway_protocol_messages, merge_ui_metadata,
    persist_terminal_turn_step, session_chat_request, update_assistant_shell,
};
use serde_json::json;
use std::sync::Mutex;
use tokio::time::Instant;

async fn run_sidechain_task(
    backend: AgentBackend,
    sidechain_session_id: String,
    task_id: String,
    prompt: String,
) {
    let manager = backend.manager.as_ref();
    let gateway = backend.gateway.as_ref();
    let mcp = backend.mcp.as_ref();
    let task_manager = backend.task_manager.as_ref();
    let stream_id = format!("sidechain:{}", task_id);
    let mut sequence = 0_u64;
    let started_at = Instant::now();

    let _ = task_manager.lock().map(|mut tasks| {
        if let Some(task) = tasks.get_sidechain_task_mut(&task_id) {
            task.mark_running_with_activity("Starting sidechain session");
        }
    });

    let user_message = storage_text_message(
        &sidechain_session_id,
        StorageMessageRole::User,
        &prompt,
        None,
        None,
    );
    let turn_id = user_message.id.clone();
    if let Err(error) = manager.add_message(&sidechain_session_id, user_message) {
        mark_sidechain_failed_and_notify(task_manager, &task_id, error.to_string());
        return;
    }

    let assistant_message =
        storage_assistant_shell_message(&sidechain_session_id, &turn_id, &stream_id, None);
    let assistant_message_id = assistant_message.id.clone();
    if let Err(error) = manager.add_message(&sidechain_session_id, assistant_message) {
        mark_sidechain_failed_and_notify(task_manager, &task_id, error.to_string());
        return;
    }

    let prelude = prelude_agent_timeline_part(
        &sidechain_session_id,
        &turn_id,
        &assistant_message_id,
        &prompt,
    );
    let _ = record_agent_timeline_part(manager, &stream_id, &mut sequence, None, prelude);

    let session = match manager.get(&sidechain_session_id) {
        Ok(Some(session)) => session,
        Ok(None) => {
            mark_sidechain_failed_and_notify(
                task_manager,
                &task_id,
                "Sidechain session disappeared",
            );
            return;
        }
        Err(error) => {
            mark_sidechain_failed_and_notify(task_manager, &task_id, error.to_string());
            return;
        }
    };
    let model = match current_or_default_model(manager, gateway, &sidechain_session_id) {
        Ok(model) => model,
        Err(error) => {
            mark_sidechain_failed_and_notify(task_manager, &task_id, error);
            return;
        }
    };
    let gateway_messages = match manager.get_messages(&sidechain_session_id, Some(100), Some(0)) {
        Ok(messages) => build_gateway_protocol_messages(mcp, &session, &messages, false),
        Err(error) => {
            mark_sidechain_failed_and_notify(task_manager, &task_id, error.to_string());
            return;
        }
    };

    let mut tool_timeline = ToolTimelineSequencer::default();
    let mut tool_events = Vec::new();
    let response = match run_agent_tool_loop(
        Some(manager),
        gateway,
        mcp,
        &session,
        &model,
        gateway_messages,
        None,
        &stream_id,
        &mut sequence,
        Some(&turn_id),
        &assistant_message_id,
        &mut tool_timeline,
        &mut tool_events,
        None,
        &backend.permission_rules,
        Some(backend.extension_store.as_ref()),
        Some(&backend),
        Some(task_manager),
        Some(&task_id),
        None,
        Some(backend.skills.clone()),
    )
    .await
    {
        Ok(AgentToolLoopOutcome::Direct(response)) => Ok(response),
        Ok(AgentToolLoopOutcome::FinalMessages(messages)) => gateway
            .router(session_chat_request(&session, model.clone(), messages))
            .await
            .map_err(|error| error.to_string()),
        Err(error) => Err(error),
    };

    if sidechain_stop_requested(Some(task_manager), Some(&task_id)) {
        persist_terminal_turn_step(
            manager,
            &sidechain_session_id,
            &turn_id,
            &assistant_message_id,
            &stream_id,
            &mut sequence,
            None,
            "",
            Some(&model),
            "aborted",
            "Sidechain task stopped",
            "The sidechain was stopped by task_stop.",
        );
        let _ = task_manager.lock().map(|mut tasks| {
            if let Some(task) = tasks.get_sidechain_task_mut(&task_id) {
                task.mark_cancelled();
            }
        });
        notify_parent_sidechain_task_best_effort(task_manager, &task_id);
        return;
    }

    match response {
        Ok(response) => {
            let assistant_content = response_text(&response);
            let visible_assistant_content = assistant_visible_content(&assistant_content, &[]);
            let text_step = text_agent_timeline_part(
                &sidechain_session_id,
                &turn_id,
                &assistant_message_id,
                &visible_assistant_content,
                TimelineStatus::Completed,
            );
            let _ = record_agent_timeline_part(manager, &stream_id, &mut sequence, None, text_step);
            let metadata = assistant_gateway_metadata(
                &turn_id,
                &stream_id,
                Some(&response.model),
                Some(&response.id),
                Some(&response.finish_reason),
                Some(json!(&response.usage)),
                "completed",
            );
            let _ = update_assistant_shell(
                manager,
                &sidechain_session_id,
                &assistant_message_id,
                &visible_assistant_content,
                Some(response.usage.total_tokens as i64),
                metadata,
            );
            let finalizer = finalizer_agent_timeline_part(
                &sidechain_session_id,
                &turn_id,
                &assistant_message_id,
                &tool_events,
                Some(response.usage.total_tokens as i64),
                Some(started_at.elapsed().as_millis() as u64),
            );
            let _ = record_agent_timeline_part(manager, &stream_id, &mut sequence, None, finalizer);
            let _ = task_manager.lock().map(|mut tasks| {
                if let Some(task) = tasks.get_sidechain_task_mut(&task_id) {
                    task.mark_completed_with_outcome(
                        sidechain_outcome_from_assistant_content(&assistant_content),
                        response.usage.total_tokens as i64,
                    );
                }
            });
            notify_parent_sidechain_task_best_effort(task_manager, &task_id);
        }
        Err(error) => {
            persist_terminal_turn_step(
                manager,
                &sidechain_session_id,
                &turn_id,
                &assistant_message_id,
                &stream_id,
                &mut sequence,
                None,
                "",
                Some(&model),
                "error",
                "Sidechain task failed",
                &error,
            );
            let _ = task_manager.lock().map(|mut tasks| {
                if let Some(task) = tasks.get_sidechain_task_mut(&task_id) {
                    task.mark_failed(error);
                }
            });
            notify_parent_sidechain_task_best_effort(task_manager, &task_id);
        }
    }
}

pub(crate) fn start_sidechain_agent_task(
    manager: &SessionManager,
    backend: AgentBackend,
    task_manager: &Mutex<TaskManager>,
    request: SidechainStartRequest,
) -> Result<SidechainStarted, anyhow::Error> {
    let parent_session = manager
        .get(&request.parent_session_id)?
        .ok_or_else(|| anyhow::anyhow!("父会话不存在: {}", request.parent_session_id))?;
    let child_session = manager.create(
        parent_session.worktree_root.as_deref(),
        Some(&format!("Task: {}", request.description)),
        parent_session
            .model_id
            .as_deref()
            .or(parent_session.model.as_deref()),
    )?;
    if let Some(provider_id) = parent_session.provider_id.as_deref() {
        if let Some(model_id) = parent_session.model_id.as_deref() {
            let _ = manager.update_model_selection(&child_session.id, provider_id, model_id);
        }
    }
    let _ = merge_ui_metadata(
        manager,
        &child_session.id,
        vec![
            ("parentSessionId", json!(request.parent_session_id)),
            ("parentTaskDescription", json!(request.description.clone())),
        ],
    );
    let task_id = match task_manager.lock() {
        Ok(mut tasks) => tasks.create_sidechain_task(
            &request.parent_session_id,
            &child_session.id,
            &request.description,
            &request.prompt,
        ),
        Err(_) => return Err(anyhow::anyhow!("后台任务状态不可用")),
    };
    let sidechain_session_id = child_session.id.clone();
    let child_task_id = task_id.clone();
    let prompt = request.prompt;
    let task_manager_for_thread = backend.task_manager.clone();
    std::thread::spawn(move || {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(run_sidechain_task(
                backend,
                sidechain_session_id,
                child_task_id,
                prompt,
            )),
            Err(error) => {
                tracing::error!(error = %error, "Failed to start sidechain runtime");
                mark_sidechain_failed_and_notify(
                    task_manager_for_thread.as_ref(),
                    &child_task_id,
                    format!("Failed to start sidechain runtime: {error}"),
                );
            }
        }
    });
    Ok(SidechainStarted {
        task_id,
        sidechain_session_id: child_session.id,
    })
}
