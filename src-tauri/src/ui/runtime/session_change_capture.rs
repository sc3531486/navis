// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use crate::extension::types::ToolCall;
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
    relative_display, resolve_worktree_root, resolve_worktree_write_path,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;

#[derive(Debug, Clone)]
pub(crate) struct PendingSessionChange {
    change_id: String,
    turn_id: String,
    message_id: String,
    call_id: String,
    tool_name: String,
    worktree_path: String,
    relative_path: String,
    absolute_path: String,
    operation: String,
    before_content: Option<String>,
    created_at: String,
}

pub(crate) fn prepare_session_change_capture(
    session: &Session,
    turn_id: Option<&str>,
    assistant_message_id: &str,
    tool_call: &ToolCall,
) -> Result<Option<PendingSessionChange>, String> {
    if !matches!(tool_call.function.name.as_str(), "edit" | "write") {
        return Ok(None);
    }
    let Some(turn_id) = turn_id else {
        return Ok(None);
    };
    let worktree_path = session
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "No worktree is bound to this session.".to_string())?
        .to_string();
    let arguments = tool_call_arguments(tool_call);
    let path = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "edit/write tool call is missing path".to_string())?;
    let root = resolve_worktree_root(&worktree_path).map_err(|error| error.to_string())?;
    let resolved =
        resolve_worktree_write_path(&root, path, false).map_err(|error| error.to_string())?;
    let before_content = if resolved.exists() {
        Some(
            fs::read_to_string(&resolved)
                .map_err(|error| format!("读取变更前文件失败 {}: {}", resolved.display(), error))?,
        )
    } else {
        None
    };
    let operation = if before_content.is_some() {
        "update"
    } else {
        "create"
    };
    let absolute_path = resolved.to_string_lossy().to_string();
    let relative_path = relative_display(&root, &resolved);
    let change_id = format!("change:{}:{}", session.id, tool_call.id);

    Ok(Some(PendingSessionChange {
        change_id,
        turn_id: turn_id.to_string(),
        message_id: assistant_message_id.to_string(),
        call_id: tool_call.id.clone(),
        tool_name: tool_call.function.name.clone(),
        worktree_path,
        relative_path,
        absolute_path,
        operation: operation.to_string(),
        before_content,
        created_at: Utc::now().to_rfc3339(),
    }))
}

pub(crate) fn record_completed_session_change(
    manager: Option<&SessionManager>,
    session_id: &str,
    pending: Option<PendingSessionChange>,
    part_id: Option<String>,
    execution: &AgentToolExecution,
) -> Result<(), String> {
    let (Some(manager), Some(pending)) = (manager, pending) else {
        return Ok(());
    };
    if execution.result.is_error || execution.event.status != "completed" {
        return Ok(());
    }
    let after_content = fs::read_to_string(&pending.absolute_path)
        .map_err(|error| format!("读取变更后文件失败 {}: {}", pending.absolute_path, error))?;
    let output = execution
        .event
        .output
        .as_ref()
        .ok_or_else(|| "edit/write completed without structured output".to_string())?;
    let operation = output
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or(pending.operation.as_str());
    let insertions = output
        .get("insertions")
        .and_then(Value::as_i64)
        .ok_or_else(|| "edit/write output missing insertions".to_string())?;
    let deletions = output
        .get("deletions")
        .and_then(Value::as_i64)
        .ok_or_else(|| "edit/write output missing deletions".to_string())?;
    let diff = output
        .get("diff")
        .and_then(Value::as_str)
        .map(str::to_string);

    manager
        .record_session_change(&SessionChange {
            id: pending.change_id,
            session_id: session_id.to_string(),
            turn_id: pending.turn_id,
            message_id: pending.message_id,
            agent_timeline_part_id: part_id,
            call_id: Some(pending.call_id),
            tool_name: pending.tool_name,
            worktree_path: Some(pending.worktree_path),
            relative_path: Some(pending.relative_path),
            absolute_path: pending.absolute_path,
            operation: operation.to_string(),
            before_content: pending.before_content,
            after_content: Some(after_content),
            diff,
            insertions,
            deletions,
            status: "active".to_string(),
            created_at: pending.created_at,
            reverted_at: None,
            metadata: Some(json!({
                "source": "tool_agent_runtime",
                "outputPath": output.get("path").cloned(),
                "outputAbsolutePath": output.get("absolutePath").cloned(),
            })),
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}
