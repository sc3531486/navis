// ── 归属扩展：navis-session ──
// 迁移目标：extensions/navis-session/ExtensionBackend/src/

use super::dto::*;
use super::session_permission_policy;
use super::session_reasoning_effort;
use super::session_transcript_view;
use super::{completed_task_seen_at, task_completed_after_seen_at};
use super::{main_session_task_flags, merge_ui_metadata};
use super::{session_display_name, session_mode, session_worktree_name};
use super::{
    ui_bool, UI_MODE_KEY, UI_PERMISSION_POLICY_KEY, UI_PINNED_KEY, UI_REASONING_EFFORT_KEY,
    UI_TASK_COMPLETED_SEEN_AT_KEY, UI_TRANSCRIPT_VIEW_KEY, UI_UNREAD_KEY, UI_WORKTREE_KEY,
};
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use crate::security::sandbox::permission::ApprovalMode;
use chrono::Utc;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tauri::State;

// ─── Session tree commands ───

#[tauri::command]
pub fn ui_list_session_tree(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
) -> Result<UiSessionTree, String> {
    build_session_tree_with_task_manager(manager.inner().as_ref(), task_manager.inner().as_ref())
}

#[tauri::command]
pub fn ui_create_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: NewSessionPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let session = manager
        .create(None, payload.name.as_deref(), None)
        .map_err(|error| error.to_string())?;

    if let Some((provider_id, model_id)) = payload
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .zip(
            payload
                .model_id
                .as_deref()
                .map(str::trim)
                .filter(|id| !id.is_empty()),
        )
    {
        manager
            .update_model_selection(&session.id, provider_id, model_id)
            .map_err(|error| error.to_string())?;
    }

    if let Some(worktree_name) = payload.worktree_name.filter(|name| !name.trim().is_empty()) {
        merge_ui_metadata(
            manager,
            &session.id,
            vec![(UI_WORKTREE_KEY, json!(worktree_name.trim()))],
        )?;
    }

    if let Some(mode) = payload.mode.filter(|mode| !mode.trim().is_empty()) {
        merge_ui_metadata(
            manager,
            &session.id,
            vec![(UI_MODE_KEY, json!(mode.trim()))],
        )?;
    }

    manager
        .set_active(&session.id)
        .map_err(|error| error.to_string())?;

    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_active_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionIdPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    manager
        .set_active(&payload.session_id)
        .map_err(|error| error.to_string())?;
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![
            (UI_UNREAD_KEY, json!(false)),
            (
                UI_TASK_COMPLETED_SEEN_AT_KEY,
                json!(Utc::now().to_rfc3339()),
            ),
        ],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_rename_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: RenameSessionPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    manager
        .update(&payload.session_id, Some(payload.name.trim()), None, None)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_model(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionModelPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let provider_id = payload.provider_id.trim();
    let model_id = payload.model_id.trim();
    if provider_id.is_empty() || model_id.is_empty() {
        return Err("Provider and model are required".to_string());
    }
    manager
        .update_model_selection(&payload.session_id, provider_id, model_id)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_permission_policy(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionPermissionPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let permission_policy = payload.permission_policy.trim();
    let permission_policy = ApprovalMode::from_str(permission_policy)
        .ok_or_else(|| format!("不支持的权限策略: {}", permission_policy))?;
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_PERMISSION_POLICY_KEY, json!(permission_policy.as_str()))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_transcript_view(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionTranscriptViewPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let transcript_view = payload.transcript_view.trim();
    if !matches!(transcript_view, "standard" | "compact" | "raw") {
        return Err(format!("不支持的 Transcript view: {}", transcript_view));
    }
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_TRANSCRIPT_VIEW_KEY, json!(transcript_view))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_reasoning_effort(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionReasoningEffortPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let reasoning_effort = payload.reasoning_effort.trim();
    if !matches!(
        reasoning_effort,
        "low" | "medium" | "high" | "extra-high" | "max"
    ) {
        return Err(format!("不支持的推理强度: {}", reasoning_effort));
    }
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_REASONING_EFFORT_KEY, json!(reasoning_effort))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_worktree_root(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionWorktreeRootPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let worktree_root = payload
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty());
    manager
        .set_worktree_root(&payload.session_id, worktree_root)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_archive_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionIdPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    manager
        .archive(&payload.session_id)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_delete_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionIdPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    manager
        .delete(&payload.session_id)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_fork_session(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionIdPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let source = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let worktree_name = session_worktree_name(&source);
    let mode = session_mode(&source);
    let fork_name = format!("{} fork", session_display_name(&source));
    let fork = manager
        .create(source.worktree_root.as_deref(), Some(&fork_name), None)
        .map_err(|error| error.to_string())?;
    if let Some((provider_id, model_id)) = source
        .provider_id
        .as_deref()
        .zip(source.model_id.as_deref())
    {
        manager
            .update_model_selection(&fork.id, provider_id, model_id)
            .map_err(|error| error.to_string())?;
    }
    if let Err(error) = manager.copy_session_timeline(&source.id, &fork.id) {
        let _ = manager.delete(&fork.id);
        return Err(error.to_string());
    }

    let mut changes = vec![
        (UI_WORKTREE_KEY, json!(worktree_name)),
        (UI_UNREAD_KEY, json!(true)),
    ];
    if let Some(mode) = mode {
        changes.push((UI_MODE_KEY, json!(mode)));
    }
    merge_ui_metadata(manager, &fork.id, changes)?;
    manager
        .set_active(&fork.id)
        .map_err(|error| error.to_string())?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_pinned(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionFlagPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let next_value = payload
        .value
        .unwrap_or_else(|| !ui_bool(&session, UI_PINNED_KEY));
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_PINNED_KEY, json!(next_value))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_set_session_unread(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: SessionFlagPayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let next_value = payload
        .value
        .unwrap_or_else(|| !ui_bool(&session, UI_UNREAD_KEY));
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_UNREAD_KEY, json!(next_value))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_move_session_to_worktree(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: MoveSessionToWorktreePayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    merge_ui_metadata(
        manager,
        &payload.session_id,
        vec![(UI_WORKTREE_KEY, json!(payload.worktree_name.trim()))],
    )?;
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_rename_worktree(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: RenameWorktreePayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let sessions = manager.list(None).map_err(|error| error.to_string())?;
    let mode = payload
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty());
    for session in sessions
        .iter()
        .filter(|session| session.status == SessionStatus::Active)
        .filter(|session| session_worktree_name(session) == payload.old_name)
        .filter(|session| mode.map_or(true, |mode| session_mode(session).as_deref() == Some(mode)))
    {
        merge_ui_metadata(
            manager,
            &session.id,
            vec![(UI_WORKTREE_KEY, json!(payload.new_name.trim()))],
        )?;
    }
    build_session_tree_with_task_manager(manager, task_manager)
}

#[tauri::command]
pub fn ui_delete_worktree(
    manager: State<'_, Arc<SessionManager>>,
    task_manager: State<'_, Arc<Mutex<TaskManager>>>,
    payload: WorktreeNamePayload,
) -> Result<UiSessionTree, String> {
    let manager = manager.inner().as_ref();
    let task_manager = task_manager.inner().as_ref();
    let sessions = manager.list(None).map_err(|error| error.to_string())?;
    let mode = payload
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|mode| !mode.is_empty());
    for session in sessions
        .iter()
        .filter(|session| session.status == SessionStatus::Active)
        .filter(|session| session_worktree_name(session) == payload.worktree_name)
        .filter(|session| mode.map_or(true, |mode| session_mode(session).as_deref() == Some(mode)))
    {
        manager
            .delete(&session.id)
            .map_err(|error| error.to_string())?;
    }
    build_session_tree_with_task_manager(manager, task_manager)
}

// ─── Private helper functions ───

fn build_session_tree_with_task_manager(
    manager: &SessionManager,
    task_manager: &Mutex<TaskManager>,
) -> Result<UiSessionTree, String> {
    let (running_sessions, completed_sessions) = main_session_task_flags(task_manager);
    build_session_tree_with_task_flags(manager, &running_sessions, &completed_sessions)
}

fn sidebar_session(
    session: &Session,
    has_running_task: bool,
    has_completed_task: bool,
) -> UiSidebarSession {
    UiSidebarSession {
        id: session.id.clone(),
        name: session_display_name(session),
        created_at: super::normalize_rfc3339_timestamp(session.created_at.trim())
            .unwrap_or_else(|| session.created_at.clone()),
        pinned: ui_bool(session, UI_PINNED_KEY),
        unread: ui_bool(session, UI_UNREAD_KEY),
        has_running_task,
        has_completed_task,
        model: session.model.clone(),
        provider_id: session.provider_id.clone(),
        model_id: session.model_id.clone(),
        mode: session_mode(session),
        worktree_root: session.worktree_root.clone(),
        permission_policy: session_permission_policy(session),
        transcript_view: session_transcript_view(session),
        reasoning_effort: session_reasoning_effort(session),
    }
}

pub(crate) fn build_session_tree_with_task_flags(
    manager: &SessionManager,
    running_sessions: &HashSet<String>,
    completed_sessions: &HashMap<String, chrono::DateTime<Utc>>,
) -> Result<UiSessionTree, String> {
    let sessions = manager.list(None).map_err(|error| error.to_string())?;
    let active_session_id = manager.get_active();
    let mut worktrees: Vec<UiSessionWorktree> = Vec::new();

    let mut active_sessions = sessions
        .iter()
        .filter(|session| session.status == SessionStatus::Active)
        .collect::<Vec<_>>();
    active_sessions.sort_by(|a, b| {
        super::normalize_rfc3339_timestamp(a.created_at.trim())
            .unwrap_or_else(|| a.created_at.clone())
            .cmp(
                &super::normalize_rfc3339_timestamp(b.created_at.trim())
                    .unwrap_or_else(|| b.created_at.clone()),
            )
            .then_with(|| a.id.cmp(&b.id))
    });

    for session in active_sessions {
        let worktree_name = session_worktree_name(session);
        let worktree_index = worktrees
            .iter()
            .position(|worktree| worktree.name == worktree_name)
            .unwrap_or_else(|| {
                worktrees.push(UiSessionWorktree {
                    name: worktree_name.clone(),
                    sessions: Vec::new(),
                    collapsed: false,
                });
                worktrees.len() - 1
            });

        let has_running_task = running_sessions.contains(&session.id);
        let has_completed_task = !has_running_task
            && active_session_id.as_deref() != Some(session.id.as_str())
            && completed_sessions
                .get(&session.id)
                .is_some_and(|completed_at| {
                    task_completed_after_seen_at(
                        *completed_at,
                        completed_task_seen_at(session).as_deref(),
                    )
                });
        worktrees[worktree_index].sessions.push(sidebar_session(
            session,
            has_running_task,
            has_completed_task,
        ));
    }

    for worktree in &mut worktrees {
        worktree.sessions.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| a.created_at.cmp(&b.created_at))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    let active_session_id = active_session_id.filter(|active_id| {
        worktrees.iter().any(|worktree| {
            worktree
                .sessions
                .iter()
                .any(|session| &session.id == active_id)
        })
    });

    Ok(UiSessionTree {
        worktrees,
        active_session_id,
    })
}
