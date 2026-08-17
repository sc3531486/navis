/**
 * ============================================================
 * Navis Input 组件 - components/ui/Input.tsx
 * ============================================================
 *
 * 基于 Kobalte TextField 封装，结合 Tailwind CSS 样式。
 * 支持文本输入和文本域两种模式，提供完整的表单集成。
 *
 * 使用 Kobalte 的 TextField 作为底层原语，确保正确的
 * 无障碍语义（label 关联、aria 属性等）。
 *
 * 来源：design/22-ui-framework.md 第二章 基础组件库
 * ============================================================
 */

import { Component, JSX, Show, splitProps } from 'solid-js';
import { TextField } from '@kobalte/core/text-field';

// ── 类型定义 ────────────────────────────────────────────

/** 输入框尺寸类型 */
export type InputSize = 'sm' | 'md' | 'lg';

/**
 * Input 组件属性。
 * 继承 Kobalte TextField.Root 的原生属性。
 */
export interface InputProps {
  /** 输入框标签文本 */
  label?: string;
  /** 占位符文本 */
  placeholder?: string;
  /** 当前值 */
  value?: string;
  /** 默认值（非受控模式） */
  defaultValue?: string;
  /** 值变化回调 */
  onChange?: (value: string) => void;
  /** 输入框尺寸 */
  size?: InputSize;
  /** 是否禁用 */
  disabled?: boolean;
  /** 是否只读 */
  readOnly?: boolean;
  /** 是否必填 */
  required?: boolean;
  /** 错误提示文本 */
  error?: string;
  /** 描述/帮助文本 */
  description?: string;
  /** 输入类型 */
  type?: string;
  /** 是否使用多行文本域 */
  multiline?: boolean;
  /** 多行文本域行数（仅 multiline 为 true 时生效） */
  rows?: number;
  /** 额外的 CSS 类名 */
  class?: string;
  /** 名称属性（用于表单提交） */
  name?: string;
}

// ── 尺寸样式映射 ────────────────────────────────────────

/** 尺寸对应的 Tailwind 类名 */
const SIZE_CLASSES: Record<InputSize, string> = {
  sm: 'h-7 px-2 text-xs',
  md: 'h-9 px-3 text-sm',
  lg: 'h-11 px-4 text-base',
};

// ── Input 组件 ──────────────────────────────────────────

/**
 * 基础输入框组件。
 * 封装 Kobalte TextField，支持 label、错误提示、帮助文本。
 *
 * @example
 * ```tsx
 * // 单行输入
 * <Input label="用户名" placeholder="请输入用户名" />
 *
 * // 带错误提示
 * <Input label="邮箱" error="邮箱格式不正确" />
 *
 * // 多行文本域
 * <Input label="描述" multiline rows={4} />
 *
 * // 受控模式
 * <Input value={name()} onChange={setName} />
 * ```
 */
const Input: Component<InputProps> = (props) => {
  const size = () => props.size ?? 'md';

  /** 输入框/文本域共用的基础类名 */
  const inputClass = () =>
    [
      'w-full rounded-[var(--radius-md)]',
      'border border-[var(--color-border)]',
      'bg-[var(--color-bg-primary)] text-[var(--color-text-primary)]',
      'placeholder:text-[var(--color-text-secondary)]',
      'focus:outline-none focus:ring-2 focus:ring-[var(--color-accent)]',
      'disabled:opacity-50 disabled:cursor-not-allowed',
      'read-only:bg-[var(--color-bg-secondary)]',
      SIZE_CLASSES[size()],
      props.class ?? '',
    ]
      .filter(Boolean)
      .join(' ');

  return (
    <TextField
      class="flex flex-col gap-1"
      value={props.value}
      defaultValue={props.defaultValue}
      onChange={props.onChange}
      name={props.name}
      disabled={props.disabled}
      readOnly={props.readOnly}
      required={props.required}
    >
      {/* 标签 */}
      <Show when={props.label}>
        <TextField.Label class="text-sm font-medium text-[var(--color-text-primary)]">
          {props.label}
        </TextField.Label>
      </Show>

      {/* 输入框或文本域 */}
      <Show
        when={props.multiline}
        fallback={
          <TextField.Input
            class={inputClass()}
            type={props.type ?? 'text'}
            placeholder={props.placeholder}
          />
        }
      >
        <TextField.TextArea
          class={inputClass()}
          placeholder={props.placeholder}
          rows={props.rows ?? 3}
          autoResize
        />
      </Show>

      {/* 描述文本 */}
      <Show when={props.description && !props.error}>
        <TextField.Description class="text-xs text-[var(--color-text-secondary)]">
          {props.description}
        </TextField.Description>
      </Show>

      {/* 错误提示 */}
      <Show when={props.error}>
        <TextField.ErrorMessage class="text-xs text-[var(--color-error)]">
          {props.error}
        </TextField.ErrorMessage>
      </Show>
    </TextField>
  );
};

export default Input;
