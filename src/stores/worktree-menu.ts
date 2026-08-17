import { dialog } from '../components/Dialog';
import type { MenuActionItem } from './menu';
import { executeDeclarativeMenuAction } from './menu-actions';
import { deleteWorktree, renameWorktree } from './session-tree';

export interface ExecuteWorktreeMenuOptions {
  worktreeIndex: number;
  worktreeName: string;
  mode?: string | null;
}

async function confirmWorktreeMenuAction(item: MenuActionItem, worktreeName: string): Promise<boolean> {
  if (item.command === 'worktree.delete') {
    return dialog.confirm({
      title: 'Delete worktree',
      message: `"${worktreeName}" and its active sessions will be permanently deleted.`,
      confirmText: 'Delete worktree',
      cancelText: 'Cancel',
      danger: true,
    });
  }

  if (item.risk !== 'high') return true;

  return dialog.confirm({
    title: item.label,
    message: `Run ${item.label}: ${worktreeName}?`,
    confirmText: item.label,
    cancelText: 'Cancel',
    danger: true,
  });
}

export async function executeWorktreeMenuItem(
  item: MenuActionItem,
  options: ExecuteWorktreeMenuOptions,
): Promise<boolean> {
  const confirmed = await confirmWorktreeMenuAction(item, options.worktreeName);
  if (!confirmed) return false;

  switch (item.command) {
    case 'worktree.rename': {
      const nextName = await dialog.input('Rename worktree', 'Enter a new worktree name', options.worktreeName);
      if (!nextName?.trim()) return false;
      await renameWorktree(options.worktreeIndex, nextName.trim(), options.mode ?? null);
      return true;
    }
    case 'worktree.delete':
      await deleteWorktree(options.worktreeIndex, options.mode ?? null);
      return true;
    default:
      return executeDeclarativeMenuAction(item);
  }
}
