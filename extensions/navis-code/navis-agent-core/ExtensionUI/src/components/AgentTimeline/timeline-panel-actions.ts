import { openRightWorkspacePanel } from '@/stores/host';
import { activeSessionId } from '@session/stores/session-tree';
import { requestEditorWorktreeFileOpen } from '@editor-ext/components/Editor/stores/editor-worktree';

export function openTimelineFilePanel(relativePath: string): boolean {
  const sessionId = activeSessionId();
  const path = relativePath.trim();
  if (!sessionId || !path) return false;

  requestEditorWorktreeFileOpen(path);
  openRightWorkspacePanel({
    id: 'editor',
    title: 'File',
    viewId: 'editor',
    sessionId,
  });
  return true;
}

export function openTimelineDiffPanel(title: string, diff: string): boolean {
  const sessionId = activeSessionId();
  const value = diff.trim();
  if (!sessionId || !value) return false;
  const panelId = encodeURIComponent(title.trim() || 'tool-diff').slice(0, 96);

  openRightWorkspacePanel({
    id: `tool-diff:${panelId}`,
    title: 'Diff',
    viewId: 'tool-diff',
    sessionId,
    config: {
      title,
      diff: value,
    },
  });
  return true;
}
