// ── 归属扩展：navis-session ──
// 迁移目标：extensions/navis-session/ExtensionBackend/src/

//! Session UI metadata projection helpers.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use serde_json::{json, Value};

// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use crate::security::sandbox::permission::ApprovalMode;

pub(crate) const DEFAULT_SESSION_WORKTREE: &str = "Sessions";
pub(crate) const UI_METADATA_KEY: &str = "ui";
pub(crate) const UI_WORKTREE_KEY: &str = "worktree";
pub(crate) const UI_MODE_KEY: &str = "mode";
pub(crate) const UI_PINNED_KEY: &str = "pinned";
pub(crate) const UI_UNREAD_KEY: &str = "unread";
pub(crate) const UI_TASK_COMPLETED_SEEN_AT_KEY: &str = "taskCompletedSeenAt";
pub(crate) const UI_PERMISSION_POLICY_KEY: &str = "permissionPolicy";
pub(crate) const UI_TRANSCRIPT_VIEW_KEY: &str = "transcriptView";
pub(crate) const UI_REASONING_EFFORT_KEY: &str = "reasoningEffort";

pub(crate) fn session_display_name(session: &Session) -> String {
    session
        .name
        .clone()
        .unwrap_or_else(|| "Untitled session".to_string())
}

pub(crate) fn ui_metadata(session: &Session) -> Option<&Value> {
    session.metadata.as_ref()?.get(UI_METADATA_KEY)
}

pub(crate) fn ui_string(session: &Session, key: &str) -> Option<String> {
    ui_metadata(session)?.get(key)?.as_str().map(str::to_string)
}

pub(crate) fn ui_bool(session: &Session, key: &str) -> bool {
    ui_metadata(session)
        .and_then(|ui| ui.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn normalize_rfc3339_timestamp(value: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc).to_rfc3339())
}

pub(crate) fn project_worktree_name(worktree_root: &str) -> String {
    Path::new(worktree_root)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(DEFAULT_SESSION_WORKTREE)
        .to_string()
}

pub(crate) fn session_worktree_name(session: &Session) -> String {
    ui_string(session, UI_WORKTREE_KEY)
        .or_else(|| session.worktree_root.as_deref().map(project_worktree_name))
        .unwrap_or_else(|| DEFAULT_SESSION_WORKTREE.to_string())
}

pub(crate) fn session_mode(session: &Session) -> Option<String> {
    ui_string(session, UI_MODE_KEY)
}

pub(crate) fn session_transcript_view(session: &Session) -> String {
    match ui_string(session, UI_TRANSCRIPT_VIEW_KEY).as_deref() {
        Some("compact") => "compact".to_string(),
        Some("raw") => "raw".to_string(),
        _ => "standard".to_string(),
    }
}

pub(crate) fn session_reasoning_effort(session: &Session) -> String {
    match ui_string(session, UI_REASONING_EFFORT_KEY).as_deref() {
        Some("low") => "low".to_string(),
        Some("medium") => "medium".to_string(),
        Some("extra-high") => "extra-high".to_string(),
        Some("max") => "max".to_string(),
        _ => "high".to_string(),
    }
}

pub(crate) fn session_permission_policy(session: &Session) -> Option<String> {
    ui_string(session, UI_PERMISSION_POLICY_KEY)
        .and_then(|value| ApprovalMode::from_str(&value))
        .map(|mode| mode.as_str().to_string())
}

pub(crate) fn merge_ui_metadata(
    manager: &SessionManager,
    session_id: &str,
    changes: Vec<(&str, Value)>,
) -> Result<(), String> {
    let session = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;

    let mut metadata = session.metadata.unwrap_or_else(|| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }

    let root = metadata
        .as_object_mut()
        .ok_or_else(|| "会话元数据格式错误".to_string())?;
    let ui = root
        .entry(UI_METADATA_KEY.to_string())
        .or_insert_with(|| json!({}));

    if !ui.is_object() {
        *ui = json!({});
    }

    let ui_object = ui
        .as_object_mut()
        .ok_or_else(|| "会话 UI 元数据格式错误".to_string())?;

    for (key, value) in changes {
        ui_object.insert(key.to_string(), value);
    }

    manager
        .update_metadata(session_id, Some(metadata))
        .map_err(|error| error.to_string())
}

pub(crate) fn main_session_task_flags(
    task_manager: &Mutex<TaskManager>,
) -> (HashSet<String>, HashMap<String, chrono::DateTime<Utc>>) {
    task_manager
        .lock()
        .map(|tasks| {
            let mut running_sessions = HashSet::new();
            let mut completed_sessions = HashMap::new();
            for task in tasks.list_tasks() {
                if !task.status.is_terminal() {
                    running_sessions.insert(task.session_id.clone());
                } else if matches!(task.status, TaskStatus::Completed) {
                    if let Some(completed_at) = task.completed_at {
                        completed_sessions
                            .entry(task.session_id.clone())
                            .and_modify(|current| {
                                if completed_at > *current {
                                    *current = completed_at;
                                }
                            })
                            .or_insert(completed_at);
                    }
                }
            }
            (running_sessions, completed_sessions)
        })
        .unwrap_or_default()
}

pub(crate) fn completed_task_seen_at(session: &Session) -> Option<String> {
    ui_string(session, UI_TASK_COMPLETED_SEEN_AT_KEY)
}

pub(crate) fn task_completed_after_seen_at(
    completed_at: chrono::DateTime<Utc>,
    seen_at: Option<&str>,
) -> bool {
    let Some(seen_at) = seen_at else {
        return true;
    };
    chrono::DateTime::parse_from_rfc3339(seen_at)
        .map(|seen_at| completed_at > seen_at.with_timezone(&Utc))
        .unwrap_or(true)
}
