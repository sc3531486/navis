// ── 归属扩展：navis-task ──
// 迁移目标：extensions/navis-task/ExtensionBackend/src/

use crate::domains::session::session::SessionManager;
use crate::domains::editor::git::diff::GitDiff;
use crate::domains::editor::git::GitStatusParser;
use crate::ui::dto::{
    CreateSessionGitRepoPayload, SessionGitDiffPayload, UiSessionGitDiff, UiSessionGitDiffFile,
};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn ui_get_session_git_diff(
    manager: State<'_, Arc<SessionManager>>,
    payload: SessionGitDiffPayload,
) -> Result<UiSessionGitDiff, String> {
    let manager = manager.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let worktree_root = session
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .ok_or_else(|| "当前会话未绑定 worktree，无法读取 Git diff".to_string())?
        .to_string();
    let staged = payload.staged.unwrap_or(false);
    if !GitStatusParser::is_repo(&worktree_root).await {
        return Ok(UiSessionGitDiff {
            session_id: payload.session_id,
            worktree_root,
            is_repo: false,
            can_create_repo: true,
            staged,
            diff: String::new(),
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            file_changes: Vec::new(),
        });
    }

    let diff = GitDiff::diff(&worktree_root, staged)
        .await
        .map_err(|error| error.to_string())?;
    let stats = GitDiff::format_diff_stats(&diff);
    let file_changes = session_git_diff_files(&worktree_root, &diff, staged).await?;

    Ok(UiSessionGitDiff {
        session_id: payload.session_id,
        worktree_root,
        is_repo: true,
        can_create_repo: false,
        staged,
        diff,
        files_changed: stats.files_changed,
        insertions: stats.insertions,
        deletions: stats.deletions,
        file_changes,
    })
}

#[tauri::command]
pub async fn ui_create_session_git_repo(
    manager: State<'_, Arc<SessionManager>>,
    payload: CreateSessionGitRepoPayload,
) -> Result<UiSessionGitDiff, String> {
    let manager = manager.inner().as_ref();
    let session = manager
        .get(&payload.session_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("会话不存在: {}", payload.session_id))?;
    let worktree_root = session
        .worktree_root
        .as_deref()
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .ok_or_else(|| "当前会话未绑定 worktree，无法创建 Git 仓库".to_string())?
        .to_string();

    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(&worktree_root)
        .arg("init")
        .output()
        .await
        .map_err(|error| format!("无法执行 git init: {}", error))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let diff = GitDiff::diff(&worktree_root, false)
        .await
        .map_err(|error| error.to_string())?;
    let stats = GitDiff::format_diff_stats(&diff);
    let file_changes = session_git_diff_files(&worktree_root, &diff, false).await?;

    Ok(UiSessionGitDiff {
        session_id: payload.session_id,
        worktree_root,
        is_repo: true,
        can_create_repo: false,
        staged: false,
        diff,
        files_changed: stats.files_changed,
        insertions: stats.insertions,
        deletions: stats.deletions,
        file_changes,
    })
}

async fn session_git_diff_files(
    worktree_root: &str,
    diff: &str,
    staged: bool,
) -> Result<Vec<UiSessionGitDiffFile>, String> {
    let mut by_path = parse_diff_file_stats(diff);

    if let Ok(status) = GitStatusParser::status(worktree_root).await {
        for change in status.changes {
            if change.staged != staged {
                continue;
            }
            let path = change.path.to_string_lossy().replace('\\', "/");
            let entry = by_path.entry(path.clone()).or_insert(UiSessionGitDiffFile {
                path,
                status: change_status_label(&change.status).to_string(),
                staged: change.staged,
                insertions: 0,
                deletions: 0,
            });
            entry.status = change_status_label(&change.status).to_string();
            entry.staged = change.staged;
        }
    }

    let mut files = by_path.into_values().collect::<Vec<_>>();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn parse_diff_file_stats(diff: &str) -> HashMap<String, UiSessionGitDiffFile> {
    let mut files = HashMap::new();
    let mut current_path: Option<String> = None;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let path = rest
                .split_whitespace()
                .nth(1)
                .or_else(|| rest.split_whitespace().next())
                .unwrap_or("")
                .trim_start_matches("b/")
                .to_string();
            current_path = if path.is_empty() {
                None
            } else {
                Some(path.clone())
            };
            if let Some(path) = &current_path {
                files.insert(
                    path.clone(),
                    UiSessionGitDiffFile {
                        path: path.clone(),
                        status: "modified".to_string(),
                        staged: false,
                        insertions: 0,
                        deletions: 0,
                    },
                );
            }
            continue;
        }

        let Some(path) = current_path.as_ref() else {
            continue;
        };
        let Some(file) = files.get_mut(path) else {
            continue;
        };
        if line.starts_with("new file mode") {
            file.status = "added".to_string();
        } else if line.starts_with("deleted file mode") {
            file.status = "deleted".to_string();
        } else if line.starts_with("rename from ") || line.starts_with("rename to ") {
            file.status = "renamed".to_string();
        } else if line.starts_with('+') && !line.starts_with("+++") {
            file.insertions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            file.deletions += 1;
        }
    }

    files
}

fn change_status_label(status: &crate::domains::editor::git::ChangeStatus) -> &'static str {
    match status {
        crate::domains::editor::git::ChangeStatus::Added => "added",
        crate::domains::editor::git::ChangeStatus::Modified => "modified",
        crate::domains::editor::git::ChangeStatus::Deleted => "deleted",
        crate::domains::editor::git::ChangeStatus::Renamed { .. } => "renamed",
        crate::domains::editor::git::ChangeStatus::Untracked => "untracked",
    }
}
