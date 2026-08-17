import { worktreeState } from '../../stores/worktree';
import type { ChatAgentTimelinePart } from '../../lib/stream';
import { openTimelineFilePanel } from './timeline-panel-actions';
import {
  isAbsoluteFilePath,
  isSafeWorktreeRelativePath,
  normalizeComparablePath,
  toolPath,
} from './tool-presentation';

export const currentWorktreeRelativePath = (path: string): string | null => {
  const rawPath = path.trim();
  if (!rawPath) return null;

  const worktreeRoot = worktreeState.currentWorktree?.path?.trim();
  const normalizedPath = normalizeComparablePath(rawPath);
  const normalizedRoot = worktreeRoot ? normalizeComparablePath(worktreeRoot) : '';

  if (normalizedRoot && normalizedPath === normalizedRoot) return null;
  if (normalizedRoot && normalizedPath.startsWith(`${normalizedRoot}/`)) {
    const localPath = rawPath.replace(/^\\\\\?\\/, '');
    return localPath
      .slice(worktreeRoot!.length)
      .replace(/^[\\/]+/, '')
      .replace(/\\/g, '/');
  }

  if (isAbsoluteFilePath(rawPath)) return null;

  const relativePath = rawPath.replace(/\\/g, '/').replace(/^\/+/, '');
  if (isSafeWorktreeRelativePath(relativePath)) return relativePath;

  const relativeMatchesWorktree = worktreeState.worktreeFiles.some(
    (file) => normalizeComparablePath(file) === normalizeComparablePath(relativePath),
  );
  return relativeMatchesWorktree ? relativePath : null;
};

export const openToolPathInFilePanel = (part: ChatAgentTimelinePart): boolean => {
  const relativePath = currentWorktreeRelativePath(toolPath(part));
  if (!relativePath) return false;
  return openTimelineFilePanel(relativePath);
};
