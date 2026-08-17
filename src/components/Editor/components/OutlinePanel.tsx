/**
 * OutlinePanel 大纲面板组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 OutlinePanel.tsx 定义。
 * 渲染当前文件的符号大纲（函数、类、变量等），支持点击跳转。
 *
 * 功能：
 * - 符号树渲染（函数、类、接口、变量等）
 * - 符号层级展示（嵌套结构）
 * - 点击符号跳转到对应位置
 * - 符号类型图标和颜色区分
 * - 折叠/展开嵌套符号
 *
 * 设计依据：design/26-editor.md S2 架构设计 OutlinePanel.tsx
 */

import { Component, createSignal, For, Show, createMemo } from 'solid-js'

// ============================================================
// 类型定义
// ============================================================

/**
 * 文档符号（对应 LSP DocumentSymbol）
 *
 * 描述文件中的一个代码符号节点。
 */
export interface DocumentSymbol {
  /** 符号名称 */
  name: string
  /** 符号类型 */
  kind: SymbolKind
  /** 所在行号（0-based） */
  line: number
  /** 所在列号（0-based） */
  column: number
  /** 符号详情（如类型签名） */
  detail?: string
  /** 子符号列表（如类的方法、接口的属性） */
  children?: DocumentSymbol[]
}

/**
 * 符号类型枚举
 * 对应 LSP 协议的 SymbolKind
 */
export enum SymbolKind {
  File = 1,
  Module = 2,
  Namespace = 3,
  Package = 4,
  Class = 5,
  Method = 6,
  Property = 7,
  Field = 8,
  Constructor = 9,
  Enum = 10,
  Interface = 11,
  Function = 12,
  Variable = 13,
  Constant = 14,
  String = 15,
  Number = 16,
  Boolean = 17,
  Array = 18,
  Object = 19,
  Key = 20,
  Null = 21,
  EnumMember = 22,
  Struct = 23,
  Event = 24,
  Operator = 25,
  TypeParameter = 26,
}

/**
 * OutlinePanel 组件 Props
 */
export interface OutlinePanelProps {
  /** 符号列表 */
  symbols: DocumentSymbol[]
  /** 点击符号的跳转回调 */
  onNavigate: (line: number, column: number) => void
  /** 当前光标行号（用于高亮当前符号） */
  currentLine?: number
  /** 是否显示面板 */
  visible?: boolean
}

// ============================================================
// 常量
// ============================================================

/**
 * 符号类型 → 图标和颜色映射
 */
const SYMBOL_CONFIG: Record<number, { icon: string; color: string }> = {
  [SymbolKind.File]: { icon: '📄', color: 'text-gray-500' },
  [SymbolKind.Module]: { icon: '📦', color: 'text-blue-500' },
  [SymbolKind.Class]: { icon: 'C', color: 'text-yellow-500' },
  [SymbolKind.Method]: { icon: 'M', color: 'text-purple-500' },
  [SymbolKind.Property]: { icon: 'p', color: 'text-cyan-500' },
  [SymbolKind.Field]: { icon: 'f', color: 'text-cyan-500' },
  [SymbolKind.Constructor]: { icon: 'C', color: 'text-blue-500' },
  [SymbolKind.Enum]: { icon: 'E', color: 'text-orange-500' },
  [SymbolKind.Interface]: { icon: 'I', color: 'text-yellow-500' },
  [SymbolKind.Function]: { icon: 'ƒ', color: 'text-blue-500' },
  [SymbolKind.Variable]: { icon: 'v', color: 'text-teal-500' },
  [SymbolKind.Constant]: { icon: 'c', color: 'text-teal-500' },
  [SymbolKind.EnumMember]: { icon: 'e', color: 'text-orange-400' },
  [SymbolKind.Struct]: { icon: 'S', color: 'text-yellow-500' },
  [SymbolKind.Event]: { icon: 'E', color: 'text-yellow-500' },
  [SymbolKind.TypeParameter]: { icon: 'T', color: 'text-teal-500' },
}

// ============================================================
// OutlinePanel 组件
// ============================================================

/**
 * 大纲面板组件
 *
 * @example
 * ```tsx
 * <OutlinePanel
 *   symbols={documentSymbols}
 *   onNavigate={(line, col) => jumpToPosition(line, col)}
 *   currentLine={cursorLine}
 * />
 * ```
 */
export const OutlinePanel: Component<OutlinePanelProps> = (props) => {
  /** 已折叠的符号名称集合 */
  const [collapsed, setCollapsed] = createSignal<Set<string>>(new Set())

  /**
   * 切换符号的折叠状态
   */
  const toggleCollapse = (symbolName: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev)
      if (next.has(symbolName)) {
        next.delete(symbolName)
      } else {
        next.add(symbolName)
      }
      return next
    })
  }

  /**
   * 判断当前符号是否包含光标行
   * 用于高亮当前所在符号
   */
  const isActiveSymbol = (symbol: DocumentSymbol): boolean => {
    if (props.currentLine === undefined) return false
    return symbol.line === props.currentLine
  }

  return (
    <Show when={props.visible !== false}>
      <div class="flex flex-col h-full bg-white dark:bg-gray-900 border-l border-gray-200 dark:border-gray-700">
        {/* 面板头部 */}
        <div class="flex items-center h-8 px-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <span class="text-xs font-medium text-gray-600 dark:text-gray-400">
            大纲
          </span>
        </div>

        {/* 符号树 */}
        <div class="flex-1 overflow-y-auto py-1">
          <Show
            when={props.symbols.length > 0}
            fallback={
              <div class="flex items-center justify-center py-8 text-xs text-gray-400 dark:text-gray-600">
                暂无符号信息
              </div>
            }
          >
            <For each={props.symbols}>
              {(symbol) => (
                <SymbolNode
                  symbol={symbol}
                  depth={0}
                  isCollapsed={collapsed().has(symbol.name)}
                  isActive={isActiveSymbol(symbol)}
                  onToggle={toggleCollapse}
                  onNavigate={props.onNavigate}
                  currentLine={props.currentLine}
                  collapsed={collapsed()}
                />
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  )
}

// ============================================================
// SymbolNode 子组件
// ============================================================

/**
 * SymbolNode 子组件 Props
 */
interface SymbolNodeProps {
  symbol: DocumentSymbol
  depth: number
  isCollapsed: boolean
  isActive: boolean
  onToggle: (name: string) => void
  onNavigate: (line: number, column: number) => void
  currentLine?: number
  collapsed: Set<string>
}

/**
 * 单个符号节点组件（递归渲染子符号）
 */
const SymbolNode: Component<SymbolNodeProps> = (props) => {
  const config = SYMBOL_CONFIG[props.symbol.kind] ?? { icon: '?', color: 'text-gray-500' }
  const hasChildren = () => props.symbol.children && props.symbol.children.length > 0

  return (
    <div>
      {/* 符号行 */}
      <div
        class={`
          flex items-center gap-1.5 py-0.5 pr-2 cursor-pointer
          hover:bg-gray-50 dark:hover:bg-gray-800
          transition-colors duration-75
          ${props.isActive ? 'bg-blue-50 dark:bg-blue-900/20 border-l-2 border-blue-500' : ''}
        `}
        style={{ 'padding-left': `${props.depth * 16 + 8}px` }}
        onClick={() => props.onNavigate(props.symbol.line, props.symbol.column)}
      >
        {/* 折叠/展开箭头（有子符号时显示） */}
        <Show
          when={hasChildren()}
          fallback={<span class="w-4 shrink-0" />}
        >
          <button
            class="w-4 h-4 shrink-0 flex items-center justify-center text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            onClick={(e) => {
              e.stopPropagation()
              props.onToggle(props.symbol.name)
            }}
          >
            <svg
              class={`w-3 h-3 transition-transform duration-150 ${props.isCollapsed ? '' : 'rotate-90'}`}
              viewBox="0 0 12 12"
              fill="currentColor"
            >
              <path d="M4.5 2l4 4-4 4" />
            </svg>
          </button>
        </Show>

        {/* 符号图标 */}
        <span class={`w-4 text-center text-xs font-bold shrink-0 ${config.color}`}>
          {config.icon}
        </span>

        {/* 符号名称 */}
        <span class="flex-1 truncate text-xs text-gray-800 dark:text-gray-200">
          {props.symbol.name}
        </span>

        {/* 符号详情（类型签名） */}
        <Show when={props.symbol.detail}>
          <span class="text-xs text-gray-400 dark:text-gray-500 truncate max-w-[120px]">
            {props.symbol.detail}
          </span>
        </Show>
      </div>

      {/* 子符号列表（递归渲染） */}
      <Show when={hasChildren() && !props.isCollapsed}>
        <For each={props.symbol.children}>
          {(child) => (
            <SymbolNode
              symbol={child}
              depth={props.depth + 1}
              isCollapsed={props.collapsed.has(child.name)}
              isActive={props.currentLine === child.line}
              onToggle={props.onToggle}
              onNavigate={props.onNavigate}
              currentLine={props.currentLine}
              collapsed={props.collapsed}
            />
          )}
        </For>
      </Show>
    </div>
  )
}
