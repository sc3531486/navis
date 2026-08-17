import { Component, For, Show } from 'solid-js';

import gatewayChevronUrl from '../../assets/gateway-v.svg';
import type { ComposerTask } from '../../stores/composer-run';
import { elapsedTimeLabel } from '../../stores/composer-run';
import { EditIcon, GuideIcon, PauseCircleIcon, TargetIcon, TrashIcon } from '../Icon';
import PlanPhaseLine from '../Plan/PlanPhaseLine';
import type { SessionTodoItem } from '../../stores/session-todos';

interface ComposerRunStackProps {
  planPhases: () => SessionTodoItem[];
  queuedTasks: () => ComposerTask[];
  activeGoalText: () => string | null;
  activeGoalStartedAt: () => string | null;
  runningTask: () => ComposerTask | null;
  guidedQueuedTaskId: () => string | null;
  goalPaused: () => boolean;
  goalExpanded: () => boolean;
  now: () => number;
  onGuideQueuedTask: (taskId: string) => void;
  onRemoveQueuedTask: (taskId: string) => void;
  onEditQueuedTask: (taskId: string) => void;
  onEditGoal: () => void;
  onToggleGoalPaused: () => void;
  onToggleGoalExpanded: () => void;
  onClearGoal: () => void;
}

const ComposerRunStack: Component<ComposerRunStackProps> = (props) => (
  <Show when={props.planPhases().length > 0 || props.queuedTasks().length > 0 || props.activeGoalText()}>
    <section class="navis-composer-run-stack" aria-label="Queue and goal">
      <Show when={props.planPhases().length > 0}>
        <PlanPhaseLine phases={props.planPhases()} />
      </Show>
      <For each={props.queuedTasks()}>
        {(task) => (
          <div
            class="navis-queue-strip"
            classList={{ 'is-guided': props.guidedQueuedTaskId() === task.id }}
            aria-label="Queued task"
          >
            <span class="navis-queue-strip-icon" aria-hidden="true">↳</span>
            <span class="navis-queue-strip-text">{task.text}</span>
            <button
              type="button"
              class="navis-queue-strip-action is-reference"
              aria-label="Guide queued task"
              title="引导执行"
              onClick={() => props.onGuideQueuedTask(task.id)}
            >
              <GuideIcon />
            </button>
            <button
              type="button"
              class="navis-queue-strip-action"
              aria-label="Remove queued task"
              title="Remove"
              onClick={() => props.onRemoveQueuedTask(task.id)}
            >
              <TrashIcon />
            </button>
            <button
              type="button"
              class="navis-queue-strip-action"
              aria-label="Edit queued task"
              title="Edit"
              onClick={() => props.onEditQueuedTask(task.id)}
            >
              <EditIcon />
            </button>
          </div>
        )}
      </For>
      <Show when={props.activeGoalText()}>
        <section
          class={`navis-goal-strip ${props.goalPaused() ? 'is-paused' : ''} ${
            props.goalExpanded() ? 'is-expanded' : ''
          }`}
          aria-label="Goal state"
        >
          <div class="navis-goal-strip-row">
            <span class={`navis-goal-strip-running-icon ${props.runningTask() && !props.goalPaused() ? 'is-running' : ''}`}>
              <TargetIcon />
            </span>
            <div class="navis-goal-strip-main">
              <span class="navis-goal-strip-title">
                {props.goalPaused() ? 'Goal paused' : props.runningTask() ? 'Running goal' : 'Goal set'}
              </span>
              <span class="navis-goal-strip-text">{props.activeGoalText()}</span>
            </div>
            <span class="navis-goal-strip-time">
              {elapsedTimeLabel(props.runningTask()?.createdAt ?? props.activeGoalStartedAt(), props.now())}
            </span>
            <button type="button" class="navis-goal-strip-icon" aria-label="Edit goal" title="Edit goal" onClick={props.onEditGoal}>
              <EditIcon />
            </button>
            <button
              type="button"
              class="navis-goal-strip-icon"
              aria-label={props.goalPaused() ? 'Resume goal' : 'Pause goal'}
              title={props.goalPaused() ? 'Resume goal' : 'Pause goal'}
              onClick={props.onToggleGoalPaused}
            >
              <PauseCircleIcon />
            </button>
            <button type="button" class="navis-goal-strip-icon" aria-label="Remove goal" title="Remove goal" onClick={props.onClearGoal}>
              <TrashIcon />
            </button>
            <button
              type="button"
              class="navis-goal-strip-chevron"
              aria-label={props.goalExpanded() ? 'Collapse goal' : 'Expand goal'}
              title={props.goalExpanded() ? 'Collapse goal' : 'Expand goal'}
              aria-expanded={props.goalExpanded()}
              onClick={props.onToggleGoalExpanded}
            >
              <span
                class="navis-goal-chevron"
                style={{ '--navis-chevron-url': `url("${gatewayChevronUrl}")` }}
                aria-hidden="true"
              />
            </button>
          </div>
          <Show when={props.goalExpanded()}>
            <div class="navis-goal-strip-detail">
              <span class="navis-goal-strip-detail-label">Goal</span>
              <span class="navis-goal-strip-detail-text">{props.activeGoalText()}</span>
            </div>
          </Show>
        </section>
      </Show>
    </section>
  </Show>
);

export default ComposerRunStack;
