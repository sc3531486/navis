// ── 归属扩展：navis-agent-core ──
// 迁移目标：extensions/navis-agent-core/ExtensionBackend/src/

use super::dto::{UiChatMessageAttachment, UiComposerRunState, UiComposerTask};
use super::session_composer_run_state;
// use [REMOVED: domains reference]
    ComposerAttachment, ComposerRuntime, ComposerTask as RuntimeComposerTask,
};
// use [REMOVED: domains reference]
use std::sync::Mutex;

pub(crate) fn composer_run_state_projection(
    manager: &SessionManager,
    composer_runtime: &Mutex<ComposerRuntime>,
    session_id: &str,
) -> Result<UiComposerRunState, String> {
    let session = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;
    let mut state = session_composer_run_state(&session);
    overlay_composer_runtime_state(composer_runtime, &mut state)?;
    Ok(state)
}

pub(crate) fn overlay_composer_runtime_state(
    composer_runtime: &Mutex<ComposerRuntime>,
    state: &mut UiComposerRunState,
) -> Result<(), String> {
    let runtime = composer_runtime
        .lock()
        .map_err(|_| "Composer runtime is unavailable".to_string())?;
    state.running_task = runtime
        .running_task(&state.session_id)
        .map(ui_composer_task);
    state.queued_tasks = runtime
        .queued_tasks(&state.session_id)
        .into_iter()
        .map(ui_composer_task)
        .collect();
    Ok(())
}

pub(crate) fn runtime_composer_task(task: UiComposerTask) -> RuntimeComposerTask {
    RuntimeComposerTask {
        id: task.id,
        kind: task.kind.unwrap_or_else(|| "prompt".to_string()),
        text: task.text,
        source_text: task.source_text,
        display_text: task.display_text,
        attachments: task
            .attachments
            .into_iter()
            .map(runtime_composer_attachment)
            .collect(),
        created_at: task.created_at,
    }
}

fn runtime_composer_attachment(attachment: UiChatMessageAttachment) -> ComposerAttachment {
    ComposerAttachment {
        kind: attachment.kind,
        name: attachment.name,
        mime_type: attachment.mime_type,
        size_bytes: attachment.size_bytes,
        data_base64: attachment.data_base64,
        text_content: attachment.text_content,
        is_truncated: attachment.is_truncated,
        model_readable: attachment.model_readable,
    }
}

pub(crate) fn ui_composer_task(task: RuntimeComposerTask) -> UiComposerTask {
    UiComposerTask {
        id: task.id,
        kind: Some(task.kind),
        text: task.text,
        source_text: task.source_text,
        display_text: task.display_text,
        attachments: task
            .attachments
            .into_iter()
            .map(ui_composer_attachment)
            .collect(),
        created_at: task.created_at,
    }
}

fn ui_composer_attachment(attachment: ComposerAttachment) -> UiChatMessageAttachment {
    UiChatMessageAttachment {
        kind: attachment.kind,
        name: attachment.name,
        mime_type: attachment.mime_type,
        size_bytes: attachment.size_bytes,
        data_base64: attachment.data_base64,
        text_content: attachment.text_content,
        is_truncated: attachment.is_truncated,
        model_readable: attachment.model_readable,
    }
}
