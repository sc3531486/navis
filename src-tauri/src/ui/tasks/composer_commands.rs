// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use super::common::{ensure_session_exists, has_active_session_task};
use super::goal_runner_commands::goal_runner_next_composer_task;
use crate::domains::agent_core::agent::TaskManager;
use crate::domains::session::session::composer_runtime::{ComposerRuntime, SubmitDisposition};
use crate::domains::session::session::SessionManager;
use crate::ui::composer_projection::{
    composer_run_state_projection, overlay_composer_runtime_state, runtime_composer_task,
    ui_composer_task,
};
use crate::ui::dto::{
    ComposerTaskClearResult, ComposerTaskFinishResult, ComposerTaskIdPayload, ComposerTaskPayload,
    ComposerTaskSubmitResult, SessionIdPayload, UiComposerRunState,
};
use crate::ui::{
    merge_ui_metadata, normalize_composer_run_state, session_composer_run_state,
    UI_COMPOSER_RUN_KEY,
};
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn ui_get_session_composer_run_state(
    manager: State<'_, Arc<SessionManager>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: SessionIdPayload,
) -> Result<UiComposerRunState, String> {
    composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        &payload.session_id,
    )
}

#[tauri::command]
pub fn ui_set_session_composer_run_state(
    manager: State<'_, Arc<SessionManager>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: UiComposerRunState,
) -> Result<UiComposerRunState, String> {
    let manager = manager.inner().as_ref();
    let mut state = normalize_composer_run_state(payload);
    state.running_task = None;
    state.queued_tasks = Vec::new();
    merge_ui_metadata(
        manager,
        &state.session_id,
        vec![(
            UI_COMPOSER_RUN_KEY,
            serde_json::to_value(&state).map_err(|error| error.to_string())?,
        )],
    )?;

    let session = manager
        .get(&state.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", state.session_id))?;

    let mut state = session_composer_run_state(&session);
    overlay_composer_runtime_state(composer_runtime.inner().as_ref(), &mut state)?;
    Ok(state)
}

#[tauri::command]
pub fn ui_submit_composer_task(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: ComposerTaskPayload,
) -> Result<ComposerTaskSubmitResult, String> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    ensure_session_exists(manager.inner().as_ref(), session_id)?;

    let task = runtime_composer_task(payload.task);
    let has_active_session_work =
        has_active_session_task(task_manager.inner().as_ref(), session_id)?;
    let (disposition, task) = composer_runtime
        .inner()
        .lock()
        .map_err(|_| "Composer runtime is unavailable".to_string())?
        .submit(session_id, task, has_active_session_work)?;
    let state = composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        session_id,
    )?;

    Ok(ComposerTaskSubmitResult {
        state,
        disposition: match disposition {
            SubmitDisposition::RunNow => "runNow",
            SubmitDisposition::Queued => "queued",
        }
        .to_string(),
        task: ui_composer_task(task),
    })
}

#[tauri::command]
pub fn ui_finish_composer_task(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: ComposerTaskIdPayload,
) -> Result<ComposerTaskFinishResult, String> {
    let session_id = payload.session_id.trim();
    let task_id = payload.task_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    if task_id.is_empty() {
        return Err("taskId cannot be empty".to_string());
    }
    ensure_session_exists(manager.inner().as_ref(), session_id)?;

    let (finished_task_kind, queued_next_task) = {
        let mut runtime = composer_runtime
            .inner()
            .lock()
            .map_err(|_| "Composer runtime is unavailable".to_string())?;
        let finished_task_kind = runtime
            .running_task(session_id)
            .filter(|task| task.id == task_id)
            .map(|task| task.kind);
        let queued_next_task = runtime.finish(session_id, task_id).map(ui_composer_task);
        (finished_task_kind, queued_next_task)
    };

    let next_task = if queued_next_task.is_some() {
        queued_next_task
    } else if finished_task_kind.as_deref() != Some("goal") {
        None
    } else {
        let goal_next_task = goal_runner_next_composer_task(
            manager.inner().as_ref(),
            task_manager.inner().as_ref(),
            session_id,
            task_id,
        )?;
        if let Some(task) = goal_next_task.as_ref() {
            composer_runtime
                .inner()
                .lock()
                .map_err(|_| "Composer runtime is unavailable".to_string())?
                .submit(session_id, runtime_composer_task(task.clone()), false)?;
        }
        goal_next_task
    };
    let state = composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        session_id,
    )?;

    Ok(ComposerTaskFinishResult { state, next_task })
}

#[tauri::command]
pub fn ui_clear_running_composer_task(
    manager: State<'_, Arc<SessionManager>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: SessionIdPayload,
) -> Result<ComposerTaskClearResult, String> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    ensure_session_exists(manager.inner().as_ref(), session_id)?;

    composer_runtime
        .inner()
        .lock()
        .map_err(|_| "Composer runtime is unavailable".to_string())?
        .clear_running(session_id);
    let state = composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        session_id,
    )?;

    Ok(ComposerTaskClearResult { state })
}

#[tauri::command]
pub fn ui_remove_queued_composer_task(
    manager: State<'_, Arc<SessionManager>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: ComposerTaskIdPayload,
) -> Result<UiComposerRunState, String> {
    let session_id = payload.session_id.trim();
    let task_id = payload.task_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    if task_id.is_empty() {
        return Err("taskId cannot be empty".to_string());
    }
    ensure_session_exists(manager.inner().as_ref(), session_id)?;

    let _ = composer_runtime
        .inner()
        .lock()
        .map_err(|_| "Composer runtime is unavailable".to_string())?
        .remove_queued(session_id, task_id);
    composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        session_id,
    )
}

#[tauri::command]
pub fn ui_promote_queued_composer_task(
    manager: State<'_, Arc<SessionManager>>,
    composer_runtime: State<'_, Arc<Mutex<ComposerRuntime>>>,
    payload: ComposerTaskIdPayload,
) -> Result<ComposerTaskSubmitResult, String> {
    let session_id = payload.session_id.trim();
    let task_id = payload.task_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    if task_id.is_empty() {
        return Err("taskId cannot be empty".to_string());
    }
    ensure_session_exists(manager.inner().as_ref(), session_id)?;

    let (disposition, task) = composer_runtime
        .inner()
        .lock()
        .map_err(|_| "Composer runtime is unavailable".to_string())?
        .promote_queued(session_id, task_id)
        .ok_or_else(|| format!("Queued task not found: {}", task_id))?;
    let state = composer_run_state_projection(
        manager.inner().as_ref(),
        composer_runtime.inner().as_ref(),
        session_id,
    )?;

    Ok(ComposerTaskSubmitResult {
        state,
        disposition: match disposition {
            SubmitDisposition::RunNow => "runNow",
            SubmitDisposition::Queued => "queued",
        }
        .to_string(),
        task: ui_composer_task(task),
    })
}
