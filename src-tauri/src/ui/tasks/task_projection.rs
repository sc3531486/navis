// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use crate::domains::agent_core::agent::{TaskKind, TaskRecord, TaskStatus, TodoItem, TodoStatus};
use crate::foundation::status::StatusClassify;
use crate::domains::session::session::Session;
use crate::ui::dto::{UiTask, UiTodoItem};
use crate::ui::ui_metadata;
use serde_json::{json, Value};

pub(crate) fn todo_items_from_value(value: &Value) -> Vec<TodoItem> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let content = item.get("content")?.as_str()?.trim();
                    if content.is_empty() {
                        return None;
                    }
                    Some(TodoItem {
                        id: item
                            .get("id")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                        content: content.to_string(),
                        status: TodoStatus::from_str(
                            item.get("status")
                                .and_then(Value::as_str)
                                .unwrap_or("pending"),
                        ),
                        priority: item
                            .get("priority")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn todo_items_from_values(values: &[Value]) -> Vec<TodoItem> {
    todo_items_from_value(&Value::Array(values.to_vec()))
}

pub(crate) fn todo_values(todos: &[TodoItem]) -> Vec<Value> {
    todos
        .iter()
        .cloned()
        .map(ui_todo_item)
        .map(|todo| json!(todo))
        .collect()
}

fn task_status_error(status: &TaskStatus) -> Option<String> {
    status.error().map(str::to_string)
}

fn latest_task_message(task: &TaskRecord) -> Option<String> {
    task.messages
        .last()
        .map(|message| message.content.trim().to_string())
        .filter(|content| !content.is_empty())
}

pub(crate) fn ui_task_record(task: &TaskRecord) -> UiTask {
    UiTask {
        id: task.id.clone(),
        session_id: task.session_id.clone(),
        parent_task_id: task.parent_task_id.clone(),
        sidechain_session_id: task.sidechain_session_id.clone(),
        kind: task.kind.as_str().to_string(),
        owner: task.owner.clone(),
        active_form: task.active_form.clone(),
        blocks: task.blocks.clone(),
        blocked_by: task.blocked_by.clone(),
        status: task.status.as_str().to_string(),
        status_presentation: task.status.status_presentation(),
        description: crate::domains::agent_core::agent::task_description(task),
        error: task_status_error(&task.status),
        created_at: task.created_at.to_rfc3339(),
        completed_at: task
            .completed_at
            .map(|completed_at| completed_at.to_rfc3339()),
        duration_ms: task.duration_ms(),
        message_count: task.messages.len(),
        tool_call_count: task.tool_calls.len(),
        latest_tool_name: task.tool_calls.last().map(|tool| tool.tool_name.clone()),
        token_count: task.token_count,
        latest_message: latest_task_message(task).or_else(|| {
            task.result
                .as_ref()
                .map(|result| result.chars().take(240).collect::<String>())
        }),
        current_activity: task.current_activity.clone(),
        result: task.result.clone(),
    }
}

pub(crate) fn is_background_task_projection(task: &TaskRecord) -> bool {
    matches!(
        task.kind,
        TaskKind::Sidechain | TaskKind::Parallel | TaskKind::Background | TaskKind::Autonomous
    )
}

pub(crate) fn ui_todo_item(todo: TodoItem) -> UiTodoItem {
    UiTodoItem {
        id: todo.id,
        content: todo.content,
        status: todo.status.as_str().to_string(),
        status_presentation: todo.status.status_presentation(),
        priority: todo.priority,
    }
}

pub(crate) fn session_todos_from_metadata(session: &Session) -> Vec<TodoItem> {
    ui_metadata(session)
        .and_then(|ui| ui.get("todos"))
        .map(todo_items_from_value)
        .unwrap_or_default()
}
