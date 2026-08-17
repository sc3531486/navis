// ── 归属扩展：navis-project ──
// 迁移目标：extensions/navis-project/ExtensionBackend/src/

use super::dto::{
    UiRecentWorktree, UiSessionWorktreeSnapshot, UiWorktree, UiWorktreeFileDocument,
    UiWorktreeFileNode,
};
use crate::foundation::config::Config;
// use [REMOVED: domains reference]
// use [REMOVED: domains reference]
use crate::extension::types::PathManager;
// use [REMOVED: domains reference]
    build_worktree_tree_with_policy_engine, read_worktree_text_file_with_policy_engine,
    resolve_worktree_root, write_worktree_text_file_with_policy_engine,
    WorktreeFileNode as FileWorktreeNode, DEFAULT_MAX_READ_BYTES, DEFAULT_MAX_TREE_ENTRIES,
};
// use [REMOVED: MCP reference]
use chrono::Utc;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

const PROJECT_RECENT_WORKTREES_KEY: &str = "project.recentWorktrees";

fn ui_recent_worktree(worktree: &RecentWorktree) -> UiRecentWorktree {
    UiRecentWorktree {
        id: worktree.id.clone(),
        name: worktree.name.clone(),
        path: worktree.root.to_string_lossy().to_string(),
        opened_at: worktree.last_opened.timestamp_millis(),
    }
}

fn ui_recent_worktrees(projects: &ProjectManager, limit: usize) -> Vec<UiRecentWorktree> {
    projects
        .list_recent_worktrees(Some(limit))
        .into_iter()
        .map(ui_recent_worktree)
        .collect()
}

fn persist_recent_worktrees(
    projects: &ProjectManager,
    config: &Arc<Mutex<Config>>,
) -> Result<(), String> {
    let serialized = projects
        .save_recent_worktrees()
        .map_err(|error| error.to_string())?;
    let value = serde_json::from_str::<Value>(&serialized).map_err(|error| error.to_string())?;
    let mut config = config.lock().map_err(|error| error.to_string())?;
    config
        .set(PROJECT_RECENT_WORKTREES_KEY, value)
        .map_err(|error| error.to_string())?;
    config.save_user_config().map_err(|error| error.to_string())
}

fn worktree_name(path: &Path) -> String {
    let name = PathManager::file_name_str(path);
    if name.is_empty() {
        path.to_string_lossy().to_string()
    } else {
        name
    }
}

fn ui_worktree(path: &Path) -> UiWorktree {
    let display = path.to_string_lossy().to_string();
    UiWorktree {
        id: display.clone(),
        name: worktree_name(path),
        path: display,
        opened_at: Utc::now().timestamp_millis(),
    }
}

fn ui_worktree_file_node(node: FileWorktreeNode) -> UiWorktreeFileNode {
    UiWorktreeFileNode {
        name: node.name,
        relative_path: node.relative_path,
        absolute_path: node.absolute_path,
        is_directory: node.is_directory,
        children: node
            .children
            .into_iter()
            .map(ui_worktree_file_node)
            .collect(),
        extension: node.extension,
    }
}

pub(crate) fn session_worktree_root(
    manager: &SessionManager,
    session_id: &str,
) -> Result<Option<PathBuf>, String> {
    let session = manager
        .get(session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", session_id))?;
    let Some(worktree_root) = session
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
    else {
        return Ok(None);
    };

    let root = resolve_worktree_root(worktree_root).map_err(|error| error.to_string())?;
    Ok(Some(root))
}

fn session_worktree_snapshot(
    manager: &SessionManager,
    mcp: &MCP,
    policy: &crate::kernel::PolicyEngine,
    session_id: &str,
) -> Result<UiSessionWorktreeSnapshot, String> {
    let Some(root) = session_worktree_root(manager, session_id)? else {
        return Ok(UiSessionWorktreeSnapshot {
            session_id: session_id.to_string(),
            worktree: None,
            worktree_files: Vec::new(),
            file_tree: Vec::new(),
        });
    };

    let (file_tree, worktree_files) = build_worktree_tree_with_policy_engine(
        policy,
        mcp.sandbox(),
        "ui",
        Some(session_id),
        &root,
        DEFAULT_MAX_TREE_ENTRIES,
    )
    .map_err(|error| error.to_string())?;

    Ok(UiSessionWorktreeSnapshot {
        session_id: session_id.to_string(),
        worktree: Some(ui_worktree(&root)),
        worktree_files,
        file_tree: file_tree.into_iter().map(ui_worktree_file_node).collect(),
    })
}

#[tauri::command]
pub fn ui_get_session_worktree_snapshot(
    manager: State<'_, Arc<SessionManager>>,
    mcp: State<'_, Arc<MCP>>,
    policy: State<'_, Arc<crate::kernel::PolicyEngine>>,
    payload: super::SessionIdPayload,
) -> Result<UiSessionWorktreeSnapshot, String> {
    session_worktree_snapshot(
        manager.inner().as_ref(),
        mcp.inner().as_ref(),
        policy.inner().as_ref(),
        &payload.session_id,
    )
}

#[tauri::command]
pub fn ui_read_session_worktree_file(
    manager: State<'_, Arc<SessionManager>>,
    mcp: State<'_, Arc<MCP>>,
    policy: State<'_, Arc<crate::kernel::PolicyEngine>>,
    payload: super::SessionWorktreeFilePayload,
) -> Result<UiWorktreeFileDocument, String> {
    let root = session_worktree_root(manager.inner().as_ref(), &payload.session_id)?
        .ok_or_else(|| "当前会话未绑定 worktree".to_string())?;
    let relative_path = payload.relative_path.trim();
    if relative_path.is_empty() {
        return Err("relativePath 不能为空".to_string());
    }

    let (resolved, content) = read_worktree_text_file_with_policy_engine(
        policy.inner().as_ref(),
        mcp.sandbox(),
        "ui",
        Some(&payload.session_id),
        &root,
        relative_path,
        DEFAULT_MAX_READ_BYTES,
        false,
    )
    .map_err(|error| error.to_string())?;

    Ok(UiWorktreeFileDocument {
        session_id: payload.session_id,
        relative_path: PathManager::relative(&resolved, &root)
            .to_string_lossy()
            .replace('\\', "/"),
        absolute_path: resolved.to_string_lossy().to_string(),
        file_name: worktree_name(&resolved),
        extension: PathManager::extension(&resolved),
        content,
    })
}

#[tauri::command]
pub fn ui_write_session_worktree_file(
    manager: State<'_, Arc<SessionManager>>,
    mcp: State<'_, Arc<MCP>>,
    policy: State<'_, Arc<crate::kernel::PolicyEngine>>,
    payload: super::WriteSessionWorktreeFilePayload,
) -> Result<UiWorktreeFileDocument, String> {
    let root = session_worktree_root(manager.inner().as_ref(), &payload.session_id)?
        .ok_or_else(|| "当前会话未绑定 worktree".to_string())?;
    let relative_path = payload.relative_path.trim();
    if relative_path.is_empty() {
        return Err("relativePath 不能为空".to_string());
    }

    let resolved = write_worktree_text_file_with_policy_engine(
        policy.inner().as_ref(),
        mcp.sandbox(),
        "ui",
        Some(&payload.session_id),
        &root,
        relative_path,
        &payload.content,
        false,
    )
    .map_err(|error| error.to_string())?;

    Ok(UiWorktreeFileDocument {
        session_id: payload.session_id,
        relative_path: PathManager::relative(&resolved, &root)
            .to_string_lossy()
            .replace('\\', "/"),
        absolute_path: resolved.to_string_lossy().to_string(),
        file_name: worktree_name(&resolved),
        extension: PathManager::extension(&resolved),
        content: payload.content,
    })
}

#[tauri::command]
pub fn ui_list_recent_worktrees(
    project_manager: State<'_, Arc<Mutex<ProjectManager>>>,
    payload: super::ListRecentWorktreesPayload,
) -> Result<Vec<UiRecentWorktree>, String> {
    let limit = payload.limit.unwrap_or(10).clamp(1, 50);
    let projects = project_manager.lock().map_err(|error| error.to_string())?;
    Ok(ui_recent_worktrees(&projects, limit))
}

#[tauri::command]
pub fn ui_record_recent_worktree(
    project_manager: State<'_, Arc<Mutex<ProjectManager>>>,
    config: State<'_, Arc<Mutex<Config>>>,
    payload: super::RecordRecentWorktreePayload,
) -> Result<Vec<UiRecentWorktree>, String> {
    let path = payload.path.trim();
    if path.is_empty() {
        return Err("Worktree 路径不能为空".to_string());
    }

    let limit = payload.limit.unwrap_or(10).clamp(1, 50);
    let mut projects = project_manager.lock().map_err(|error| error.to_string())?;
    projects.record_recent_directory(Path::new(path));
    persist_recent_worktrees(&projects, config.inner())?;
    Ok(ui_recent_worktrees(&projects, limit))
}

#[tauri::command]
pub fn ui_remove_recent_worktree(
    project_manager: State<'_, Arc<Mutex<ProjectManager>>>,
    config: State<'_, Arc<Mutex<Config>>>,
    payload: super::RemoveRecentWorktreePayload,
) -> Result<Vec<UiRecentWorktree>, String> {
    let path = payload.path.trim();
    if path.is_empty() {
        return Err("Worktree 路径不能为空".to_string());
    }

    let limit = payload.limit.unwrap_or(10).clamp(1, 50);
    let mut projects = project_manager.lock().map_err(|error| error.to_string())?;
    projects.remove_from_recent(path);
    persist_recent_worktrees(&projects, config.inner())?;
    Ok(ui_recent_worktrees(&projects, limit))
}
