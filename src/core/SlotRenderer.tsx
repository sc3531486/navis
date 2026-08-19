import { Component, createSignal, onCleanup, onMount, For, Show, type JSX } from 'solid-js';
import { NavisContext } from './context';

export interface SlotRendererProps {
  ctx: NavisContext;
  target: string;
  class?: string;
  fallback?: JSX.Element;
}

export const SlotRenderer: Component<SlotRendererProps> = (props) => {
  const [tick, setTick] = createSignal(0);

  onMount(() => {
    const unsub = props.ctx.on(`slot:${props.target}:updated`, () => {
      setTick((t) => t + 1);
    });
    onCleanup(unsub);
  });

  const items = () => props.ctx.getSlotItems(props.target);

  return (
    <Show when={items().length > 0} fallback={props.fallback}>
      <div class={props.class} data-navis-slot={props.target}>
        <For each={items()}>
          {(item) => <>{item.component()}</>}
        </For>
      </div>
    </Show>
  );
};