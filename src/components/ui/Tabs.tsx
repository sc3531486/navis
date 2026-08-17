/**
 * ============================================================
 * Navis Tabs 组件 - components/ui/Tabs.tsx
 * ============================================================
 *
 * 基于 Kobalte Tabs 封装，结合 Tailwind CSS 样式。
 * 提供标签页切换功能，支持受控和非受控模式。
 *
 * 使用 Kobalte 的 Tabs 作为底层原语，确保正确的
 * 无障碍语义（role="tablist"/"tab"/"tabpanel"、键盘导航等）。
 *
 * 来源：design/22-ui-framework.md 第二章 基础组件库
 * ============================================================
 */

import { Component, JSX, For, Show, splitProps } from 'solid-js';
import { Tabs } from '@kobalte/core/tabs';

// ── 类型定义 ────────────────────────────────────────────

/** 单个标签页配置 */
export interface TabItem {
  /** 标签唯一 ID（也用作 value） */
  id: string;
  /** 标签显示文本 */
  label: string;
  /** 标签内容 */
  content: JSX.Element;
  /** 是否禁用此标签 */
  disabled?: boolean;
}

/** Tabs 组件尺寸 */
export type TabsSize = 'sm' | 'md';

/**
 * Tabs 组件属性。
 */
export interface TabsProps {
  /** 标签页列表 */
  tabs: TabItem[];
  /** 当前激活的标签 ID */
  value?: string;
  /** 默认激活的标签 ID（非受控模式） */
  defaultValue?: string;
  /** 标签切换回调 */
  onChange?: (value: string) => void;
  /** 组件尺寸 */
  size?: TabsSize;
  /** 额外的 CSS 类名 */
  class?: string;
}

// ── 尺寸样式映射 ────────────────────────────────────────

/** 标签触发器尺寸类名 */
const TRIGGER_SIZE_CLASSES: Record<TabsSize, string> = {
  sm: 'px-2 py-1 text-xs',
  md: 'px-3 py-1.5 text-sm',
};

// ── Tabs 组件 ──────────────────────────────────────────

/**
 * 基础标签页组件。
 * 封装 Kobalte Tabs，提供一致的样式和无障碍支持。
 *
 * @example
 * ```tsx
 * const tabs = [
 *   { id: 'chat', label: '聊天', content: <ChatView /> },
 *   { id: 'editor', label: '编辑器', content: <EditorView /> },
 *   { id: 'terminal', label: '终端', content: <TerminalView /> },
 * ];
 *
 * <Tabs tabs={tabs} value={activeTab()} onChange={setActiveTab} />
 * ```
 */
const TabsComponent: Component<TabsProps> = (props) => {
  const size = () => props.size ?? 'md';

  /** 标签触发器类名 */
  const triggerClass = () =>
    [
      'relative cursor-pointer transition-colors',
      'text-[var(--color-text-secondary)]',
      'hover:text-[var(--color-text-primary)]',
      'data-[selected]:text-[var(--color-accent)]',
      'disabled:opacity-50 disabled:cursor-not-allowed',
      'focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)]',
      TRIGGER_SIZE_CLASSES[size()],
    ]
      .filter(Boolean)
      .join(' ');

  return (
    <Tabs
      class={['flex flex-col', props.class ?? ''].filter(Boolean).join(' ')}
      value={props.value}
      defaultValue={props.defaultValue}
      onChange={props.onChange}
    >
      {/* ── 标签列表 ── */}
      <Tabs.List
        class="flex items-center border-b border-[var(--color-border)]
               bg-[var(--color-bg-secondary)] overflow-x-auto flex-shrink-0"
      >
        <For each={props.tabs}>
          {(tab) => (
            <Tabs.Trigger
              value={tab.id}
              class={triggerClass()}
              disabled={tab.disabled}
            >
              {tab.label}
              {/* 选中指示条 */}
              <Tabs.Indicator
                class="absolute bottom-0 left-0 right-0 h-0.5
                       bg-[var(--color-accent)] transition-transform"
              />
            </Tabs.Trigger>
          )}
        </For>
      </Tabs.List>

      {/* ── 标签内容 ── */}
      <For each={props.tabs}>
        {(tab) => (
          <Tabs.Content
            value={tab.id}
            class="flex-1 overflow-y-auto p-3"
          >
            {tab.content}
          </Tabs.Content>
        )}
      </For>
    </Tabs>
  );
};

export default TabsComponent;
