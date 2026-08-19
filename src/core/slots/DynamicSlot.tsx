// 动态插槽渲染组件：从 slotStore 读取贡献并渲染，支持递归嵌套子插槽。
import { Component, createSignal, onCleanup, onMount, Show, For, type JSX } from 'solid-js';
import { slotStore } from './SlotStore';
import { componentRegistry } from '../components/ComponentRegistry';

export interface DynamicSlotProps {
  name: string;
  class?: string;
  fallback?: JSX.Element;
}

export const DynamicSlot: Component<DynamicSlotProps> = (props) => {
  const [tick, setTick] = createSignal(0);

  onMount(() => {
    const unsubSlots = slotStore.subscribe(() => setTick((t) => t + 1));
    const unsubComponents = componentRegistry.subscribe(() => setTick((t) => t + 1));
    onCleanup(() => {
      unsubSlots();
      unsubComponents();
    });
  });

  void tick;
  const items = () => slotStore.getContributions(props.name);

  return (
    <Show when={items().length > 0} fallback={props.fallback}>
      <div class={props.class} data-navis-slot={props.name}>
        <For each={items()}>
          {(item) => <>{item.component() as JSX.Element}</>}
        </For>
      </div>
    </Show>
  );
};

export default DynamicSlot;