// 兼容别名：SlotRenderer 已并入 DynamicSlot（读取全局 slotStore）。
// 保留旧签名以兼容早期调用方；ctx 参数仅用于历史兼容，不再依赖。
import { Component, type JSX } from 'solid-js';
import type { NavisContext } from './context';
import { DynamicSlot } from './slots/DynamicSlot';

export interface SlotRendererProps {
  ctx: NavisContext;
  target: string;
  class?: string;
  fallback?: JSX.Element;
}

export const SlotRenderer: Component<SlotRendererProps> = (props) => {
  void props.ctx;
  return <DynamicSlot name={props.target} class={props.class} fallback={props.fallback} />;
};

export default SlotRenderer;