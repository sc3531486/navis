// ── 归属扩展：navis-terminal ──
// 迁移目标：extensions/navis-terminal/ExtensionBackend/src/

use crate::domains::terminal::terminal::TerminalManager;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{ipc::Channel, State};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePtyPayload {
    pub session_id: String,
    pub shell: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyWritePayload {
    pub pty_id: String,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyResizePayload {
    pub pty_id: String,
    pub cols: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyClosePayload {
    pub session_id: String,
    pub pty_id: String,
}

#[tauri::command]
pub async fn ui_terminal_create_pty(
    terminal: State<'_, Arc<TerminalManager>>,
    payload: CreatePtyPayload,
    channel: Channel,
) -> Result<serde_json::Value, String> {
    let terminal_id = terminal
        .inner()
        .clone()
        .create_pty(
            &payload.session_id,
            payload.shell.as_deref(),
            payload.cwd.map(PathBuf::from),
            channel,
        )
        .await?;

    Ok(serde_json::json!({
        "ptyId": terminal_id,
        "sessionId": payload.session_id,
    }))
}

#[tauri::command]
pub async fn ui_terminal_write_pty(
    terminal: State<'_, Arc<TerminalManager>>,
    payload: PtyWritePayload,
) -> Result<(), String> {
    terminal
        .inner()
        .write_to_pty(&payload.pty_id, &payload.data)
        .await
}

#[tauri::command]
pub async fn ui_terminal_resize_pty(
    terminal: State<'_, Arc<TerminalManager>>,
    payload: PtyResizePayload,
) -> Result<(), String> {
    terminal
        .inner()
        .resize_terminal(&payload.pty_id, payload.cols, payload.rows)
}

#[tauri::command]
pub async fn ui_terminal_close_pty(
    terminal: State<'_, Arc<TerminalManager>>,
    payload: PtyClosePayload,
) -> Result<(), String> {
    terminal
        .inner()
        .close_pty(&payload.session_id, &payload.pty_id)
        .await
}
