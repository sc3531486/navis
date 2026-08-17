/**
 * EditorTabs 编辑器标签栏组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 EditorTabs.tsx 定义。
 * 渲染编辑器顶部的文件标签栏，支持标签切换、关闭、固定、右键菜单等交互。
 *
 * 功能：
 * - 标签列表渲染（响应式，自动滚动到活跃标签）
 * - 标签切换（点击切换活跃标签）
 * - 标签关闭（点击关闭按钮或中键点击）
 * - 标签固定（右键菜单 → 固定/取消固定）
 * - 标签拖拽排序（预留）
 * - 脏状态指示器（未保存的小圆点）
 * - 语言图标显示（根据文件类型）
 *
 * 设计依据：design/26-editor.md S2 架构设计 EditorTabs.tsx
 */

import { Component, createSignal, For, Show, onCleanup, onMount, createEffect } from 'solid-js'
import { editorState, editorTabsAPI } from './stores/editor'
import {
  closeAllEditorTabsWithGuard,
  closeEditorTabWithGuard,
  closeOtherEditorTabsWithGuard,
} from './stores/editor-close-guard'
import CloseIcon from '../Icon/CloseIcon'
import type { EditorTab } from './types'

// ============================================================
// 语言图标映射
// ============================================================

/**
 * 语言标识 → 图标颜色映射
 *
 * 用于在标签栏中显示与语言对应的彩色圆点。
 * 颜色参考 VS Code 文件图标主题。
 */
const LANGUAGE_COLORS: Record<string, string> = {
  typescript: '#3178c6',
  javascript: '#f7df1e',
  python: '#3572a5',
  rust: '#dea584',
  go: '#00add8',
  java: '#b07219',
  c: '#555555',
  cpp: '#f34b7d',
  html: '#e34c26',
  css: '#563d7c',
  scss: '#c6538c',
  vue: '#41b883',
  svelte: '#ff3e00',
  json: '#000000',
  yaml: '#cb171e',
  markdown: '#083fa1',
  shell: '#89e051',
  sql: '#e38c00',
  text: '#999999',
}

// ============================================================
// EditorTabs 组件
// ============================================================

/**
 * 编辑器标签栏组件
 *
 * @example
 * ```tsx
 * <EditorTabs />
 * ```
 */
interface EditorTabsProps {
  sessionId: string | null
}

export const EditorTabs: Component<EditorTabsProps> = (props) => {
  /** 右键菜单是否显示 */
  const [contextMenu, setContextMenu] = createSignal<{
    visible: boolean
    x: number
    y: number
    tabId: string
  }>({ visible: false, x: 0, y: 0, tabId: '' })

  /** 标签栏容器 DOM 引用（用于自动滚动） */
  let tabBarRef: HTMLDivElement | undefined

  // ---- 自动滚动到活跃标签 ----

  /**
   * 当活跃标签变更时，自动滚动标签栏使其可见
   */
  createEffect(() => {
    const activeId = editorState.activeTabId
    if (!activeId || !tabBarRef) return

    // 查找活跃标签的 DOM 元素
    const activeEl = tabBarRef.querySelector(`[data-tab-id="${activeId}"]`)
    if (activeEl) {
      activeEl.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' })
    }
  })

  // ---- 关闭右键菜单 ----

  /** 点击任意位置关闭右键菜单 */
  const handleDocumentClick = () => {
    setContextMenu((prev) => ({ ...prev, visible: false }))
  }

  onMount(() => {
    document.addEventListener('click', handleDocumentClick)
  })

  onCleanup(() => {
    document.removeEventListener('click', handleDocumentClick)
  })

  // ---- 事件处理 ----

  /**
   * 标签点击 → 切换到该标签
   */
  const handleTabClick = (tab: EditorTab) => {
    editorTabsAPI.switchTo(tab.id)
  }

  /**
   * 关闭按钮点击 → 关闭标签
   */
  const handleCloseClick = (e: MouseEvent, tab: EditorTab) => {
    e.stopPropagation() // 阻止事件冒泡到标签点击
    void closeEditorTabWithGuard(props.sessionId, tab.id)
  }

  /**
   * 中键点击 → 关闭标签
   */
  const handleMiddleClick = (e: MouseEvent, tab: EditorTab) => {
    if (e.button === 1) {
      e.preventDefault()
      void closeEditorTabWithGuard(props.sessionId, tab.id)
    }
  }

  /**
   * 右键点击 → 显示上下文菜单
   */
  const handleContextMenu = (e: MouseEvent, tab: EditorTab) => {
    e.preventDefault()
    setContextMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      tabId: tab.id,
    })
  }

  // ---- 上下文菜单操作 ----

  /** 关闭当前标签 */
  const handleClose = () => {
    void closeEditorTabWithGuard(props.sessionId, contextMenu().tabId)
    setContextMenu((prev) => ({ ...prev, visible: false }))
  }

  /** 关闭其他标签 */
  const handleCloseOthers = () => {
    void closeOtherEditorTabsWithGuard(props.sessionId, contextMenu().tabId)
    setContextMenu((prev) => ({ ...prev, visible: false }))
  }

  /** 关闭所有标签 */
  const handleCloseAll = () => {
    void closeAllEditorTabsWithGuard(props.sessionId)
    setContextMenu((prev) => ({ ...prev, visible: false }))
  }

  /** 切换固定状态 */
  const handleTogglePin = () => {
    editorTabsAPI.togglePin(contextMenu().tabId)
    setContextMenu((prev) => ({ ...prev, visible: false }))
  }

  // ---- 渲染 ----

  return (
    <div class="relative">
      {/* 标签栏主体 */}
      <div
        ref={tabBarRef}
        class="flex items-stretch h-9 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 overflow-x-auto scrollbar-none"
        role="tablist"
        aria-label="编辑器标签"
      >
        <For each={editorState.tabs}>
          {(tab) => (
            <div
              data-tab-id={tab.id}
              role="tab"
              aria-selected={tab.id === editorState.activeTabId}
              class={`
                group flex items-center gap-1.5 px-3 min-w-0 max-w-[200px] cursor-pointer
                border-r border-gray-200 dark:border-gray-700 select-none
                transition-colors duration-100
                ${
                  tab.id === editorState.activeTabId
                    ? 'bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 border-b-2 border-b-blue-500'
                    : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                }
              `}
              onClick={() => handleTabClick(tab)}
              onMouseDown={(e) => handleMiddleClick(e, tab)}
              onContextMenu={(e) => handleContextMenu(e, tab)}
            >
              {/* 固定指示器 */}
              <Show when={tab.isPinned}>
                <svg class="w-3 h-3 shrink-0 text-blue-500" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1-.707.708l-.8-.8-3.535 3.535c.268.59.408 1.236.408 1.9 0 .94-.28 1.87-.828 2.672l-.172.243a.5.5 0 0 1-.756.05L5.17 9.556l-3.536 3.536a.5.5 0 0 1-.707-.708L4.464 8.85.646 5.032a.5.5 0 0 1 .05-.756l.243-.172A4.5 4.5 0 0 1 3.6 3.28c.664 0 1.31.14 1.9.408L9.035 .152l-.8-.8a.5.5 0 0 1 .146-.354l1.447.724z" />
                </svg>
              </Show>

              {/* 语言颜色圆点 */}
              <Show when={!tab.isPinned}>
                <span
                  class="w-2.5 h-2.5 rounded-full shrink-0"
                  style={{ 'background-color': LANGUAGE_COLORS[tab.language] ?? LANGUAGE_COLORS.text }}
                  title={tab.language}
                />
              </Show>

              {/* 文件名 */}
              <span class="truncate text-xs">{tab.fileName}</span>

              {/* 脏状态指示器 */}
              <Show when={tab.isDirty && tab.id !== editorState.activeTabId}>
                <span class="w-2 h-2 rounded-full bg-orange-400 shrink-0" title="未保存" />
              </Show>

              {/* 关闭按钮 */}
              <button
                class={`
                  shrink-0 w-4 h-4 flex items-center justify-center rounded
                  opacity-0 group-hover:opacity-100 hover:bg-gray-200 dark:hover:bg-gray-600
                  transition-opacity duration-100
                  ${tab.isDirty ? 'opacity-100' : ''}
                `}
                onClick={(e) => handleCloseClick(e, tab)}
                title="关闭标签"
                aria-label={`关闭 ${tab.fileName}`}
              >
                <Show
                  when={tab.isDirty}
                  fallback={
                    <CloseIcon class="is-small" />
                  }
                >
                  {/* 脏状态显示小圆点而非关闭图标 */}
                  <span class="w-2 h-2 rounded-full bg-orange-400" />
                </Show>
              </button>
            </div>
          )}
        </For>
      </div>

      {/* 右键上下文菜单 */}
      <Show when={contextMenu().visible}>
        <div
          class="fixed z-50 min-w-[160px] py-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg"
          style={{
            left: `${contextMenu().x}px`,
            top: `${contextMenu().y}px`,
          }}
        >
          {/* 切换固定 */}
          <button
            class="w-full px-3 py-1.5 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-700"
            onClick={handleTogglePin}
          >
            {editorState.tabs.find((t) => t.id === contextMenu().tabId)?.isPinned ? '取消固定' : '固定标签'}
          </button>

          <div class="h-px my-1 bg-gray-200 dark:bg-gray-700" />

          {/* 关闭当前 */}
          <button
            class="w-full px-3 py-1.5 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-700"
            onClick={handleClose}
          >
            关闭
          </button>

          {/* 关闭其他 */}
          <button
            class="w-full px-3 py-1.5 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-700"
            onClick={handleCloseOthers}
          >
            关闭其他
          </button>

          {/* 关闭所有 */}
          <button
            class="w-full px-3 py-1.5 text-left text-xs hover:bg-gray-100 dark:hover:bg-gray-700"
            onClick={handleCloseAll}
          >
            关闭所有
          </button>
        </div>
      </Show>
    </div>
  )
}
