// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use super::task_projection::{
    is_background_task_projection, session_todos_from_metadata, ui_task_record, ui_todo_item,
};
use crate::domains::agent_core::agent::{notify_parent_sidechain_task, TaskKind, TaskManager};
use crate::foundation::stream::{stream_kind, StreamIndex};
use crate::domains::session::session::SessionManager;
use crate::domains::session::session::TimelineStatus;
use crate::ui::dto::{
    CancelStreamPayload, ClearFinishedTasksPayload, ListTasksPayload, ListTodosPayload,
    TaskIdPayload, ToolApprovalResponsePayload, UiTask, UiTodoItem,
};
use crate::ui::runtime::tool_approval::ToolApprovalDecision;
use crate::ui::ToolApprovalStore;
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn ui_list_tasks(
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: ListTasksPayload,
) -> Result<Vec<UiTask>, String> {
    let limit = payload.limit.unwrap_or(20).clamp(1, 100);
    let tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "后台任务状态不可用".to_string())?;
    let mut rows: Vec<UiTask> = match payload.session_id.as_deref() {
        Some(session_id) if !session_id.trim().is_empty() => {
            let session_id = session_id.trim();
            tasks
                .get_tasks_by_session(session_id)
                .into_iter()
                .filter(|task| is_background_task_projection(task))
                .map(ui_task_record)
                .collect()
        }
        _ => tasks
            .list_tasks()
            .into_iter()
            .filter(|task| is_background_task_projection(task))
            .map(ui_task_record)
            .collect(),
    };

    rows.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(rows.into_iter().take(limit).collect())
}

#[tauri::command]
pub fn ui_stop_task(
    _manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: TaskIdPayload,
) -> Result<bool, String> {
    let task_id = payload.task_id.trim();
    if task_id.is_empty() {
        return Err("taskId cannot be empty".to_string());
    }
    let task_manager = task_manager.inner();
    let mut tasks = task_manager
        .lock()
        .map_err(|_| "Background task state is unavailable".to_string())?;
    let task = tasks
        .get_task_mut(task_id)
        .ok_or_else(|| format!("Task not found: {}", task_id))?;
    if task.status.is_terminal() {
        return Ok(false);
    }
    let is_sidechain = task.kind == TaskKind::Sidechain;
    task.mark_cancelled();
    drop(tasks);
    if is_sidechain {
        notify_parent_sidechain_task(task_manager.as_ref(), task_id)
            .map_err(|error| format!("Task stopped, but parent notification failed: {error}"))?;
    }
    Ok(true)
}

#[tauri::command]
pub fn ui_clear_finished_tasks(
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: ClearFinishedTasksPayload,
) -> Result<usize, String> {
    let mut tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "Background task state is unavailable".to_string())?;
    let removed = match payload.session_id.as_deref().map(str::trim) {
        Some(session_id) if !session_id.is_empty() => {
            tasks.cleanup_completed_by_session(session_id)
        }
        _ => tasks.cleanup_completed(),
    };
    Ok(removed)
}

#[tauri::command]
pub fn ui_list_session_todos(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: ListTodosPayload,
) -> Result<Vec<UiTodoItem>, String> {
    let live_todos = task_manager
        .inner()
        .lock()
        .map_err(|_| "Todo 状态不可用".to_string())?
        .get_todos(&payload.session_id);
    if !live_todos.is_empty() {
        return Ok(live_todos.into_iter().map(ui_todo_item).collect());
    }

    let persisted_todos = manager
        .inner()
        .as_ref()
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .map(|session| session_todos_from_metadata(&session))
        .unwrap_or_default();
    Ok(persisted_todos.into_iter().map(ui_todo_item).collect())
}

#[tauri::command]
pub fn ui_cancel_stream(
    stream_index: State<'_, Arc<StreamIndex>>,
    manager: State<'_, Arc<SessionManager>>,
    payload: CancelStreamPayload,
) -> Result<bool, String> {
    let stream_id = payload.stream_id.trim();
    if stream_id.is_empty() {
        return Err("streamId 不能为空".to_string());
    }
    let Some(info) = stream_index.cancel_info(stream_id) else {
        return Ok(false);
    };

    if info.source.is_kind(stream_kind::AGENT) {
        let session_id = info
            .source
            .meta("session_id")
            .unwrap_or(info.source.id.as_str());
        if let (Some(turn_id), Some(message_id)) = (
            info.source.meta("turn_id"),
            info.source.meta("assistant_message_id"),
        ) {
            manager
                .inner()
                .close_active_agent_timeline_parts(
                    session_id,
                    turn_id,
                    message_id,
                    TimelineStatus::Aborted,
                )
                .map_err(|error| error.to_string())?;
        }
    }

    Ok(true)
}

#[tauri::command]
pub fn ui_respond_tool_approval(
    approvals: State<'_, Arc<ToolApprovalStore>>,
    payload: ToolApprovalResponsePayload,
) -> Result<bool, String> {
    let request_id = payload.request_id.trim();
    if request_id.is_empty() {
        return Err("requestId 不能为空".to_string());
    }
    let decision = ToolApprovalDecision::from_str(payload.decision.trim()).ok_or_else(|| {
        format!(
            "不支持的工具授权决策: {}，仅支持 allow_once / allow_session / allow_project / deny_always",
            payload.decision
        )
    })?;
    approvals.respond(request_id, decision)
}
