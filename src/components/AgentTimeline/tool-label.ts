import { isTextAgentTimelinePart } from '../../lib/agent-timeline';
import type { ChatAgentTimelinePart } from '../../lib/stream';
import { isLiveTimelinePart, timelineMetadataDisplayKind, timelineToolKind } from './tool-kind';
import { timelineTarget } from './tool-path';

export const agentTimelinePartLabel = (part: ChatAgentTimelinePart): string => {
  const target = timelineTarget(part);
  const fallback =
    part.title?.trim() ||
    part.tool?.trim() ||
    part.gatewayTool?.trim() ||
    timelineMetadataDisplayKind(part) ||
    part.detail?.trim() ||
    'Unknown tool step';
  const suffix = target ? ` ${target}` : '';
  const active = isLiveTimelinePart(part);
  if (part.kind === 'text') {
    return part.text ?? part.detail ?? part.summary ?? part.title ?? 'Assistant text';
  }
  switch (timelineToolKind(part)) {
    case 'read':
      return `${active ? 'Reading' : 'Read'}${suffix}`;
    case 'list':
      return `${active ? 'Listing' : 'Listed'}${suffix}`;
    case 'glob':
      return `${active ? 'Globbing' : 'Globbed'}${suffix}`;
    case 'grep':
      return `${active ? 'Grepping' : 'Grepped'}${suffix}`;
    case 'search':
      return `${active ? 'Searching' : 'Searched'}${suffix}`;
    case 'inspect':
      return `${active ? 'Inspecting' : 'Inspected'}${suffix}`;
    case 'edit':
      return `${active ? 'Editing' : 'Edited'}${suffix}`;
    case 'terminal':
      return `${active ? 'Running' : 'Ran'}${suffix}`;
    case 'lsp':
      return `Checked${suffix}`;
    case 'todo':
      return target ? `Updated todo ${target}` : (part.title ?? 'Updated todo');
    case 'skill':
      return target ? `Used skill ${target}` : (part.title ?? 'Used skill');
    case 'webfetch':
      return target ? `Fetched ${target}` : (part.title ?? 'Fetched web page');
    case 'websearch':
      return target ? `Searched web ${target}` : (part.title ?? 'Searched web');
    case 'mcp_resource':
      return target ? `Read resource ${target}` : (part.title ?? 'Read resource');
    case 'browser':
      return target ? `Browsed ${target}` : (part.title ?? 'Browser action');
    case 'sidechain':
      return target ? `Agent ${target}` : (part.title ?? 'Agent task');
    case 'reasoning':
      return target ? `Thinking ${target}` : (part.title ?? 'Thinking');
    case 'permission':
      return target ? `Needs approval ${target}` : (part.title ?? 'Needs approval');
    case 'error':
      return target ? `Error ${target}` : (part.title ?? 'Error');
    case 'summary':
      return target ? `Compacted ${target}` : (part.title ?? 'Compacted context');
    default:
      return `${fallback}${suffix}`;
  }
};

export const editLabelParts = (label: string): { main: string; additions?: string; deletions?: string } => {
  const match = label.match(/^(.*?)(?:\s+(\+\d+))?(?:\s+(-\d+))?$/);
  return {
    main: match?.[1]?.trim() || label,
    additions: match?.[2],
    deletions: match?.[3],
  };
};

export const timelineStatusLabel = (part: ChatAgentTimelinePart): string => {
  if (isTextAgentTimelinePart(part)) return '';
  switch (part.status) {
    case 'running':
      return 'Running';
    case 'waiting_permission':
      return 'Waiting';
    case 'completed':
      return 'Done';
    case 'error':
      return 'Error';
    case 'retrying':
      return 'Retrying';
    case 'denied':
      return 'Denied';
    case 'aborted':
    case 'interrupted':
      return 'Aborted';
    case 'reused':
      return 'Reused';
    case 'compacted':
      return 'Compacted';
    default:
      return part.status ?? '';
  }
};

export const terminalDetailStatusLabel = (part: ChatAgentTimelinePart): string => {
  switch (part.status) {
    case 'running':
    case 'waiting_permission':
      return 'Running';
    case 'retrying':
      return 'Retrying';
    case 'completed':
      return 'Success';
    case 'error':
      return 'Failed';
    case 'denied':
      return 'Denied';
    case 'aborted':
    case 'interrupted':
      return 'Aborted';
    default:
      return timelineStatusLabel(part);
  }
};

export const timelineDurationLabel = (durationMs: number, active: boolean): string => {
  const seconds = Math.max(0, Math.floor(durationMs / 1000));
  if (seconds < 1) return active ? '0s' : '<1s';
  if (seconds < 60) return `${Math.max(1, seconds)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
};

export const timelineDurationMs = (part: ChatAgentTimelinePart, nowMs: number): number | null => {
  if (typeof part.durationMs === 'number' && Number.isFinite(part.durationMs)) {
    return Math.max(0, part.durationMs);
  }
  const startedAt = part.startedAt ?? part.createdAt;
  const startedAtMs = Date.parse(startedAt);
  if (Number.isNaN(startedAtMs)) return null;
  const completedAtMs = part.completedAt ? Date.parse(part.completedAt) : NaN;
  const updatedAtMs = part.updatedAt ? Date.parse(part.updatedAt) : NaN;
  const active = isLiveTimelinePart(part);
  const endMs = Number.isNaN(completedAtMs)
    ? active
      ? nowMs
      : updatedAtMs
    : completedAtMs;
  if (Number.isNaN(endMs)) return null;
  if (endMs < startedAtMs) return null;
  return endMs - startedAtMs;
};
