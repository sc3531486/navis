import { Component, For, Show, createEffect, onCleanup } from 'solid-js';
import { openRightWorkspacePanel, type RightWorkspacePanel } from '@/stores/host';
import { chatMessageState, type ChatMessage } from '@session/stores/chat-messages';
import {
  isRunningTaskStatus,
  taskDurationLabel,
  type TaskProjection,
} from '@task-ext/stores/task-projection';
import { activeSessionId } from '@session/stores/session-tree';
import { subscribeSessionTodosPolling, sessionTodosState } from '@session/stores/session-todos';
import { statusClass } from '@/lib/status';

/* ── Types ──────────────────────────────────────────────────────────── */

export interface PanelSection {
  title: string;
  body: string;
}

export interface UiSessionMessages {
  messages: ChatMessage[];
  total: number;
}

export interface UiSessionGitDiff {
  sessionId: string;
  worktreeRoot: string;
  isRepo: boolean;
  canCreateRepo: boolean;
  staged: boolean;
  diff: string;
  filesChanged: number;
  insertions: number;
  deletions: number;
  fileChanges: UiSessionGitDiffFile[];
}

export interface UiSessionGitDiffFile {
  path: string;
  status: string;
  staged: boolean;
  insertions: number;
  deletions: number;
}

export interface UiSessionChange {
  id: string;
  sessionId: string;
  turnId: string;
  messageId: string;
  agentTimelinePartId?: string | null;
  callId?: string | null;
  toolName: string;
  worktreePath?: string | null;
  relativePath?: string | null;
  absolutePath: string;
  operation: string;
  beforeContent?: string | null;
  afterContent?: string | null;
  diff?: string | null;
  insertions: number;
  deletions: number;
  status: string;
  createdAt: string;
  revertedAt?: string | null;
  metadata?: unknown;
}

export interface DesignDocLink {
  id: string;
  title: string;
  path: string;
  area: string;
}

/* ── Constants ──────────────────────────────────────────────────────── */

export const DESIGN_DOCS: DesignDocLink[] = [
  { id: 'kernel', title: 'Kernel', path: 'design/kernel.md', area: 'Architecture' },
  { id: 'architecture', title: 'Architecture overview', path: 'design/00-architecture-overview.md', area: 'Architecture' },
  { id: 'ui-framework', title: 'UI framework', path: 'design/22-ui-framework.md', area: 'UI' },
  { id: 'extension', title: 'Extension', path: 'design/07-extension.md', area: 'Extension' },
  { id: 'agent', title: 'Agent', path: 'design/16-agent.md', area: 'Agent' },
  { id: 'task-sidechain', title: 'Task sidechain', path: 'design/17-task-sidechain.md', area: 'Agent' },
  { id: 'mcp', title: 'MCP', path: 'design/13-mcp.md', area: 'Tool' },
  { id: 'gateway', title: 'Gateway', path: 'design/12-gateway.md', area: 'AI' },
  { id: 'storage', title: 'Storage', path: 'design/04-storage.md', area: 'Foundation' },
  { id: 'sandbox', title: 'Sandbox', path: 'design/06-sandbox.md', area: 'Security' },
];

export const kernelPrimitiveRows = [
  ['Registry', 'Available capabilities and lifecycle state'],
  ['Pipeline', 'How capabilities execute with cancellation, retry, progress, audit'],
  ['Event Bus', 'Discrete state notification after facts are written'],
  ['Policy', 'Who can do what at enforced checkpoints'],
] as const;

/* ── Helpers ────────────────────────────────────────────────────────── */

export const taskStatusLabel = (status: string): string => {
  switch (status) {
    case 'pending':
      return 'Pending';
    case 'blocked':
      return 'Blocked';
    case 'running':
      return 'Running';
    case 'waiting_confirm':
      return 'Waiting confirmation';
    case 'completed':
      return 'Completed';
    case 'failed':
      return 'Failed';
    case 'cancelled':
      return 'Cancelled';
    default:
      return status || 'Unknown';
  }
};

export const readableTaskKind = (kind?: string | null): string => {
  switch (kind) {
    case 'turn':
      return 'Turn';
    case 'sidechain':
      return 'Sidechain';
    case 'parallel':
      return 'Parallel';
    case 'background':
      return 'Background';
    case 'autonomous':
      return 'Autonomous';
    default:
      return 'Task';
  }
};

export const todoStatusLabel = (status: string): string => {
  switch (status) {
    case 'in_progress':
      return 'In progress';
    case 'completed':
      return 'Completed';
    default:
      return 'Pending';
  }
};

export const openTaskTranscript = (task: TaskProjection): void => {
  if (!task.sidechainSessionId) return;
  openRightWorkspacePanel({
    id: `session-${task.sidechainSessionId}`,
    title: task.description || 'Task transcript',
    viewId: 'session-transcript',
    sessionId: task.sidechainSessionId,
  });
};

export const messageRoleLabel = (role: ChatMessage['role']): string => {
  if (role === 'user') return 'You';
  if (role === 'assistant') return 'Navis Go';
  if (role === 'system') return 'System';
  return 'Tool';
};

export const messageTimeLabel = (createdAt: string): string => {
  const timestamp = Date.parse(createdAt);
  if (Number.isNaN(timestamp)) return createdAt;
  return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};

export const visibleSessionChanges = (changes: UiSessionChange[]): UiSessionChange[] => {
  const turns = new Map<string, UiSessionChange[]>();
  for (const change of changes) {
    const list = turns.get(change.turnId) ?? [];
    list.push(change);
    turns.set(change.turnId, list);
  }
  const activeTurn = chatMessageState.activeTurnId;
  if (activeTurn && turns.has(activeTurn)) {
    return turns.get(activeTurn) ?? [];
  }
  const latestTurn = [...turns.keys()].at(-1);
  return latestTurn ? turns.get(latestTurn) ?? [] : [];
};

/* ── Shared Components ──────────────────────────────────────────────── */

export const WorkspaceSectionList: Component<{ sections: PanelSection[] }> = (props) => (
  <div class="navis-workspace-section-list">
    <For each={props.sections}>
      {(section) => (
        <section class="navis-workspace-section">
          <div class="navis-workspace-section-title">{section.title}</div>
          <p>{section.body}</p>
        </section>
      )}
    </For>
  </div>
);

export const SessionTodosSection: Component<{ compact?: boolean }> = (props) => {
  createEffect(() => {
    const release = subscribeSessionTodosPolling(activeSessionId());
    onCleanup(release);
  });

  return (
    <Show when={!sessionTodosState.loading}>
      <Show when={!sessionTodosState.error}>
        <Show when={sessionTodosState.todos.length > 0}>
          <section class="navis-workspace-section">
            <div class="navis-workspace-section-title">{props.compact ? 'Plan phases' : 'Plan document'}</div>
            <div class="navis-workspace-plan-queue">
              <For each={sessionTodosState.todos}>
                {(todo, index) => (
                  <div class={`navis-workspace-plan-queue-item ${statusClass(todo.statusPresentation)}`}>
                    <span class="navis-workspace-plan-phase-index">
                      {index() + 1}
                    </span>
                    <span class="navis-workspace-plan-phase-content">
                      {todo.content}
                    </span>
                    <span class="navis-workspace-plan-phase-status">
                      {props.compact ? todo.priority ?? '' : todoStatusLabel(todo.status)}
                    </span>
                  </div>
                )}
              </For>
            </div>
          </section>
        </Show>
      </Show>
    </Show>
  );
};
