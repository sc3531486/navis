/**
 * ============================================================
 * Navis Tooltip 组件 - components/ui/Tooltip.tsx
 * ============================================================
 *
 * 基于 Kobalte Tooltip 封装，结合 Tailwind CSS 样式。
 * 提供鼠标悬停或聚焦时的提示信息。
 *
 * 使用 Kobalte 的 Tooltip 作为底层原语，确保正确的
 * 无障碍语义（aria-describedby、自动定位等）。
 *
 * 来源：design/22-ui-framework.md 第二章 基础组件库
 * ============================================================
 */

import { Component, JSX } from 'solid-js';
import * as Tooltip from '@kobalte/core/tooltip';

// ── 类型定义 ────────────────────────────────────────────

/** 提示框出现位置 */
export type TooltipPlacement =
  | 'top'
  | 'bottom'
  | 'left'
  | 'right'
  | 'top-start'
  | 'top-end'
  | 'bottom-start'
  | 'bottom-end';

/**
 * Tooltip 组件属性。
 */
export interface TooltipProps {
  /** 提示内容 */
  content: JSX.Element;
  /** 提示框位置 */
  placement?: TooltipPlacement;
  /** 显示延迟（毫秒） */
  openDelay?: number;
  /** 是否禁用 */
  disabled?: boolean;
  /** 触发提示的子元素（必须是单个元素） */
  children: JSX.Element;
}

// ── Tooltip 组件 ────────────────────────────────────────

/**
 * 基础提示框组件。
 * 封装 Kobalte Tooltip，在鼠标悬停或键盘聚焦时显示提示信息。
 *
 * @example
 * ```tsx
 * // 基本用法
 * <Tooltip content="点击提交">
 *   <Button>提交</Button>
 * </Tooltip>
 *
 * // 自定义位置和延迟
 * <Tooltip content="右侧提示" placement="right" openDelay={500}>
 *   <span>悬停查看</span>
 * </Tooltip>
 * ```
 */
const TooltipComponent: Component<TooltipProps> = (props) => {
  const placement = () => props.placement ?? 'top';
  const openDelay = () => props.openDelay ?? 300;

  return (
    <Tooltip.Root
      placement={placement()}
      openDelay={openDelay()}
      gutter={4}
    >
      <Tooltip.Trigger>{props.children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          class="z-50 px-2 py-1 text-xs
                 bg-[var(--color-text-primary)] text-[var(--color-bg-primary)]
                 rounded-[var(--radius-sm)]
                 shadow-[var(--shadow-sm)]
                 animate-in fade-in-0 zoom-in-95"
        >
          {props.content}
          <Tooltip.Arrow
            class="fill-[var(--color-text-primary)]"
            size={8}
          />
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
};

export default TooltipComponent;
