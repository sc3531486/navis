import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import { createPollingCoordinator, formatDuration, isStatusLive } from '../lib/status';
import type { StatusPresentation } from '../lib/status';

export interface TaskProjection {
  id: string;
  sessionId: string;
  parentTaskId?: string | null;
  sidechainSessionId?: string | null;
  kind: string;
  owner?: string | null;
  activeForm?: string | null;
  blocks: string[];
  blockedBy: string[];
  status: string;
  statusPresentation: StatusPresentation;
  description: string;
  error?: string | null;
  createdAt: string;
  completedAt?: string | null;
  durationMs: number;
  messageCount: number;
  toolCallCount: number;
  latestToolName?: string | null;
  tokenCount: number;
  latestMessage?: string | null;
  currentActivity?: string | null;
  result?: string | null;
}

export const isSubagentTaskKind = (kind?: string | null): boolean =>
  kind === 'sidechain' || kind === 'parallel' || kind === 'background' || kind === 'autonomous';

export const isRunningTaskStatus = (task: TaskProjection): boolean =>
  isStatusLive(task.statusPresentation);

export const taskDurationLabel = (durationMs: number): string => formatDuration(durationMs, true);

export const readableToolName = (toolName?: string | null): string => {
  const name = (toolName ?? '').trim();
  if (!name) return '';
  if (name === 'terminal.run_command') return 'Bash';
  if (name === 'fs.read_file') return 'Read';
  if (name === 'fs.list_files') return 'List';
  if (name === 'fs.replace_in_file') return 'Edit';
  if (name === 'fs.write_file') return 'Edit';
  if (name === 'fs.glob') return 'Glob';
  if (name === 'fs.grep') return 'Grep';
  if (name === 'web.fetch') return 'WebFetch';
  if (name === 'web.search') return 'WebSearch';
  return name
    .replace(/^fs\./, '')
    .replace(/^web\./, '')
    .replace(/^navis\./, '')
    .split(/[._-]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
};

export const [taskProjectionState, setTaskProjectionState] = createStore<{
  sessionId: string | null;
  tasks: TaskProjection[];
  loading: boolean;
  error: string | null;
}>({
  sessionId: null,
  tasks: [],
  loading: false,
  error: null,
});

const taskProjectionPolling = createPollingCoordinator(refreshTaskProjection);

export async function refreshTaskProjection(sessionId: string | null): Promise<void> {
  if (!sessionId) {
    setTaskProjectionState({
      sessionId: null,
      tasks: [],
      loading: false,
      error: null,
    });
    return;
  }

  setTaskProjectionState({
    sessionId,
    loading: taskProjectionState.sessionId !== sessionId,
    error: null,
  });

  try {
    const rows = await invoke<TaskProjection[]>('ui_list_tasks', {
      payload: {
        sessionId,
        limit: 50,
      },
    });
    if (taskProjectionState.sessionId !== sessionId) return;
    setTaskProjectionState({
      tasks: rows.filter((task) => isSubagentTaskKind(task.kind)),
      loading: false,
      error: null,
    });
  } catch (error) {
    if (taskProjectionState.sessionId !== sessionId) return;
    setTaskProjectionState({
      loading: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

export const subscribeTaskProjectionPolling = (sessionId: string | null) =>
  taskProjectionPolling.subscribe(sessionId);
