// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

//! UI runtime adapters for Agent application ports.
//!
//! Concrete task/session storage and UI projection stay here. The tool domain
//! receives only the application runtime contracts.

use super::agent_tool_loop::AgentBackend;
use super::sidechain_task::start_sidechain_agent_task;
use crate::domains::agent_core::agent::{sidechain_stop_requested, TaskManager, TaskRecord, TaskStatus};
use crate::domains::agent_core::application::runtime::{
    AgentControlPorts, SidechainPort, SidechainReadRequest, SidechainStartRequest,
    SidechainStarted, SidechainStatus, SidechainTaskSnapshot, TodoPort, TodoUpdate,
    TodoUpdateRequest,
};
use crate::domains::session::session::SessionManager;
use crate::domains::agent_core::tool_runtime::special::SpecialAgentToolHost;
use crate::ui::merge_ui_metadata;
use crate::ui::tasks::{todo_items_from_values, todo_values};
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct UiAgentControlPort {
    manager: Arc<SessionManager>,
    task_manager: Arc<Mutex<TaskManager>>,
    backend: AgentBackend,
}

impl UiAgentControlPort {
    fn new(backend: &AgentBackend) -> Self {
        Self {
            manager: backend.manager.clone(),
            task_manager: backend.task_manager.clone(),
            backend: backend.clone(),
        }
    }

    fn project_task_snapshot(task: &TaskRecord) -> SidechainTaskSnapshot {
        let status = match task.status {
            TaskStatus::Pending => SidechainStatus::Pending,
            TaskStatus::Running => SidechainStatus::Running,
            TaskStatus::WaitingConfirm => SidechainStatus::WaitingPermission,
            TaskStatus::Completed => SidechainStatus::Completed,
            TaskStatus::Failed { .. } => SidechainStatus::Failed,
            TaskStatus::Cancelled => SidechainStatus::Cancelled,
            TaskStatus::Blocked => SidechainStatus::Pending,
        };
        SidechainTaskSnapshot {
            task_id: task.id.clone(),
            sidechain_session_id: task.sidechain_session_id.clone().unwrap_or_default(),
            status,
            description: task.description.clone(),
            current_activity: task.current_activity.clone(),
            summary: task
                .outcome
                .as_ref()
                .map(|outcome| outcome.summary.trim())
                .filter(|summary| !summary.is_empty())
                .or_else(|| {
                    task.result
                        .as_deref()
                        .map(str::trim)
                        .filter(|result| !result.is_empty())
                })
                .map(str::to_string),
            structured_output: task
                .outcome
                .as_ref()
                .map(|outcome| outcome.structured_output.clone()),
            tool_call_count: task.tool_calls.len() as u64,
            token_count: task.token_count.max(0) as u64,
            duration_ms: task.duration_ms(),
            error: task.status.error().map(str::to_string),
        }
    }

    fn find_task_snapshot(&self, task_id: &str) -> Result<Option<SidechainTaskSnapshot>> {
        let tasks = self
            .task_manager
            .lock()
            .map_err(|_| anyhow!("后台任务状态不可用"))?;
        Ok(tasks
            .get_sidechain_task(task_id)
            .map(Self::project_task_snapshot))
    }
}

fn update_todos(
    manager: &SessionManager,
    task_manager: &Mutex<TaskManager>,
    request: TodoUpdateRequest,
) -> Result<TodoUpdate> {
    let new_todos = todo_items_from_values(&request.todos);
    let old_todos = task_manager
        .lock()
        .map_err(|_| anyhow!("Todo 状态不可用"))?
        .get_todos(&request.session_id);
    let persisted_todos = todo_values(&new_todos);

    merge_ui_metadata(
        manager,
        &request.session_id,
        vec![("todos", Value::Array(persisted_todos.clone()))],
    )
    .map_err(|error| anyhow!(error))?;

    task_manager
        .lock()
        .map_err(|_| anyhow!("Todo 状态不可用"))?
        .set_todos(&request.session_id, new_todos);

    Ok(TodoUpdate {
        previous: todo_values(&old_todos),
        current: persisted_todos,
    })
}

impl TodoPort for UiAgentControlPort {
    fn update(&self, request: TodoUpdateRequest) -> Result<TodoUpdate> {
        update_todos(self.manager.as_ref(), self.task_manager.as_ref(), request)
    }
}

impl SidechainPort for UiAgentControlPort {
    fn start(&self, request: SidechainStartRequest) -> Result<SidechainStarted> {
        start_sidechain_agent_task(
            self.manager.as_ref(),
            self.backend.clone(),
            self.task_manager.as_ref(),
            request,
        )
    }

    fn read(&self, request: SidechainReadRequest) -> Result<Option<SidechainTaskSnapshot>> {
        let deadline = Instant::now() + Duration::from_millis(request.timeout_ms);
        loop {
            let snapshot = self.find_task_snapshot(&request.task_id)?;
            let Some(snapshot) = snapshot else {
                return Ok(None);
            };
            if !request.wait || snapshot.status.is_terminal() || Instant::now() >= deadline {
                return Ok(Some(snapshot));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn stop(&self, task_id: &str) -> Result<Option<SidechainTaskSnapshot>> {
        let should_notify = {
            let mut tasks = self
                .task_manager
                .lock()
                .map_err(|_| anyhow!("后台任务状态不可用"))?;
            let Some(task) = tasks.get_sidechain_task_mut(task_id) else {
                return Ok(None);
            };
            if task.status.is_terminal() {
                false
            } else {
                task.mark_cancelled();
                true
            }
        };

        if should_notify {
            crate::domains::agent_core::agent::notify_parent_sidechain_task(self.task_manager.as_ref(), task_id)
                .map_err(|error| anyhow!(error))?;
        }
        self.find_task_snapshot(task_id)
    }

    fn stop_requested(&self, task_id: &str) -> Result<bool> {
        Ok(sidechain_stop_requested(
            Some(self.task_manager.as_ref()),
            Some(task_id),
        ))
    }
}

pub(crate) fn build_agent_control_host(backend: &AgentBackend) -> SpecialAgentToolHost {
    let adapter = Arc::new(UiAgentControlPort::new(backend));
    let runtime = Arc::new(AgentControlPorts::new(adapter.clone(), adapter));
    SpecialAgentToolHost::with_runtime(runtime)
}

#[cfg(test)]
mod tests {
    use super::UiAgentControlPort;
    use crate::domains::agent_core::agent::{SidechainOutcome, TaskManager, TaskRecord};
    use crate::domains::agent_core::application::runtime::SidechainStatus;
    use serde_json::json;

    fn sidechain_task() -> TaskRecord {
        TaskRecord::new_sidechain(
            "task-1",
            "session-1",
            "sidechain-session-1",
            "Inspect the repository",
            "Inspect the repository",
        )
    }

    #[test]
    fn running_sidechain_snapshot_is_not_ready() {
        let mut task = sidechain_task();
        task.mark_running_with_activity("Reading files");
        let snapshot = UiAgentControlPort::project_task_snapshot(&task);
        assert_eq!(snapshot.status, SidechainStatus::Running);
        assert_eq!(snapshot.current_activity.as_deref(), Some("Reading files"));
    }

    #[test]
    fn completed_sidechain_snapshot_preserves_structured_result() {
        let mut task = sidechain_task();
        task.mark_completed_with_outcome(
            SidechainOutcome::new("Found the relevant module", json!({"files": 3})),
            12,
        );
        let snapshot = UiAgentControlPort::project_task_snapshot(&task);
        assert_eq!(snapshot.status, SidechainStatus::Completed);
        assert_eq!(snapshot.structured_output.unwrap()["files"], 3);
        assert_eq!(snapshot.token_count, 12);
    }

    #[test]
    fn failed_and_cancelled_sidechains_expose_terminal_status() {
        let mut failed = sidechain_task();
        failed.mark_failed("provider unavailable");
        let failed_snapshot = UiAgentControlPort::project_task_snapshot(&failed);
        assert_eq!(failed_snapshot.status, SidechainStatus::Failed);
        assert_eq!(
            failed_snapshot.error.as_deref(),
            Some("provider unavailable")
        );

        let mut cancelled = sidechain_task();
        cancelled.mark_cancelled();
        let cancelled_snapshot = UiAgentControlPort::project_task_snapshot(&cancelled);
        assert_eq!(cancelled_snapshot.status, SidechainStatus::Cancelled);
    }

    #[test]
    fn task_manager_snapshot_counts_tool_calls() {
        let task = sidechain_task();
        let snapshot = UiAgentControlPort::project_task_snapshot(&task);
        assert_eq!(snapshot.tool_call_count, 0);
        let _ = TaskManager::new();
    }
}
