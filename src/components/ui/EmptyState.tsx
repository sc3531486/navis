/**
 * 公共空状态组件
 *
 * 用于展示空状态占位信息，如无内容、无数据、加载失败等场景。
 * 提供标题和正文两段式布局，使用统一的 CSS 类名样式。
 */

import { Component } from 'solid-js';

/**
 * 空状态组件属性
 */
export interface EmptyStateProps {
  /** 空状态标题 */
  title: string;
  /** 空状态说明文本 */
  body: string;
}

/**
 * 空状态占位组件
 *
 * @example
 * ```tsx
 * <EmptyState title="No results" body="Try adjusting your search criteria." />
 * ```
 */
export const EmptyState: Component<EmptyStateProps> = (props) => (
  <div class="navis-workspace-empty">
    <div class="navis-workspace-empty-title">{props.title}</div>
    <div class="navis-workspace-empty-body">{props.body}</div>
  </div>
);
