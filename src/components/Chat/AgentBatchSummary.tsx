import { Component, For, Show, createMemo, createSignal } from 'solid-js';

import {
  isRunningTaskStatus,
  readableToolName,
  taskDurationLabel,
  type TaskProjection,
} from '../../stores/task-projection';
import { openBackgroundTasksPanel } from './panel-actions';
import { statusClass } from '../../lib/status';

const taskStatusShortLabel = (status: string): string => {
  switch (status) {
    case 'pending':
      return 'Pending';
    case 'blocked':
      return 'Blocked';
    case 'running':
      return 'Running';
    case 'waiting_confirm':
      return 'Waiting';
    case 'completed':
      return '';
    case 'failed':
      return 'Failed';
    case 'cancelled':
      return 'Stopped';
    default:
      return status || '';
  }
};

const agentTaskDescription = (task: TaskProjection): string =>
  task.description?.trim() ||
  task.currentActivity?.trim() ||
  task.latestMessage?.trim() ||
  'Background task';

const agentTaskActivity = (task: TaskProjection): string => {
  const latestTool = readableToolName(task.latestToolName);
  if (latestTool) return latestTool;
  return task.currentActivity?.trim() ?? '';
};

export const AgentBatchSummary: Component<{ tasks: TaskProjection[] }> = (props) => {
  const [expanded, setExpanded] = createSignal(false);
  const runningTasks = createMemo(() => props.tasks.filter(isRunningTaskStatus));
  const label = () => {
    const runningCount = runningTasks().length;
    return `Running ${runningCount} ${runningCount === 1 ? 'agent' : 'agents'}`;
  };

  return (
    <div class="navis-agent-batch" aria-live="polite">
      <button
        type="button"
        class="navis-agent-batch-summary"
        aria-expanded={expanded()}
        onClick={() => setExpanded((value) => !value)}
      >
        <span class="navis-agent-batch-summary-label">{label()}</span>
        <span class={`navis-agent-trace-summary-chevron ${expanded() ? 'is-open' : ''}`} aria-hidden="true" />
      </button>
      <Show when={expanded()}>
        <div class="navis-agent-batch-list">
          <For each={props.tasks}>
            {(task) => {
              const active = () => isRunningTaskStatus(task);
              const status = () => taskStatusShortLabel(task.status);
              const activity = () => agentTaskActivity(task);
              return (
                <button
                  type="button"
                  class={`navis-agent-batch-row ${statusClass(task.statusPresentation)}`}
                  onClick={() => openBackgroundTasksPanel({
                    selectedTaskId: task.id,
                    selectedSidechainSessionId: task.sidechainSessionId,
                  })}
                  title={agentTaskDescription(task)}
                >
                  <span class="navis-agent-batch-dot" aria-hidden="true" />
                  <span class="navis-agent-batch-copy">
                    <span>{active() ? 'Running agent' : 'Ran agent'}</span>
                    <span>{taskDurationLabel(task.durationMs)}</span>
                    <span>{agentTaskDescription(task)}</span>
                    <Show when={activity()}>
                      {(value) => <span>{value()}</span>}
                    </Show>
                    <Show when={status()}>
                      {(value) => <span>{value()}</span>}
                    </Show>
                  </span>
                </button>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default AgentBatchSummary;
