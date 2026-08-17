import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { addRecentWorktree } from './project';
import { setSessionWorktreeRootWithEditorGuard } from './editor-context-transition';

export interface BindComposerWorktreeOptions {
  ensureSessionId: () => Promise<string | null>;
  onInfo?: (message: string) => void;
  onMissingSession?: () => void;
}

export function pathNameFromPath(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts.at(-1) ?? path;
}

export function worktreeLabel(worktreeRoot: string | null | undefined): string {
  return worktreeRoot ? pathNameFromPath(worktreeRoot) : 'Local';
}

export async function rememberRecentWorktree(path: string): Promise<void> {
  await addRecentWorktree({ path });
}

export async function bindComposerWorktree(
  worktreeRoot: string | null,
  options: BindComposerWorktreeOptions,
): Promise<boolean> {
  const sessionId = await options.ensureSessionId();
  if (!sessionId) {
    options.onMissingSession?.();
    return false;
  }

  const changed = await setSessionWorktreeRootWithEditorGuard(sessionId, worktreeRoot);
  if (!changed) {
    return false;
  }

  if (worktreeRoot) {
    await rememberRecentWorktree(worktreeRoot);
    options.onInfo?.(`Worktree changed to ${pathNameFromPath(worktreeRoot)}`);
  } else {
    options.onInfo?.('Worktree changed to Local');
  }

  return true;
}

export async function chooseComposerWorktree(options: BindComposerWorktreeOptions): Promise<boolean> {
  const result = await openDialog({
    multiple: false,
    directory: true,
    title: 'Choose worktree folder',
  });
  const [path] = Array.isArray(result) ? result : result ? [result] : [];
  if (!path) return false;

  return bindComposerWorktree(path, options);
}
