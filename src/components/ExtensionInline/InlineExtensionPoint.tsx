/**
 * inline 扩展点通用渲染组件（阶段 6，design/34 §阶段6）。
 *
 * fail-closed：无 command 的 inline 项不渲染也不报错；渲染为轻量触发按钮，
 * 点击经 executeExtensionPoint 派发扩展命令（动作族：RunScript/OpenView 等）。
 * 宿主通过 class 传入各自的样式。
 */
import { Component } from 'solid-js';
import type { UiExtensionPointRegistration } from '@/lib/extension-ui';
import { executeExtensionPoint } from '@/stores/extension-points';

interface InlineExtensionPointProps {
  point: UiExtensionPointRegistration;
  class?: string;
}

const InlineExtensionPoint: Component<InlineExtensionPointProps> = (props) => {
  if (!props.point.command) return null;
  return (
    <button
      type="button"
      class={props.class}
      title={props.point.label ?? props.point.id}
      aria-label={props.point.label ?? props.point.id}
      onClick={() => executeExtensionPoint(props.point)}
    >
      {props.point.label ?? props.point.id}
    </button>
  );
};

export default InlineExtensionPoint;