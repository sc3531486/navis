import type { ChatAgentTimelinePart } from '@/lib/stream';
import { isLiveTimelinePart, type TimelineToolKind, timelineToolKind } from './tool-kind';
import { timelineTarget } from './tool-path';
import {
  firstRecordArray,
  firstStringArray,
  partRecord,
  recordArray,
  recordBoolean,
  recordNumber,
  recordString,
  toolMetadata,
  toolOutput,
} from './tool-record';

export const terminalOutputText = (part: ChatAgentTimelinePart): string => {
  const output = toolOutput(part);
  const stdout = recordString(output, 'stdout') ?? '';
  const stderr = recordString(output, 'stderr') ?? '';
  return [stdout, stderr].filter(Boolean).join('\n');
};

export const tailLines = (value: string, limit: number): string => {
  const lines = value.split('\n').filter((line) => line.trim().length > 0);
  return lines.slice(Math.max(0, lines.length - limit)).join('\n');
};

const previewLines = (value: string, limit: number): string => {
  const lines = value.split('\n');
  const visible = lines.slice(0, limit);
  const omitted = lines.length - visible.length;
  return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more lines` : visible.join('\n');
};

export const formatToolBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unitIndex]}`;
};

export const toolResultSummary = (part: ChatAgentTimelinePart): string => {
  if (isLiveTimelinePart(part)) {
    const target = timelineTarget(part);
    switch (timelineToolKind(part)) {
      case 'list':
        return `Scanning ${target}`;
      case 'read':
        return `Opening ${target}`;
      case 'glob':
      case 'grep':
      case 'search':
      case 'websearch':
        return `Searching ${target}`;
      case 'terminal':
        return 'Executing command';
      case 'edit':
        return `Preparing ${target}`;
      default:
        return target;
    }
  }
  const metadata = toolMetadata(part);
  switch (timelineToolKind(part)) {
    case 'read': {
      const lines = recordNumber(metadata, 'lineCount');
      const bytes = recordNumber(metadata, 'bytes');
      if (typeof lines === 'number') return `Read ${lines} ${lines === 1 ? 'line' : 'lines'}`;
      if (typeof bytes === 'number') return `Read ${bytes} bytes`;
      return '';
    }
    case 'list': {
      const entries = recordNumber(metadata, 'entryCount');
      return typeof entries === 'number' ? `Listed ${entries} ${entries === 1 ? 'entry' : 'entries'}` : '';
    }
    case 'glob': {
      const files = recordNumber(metadata, 'fileCount') ?? recordNumber(metadata, 'resultCount');
      return typeof files === 'number' ? `Matched ${files} ${files === 1 ? 'file' : 'files'}` : '';
    }
    case 'grep':
    case 'search': {
      const results = recordNumber(metadata, 'resultCount');
      return typeof results === 'number' ? `Found ${results} ${results === 1 ? 'result' : 'results'}` : '';
    }
    case 'websearch': {
      const results = recordNumber(metadata, 'resultCount');
      return typeof results === 'number' ? `Found ${results} ${results === 1 ? 'result' : 'results'}` : '';
    }
    case 'mcp_resource': {
      const bytes = recordNumber(metadata, 'bytes');
      return typeof bytes === 'number' ? formatToolBytes(bytes) : '';
    }
    case 'edit':
      return '';
    case 'terminal': {
      const exitCode = toolOutput(part).exitCode;
      if (exitCode === null || typeof exitCode === 'number') return `Exited ${exitCode ?? 'unknown'}`;
      if (recordBoolean(toolOutput(part), 'timedOut')) return 'Timed out';
      return '';
    }
    default:
      return '';
  }
};

const toolPreviewKinds = new Set<TimelineToolKind>([
  'read',
  'list',
  'glob',
  'grep',
  'search',
  'inspect',
  'terminal',
  'lsp',
  'todo',
  'skill',
  'webfetch',
  'websearch',
  'mcp_resource',
  'browser',
  'sidechain',
]);

export const toolHasUsefulPreview = (part: ChatAgentTimelinePart, detail: string): boolean => {
  if (!toolPreviewKinds.has(timelineToolKind(part))) return false;
  if (!detail) return false;
  return detail.includes('\n') || detail.length > 120 || recordBoolean(toolOutput(part), 'truncated') === true;
};

const formatRecordRows = (record: Record<string, unknown>, keys: readonly string[]): string => {
  const rows = keys.map((key) => {
    const value = record[key];
    if (value === undefined || value === null || value === '') return '';
    if (typeof value === 'number' && key.toLowerCase().includes('bytes')) {
      return `${key}: ${formatToolBytes(value)}`;
    }
    if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
      return `${key}: ${String(value)}`;
    }
    return '';
  });
  return rows.filter(Boolean).join('\n');
};

const formatUnknownToolOutput = (record: Record<string, unknown>): string => {
  const entries = Object.entries(record).filter(([, value]) => value !== undefined && value !== null && value !== '');
  if (entries.length === 0) return '';
  try {
    return JSON.stringify(Object.fromEntries(entries), null, 2);
  } catch {
    return '';
  }
};

const formatSidechainToolOutput = (record: Record<string, unknown>): string => {
  const structured = partRecord(record.structuredOutput as Record<string, unknown> | null | undefined);
  const rows = [
    ['taskId', recordString(record, 'taskId')],
    ['sidechainSessionId', recordString(record, 'sidechainSessionId')],
    ['status', recordString(record, 'status') ?? recordString(structured, 'status')],
    ['retrievalStatus', recordString(record, 'retrievalStatus') ?? recordString(structured, 'retrievalStatus')],
    ['description', recordString(record, 'description')],
    ['summary', recordString(record, 'summary')],
    ['currentActivity', recordString(record, 'currentActivity') ?? recordString(structured, 'currentActivity')],
    ['toolCallCount', recordNumber(record, 'toolCallCount') ?? recordNumber(structured, 'toolCallCount')],
    ['tokenCount', recordNumber(record, 'tokenCount') ?? recordNumber(structured, 'tokenCount')],
    ['durationMs', recordNumber(record, 'durationMs') ?? recordNumber(structured, 'durationMs')],
  ];

  return rows
    .filter((row): row is [string, string | number] => row[1] !== undefined && row[1] !== null && row[1] !== '')
    .map(([key, value]) => `${key}: ${String(value)}`)
    .join('\n');
};

export const structuredToolDetail = (part: ChatAgentTimelinePart): string => {
  const output = toolOutput(part);
  switch (timelineToolKind(part)) {
    case 'read': {
      const content = recordString(output, 'content') ?? '';
      const truncated = recordBoolean(output, 'truncated');
      const suffix = truncated ? '\n... file content was truncated by read limit' : '';
      return `${content}${suffix}`.trim();
    }
    case 'list': {
      const entries = recordArray(output, 'entries');
      const visible = entries.slice(0, 24).map((entry) => {
        const kind = recordString(entry, 'kind') ?? 'item';
        const path = recordString(entry, 'path') ?? recordString(entry, 'name') ?? '';
        const bytes = recordNumber(entry, 'bytes');
        return bytes == null ? `${kind} ${path}` : `${kind} ${path} · ${formatToolBytes(bytes)}`;
      });
      const omitted = entries.length - visible.length;
      return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more entries` : visible.join('\n');
    }
    case 'glob': {
      const files = firstStringArray(output, ['files', 'paths', 'matches', 'results']);
      const visible = files.slice(0, 32);
      const omitted = files.length - visible.length;
      return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more files` : visible.join('\n');
    }
    case 'grep':
    case 'search': {
      const results = firstRecordArray(output, ['results', 'matches']);
      const visible = results.slice(0, 24).map((result) => {
        const path = recordString(result, 'path') ?? '';
        const line = recordNumber(result, 'line') ?? recordNumber(result, 'lineNumber');
        const preview = recordString(result, 'preview') ?? recordString(result, 'text') ?? recordString(result, 'lineText');
        const location = line == null ? path : `${path}:${line}`;
        return preview ? `${location} ${preview}` : location;
      });
      const omitted = results.length - visible.length;
      return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more results` : visible.join('\n');
    }
    case 'inspect': {
      const rows = [
        ['path', recordString(output, 'path')],
        ['kind', recordString(output, 'kind')],
        ['bytes', recordNumber(output, 'bytes') == null ? undefined : formatToolBytes(recordNumber(output, 'bytes')!)],
        ['readonly', recordBoolean(output, 'readonly') == null ? undefined : String(recordBoolean(output, 'readonly'))],
      ];
      return rows
        .filter((row): row is [string, string] => typeof row[1] === 'string' && row[1].length > 0)
        .map(([key, value]) => `${key}: ${value}`)
        .join('\n');
    }
    case 'lsp': {
      const diagnostics = firstRecordArray(output, ['diagnostics', 'items', 'results']);
      if (diagnostics.length > 0) {
        const visible = diagnostics.slice(0, 24).map((item) => {
          const path = recordString(item, 'path') ?? recordString(item, 'uri') ?? '';
          const line = recordNumber(item, 'line') ?? recordNumber(item, 'lineNumber');
          const severity = recordString(item, 'severity') ?? recordString(item, 'level') ?? 'diagnostic';
          const message = recordString(item, 'message') ?? recordString(item, 'text') ?? '';
          const location = line == null ? path : `${path}:${line}`;
          return `${severity} ${location} ${message}`.trim();
        });
        const omitted = diagnostics.length - visible.length;
        return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more diagnostics` : visible.join('\n');
      }
      return formatUnknownToolOutput(output);
    }
    case 'todo': {
      const todos = firstRecordArray(output, ['todos', 'items']);
      if (todos.length > 0) {
        return todos.map((item) => {
          const status = recordString(item, 'status') ?? 'todo';
          const content = recordString(item, 'content') ?? recordString(item, 'text') ?? recordString(item, 'title') ?? '';
          return `${status} ${content}`.trim();
        }).join('\n');
      }
      return formatUnknownToolOutput(output);
    }
    case 'skill':
      return formatRecordRows(output, ['name', 'skill', 'path', 'status', 'summary']) || formatUnknownToolOutput(output);
    case 'webfetch': {
      const content = recordString(output, 'content') ?? recordString(output, 'text') ?? recordString(output, 'markdown') ?? '';
      const rows = formatRecordRows(output, ['url', 'status', 'statusCode', 'title']);
      const preview = content ? previewLines(content, 18) : '';
      return [rows, preview].filter(Boolean).join('\n');
    }
    case 'websearch': {
      const results = firstRecordArray(output, ['results', 'items']);
      const visible = results.slice(0, 24).map((result) => {
        const title = recordString(result, 'title') ?? '';
        const url = recordString(result, 'url') ?? recordString(result, 'link') ?? '';
        const snippet = recordString(result, 'snippet') ?? recordString(result, 'preview') ?? '';
        return [title, url, snippet].filter(Boolean).join(' ');
      });
      const omitted = results.length - visible.length;
      return omitted > 0 ? `${visible.join('\n')}\n... ${omitted} more results` : visible.join('\n');
    }
    case 'mcp_resource': {
      const content = recordString(output, 'content') ?? recordString(output, 'text') ?? '';
      const rows = formatRecordRows(output, ['uri', 'mimeType', 'bytes']);
      return [rows, content ? previewLines(content, 18) : ''].filter(Boolean).join('\n');
    }
    case 'browser': {
      const content = recordString(output, 'text') ?? recordString(output, 'content') ?? recordString(output, 'snapshot') ?? '';
      const rows = formatRecordRows(output, ['url', 'title', 'action', 'status']);
      return [rows, content ? previewLines(content, 18) : ''].filter(Boolean).join('\n') || formatUnknownToolOutput(output);
    }
    case 'sidechain':
      return formatSidechainToolOutput(output);
    default:
      return '';
  }
};
