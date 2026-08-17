import type { ChatAgentTimelinePart } from '../../lib/stream';
import { recordString, toolInput, toolOutput } from './tool-record';

const displayTimelineTarget = (target: string): string => {
  const trimmed = target.trim();
  if (!trimmed || trimmed === '.') return 'Worktree root';
  return trimmed;
};

export const toolPath = (part: ChatAgentTimelinePart): string =>
  recordString(toolInput(part), 'path') ??
  recordString(toolOutput(part), 'path') ??
  part.summary?.replace(/\s+[+-]\d+.*$/, '').trim() ??
  '';

export const normalizeComparablePath = (value: string): string =>
  value.trim().replace(/^\\\\\?\\/, '').replace(/\\/g, '/').replace(/\/+$/g, '').toLowerCase();

export const isAbsoluteFilePath = (value: string): boolean =>
  /^[a-z]:[\\/]/i.test(value.trim().replace(/^\\\\\?\\/, '')) ||
  value.trim().startsWith('/') ||
  value.trim().startsWith('\\\\');

export const isSafeWorktreeRelativePath = (value: string): boolean => {
  const normalized = value.replace(/\\/g, '/').replace(/^\/+/, '');
  return Boolean(normalized) && normalized !== '..' && !normalized.startsWith('../') && !normalized.includes('/../');
};

export const timelineTarget = (part: ChatAgentTimelinePart): string => {
  const input = toolInput(part);
  const output = toolOutput(part);
  const target =
    recordString(input, 'path') ??
    recordString(input, 'file_path') ??
    recordString(input, 'command') ??
    recordString(output, 'path') ??
    recordString(output, 'command') ??
    recordString(input, 'description') ??
    recordString(output, 'description') ??
    recordString(input, 'task') ??
    recordString(output, 'task') ??
    part.summary?.trim() ??
    '';
  return displayTimelineTarget(target);
};
