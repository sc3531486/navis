// ── 归属扩展：navis-ai-platform ──
// 迁移目标：extensions/navis-ai-platform/ExtensionBackend/src/

//! LSP 的 Tauri IPC 适配器。
//!
//! 这里负责把 UI 的 Session/file payload 转换为 `tool::lsp` 的业务调用，
//! 并将领域诊断投影为前端使用的 DTO。LSP 生命周期和语言服务逻辑仍归
//! `tool::lsp` 管理，避免工具域反向依赖 UI。

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::domains::session::session::SessionManager;
use crate::domains::editor::file::worktree_fs::resolve_worktree_path;
use crate::domains::ai_platform::lsp::diagnostics::{Diagnostic, DiagnosticSeverity};
use crate::domains::ai_platform::lsp::manager::{CompletionItem, DefinitionLocation, HoverInfo, LSPManager};
use crate::ui::worktree::session_worktree_root;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspPositionPayload {
    pub session_id: String,
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LspFilePayload {
    pub session_id: String,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiDiagnostic {
    pub id: String,
    pub file_path: String,
    pub severity: u8,
    pub message: String,
    pub source: String,
    pub code: Option<String>,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

fn resolve_session_lsp_file(
    manager: &SessionManager,
    session_id: &str,
    file_path: &str,
) -> Result<(PathBuf, String), String> {
    let root = session_worktree_root(manager, session_id)?
        .ok_or_else(|| "当前会话未绑定 worktree".to_string())?;
    let resolved =
        resolve_worktree_path(&root, file_path, false).map_err(|error| error.to_string())?;
    if !resolved.is_file() {
        return Err("LSP 文件路径不是普通文件".to_string());
    }
    Ok((root, resolved.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn lsp_completion(
    manager: State<'_, Arc<LSPManager>>,
    sessions: State<'_, Arc<SessionManager>>,
    payload: LspPositionPayload,
) -> Result<Vec<CompletionItem>, String> {
    let (_, file_path) = resolve_session_lsp_file(
        sessions.inner().as_ref(),
        &payload.session_id,
        &payload.file_path,
    )?;
    manager.completion(&file_path, payload.line, payload.character)
}

#[tauri::command]
pub fn lsp_hover(
    manager: State<'_, Arc<LSPManager>>,
    sessions: State<'_, Arc<SessionManager>>,
    payload: LspPositionPayload,
) -> Result<Option<HoverInfo>, String> {
    let (_, file_path) = resolve_session_lsp_file(
        sessions.inner().as_ref(),
        &payload.session_id,
        &payload.file_path,
    )?;
    manager.hover(&file_path, payload.line, payload.character)
}

#[tauri::command]
pub fn lsp_definition(
    manager: State<'_, Arc<LSPManager>>,
    sessions: State<'_, Arc<SessionManager>>,
    payload: LspPositionPayload,
) -> Result<Vec<DefinitionLocation>, String> {
    let (root, file_path) = resolve_session_lsp_file(
        sessions.inner().as_ref(),
        &payload.session_id,
        &payload.file_path,
    )?;
    manager.definition(&file_path, &root, payload.line, payload.character)
}

#[tauri::command]
pub fn lsp_diagnostics(
    manager: State<'_, Arc<LSPManager>>,
    sessions: State<'_, Arc<SessionManager>>,
    payload: LspFilePayload,
) -> Result<Vec<UiDiagnostic>, String> {
    let (_, file_path) = resolve_session_lsp_file(
        sessions.inner().as_ref(),
        &payload.session_id,
        &payload.file_path,
    )?;
    let diagnostics = manager.diagnostics_for_file(&file_path)?;
    Ok(diagnostics
        .into_iter()
        .enumerate()
        .map(|(index, diagnostic)| ui_diagnostic(&file_path, index, diagnostic))
        .collect())
}

#[tauri::command]
pub fn lsp_format(
    manager: State<'_, Arc<LSPManager>>,
    sessions: State<'_, Arc<SessionManager>>,
    payload: LspFilePayload,
) -> Result<Option<String>, String> {
    let (_, file_path) = resolve_session_lsp_file(
        sessions.inner().as_ref(),
        &payload.session_id,
        &payload.file_path,
    )?;
    manager.format_file(&file_path)
}

fn ui_diagnostic(file_path: &str, index: usize, diagnostic: Diagnostic) -> UiDiagnostic {
    UiDiagnostic {
        id: format!(
            "{}:{}:{}:{}",
            file_path, diagnostic.range.start_line, diagnostic.range.start_character, index
        ),
        file_path: file_path.to_string(),
        severity: match diagnostic.severity {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Information => 3,
            DiagnosticSeverity::Hint => 4,
        },
        message: diagnostic.message,
        source: diagnostic.source.unwrap_or_else(|| "lsp".to_string()),
        code: diagnostic.code,
        start_line: diagnostic.range.start_line,
        start_column: diagnostic.range.start_character,
        end_line: diagnostic.range.end_line,
        end_column: diagnostic.range.end_character,
    }
}
