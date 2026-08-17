/**
 * ============================================================
 * Navis Button 组件 - components/ui/Button.tsx
 * ============================================================
 *
 * 基于 Kobalte Button 封装，结合 Tailwind CSS 样式。
 * 支持多种变体、尺寸、禁用状态和加载状态。
 *
 * 使用 Kobalte 的 Button 作为底层原语，确保正确的
 * 无障碍语义（role="button"、键盘事件等）。
 *
 * 来源：design/22-ui-framework.md 第二章 基础组件库
 * ============================================================
 */

import { Component, JSX, Show, splitProps } from 'solid-js';
import { Button as KobalteButton } from '@kobalte/core/button';
import { LoadingSpinner } from './LoadingSpinner';

// ── 类型定义 ────────────────────────────────────────────

/** 按钮变体类型 */
export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

/** 按钮尺寸类型 */
export type ButtonSize = 'sm' | 'md' | 'lg';

/**
 * Button 组件属性。
 */
export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  /** 按钮变体（影响颜色和样式） */
  variant?: ButtonVariant;
  /** 按钮尺寸 */
  size?: ButtonSize;
  /** 是否禁用 */
  disabled?: boolean;
  /** 是否显示加载状态 */
  loading?: boolean;
  /** 子元素 */
  children: JSX.Element;
}

// ── 变体样式映射 ────────────────────────────────────────

/** 变体对应的 Tailwind 类名 */
const VARIANT_CLASSES: Record<ButtonVariant, string> = {
  primary:
    'bg-[var(--color-accent)] text-white hover:opacity-90 active:opacity-80',
  secondary:
    'bg-[var(--color-bg-secondary)] text-[var(--color-text-primary)] border border-[var(--color-border)] hover:bg-[var(--color-bg-primary)]',
  ghost:
    'bg-transparent text-[var(--color-text-primary)] hover:bg-[var(--color-bg-secondary)]',
  danger:
    'bg-[var(--color-error)] text-white hover:opacity-90 active:opacity-80',
};

/** 尺寸对应的 Tailwind 类名 */
const SIZE_CLASSES: Record<ButtonSize, string> = {
  sm: 'h-7 px-2 text-xs rounded-[var(--radius-sm)]',
  md: 'h-9 px-3 text-sm rounded-[var(--radius-md)]',
  lg: 'h-11 px-4 text-base rounded-[var(--radius-md)]',
};

// ── Button 组件 ──────────────────────────────────────────

/**
 * 基础按钮组件。
 * 封装 Kobalte Button，提供统一的变体、尺寸和状态管理。
 *
 * @example
 * ```tsx
 * <Button variant="primary" size="md">确认</Button>
 * <Button variant="danger" loading>删除中...</Button>
 * <Button variant="ghost" disabled>已禁用</Button>
 * ```
 */
const Button: Component<ButtonProps> = (props) => {
  /** 分离自定义属性与原生按钮属性 */
  const [local, rest] = splitProps(props, [
    'variant',
    'size',
    'disabled',
    'loading',
    'children',
    'class',
  ]);

  /** 默认值 */
  const variant = () => local.variant ?? 'primary';
  const size = () => local.size ?? 'md';
  const isDisabled = () => local.disabled || local.loading;

  /** 组合类名 */
  const className = () =>
    [
      'inline-flex items-center justify-center gap-1.5',
      'font-medium transition-opacity cursor-pointer',
      'disabled:opacity-50 disabled:cursor-not-allowed',
      VARIANT_CLASSES[variant()],
      SIZE_CLASSES[size()],
      local.class ?? '',
    ]
      .filter(Boolean)
      .join(' ');

  return (
    <KobalteButton
      {...rest}
      class={className()}
      disabled={isDisabled()}
    >
      <Show when={local.loading}>
        <LoadingSpinner />
      </Show>
      {local.children}
    </KobalteButton>
  );
};

export default Button;
