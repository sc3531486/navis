import type { ChatAgentTimelinePart } from '@/lib/stream';
import { isStatusLive } from '@/lib/status';

export type TimelineToolKind =
  | 'read'
  | 'list'
  | 'glob'
  | 'grep'
  | 'search'
  | 'inspect'
  | 'edit'
  | 'terminal'
  | 'lsp'
  | 'todo'
  | 'plan'
  | 'clarify'
  | 'skill'
  | 'webfetch'
  | 'websearch'
  | 'mcp_resource'
  | 'browser'
  | 'sidechain'
  | 'reasoning'
  | 'permission'
  | 'error'
  | 'summary'
  | 'other';

export const isExceptionalTimelineStatus = (status: ChatAgentTimelinePart['status']): boolean =>
  status === 'error' ||
  status === 'denied' ||
  status === 'aborted' ||
  status === 'interrupted';

export const isLiveTimelinePart = (part: ChatAgentTimelinePart): boolean =>
  isStatusLive(part.statusPresentation);

const timelineToolAliases: Partial<Record<TimelineToolKind, readonly string[]>> = {
  read: ['fs_read_file', 'read'],
  list: ['fs_list_files', 'list'],
  glob: ['glob'],
  grep: ['grep'],
  search: ['fs_search_files', 'search', 'web_search'],
  inspect: ['fs_file_info', 'inspect'],
  edit: ['fs_write_file', 'write', 'write_as_edit', 'fs_replace_in_file', 'edit'],
  terminal: ['bash', 'git'],
  lsp: ['lsp', 'lsp_diagnostic', 'lsp_symbol'],
  todo: ['todo', 'todo_write'],
  plan: ['plan', 'exit_plan_mode', 'navis_exit_plan_mode'],
  clarify: ['clarify', 'navis_clarify'],
  skill: ['skill'],
  webfetch: ['webfetch', 'web_fetch'],
  websearch: ['websearch', 'web_search'],
  mcp_resource: ['mcp_resource'],
  browser: ['browser'],
  sidechain: ['agent', 'sidechain', 'task'],
};

const timelineToolName = (part: ChatAgentTimelinePart): string =>
  (part.tool || part.gatewayTool || part.title || '').toLowerCase();

const normalizedTimelineToolName = (part: ChatAgentTimelinePart): string =>
  timelineToolName(part).replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '');

export const timelineMetadataDisplayKind = (part: ChatAgentTimelinePart): TimelineToolKind | undefined => {
  const displayKind = part.metadata?.displayKind;
  if (typeof displayKind !== 'string') return undefined;
  const normalized = displayKind.trim().toLowerCase().replace(/-/g, '_');
  switch (normalized) {
    case 'read':
    case 'list':
    case 'glob':
    case 'grep':
    case 'search':
    case 'inspect':
    case 'edit':
    case 'write_as_edit':
    case 'bash':
    case 'git':
    case 'lsp':
    case 'todo':
    case 'plan':
    case 'clarify':
    case 'skill':
    case 'webfetch':
    case 'web_fetch':
    case 'websearch':
    case 'web_search':
    case 'mcp_resource':
    case 'browser':
    case 'task':
    case 'task_output':
    case 'task_stop':
      if (normalized === 'write_as_edit') return 'edit';
      if (normalized === 'bash' || normalized === 'git') return 'terminal';
      if (normalized === 'web_fetch') return 'webfetch';
      if (normalized === 'web_search') return 'websearch';
      if (normalized === 'task' || normalized === 'task_output' || normalized === 'task_stop') return 'sidechain';
      return normalized as TimelineToolKind;
    case 'terminal':
    case 'sidechain':
    case 'reasoning':
    case 'permission':
    case 'error':
    case 'summary':
    case 'other':
      return normalized;
    default:
      return undefined;
  }
};

export const timelineToolKind = (part: ChatAgentTimelinePart): TimelineToolKind => {
  if (part.kind === 'reasoning') return 'reasoning';
  if (part.kind === 'permission') return 'permission';
  if (part.kind === 'sidechain') return 'sidechain';
  if (part.kind === 'error') return 'error';
  if (part.kind === 'summary') return 'summary';
  const displayKind = timelineMetadataDisplayKind(part);
  if (displayKind) return displayKind;
  const normalized = normalizedTimelineToolName(part);
  if (!normalized) return 'other';
  if (normalized.startsWith('terminal_')) return 'terminal';
  for (const [kind, aliases] of Object.entries(timelineToolAliases) as Array<
    [TimelineToolKind, readonly string[]]
  >) {
    if (aliases.includes(normalized)) return kind;
  }
  return 'other';
};
