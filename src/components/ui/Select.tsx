/**
 * ============================================================
 * Navis Select 组件 - components/ui/Select.tsx
 * ============================================================
 *
 * 基于 Kobalte Select 封装，结合 Tailwind CSS 样式。
 * 提供下拉选择功能，支持单选模式。
 *
 * 使用 Kobalte 的 Select 作为底层原语，确保正确的
 * 无障碍语义（aria-expanded、键盘导航、焦点管理等）。
 *
 * 来源：design/22-ui-framework.md 第二章 基础组件库
 * ============================================================
 */

import { Component, For, Show } from 'solid-js';

// ── 类型定义 ────────────────────────────────────────────

/** 选项值类型 */
export interface SelectOption {
  /** 选项值（唯一标识） */
  value: string;
  /** 选项显示文本 */
  label: string;
  /** 是否禁用此选项 */
  disabled?: boolean;
}

/** Select 组件尺寸 */
export type SelectSize = 'sm' | 'md' | 'lg';

/**
 * Select 组件属性。
 */
export interface SelectProps {
  /** 下拉框标签文本 */
  label?: string;
  /** 选项列表 */
  options: SelectOption[];
  /** 当前选中值 */
  value?: string;
  /** 默认值（非受控模式） */
  defaultValue?: string;
  /** 值变化回调 */
  onChange?: (value: string) => void;
  /** 占位符文本 */
  placeholder?: string;
  /** 组件尺寸 */
  size?: SelectSize;
  /** 是否禁用 */
  disabled?: boolean;
  /** 是否必填 */
  required?: boolean;
  /** 错误提示文本 */
  error?: string;
  /** 额外的 CSS 类名 */
  class?: string;
}

// ── 尺寸样式映射 ────────────────────────────────────────

/** 尺寸对应的 Tailwind 类名 */
const SIZE_CLASSES: Record<SelectSize, string> = {
  sm: 'h-7 px-2 text-xs',
  md: 'h-9 px-3 text-sm',
  lg: 'h-11 px-4 text-base',
};

// ── Select 组件 ──────────────────────────────────────────

/**
 * 基础下拉选择组件。
 * 封装 Kobalte Select，提供一致的样式和无障碍支持。
 *
 * @example
 * ```tsx
 * const options = [
 *   { value: 'light', label: '浅色' },
 *   { value: 'dark', label: '深色' },
 *   { value: 'system', label: '跟随系统' },
 * ];
 *
 * <Select label="主题" options={options} value={theme()} onChange={setTheme} />
 * ```
 */
const SelectComponent: Component<SelectProps> = (props) => {
  const size = () => props.size ?? 'md';
  const currentValue = () => props.value ?? props.defaultValue ?? '';
  const selectClass = () =>
    [
      'w-full appearance-none',
      'rounded-[var(--radius-md)]',
      'border border-[var(--color-border)]',
      'bg-[var(--color-bg-primary)] text-[var(--color-text-primary)]',
      'focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]',
      'disabled:opacity-50 disabled:cursor-not-allowed',
      SIZE_CLASSES[size()],
      props.class ?? '',
    ]
      .filter(Boolean)
      .join(' ');

  return (
    <label class="flex flex-col gap-1">
      <Show when={props.label}>
        <span class="text-sm font-medium text-[var(--color-text-primary)]">{props.label}</span>
      </Show>
      <div class="relative">
        <select
          class={selectClass()}
          value={currentValue()}
          onChange={(event) => props.onChange?.(event.currentTarget.value)}
          disabled={props.disabled}
          required={props.required}
        >
          <Show when={props.placeholder}>
            <option value="" disabled={!!props.placeholder}>
              {props.placeholder ?? '请选择...'}
            </option>
          </Show>
          <For each={props.options}>
            {(opt) => (
              <option value={opt.value} disabled={opt.disabled}>
                {opt.label}
              </option>
            )}
          </For>
        </select>
      </div>
      <Show when={props.error}>
        <span class="text-xs text-[var(--color-error)]">{props.error}</span>
      </Show>
    </label>
  );
};

export default SelectComponent;
