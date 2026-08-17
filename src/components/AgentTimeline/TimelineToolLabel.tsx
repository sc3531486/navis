import { Component, Show, createEffect, createSignal, onCleanup } from 'solid-js';
import ShimmerText from '../ui/ShimmerText';
import type { ChatAgentTimelinePart } from '../../lib/stream';
import {
  agentTimelinePartLabel,
  editLabelParts,
  isLiveTimelinePart,
  timelineToolKind,
} from './tool-presentation';

const RollingTraceCount: Component<{ value: string; kind: 'add' | 'delete' }> = (props) => {
  const [displayValue, setDisplayValue] = createSignal('');
  const [previousValue, setPreviousValue] = createSignal('');
  const [animating, setAnimating] = createSignal(false);
  let timer: number | undefined;

  createEffect(() => {
    const nextValue = props.value;
    const currentValue = displayValue();
    if (nextValue === currentValue) return;
    setPreviousValue(currentValue);
    setDisplayValue(nextValue);
    setAnimating(true);
    if (timer !== undefined) window.clearTimeout(timer);
    timer = window.setTimeout(() => setAnimating(false), 260);
  });

  onCleanup(() => {
    if (timer !== undefined) window.clearTimeout(timer);
  });

  return (
    <span class={`navis-agent-trace-count is-${props.kind} ${animating() ? 'is-rolling' : ''}`}>
      <Show when={animating() && previousValue()}>
        <span class="navis-agent-trace-count-old">{previousValue()}</span>
      </Show>
      <span class="navis-agent-trace-count-new">{displayValue()}</span>
    </span>
  );
};

export const AgentTimelinePartLabel: Component<{ part: ChatAgentTimelinePart }> = (props) => {
  const label = () => agentTimelinePartLabel(props.part);
  const isEdit = () => timelineToolKind(props.part) === 'edit';
  const active = () => isLiveTimelinePart(props.part);
  const parts = () => editLabelParts(label());
  return (
    <Show when={isEdit()} fallback={<ShimmerText active={active()}>{label()}</ShimmerText>}>
      <ShimmerText active={active()}>{parts().main}</ShimmerText>
      <Show when={parts().additions}>
        {(value) => <RollingTraceCount value={value()} kind="add" />}
      </Show>
      <Show when={parts().deletions}>
        {(value) => <RollingTraceCount value={value()} kind="delete" />}
      </Show>
    </Show>
  );
};
