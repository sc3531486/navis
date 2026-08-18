/**
 * ============================================================
 * Navis worktree 状态 - @session/stores/worktree.ts
 * ============================================================
 *
 * 管理当前 session 绑定的项目 worktree，包括：
 *   - 当前 worktree 信息
 *   - 最近使用的 worktree 列表
 *   - worktree 文件列表
 *   - 加载状态
 *
 * worktree 是 project/session 下实际承载文件的目录上下文。
 * ============================================================
 */

import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';

// ── 类型定义 ────────────────────────────────────────────

/**
 * Worktree 描述。
 */
export interface Worktree {
  /** Worktree 唯一 ID */
  id: string;
  /** Worktree 名称（通常为目录名） */
  name: string;
  /** Worktree 根目录的绝对路径 */
  path: string;
  /** Worktree 打开时间戳 */
  openedAt: number;
  /** Worktree 描述（可选） */
  description?: string;
  /** Git 分支名（可选） */
  gitBranch?: string;
  /** 是否为收藏的 worktree */
  starred?: boolean;
}

/**
 * Worktree 文件节点。
 * 用于文件树展示。
 */
export interface WorktreeFileNode {
  /** 文件/目录名 */
  name: string;
  /** 相对于 worktree 根目录的路径 */
  relativePath: string;
  /** 绝对路径 */
  absolutePath: string;
  /** 是否为目录 */
  isDirectory: boolean;
  /** 子节点（仅目录有值） */
  children?: WorktreeFileNode[];
  /** 文件扩展名（仅文件有值） */
  extension?: string;
}

export interface WorktreeFileDocument {
  sessionId: string;
  relativePath: string;
  absolutePath: string;
  fileName: string;
  extension?: string;
  content: string;
}

/**
 * Worktree 状态接口。
 */
export interface WorktreeState {
  /** 当前 session 绑定的 worktree */
  currentWorktree: Worktree | null;
  /** 最近使用的 worktree 列表（按打开时间倒序） */
  recentWorktrees: Worktree[];
  /** 当前 worktree 的文件路径列表（扁平结构，用于快速搜索） */
  worktreeFiles: string[];
  /** 当前 worktree 的文件树（树形结构，用于文件树视图） */
  fileTree: WorktreeFileNode[];
  /** 是否正在加载 worktree 数据 */
  isLoading: boolean;
  /** 是否正在加载文件树 */
  isLoadingFileTree: boolean;
  /** Worktree 错误信息 */
  error: string | null;
}

// ── 默认值 ──────────────────────────────────────────────

/** Worktree 状态默认值 */
const defaultWorktreeState: WorktreeState = {
  currentWorktree: null,
  recentWorktrees: [],
  worktreeFiles: [],
  fileTree: [],
  isLoading: false,
  isLoadingFileTree: false,
  error: null,
};

// ── Store 实例 ──────────────────────────────────────────

/**
 * Worktree 状态 store。
 */
export const [worktreeState, setWorktreeState] = createStore<WorktreeState>({
  ...defaultWorktreeState,
});

interface UiWorktree {
  id: string;
  name: string;
  path: string;
  openedAt: number;
}

interface UiSessionWorktreeSnapshot {
  sessionId: string;
  worktree: UiWorktree | null;
  worktreeFiles: string[];
  fileTree: WorktreeFileNode[];
}

interface UiWorktreeFileDocument {
  sessionId: string;
  relativePath: string;
  absolutePath: string;
  fileName: string;
  extension?: string;
  content: string;
}

function worktreeFromUi(worktree: UiWorktree | null): Worktree | null {
  if (!worktree) return null;
  return {
    id: worktree.id,
    name: worktree.name,
    path: worktree.path,
    openedAt: worktree.openedAt,
  };
}

function documentFromUi(document: UiWorktreeFileDocument): WorktreeFileDocument {
  return {
    sessionId: document.sessionId,
    relativePath: document.relativePath,
    absolutePath: document.absolutePath,
    fileName: document.fileName,
    extension: document.extension,
    content: document.content,
  };
}

function applyWorktreeSnapshot(snapshot: UiSessionWorktreeSnapshot): void {
  setCurrentWorktree(worktreeFromUi(snapshot.worktree));
  setWorktreeFiles(snapshot.worktreeFiles);
  setFileTree(snapshot.fileTree);
  setWorktreeError(null);
}

// ── 便捷操作函数 ────────────────────────────────────────

/**
 * 设置当前 worktree。
 *
 * @param worktree - worktree 信息，null 表示关闭当前 worktree
 */
export function setCurrentWorktree(worktree: Worktree | null): void {
  setWorktreeState('currentWorktree', worktree);
  setWorktreeState('error', null);

  // 同时更新最近使用列表
  if (worktree) {
    setWorktreeState('recentWorktrees', (prev) => {
      const filtered = prev.filter((item) => item.id !== worktree.id);
      return [worktree, ...filtered].slice(0, 20);
    });
  }
}

/**
 * 设置 worktree 文件列表。
 *
 * @param files - 文件路径数组（相对于 worktree 根目录）
 */
export function setWorktreeFiles(files: string[]): void {
  setWorktreeState('worktreeFiles', files);
}

/**
 * 设置文件树。
 *
 * @param tree - 文件树节点数组
 */
export function setFileTree(tree: WorktreeFileNode[]): void {
  setWorktreeState('fileTree', tree);
}

/**
 * 设置 worktree 加载状态。
 */
export function setWorktreeLoading(loading: boolean): void {
  setWorktreeState('isLoading', loading);
}

/**
 * 设置文件树加载状态。
 */
export function setFileTreeLoading(loading: boolean): void {
  setWorktreeState('isLoadingFileTree', loading);
}

/**
 * 设置 worktree 错误信息。
 *
 * @param error - 错误描述，null 表示清除错误
 */
export function setWorktreeError(error: string | null): void {
  setWorktreeState('error', error);
}

/**
 * 从最近列表中移除一个 worktree。
 *
 * @param worktreeId - 要移除的 worktree ID
 */
export function removeRecentWorktree(worktreeId: string): void {
  setWorktreeState('recentWorktrees', (prev) =>
    prev.filter((worktree) => worktree.id !== worktreeId),
  );
}

/**
 * 切换 worktree 的收藏状态。
 *
 * @param worktreeId - 要切换收藏状态的 worktree ID
 */
export function toggleWorktreeStar(worktreeId: string): void {
  setWorktreeState('recentWorktrees', (worktree) => worktree.id === worktreeId, 'starred', (value) => !value);
}

/**
 * 重置 worktree 状态。
 */
export function resetWorktreeState(): void {
  setWorktreeState({ ...defaultWorktreeState });
}

export async function loadSessionWorktree(sessionId: string | null): Promise<void> {
  if (!sessionId) {
    setCurrentWorktree(null);
    setWorktreeFiles([]);
    setFileTree([]);
    setWorktreeLoading(false);
    setFileTreeLoading(false);
    setWorktreeError(null);
    return;
  }

  setWorktreeLoading(true);
  setFileTreeLoading(true);
  setWorktreeError(null);

  try {
    const snapshot = await invoke<UiSessionWorktreeSnapshot>('ui_get_session_worktree_snapshot', {
      payload: { sessionId },
    });
    applyWorktreeSnapshot(snapshot);
  } catch (error) {
    setCurrentWorktree(null);
    setWorktreeFiles([]);
    setFileTree([]);
    setWorktreeError(error instanceof Error ? error.message : String(error));
  } finally {
    setWorktreeLoading(false);
    setFileTreeLoading(false);
  }
}

export async function readSessionWorktreeFile(
  sessionId: string,
  relativePath: string,
): Promise<WorktreeFileDocument> {
  const document = await invoke<UiWorktreeFileDocument>('ui_read_session_worktree_file', {
    payload: {
      sessionId,
      relativePath,
    },
  });
  return documentFromUi(document);
}

export async function writeSessionWorktreeFile(
  sessionId: string,
  relativePath: string,
  content: string,
): Promise<WorktreeFileDocument> {
  const document = await invoke<UiWorktreeFileDocument>('ui_write_session_worktree_file', {
    payload: {
      sessionId,
      relativePath,
      content,
    },
  });
  return documentFromUi(document);
}
