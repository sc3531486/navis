import { Component, For } from 'solid-js';
import HoverTooltip from '../ui/HoverTooltip';
import type { SessionTodoItem } from '../../stores/session-todos';

interface PlanPhaseLineProps {
  phases: SessionTodoItem[];
}

const planPhaseStatusLabel = (status: string): string => {
  switch (status) {
    case 'completed':
      return 'Completed';
    case 'in_progress':
      return 'In progress';
    default:
      return 'Pending';
  }
};

const PlanPhaseLine: Component<PlanPhaseLineProps> = (props) => {
  const completedCount = () =>
    props.phases.filter((phase) => phase.status === 'completed').length;
  const progressPercent = () => {
    const count = props.phases.length;
    if (count <= 1) return completedCount() > 0 ? 100 : 0;
    const activeIndex = props.phases.findIndex((phase) => phase.status === 'in_progress');
    const index = activeIndex >= 0 ? activeIndex : Math.max(0, completedCount() - 1);
    return Math.max(0, Math.min(100, (index / (count - 1)) * 100));
  };

  return (
    <div class="navis-plan-phase-line" style={{ '--navis-plan-progress': `${progressPercent()}%` }}>
      <div class="navis-plan-phase-track" aria-hidden="true" />
      <For each={props.phases}>
        {(phase, index) => (
          <span
            class="navis-plan-phase-node"
            style={{
              left: props.phases.length <= 1 ? '50%' : `${(index() / (props.phases.length - 1)) * 100}%`,
            }}
          >
            <HoverTooltip
              label={`Phase ${index() + 1}: ${phase.content}`}
              detail={`${planPhaseStatusLabel(phase.status)}${phase.priority ? ` · ${phase.priority}` : ''}`}
            >
              <button
                type="button"
                class={`navis-plan-phase-dot is-${phase.status}`}
                aria-label={`Phase ${index() + 1}: ${phase.content}, ${planPhaseStatusLabel(phase.status)}`}
              >
                <span>{phase.status === 'completed' ? '✓' : ''}</span>
              </button>
            </HoverTooltip>
          </span>
        )}
      </For>
    </div>
  );
};

export default PlanPhaseLine;
