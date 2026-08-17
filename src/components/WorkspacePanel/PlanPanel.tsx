import { Component, For, Show, createEffect, onCleanup } from 'solid-js';
import { EmptyState } from '../ui/EmptyState';
import { composerRunState } from '../../stores/composer-run';
import { activeSessionId } from '../../stores/session-tree';
import {
  sessionTodosState,
  subscribeSessionTodosPolling,
} from '../../stores/session-todos';
import MessageContentRenderer from '../ui/MessageContentRenderer';
import { statusClass } from '../../lib/status';
import {
  todoStatusLabel,
} from './shared';

const PlanPanel: Component = () => {
  createEffect(() => {
    const release = subscribeSessionTodosPolling(activeSessionId());
    onCleanup(release);
  });

  const pendingPlanReview = () =>
    composerRunState.sessionId === activeSessionId() ? composerRunState.pendingPlanReview : null;
  const hasPendingPlanReview = () => Boolean(pendingPlanReview());
  const hasPlanTodos = () =>
    sessionTodosState.sessionId === activeSessionId() && sessionTodosState.todos.length > 0;
  const hasPlanContent = () => hasPendingPlanReview() || hasPlanTodos();

  return (
    <div class="navis-workspace-plan">
      <Show
        when={hasPlanContent()}
        fallback={<EmptyState title="No plan for this session" body="Enable Plan mode from the composer to create a plan." />}
      >
        <Show when={pendingPlanReview()}>
          {(review) => (
            <section class="navis-workspace-plan-document">
              <div class="navis-workspace-section-title">Plan review</div>
              <div class="navis-workspace-plan-document-body">
                <MessageContentRenderer content={review().planContent || review().requestText} />
              </div>
            </section>
          )}
        </Show>

        <Show when={hasPlanTodos()}>
          <section class="navis-workspace-section">
            <div class="navis-workspace-section-title">Plan phases</div>
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
                      {todo.priority ?? todoStatusLabel(todo.status)}
                    </span>
                  </div>
                )}
              </For>
            </div>
          </section>
        </Show>
      </Show>
    </div>
  );
};

export default PlanPanel;
