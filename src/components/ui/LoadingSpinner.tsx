/**
 * 公共加载状态组件
 *
 * 提供统一的加载旋转指示器，用于懒加载占位、按钮 loading 状态等场景。
 * 同时导出 LoadingPlaceholder 作为加载占位符的别名。
 */

import { Component } from 'solid-js';

/**
 * 加载旋转指示器组件
 *
 * @example
 * ```tsx
 * <LoadingSpinner text="加载中..." />
 * <LoadingSpinner /> // 默认文本 "Loading..."
 * ```
 */
export const LoadingSpinner: Component<{ text?: string }> = (props) => (
  <div class="flex items-center justify-center p-4 text-[13px] text-[#888]">
    <svg class="mr-2 h-4 w-4 animate-spin" viewBox="0 0 16 16" fill="none">
      <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" stroke-dasharray="25.12" stroke-dashoffset="8" />
    </svg>
    {props.text ?? 'Loading...'}
  </div>
);

/**
 * 加载占位符（LoadingSpinner 的别名）
 * 用于扩展点懒加载时的统一占位符样式。
 */
export const LoadingPlaceholder = LoadingSpinner;
