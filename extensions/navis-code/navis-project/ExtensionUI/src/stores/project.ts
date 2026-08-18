import { invoke } from '@tauri-apps/api/core';

import { createStore } from 'solid-js/store';

const MAX_RECENT_WORKTREES = 10;

// ── 类型定义 ────────────────────────────────────────────

/**
 * 最近 Worktree 描述。
 */
export interface RecentWorktree {
  /** Worktree 唯一 ID */
  id: string;
  /** Worktree 名称 */
  name: string;
  /** Worktree 根目录的绝对路径 */
  path: string;
  /** Worktree 最后绑定时间戳 */
  openedAt: number;
  /** Worktree 描述（可选） */
  description?: string;
  /** Git 分支名（可选） */
  gitBranch?: string;
  /** 是否为收藏的 Worktree */
  starred?: boolean;
}

/**
 * 项目状态接口
 */
export interface ProjectState {
  /** 当前打开的项目 ID */
  currentProjectId: string | null;
  /** 最近使用的 Worktree 列表（按绑定时间倒序） */
  recentWorktrees: RecentWorktree[];
  /** 最近目录是否已经从后端加载 */
  recentWorktreesLoaded: boolean;
}

/** 项目状态默认值 */
const defaultProjectState: ProjectState = {
  currentProjectId: null,
  recentWorktrees: [],
  recentWorktreesLoaded: false,
};

// ── Store 实例 ──────────────────────────────────────────

/**
 * 项目状态 store。
 *
 * @example
 * ```tsx
 * import { projectState, setCurrentProject } from '@project-ext/stores/project';
 *
 * // 显示当前项目 ID
 * <span>{projectState.currentProjectId ?? '未打开项目'}</span>
 *
 * // 设置当前项目
 * setCurrentProject('project-id-123');
 * ```
 */
export const [projectState, setProjectState] = createStore<ProjectState>({
  ...defaultProjectState,
});

function applyRecentWorktrees(worktrees: RecentWorktree[]): void {
  setProjectState({
    recentWorktrees: worktrees.slice(0, MAX_RECENT_WORKTREES),
    recentWorktreesLoaded: true,
  });
}

// ── 便捷操作函数 ────────────────────────────────────────

/**
 * 设置当前项目 ID。
 *
 * @param projectId - 项目 ID，null 表示关闭当前项目
 */
export function setCurrentProject(projectId: string | null): void {
  setProjectState('currentProjectId', projectId);
}

/**
 * 从后端加载最近打开过的目录。
 */
export async function loadRecentWorktrees(limit = MAX_RECENT_WORKTREES): Promise<void> {
  const worktrees = await invoke<RecentWorktree[]>('ui_list_recent_worktrees', {
    payload: { limit },
  });
  applyRecentWorktrees(worktrees);
}

/**
 * 记录最近打开过的目录，并刷新最近目录菜单。
 */
export async function addRecentWorktree(worktree: Pick<RecentWorktree, 'path'>): Promise<void> {
  const worktrees = await invoke<RecentWorktree[]>('ui_record_recent_worktree', {
    payload: { path: worktree.path, limit: MAX_RECENT_WORKTREES },
  });
  applyRecentWorktrees(worktrees);
}

/**
 * 从最近目录列表中移除一个目录。
 *
 * @param worktreePath - 要移除的目录路径
 */
export async function removeRecentBoundWorktree(worktreePath: string): Promise<void> {
  const worktrees = await invoke<RecentWorktree[]>('ui_remove_recent_worktree', {
    payload: { path: worktreePath, limit: MAX_RECENT_WORKTREES },
  });
  applyRecentWorktrees(worktrees);
}

/**
 * 重置项目状态。
 */
export function resetProjectState(): void {
  setProjectState({ ...defaultProjectState });
}
