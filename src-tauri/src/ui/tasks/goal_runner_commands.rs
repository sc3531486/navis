// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use crate::domains::agent_core::agent::{
    apply_goal_runner_command, decide_goal_runner_next_task, GoalRunnerCommand, GoalRunnerDecision,
    GoalRunnerRequest, GoalRunnerStatePatch, TaskManager,
};
use crate::domains::session::session::SessionManager;
use crate::ui::dto::{
    GoalRunnerControlPayload, GoalRunnerPayload, UiComposerRunState, UiComposerTask,
};
use crate::ui::{merge_ui_metadata, session_composer_run_state, UI_COMPOSER_RUN_KEY};
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub fn ui_start_goal_runner(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: GoalRunnerPayload,
) -> Result<UiComposerRunState, String> {
    let session_id = payload.session_id.trim();
    let goal = payload.goal.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    if goal.is_empty() {
        return Err("goal cannot be empty".to_string());
    }

    let mut tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "Goal task state is unavailable".to_string())?;
    let patch = apply_goal_runner_command(
        &mut tasks,
        GoalRunnerCommand::Start {
            session_id: session_id.to_string(),
            goal: goal.to_string(),
            prompt: payload.prompt,
        },
    );

    apply_goal_runner_state_patch(manager.inner().as_ref(), session_id, patch)
}

#[tauri::command]
pub fn ui_pause_goal_runner(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: GoalRunnerControlPayload,
) -> Result<UiComposerRunState, String> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    let goal = session_composer_run_state(
        &manager
            .inner()
            .as_ref()
            .get(session_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?,
    )
    .active_goal_text
    .unwrap_or_default();

    let mut tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "Goal task state is unavailable".to_string())?;
    let patch = apply_goal_runner_command(
        &mut tasks,
        GoalRunnerCommand::Pause {
            session_id: session_id.to_string(),
            active_goal: goal,
        },
    );

    apply_goal_runner_state_patch(manager.inner().as_ref(), session_id, patch)
}

#[tauri::command]
pub fn ui_resume_goal_runner(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: GoalRunnerControlPayload,
) -> Result<UiComposerRunState, String> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    let current_state = {
        let session = manager
            .inner()
            .as_ref()
            .get(session_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("会话不存在: {}", session_id))?;
        session_composer_run_state(&session)
    };
    let goal = current_state.active_goal_text.unwrap_or_default();
    if goal.trim().is_empty() {
        return Err("No active goal".to_string());
    }

    let mut tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "Goal task state is unavailable".to_string())?;
    let patch = apply_goal_runner_command(
        &mut tasks,
        GoalRunnerCommand::Resume {
            session_id: session_id.to_string(),
            active_goal: goal,
        },
    );

    apply_goal_runner_state_patch(manager.inner().as_ref(), session_id, patch)
}

#[tauri::command]
pub fn ui_stop_goal_runner(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: GoalRunnerControlPayload,
) -> Result<UiComposerRunState, String> {
    let session_id = payload.session_id.trim();
    if session_id.is_empty() {
        return Err("sessionId cannot be empty".to_string());
    }
    let mut tasks = task_manager
        .inner()
        .lock()
        .map_err(|_| "Goal task state is unavailable".to_string())?;
    let patch = apply_goal_runner_command(
        &mut tasks,
        GoalRunnerCommand::Stop {
            session_id: session_id.to_string(),
        },
    );

    apply_goal_runner_state_patch(manager.inner().as_ref(), session_id, patch)
}

fn apply_goal_runner_state_patch(
    manager: &SessionManager,
    session_id: &str,
    patch: GoalRunnerStatePatch,
) -> Result<UiComposerRunState, String> {
    let mut state = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .map(|session| session_composer_run_state(&session))
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;
    state.goal_tracking_enabled = patch.goal_tracking_enabled;
    state.goal_paused = patch.goal_paused;
    state.active_goal_text = patch.active_goal_text;
    if patch.reset_started_at {
        state.active_goal_started_at = None;
    } else if state.active_goal_started_at.is_none() {
        state.active_goal_started_at = Some(chrono::Utc::now().to_rfc3339());
    }

    merge_ui_metadata(
        manager,
        session_id,
        vec![(
            UI_COMPOSER_RUN_KEY,
            serde_json::to_value(&state).map_err(|error| error.to_string())?,
        )],
    )?;

    let session = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;
    Ok(session_composer_run_state(&session))
}

pub(super) fn goal_runner_next_composer_task(
    manager: &SessionManager,
    task_manager: &Mutex<TaskManager>,
    session_id: &str,
    completed_task_id: &str,
) -> Result<Option<UiComposerTask>, String> {
    let state = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .map(|session| session_composer_run_state(&session))
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;

    let mut tasks = task_manager
        .lock()
        .map_err(|_| "Goal task state is unavailable".to_string())?;
    let decision = decide_goal_runner_next_task(
        &mut tasks,
        GoalRunnerRequest::CompletedTask {
            session_id: session_id.to_string(),
            completed_task_id: completed_task_id.to_string(),
        },
        state.active_goal_text.as_deref(),
        state.goal_paused,
    );

    let GoalRunnerDecision::Continue {
        task_id,
        prompt,
        display_text,
    } = decision
    else {
        return Ok(None);
    };

    Ok(Some(UiComposerTask {
        id: task_id,
        kind: Some("goal".to_string()),
        text: prompt,
        source_text: Some(display_text.clone()),
        display_text: Some(display_text),
        attachments: Vec::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}
