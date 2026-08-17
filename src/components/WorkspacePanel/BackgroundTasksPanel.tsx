import { invoke } from '@tauri-apps/api/core';
import { EmptyState } from '../ui/EmptyState';
import { Component, For, Show, createEffect, createMemo, onCleanup } from 'solid-js';
import { statusClass } from '../../lib/status';
import type { RightWorkspacePanel } from '../../stores/app';
import { refreshTaskProjection, subscribeTaskProjectionPolling, taskProjectionState, taskDurationLabel, isRunningTaskStatus, readableToolName, type TaskProjection } from '../../stores/task-projection';
import { activeSessionId } from '../../stores/session-tree';
import {
  taskStatusLabel,
  openTaskTranscript,
} from './shared';
import {
  WorkspacePanelScrollArea,
  WorkspacePanelSectionHeader,
} from './WorkspacePanelFrame';

interface BackgroundTasksPanelConfig {
  selectedTaskId?: string | null;
  selectedSidechainSessionId?: string | null;
}

const backgroundTasksPanelConfig = (config: unknown): BackgroundTasksPanelConfig => {
  if (!config || typeof config !== 'object') return {};
  const record = config as Record<string, unknown>;
  return {
    selectedTaskId: typeof record.selectedTaskId === 'string' ? record.selectedTaskId : null,
    selectedSidechainSessionId: typeof record.selectedSidechainSessionId === 'string'
      ? record.selectedSidechainSessionId
      : null,
  };
};

const BackgroundTasksPanel: Component<{ panel: RightWorkspacePanel }> = (props) => {
  let panelRef: HTMLDivElement | undefined;

  createEffect(() => {
    const release = subscribeTaskProjectionPolling(activeSessionId());
    onCleanup(release);
  });
  const visibleTasks = () => taskProjectionState.tasks;
  const requestedSelection = () => backgroundTasksPanelConfig(props.panel.config);
  const isSelectedTask = (task: TaskProjection): boolean => {
    const selection = requestedSelection();
    return Boolean(
      (selection.selectedTaskId && task.id === selection.selectedTaskId) ||
      (selection.selectedSidechainSessionId && task.sidechainSessionId === selection.selectedSidechainSessionId),
    );
  };
  const hasLoaded = () => !taskProjectionState.loading || taskProjectionState.tasks.length > 0;
  const runningTasks = createMemo(() => visibleTasks().filter(isRunningTaskStatus));
  const finishedTasks = createMemo(() => visibleTasks().filter((task) => !isRunningTaskStatus(task)));
  createEffect(() => {
    const selection = requestedSelection();
    const selector = selection.selectedTaskId
      ? `[data-task-id="${CSS.escape(selection.selectedTaskId)}"]`
      : selection.selectedSidechainSessionId
        ? `[data-sidechain-session-id="${CSS.escape(selection.selectedSidechainSessionId)}"]`
        : '';
    if (!selector) return;
    visibleTasks();
    queueMicrotask(() => {
      panelRef?.querySelector<HTMLElement>(selector)?.scrollIntoView({
        block: 'nearest',
        behavior: 'smooth',
      });
    });
  });
  const stopTask = async (task: TaskProjection): Promise<void> => {
    if (!isRunningTaskStatus(task)) return;
    await invoke('ui_stop_task', { payload: { taskId: task.id } });
    await refreshTaskProjection(activeSessionId());
  };
  const clearFinished = async (): Promise<void> => {
    await invoke('ui_clear_finished_tasks', { payload: { sessionId: activeSessionId() } });
    await refreshTaskProjection(activeSessionId());
  };

  const TaskCard: Component<{ task: TaskProjection }> = (props) => {
    const toolName = () => readableToolName(props.task.latestToolName);
    const canStop = () => isRunningTaskStatus(props.task);
    const primaryToolLabel = () => {
      const name = toolName();
      if (name) return name;
      if (props.task.kind === 'sidechain') return 'Agent';
      if (props.task.kind === 'parallel') return 'Agent';
      if (props.task.kind === 'background') return 'Background';
      if (props.task.kind === 'autonomous') return 'Agent';
      return 'Task';
    };
    const durationLabel = () => {
      const status = props.task.status;
      if (props.task.statusPresentation.terminal || isRunningTaskStatus(props.task)) {
        return taskDurationLabel(props.task.durationMs);
      }
      return '';
    };
    return (
      <article
        class={`navis-workspace-card navis-workspace-task ${statusClass(props.task.statusPresentation)} ${isSelectedTask(props.task) ? 'is-selected' : ''}`}
        data-task-id={props.task.id}
        data-sidechain-session-id={props.task.sidechainSessionId ?? undefined}
        aria-current={isSelectedTask(props.task) ? 'true' : undefined}
      >
        <Show when={canStop()}>
          <button
            type="button"
            class="navis-workspace-task-stop"
            aria-label="Stop task"
            title="Stop task"
            onClick={() => void stopTask(props.task)}
          >
            <span aria-hidden="true" />
          </button>
        </Show>
        <div class="navis-workspace-task-title-row">
          <span class="navis-workspace-task-dot" aria-hidden="true" />
          <div class="navis-workspace-task-title">{props.task.description || 'Background task'}</div>
        </div>
        <div class="navis-workspace-task-line">
          <span>{primaryToolLabel()}</span>
          <span class={`navis-workspace-task-status ${statusClass(props.task.statusPresentation)}`}>{taskStatusLabel(props.task.status)}</span>
          <Show when={durationLabel()}>
            <span>{durationLabel()}</span>
          </Show>
        </div>
        <Show when={props.task.sidechainSessionId}>
          <button type="button" class="navis-workspace-task-link" onClick={() => openTaskTranscript(props.task)}>
            View transcript
          </button>
        </Show>
      </article>
    );
  };

  return (
    <div ref={panelRef} class="navis-workspace-task-panel">
      <Show
        when={hasLoaded()}
        fallback={<EmptyState title="Loading background tasks" body="Reading child agent work for the current session." />}
      >
        <Show
          when={!taskProjectionState.error || visibleTasks().length > 0}
          fallback={<EmptyState title="Failed to load background tasks" body={taskProjectionState.error ?? 'Unknown error'} />}
        >
          <Show
            when={visibleTasks().length > 0}
            fallback={<EmptyState title="No background tasks" body="The current session does not have any child agent work yet." />}
          >
            <WorkspacePanelScrollArea class="navis-workspace-task-list">
              <Show when={runningTasks().length > 0}>
                <section class="navis-workspace-task-section">
                  <WorkspacePanelSectionHeader title="Running" />
                  <For each={runningTasks()}>
                    {(task) => <TaskCard task={task} />}
                  </For>
                </section>
              </Show>
              <Show when={finishedTasks().length > 0}>
                <section class="navis-workspace-task-section">
                  <WorkspacePanelSectionHeader
                    title="Finished"
                    action={
                    <button type="button" class="navis-workspace-task-clear" onClick={() => void clearFinished()}>
                      Clear
                    </button>
                    }
                  />
                  <For each={finishedTasks()}>
                    {(task) => <TaskCard task={task} />}
                  </For>
                </section>
              </Show>
            </WorkspacePanelScrollArea>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

export default BackgroundTasksPanel;
