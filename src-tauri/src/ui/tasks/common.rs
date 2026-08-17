// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use crate::domains::agent_core::agent::{TaskKind, TaskManager};
use crate::domains::session::session::SessionManager;
use std::sync::Mutex;

pub(super) fn ensure_session_exists(
    manager: &SessionManager,
    session_id: &str,
) -> Result<(), String> {
    manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .map(|_| ())
        .ok_or_else(|| format!("会话不存在: {}", session_id))
}

pub(super) fn has_active_session_task(
    task_manager: &Mutex<TaskManager>,
    session_id: &str,
) -> Result<bool, String> {
    let tasks = task_manager
        .lock()
        .map_err(|_| "Task state is unavailable".to_string())?;
    Ok(tasks
        .get_tasks_by_session(session_id)
        .into_iter()
        .any(|task| task.kind == TaskKind::Turn && !task.status.is_terminal()))
}
