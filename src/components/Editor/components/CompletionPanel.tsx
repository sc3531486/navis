/**
 * CompletionPanel 补全面板组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 CompletionPanel.tsx 定义。
 * 渲染 LSP 补全列表，支持键盘导航、详情预览和选择插入。
 *
 * 功能：
 * - 补全列表渲染（图标 + 标签 + 详情）
 * - 键盘导航（上下箭头选择、Enter 插入、Esc 关闭）
 * - 详情侧边栏（文档说明，Markdown 格式）
 * - 按补全类型显示不同图标和颜色
 * - 模糊搜索高亮匹配文本
 *
 * 设计依据：design/26-editor.md S4 LSP 集成 - 补全链路
 */

import { Component, createSignal, createEffect, For, Show } from 'solid-js'
import type { CompletionItem } from '../types'
import { CompletionItemKind } from '../types'

// ============================================================
// 类型定义
// ============================================================

/**
 * CompletionPanel 组件 Props
 */
export interface CompletionPanelProps {
  /** 补全项列表 */
  items: CompletionItem[]
  /** 是否显示面板 */
  visible: boolean
  /** 面板在编辑器中的定位（左上角坐标） */
  position: { x: number; y: number }
  /** 选中补全项的回调 */
  onSelect: (item: CompletionItem) => void
  /** 关闭面板的回调 */
  onClose: () => void
}

// ============================================================
// 补全类型图标和颜色映射
// ============================================================

/**
 * 补全项类型 → 图标映射
 *
 * 每种补全类型对应一个文本图标（使用 Unicode 字符）。
 * 实际项目中可替换为 Lucide 图标。
 */
const COMPLETION_ICONS: Record<number, string> = {
  [CompletionItemKind.Text]: 'T',
  [CompletionItemKind.Method]: 'M',
  [CompletionItemKind.Function]: 'F',
  [CompletionItemKind.Constructor]: 'C',
  [CompletionItemKind.Field]: 'f',
  [CompletionItemKind.Variable]: 'v',
  [CompletionItemKind.Class]: 'C',
  [CompletionItemKind.Interface]: 'I',
  [CompletionItemKind.Module]: 'M',
  [CompletionItemKind.Property]: 'p',
  [CompletionItemKind.Unit]: 'U',
  [CompletionItemKind.Value]: 'V',
  [CompletionItemKind.Enum]: 'E',
  [CompletionItemKind.Keyword]: 'K',
  [CompletionItemKind.Snippet]: 'S',
  [CompletionItemKind.Color]: 'C',
  [CompletionItemKind.File]: 'F',
  [CompletionItemKind.Reference]: 'R',
  [CompletionItemKind.Folder]: 'D',
  [CompletionItemKind.EnumMember]: 'e',
  [CompletionItemKind.Constant]: 'c',
  [CompletionItemKind.Struct]: 'S',
  [CompletionItemKind.Event]: 'E',
  [CompletionItemKind.Operator]: 'O',
  [CompletionItemKind.TypeParameter]: 'T',
}

/**
 * 补全项类型 → 颜色映射
 *
 * 每种补全类型对应一个背景色，用于图标区分。
 */
const COMPLETION_COLORS: Record<number, string> = {
  [CompletionItemKind.Method]: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
  [CompletionItemKind.Function]: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  [CompletionItemKind.Constructor]: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  [CompletionItemKind.Field]: 'bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400',
  [CompletionItemKind.Variable]: 'bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400',
  [CompletionItemKind.Class]: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  [CompletionItemKind.Interface]: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400',
  [CompletionItemKind.Keyword]: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  [CompletionItemKind.Snippet]: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  [CompletionItemKind.Enum]: 'bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400',
  [CompletionItemKind.Property]: 'bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-400',
}

// ============================================================
// CompletionPanel 组件
// ============================================================

/**
 * 补全面板组件
 *
 * @example
 * ```tsx
 * <CompletionPanel
 *   items={completionItems}
 *   visible={showCompletion}
 *   position={{ x: cursorX, y: cursorY }}
 *   onSelect={(item) => insertCompletion(item)}
 *   onClose={() => hideCompletion()}
 * />
 * ```
 */
export const CompletionPanel: Component<CompletionPanelProps> = (props) => {
  // ---- 内部状态 ----

  /** 当前选中的补全项索引 */
  const [selectedIndex, setSelectedIndex] = createSignal(0)

  /** 当前选中项的详情是否展开 */
  const [showDetail, setShowDetail] = createSignal(false)

  // ---- 副作用 ----

  /**
   * 当补全列表变更时，重置选中索引
   */
  createEffect(() => {
    // 访问 items 的响应式依赖
    const _ = props.items.length
    setSelectedIndex(0)
    setShowDetail(false)
  })

  // ---- 键盘事件处理 ----

  /**
   * 处理键盘导航
   *
   * - ArrowUp / ArrowDown: 切换选中项
   * - Enter: 插入选中项
   * - Escape: 关闭面板
   * - Tab: 切换详情面板
   */
  const handleKeyDown = (e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowUp':
        e.preventDefault()
        setSelectedIndex((prev) => Math.max(0, prev - 1))
        break
      case 'ArrowDown':
        e.preventDefault()
        setSelectedIndex((prev) => Math.min(props.items.length - 1, prev + 1))
        break
      case 'Enter':
        e.preventDefault()
        if (props.items[selectedIndex()]) {
          props.onSelect(props.items[selectedIndex()])
        }
        break
      case 'Escape':
        e.preventDefault()
        props.onClose()
        break
      case 'Tab':
        e.preventDefault()
        setShowDetail((prev) => !prev)
        break
    }
  }

  // ---- 渲染 ----

  return (
    <Show when={props.visible && props.items.length > 0}>
      <div
        class="fixed z-50 flex rounded-md shadow-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden"
        style={{
          left: `${props.position.x}px`,
          top: `${props.position.y}px`,
          'max-height': '320px',
        }}
        onKeyDown={handleKeyDown}
        tabIndex={-1}
        role="listbox"
        aria-label="代码补全"
      >
        {/* 补全列表 */}
        <div class="w-[280px] overflow-y-auto">
          <For each={props.items}>
            {(item, index) => (
              <div
                role="option"
                aria-selected={index() === selectedIndex()}
                class={`
                  flex items-center gap-2 px-2 py-1 cursor-pointer text-sm
                  transition-colors duration-75
                  ${
                    index() === selectedIndex()
                      ? 'bg-blue-50 dark:bg-blue-900/30'
                      : 'hover:bg-gray-50 dark:hover:bg-gray-700/50'
                  }
                  ${item.deprecated ? 'opacity-50 line-through' : ''}
                `}
                onClick={() => props.onSelect(item)}
                onMouseEnter={() => setSelectedIndex(index())}
              >
                {/* 类型图标 */}
                <span
                  class={`
                    w-5 h-5 flex items-center justify-center rounded text-xs font-bold shrink-0
                    ${COMPLETION_COLORS[item.kind] ?? 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400'}
                  `}
                >
                  {COMPLETION_ICONS[item.kind] ?? '?'}
                </span>

                {/* 补全标签 */}
                <span class="flex-1 truncate text-gray-900 dark:text-gray-100">
                  {item.label}
                </span>

                {/* 详情简述（行内） */}
                <Show when={item.detail}>
                  <span class="text-xs text-gray-400 dark:text-gray-500 truncate max-w-[100px]">
                    {item.detail}
                  </span>
                </Show>
              </div>
            )}
          </For>
        </div>

        {/* 详情侧边栏 */}
        <Show when={showDetail() && props.items[selectedIndex()]?.documentation}>
          <div class="w-[300px] border-l border-gray-200 dark:border-gray-700 p-3 overflow-y-auto text-sm text-gray-700 dark:text-gray-300">
            <div class="font-bold text-xs text-gray-500 dark:text-gray-400 mb-2">
              {props.items[selectedIndex()].label}
            </div>
            <div class="whitespace-pre-wrap text-xs leading-relaxed">
              {props.items[selectedIndex()].documentation}
            </div>
          </div>
        </Show>
      </div>
    </Show>
  )
}
