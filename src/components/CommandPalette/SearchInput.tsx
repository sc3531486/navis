/**
 * SearchInput 搜索输入框组件
 *
 * 职责：
 * 1. 接收用户输入并触发搜索
 * 2. 显示当前搜索范围提示（> 命令 / @ 文件 / / Slash commands / # 符号）
 * 3. 处理键盘事件（转发给 useCommandPalette）
 * 4. 自动聚焦（面板打开时）
 *
 * 设计依据：design/23-command-palette.md 第四章"交互设计"
 */

import { createEffect, onMount, type JSX } from 'solid-js'
import type { CommandScope } from './store'

/**
 * SearchInput 组件属性
 */
export interface SearchInputProps {
  /** 当前查询文本 */
  query: string
  /** 查询变更回调 */
  onQueryChange: (query: string) => void
  /** 键盘事件回调（用于上下导航、确认、关闭） */
  onKeyDown: (e: KeyboardEvent) => void
  /** 当前搜索范围（由前缀触发器决定） */
  scope: CommandScope | null
  /** 占位符文本 */
  placeholder?: string
}

/**
 * 搜索范围对应的提示文本
 *
 * 对应 design/23-command-palette.md 第四章：
 * - ">" → 过滤命令
 * - "@" → 搜索文件
 * - "/" → 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
 * - "#" → 搜索符号
 */
const SCOPE_PLACEHOLDERS: Record<CommandScope, string> = {
  commands: 'Search commands...',
  files: 'Search files...',
  slash: 'Search slash commands...',
  symbols: 'Search symbols...',
}

/**
 * 搜索范围对应的图标文本
 */
const SCOPE_INDICATORS: Record<CommandScope, string> = {
  commands: '> Commands',
  files: '@ Files',
  slash: '/ Slash',
  symbols: '# Symbols',
}

/**
 * SearchInput 搜索输入框组件
 *
 * 使用原生 input 元素，配合 Tailwind CSS 样式。
 * 面板打开时自动聚焦到输入框。
 */
export function SearchInput(props: SearchInputProps): JSX.Element {
  /** input 元素引用，用于自动聚焦 */
  let inputRef: HTMLInputElement | undefined

  /**
   * 面板打开时自动聚焦输入框
   * 使用 createEffect 监听 query 变化（query 在面板打开时被重置为空字符串）
   */
  createEffect(() => {
    // 依赖 props.query 以触发重执行
    props.query
    // 延迟一帧确保 DOM 已渲染
    requestAnimationFrame(() => {
      inputRef?.focus()
    })
  })

  /** 组件挂载后聚焦 */
  onMount(() => {
    inputRef?.focus()
  })

  /** 计算占位符文本 */
  const placeholder = (): string => {
    const currentScope = props.scope
    if (currentScope) {
      return SCOPE_PLACEHOLDERS[currentScope]
    }
    return props.placeholder ?? 'Search commands, files, slash commands, or symbols...'
  }

  /** 处理输入事件 */
  const handleInput: JSX.EventHandlerUnion<HTMLInputElement, InputEvent> = (e) => {
    props.onQueryChange(e.currentTarget.value)
  }

  return (
    <div class="navis-command-search flex items-center gap-2 px-3 py-2 border-b border-[var(--color-border)]">
      {/* 搜索图标 */}
      <svg
        class="w-4 h-4 text-[var(--color-text-secondary)] shrink-0"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        stroke-width="2"
        aria-hidden="true"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
        />
      </svg>

      {/* 搜索范围指示器（有前缀时显示） */}
      {props.scope && (
        <span class="navis-command-scope inline-flex items-center px-1.5 py-0.5 rounded bg-[var(--color-accent)]/10 text-[var(--color-accent)] shrink-0">
          {SCOPE_INDICATORS[props.scope]}
        </span>
      )}

      {/* 搜索输入框 */}
      <input
        ref={inputRef}
        type="text"
        value={props.query}
        onInput={handleInput}
        onKeyDown={props.onKeyDown}
        placeholder={placeholder()}
        class="flex-1 bg-transparent text-[var(--color-text-primary)] placeholder:text-[var(--color-text-secondary)] outline-none text-sm"
        autocomplete="off"
        spellcheck={false}
        aria-label="Command search"
        aria-autocomplete="list"
        role="combobox"
      />
    </div>
  )
}
