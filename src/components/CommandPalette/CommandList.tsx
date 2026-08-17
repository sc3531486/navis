/**
 * CommandList 命令列表组件
 *
 * 职责：
 * 1. 渲染过滤后的命令列表
 * 2. 管理列表的滚动行为（确保选中项可见）
 * 3. 显示空状态提示（无匹配结果时）
 * 4. 按分类分组展示命令（可选）
 *
 * 设计依据：design/23-command-palette.md 第四章"交互设计" 中的列表布局
 */

import { createEffect, For, Show, type JSX } from 'solid-js'
import { CommandItem } from './CommandItem'
import type { Command, CommandScope } from './store'

/**
 * CommandList 组件属性
 */
export interface CommandListProps {
  /** 当前展示的命令列表（已过滤/推荐） */
  commands: Command[]
  /** 当前选中的命令索引 */
  selectedIndex: number
  /** 当前搜索范围 */
  scope: CommandScope | null
  /** 鼠标悬停回调 */
  onMouseEnter: (index: number) => void
  /** 命令点击回调 */
  onClick: (command: Command) => void
  /** 是否显示底部键盘提示 */
  showHints?: boolean
}

/**
 * 搜索范围对应的空状态提示文本
 */
const EMPTY_MESSAGES: Record<string, string> = {
  default: 'No matching commands',
  commands: 'No matching commands',
  files: 'No matching files',
  slash: 'No matching slash commands',
  symbols: 'No matching symbols',
}

/**
 * 搜索范围对应的标题
 */
const SCOPE_TITLES: Record<string, string> = {
  commands: 'Commands',
  files: 'Files',
  slash: 'Slash commands',
  symbols: 'Symbols',
}

/**
 * CommandList 命令列表组件
 *
 * 渲染命令列表，包含：
 * - 滚动容器（限制最大高度）
 * - 分类分组标题（可选）
 * - 命令项列表
 * - 空状态提示
 * - 快捷操作提示栏（底部）
 */
export function CommandList(props: CommandListProps): JSX.Element {
  /** 列表容器引用，用于控制滚动 */
  let listRef: HTMLDivElement | undefined

  /**
   * 选中项变更时，确保选中项在可视区域内
   *
   * 使用 scrollIntoView 实现平滑滚动。
   * block: 'nearest' 确保只在必要时滚动（已在可视区域内则不滚动）。
   */
  createEffect(() => {
    // 依赖 selectedIndex 以在每次变更时触发
    const idx = props.selectedIndex
    if (idx < 0 || !listRef) return

    // 延迟一帧确保 DOM 已更新
    requestAnimationFrame(() => {
      const selectedEl = listRef?.querySelector(`[data-index="${idx}"]`)
      if (selectedEl) {
        selectedEl.scrollIntoView({
          block: 'nearest',
          behavior: 'smooth',
        })
      }
    })
  })

  /**
   * 获取当前列表的标题
   * 有 scope 时显示范围名称，无 scope 时显示 "所有命令"
   */
  const sectionTitle = (): string => {
    const currentScope = props.scope
    if (currentScope) {
      return SCOPE_TITLES[currentScope] ?? 'Results'
    }
    return 'All commands'
  }

  return (
    <div
      ref={listRef}
      class="navis-command-palette-list overflow-y-auto overscroll-contain py-1"
      role="listbox"
      aria-label="Command list"
    >
      {/* 有命令时显示列表 */}
      <Show
        when={props.commands.length > 0}
        fallback={
          /* 空状态提示 */
          <div class="flex flex-col items-center justify-center py-12 px-4 text-center">
            {/* 空状态图标 */}
            <svg
              class="w-12 h-12 text-[var(--color-text-secondary)]/40 mb-3"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              stroke-width="1.5"
              aria-hidden="true"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
              />
            </svg>
            <p class="text-sm text-[var(--color-text-secondary)]">
              {EMPTY_MESSAGES[props.scope ?? 'default'] ?? EMPTY_MESSAGES.default}
            </p>
            <p class="text-xs text-[var(--color-text-secondary)]/60 mt-1">
              Try a different search term
            </p>
          </div>
        }
      >
        {/* 分组标题 */}
        <div class="navis-command-section-title px-3 py-1 text-[var(--color-text-secondary)]">
          {sectionTitle()}
        </div>

        {/* 命令项列表 */}
        <For each={props.commands}>
          {(command, index) => (
            <div data-index={index()}>
              <CommandItem
                command={command}
                isSelected={index() === props.selectedIndex}
                index={index()}
                onMouseEnter={props.onMouseEnter}
                onClick={props.onClick}
              />
            </div>
          )}
        </For>
      </Show>

      {/* 底部操作提示 */}
      <Show when={props.showHints}>
        <div class="navis-command-palette-hints flex items-center gap-3 px-3 py-1.5 mt-1 border-t border-[var(--color-border)] text-[var(--color-text-secondary)]">
          <span class="flex items-center gap-1">
            <kbd class="inline-flex items-center justify-center w-5 h-5 rounded border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-[10px]">
              ↑
            </kbd>
            <kbd class="inline-flex items-center justify-center w-5 h-5 rounded border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-[10px]">
              ↓
            </kbd>
            <span class="ml-1">Navigate</span>
          </span>
          <span class="flex items-center gap-1">
            <kbd class="inline-flex items-center px-1.5 h-5 rounded border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-[10px]">
              Enter
            </kbd>
            <span class="ml-1">Select</span>
          </span>
          <span class="flex items-center gap-1">
            <kbd class="inline-flex items-center px-1.5 h-5 rounded border border-[var(--color-border)] bg-[var(--color-bg-secondary)] font-mono text-[10px]">
              Esc
            </kbd>
            <span class="ml-1">Close</span>
          </span>
          <span class="ml-auto flex items-center gap-1 text-[var(--color-text-secondary)]/60">
            <span>Prefixes:</span>
            <code class="font-mono">&gt;</code>
            <span>Commands</span>
            <code class="font-mono">@</code>
            <span>Files</span>
            <code class="font-mono">/</code>
            <span>Slash</span>
            <code class="font-mono">#</code>
            <span>Symbols</span>
          </span>
        </div>
      </Show>
    </div>
  )
}
