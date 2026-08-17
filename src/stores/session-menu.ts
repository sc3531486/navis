import { dialog } from '../components/Dialog';
import { openRightWorkspacePanel } from './app';
import { selectSessionWithEditorGuard } from './editor-context-transition';
import { openSettingsDialog } from '../components/Settings/openSettingsDialog';
import { loadEditorSettings, openSessionExternalEditor, settingsState } from './settings';
import type { MenuActionItem, MenuTarget } from './menu';
import { executeDeclarativeMenuAction } from './menu-actions';
import {
  archiveSession,
  forkSession,
  moveSessionToWorktreeName,
  removeSession,
  renameSession,
  setSessionTranscriptView,
  setSessionUnread,
  toggleSessionPin,
  type SidebarSession,
  type TranscriptView,
} from './session-tree';

export interface SessionMenuSubmenuOptions {
  sessionId: string;
  target: MenuTarget;
  availableWorktreeNames?: string[];
  currentWorktreeName?: string;
}

export interface ExecuteSessionMenuOptions {
  sessionId: string;
  sessionName: string;
  currentWorktreeName?: string;
}

const SESSION_TRANSCRIPT_VIEWS: Array<{ value: TranscriptView; label: string }> = [
  { value: 'standard', label: 'Standard' },
  { value: 'compact', label: 'Compact' },
  { value: 'raw', label: 'Raw transcript' },
];

const OPEN_EXTERNAL_EDITOR_PREFIX = 'session.openIn.externalEditor:';

function externalEditorCommand(editorId: string): string {
  return `${OPEN_EXTERNAL_EDITOR_PREFIX}${encodeURIComponent(editorId)}`;
}

function encodedWorktreeCommand(worktreeName: string): string {
  return `session.moveToWorktree:${encodeURIComponent(worktreeName)}`;
}

async function confirmDangerousMenuAction(item: MenuActionItem, sessionName: string): Promise<boolean> {
  if (item.risk !== 'high') return true;

  return dialog.confirm({
    title: item.label === 'Delete' ? 'Delete session?' : item.label,
    message:
      item.label === 'Delete'
        ? `"${sessionName}" will be permanently deleted. This can't be undone.`
        : `Run ${item.label}: ${sessionName}?`,
    confirmText: item.label,
    cancelText: 'Cancel',
    danger: true,
  });
}

export function getSessionMenuItems(baseItems: MenuActionItem[], session?: SidebarSession): MenuActionItem[] {
  return baseItems.map((item) => {
    if (item.command === 'session.pin') {
      return {
        ...item,
        label: session?.pinned ? 'Unpin' : 'Pin',
      };
    }

    if (item.command === 'session.markUnread') {
      return {
        ...item,
        id: session?.unread ? `${item.id}.read` : item.id,
        label: session?.unread ? 'Mark as read' : 'Mark as unread',
        command: session?.unread ? 'session.markRead' : item.command,
      };
    }

    return item;
  });
}

export function getSessionSubmenuItems(
  item: MenuActionItem,
  options: SessionMenuSubmenuOptions,
): MenuActionItem[] {
  if (item.command === 'session.openIn') {
    const configuredEditors = settingsState.editor.externalEditors.slice().sort((left, right) => {
      if (left.isDefault !== right.isDefault) return left.isDefault ? -1 : 1;
      return left.name.localeCompare(right.name);
    });
    const editorItems = configuredEditors.map((editor) => ({
      id: `${options.sessionId}.open-in.external.${editor.id}`,
      label: editor.isDefault ? `${editor.name} (default)` : editor.name,
      target: options.target,
      command: externalEditorCommand(editor.id),
      group: 'open-in-editor',
    }));

    return [
      {
        id: `${options.sessionId}.open-in.current`,
        label: 'Current project',
        target: options.target,
        command: 'session.openIn.current',
        group: 'open-in',
      },
      {
        id: `${options.sessionId}.open-in.right`,
        label: 'Right panel',
        target: options.target,
        command: 'session.openIn.right',
        group: 'open-in',
      },
      ...editorItems,
      {
        id: `${options.sessionId}.open-in.configure-tools`,
        label: configuredEditors.length > 0 ? 'Configure coding tools...' : 'Configure coding tools...',
        target: options.target,
        command: 'session.openIn.configureExternalEditors',
        group: 'open-in-configure',
      },
    ];
  }

  if (item.command === 'session.moveToWorktree') {
    const existingWorktrees = (options.availableWorktreeNames ?? [])
      .filter((name) => name !== options.currentWorktreeName)
      .map((name) => ({
        id: `${options.sessionId}.move-to.${encodeURIComponent(name)}`,
        label: name,
        target: options.target,
        command: encodedWorktreeCommand(name),
        group: 'move-to-existing',
      }));

    return [
      ...existingWorktrees,
      {
        id: `${options.sessionId}.move-to.new`,
        label: 'New worktree...',
        target: options.target,
        command: 'session.moveToWorktree.new',
        group: 'move-to-new',
      },
    ];
  }

  if (item.command === 'session.transcriptView') {
    return SESSION_TRANSCRIPT_VIEWS.map((view) => ({
      id: `${options.sessionId}.transcript-view.${view.value}`,
      label: view.label,
      target: options.target,
      command: `session.transcriptView.${view.value}`,
      group: 'transcript-view',
    }));
  }

  return [];
}

export async function executeSessionMenuItem(
  item: MenuActionItem,
  options: ExecuteSessionMenuOptions,
): Promise<boolean> {
  const confirmed = await confirmDangerousMenuAction(item, options.sessionName);
  if (!confirmed) return false;

  switch (item.command) {
    case 'session.openIn.current':
      await selectSessionWithEditorGuard(options.sessionId);
      return true;
    case 'session.openIn.right':
      openRightWorkspacePanel({
        id: `session-${options.sessionId}`,
        title: options.sessionName,
        viewId: 'session-transcript',
        sessionId: options.sessionId,
      });
      return true;
    case 'session.openIn.configureExternalEditors':
      await openSettingsDialog('coding');
      return true;
    case 'session.pin':
      await toggleSessionPin(options.sessionId);
      return true;
    case 'session.markUnread':
      await setSessionUnread(options.sessionId, true);
      return true;
    case 'session.markRead':
      await setSessionUnread(options.sessionId, false);
      return true;
    case 'session.rename': {
      const nextName = await dialog.input('Rename session', 'Enter a new session name', options.sessionName);
      if (!nextName?.trim()) return false;
      await renameSession(options.sessionId, nextName.trim());
      return true;
    }
    case 'session.fork':
      await forkSession(options.sessionId);
      return true;
    case 'session.moveToWorktree.new': {
      const nextName = await dialog.input('New worktree', 'Enter a new worktree name', '');
      if (!nextName?.trim()) return false;
      await moveSessionToWorktreeName(options.sessionId, nextName.trim());
      return true;
    }
    case 'session.archive':
      await archiveSession(options.sessionId);
      return true;
    case 'session.delete':
      await removeSession(options.sessionId);
      return true;
    default:
      break;
  }

  if (item.command.startsWith('session.moveToWorktree:')) {
    const targetWorktreeName = decodeURIComponent(item.command.replace('session.moveToWorktree:', ''));
    if (targetWorktreeName && targetWorktreeName !== options.currentWorktreeName) {
      await moveSessionToWorktreeName(options.sessionId, targetWorktreeName);
      return true;
    }
    return false;
  }

  if (item.command.startsWith(OPEN_EXTERNAL_EDITOR_PREFIX)) {
    const editorId = decodeURIComponent(item.command.slice(OPEN_EXTERNAL_EDITOR_PREFIX.length));
    if (!settingsState.loaded) {
      await loadEditorSettings();
    }
    await openSessionExternalEditor(options.sessionId, editorId);
    return true;
  }

  if (item.command.startsWith('session.transcriptView.')) {
    const nextView = item.command.replace('session.transcriptView.', '') as TranscriptView;
    await setSessionTranscriptView(options.sessionId, nextView);
    return true;
  }

  return executeDeclarativeMenuAction(item);
}
