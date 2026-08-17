/**
 * DiffView Diff 对比视图组件
 *
 * 严格遵循 design/26-editor.md 第五章"Diff 视图"设计。
 * 支持 Unified（上下对照）和 Side-by-side（左右对照）两种显示模式，
 * 提供逐块确认/拒绝、全部确认/拒绝、手动编辑 Diff 的交互能力。
 *
 * 功能：
 * - Unified 模式：上下对照，适合小改动
 * - Side-by-side 模式：左右对照，适合大改动
 * - 逐块确认/拒绝（每个 Hunk 独立操作）
 * - 全部确认/拒绝（一键操作所有 Hunk）
 * - 变更行高亮（新增行绿色、删除行红色）
 * - 行号显示（原始/修改两侧行号）
 *
 * 设计依据：design/26-editor.md S5 Diff 视图
 */

import { Component, createSignal, For, Show, Match, Switch } from 'solid-js'
import type { DiffViewProps, DiffHunk, DiffChange, DiffMode } from './types'
import {
  computeDiff,
  DIFF_LINE_CLASSES,
  DIFF_LINE_PREFIX,
  formatHunkHeader,
} from './extensions/diff-extension'

// ============================================================
// DiffView 组件
// ============================================================

/**
 * Diff 对比视图组件
 *
 * @example
 * ```tsx
 * <DiffView
 *   filePath="/src/app.ts"
 *   original={originalText}
 *   modified={modifiedText}
 *   hunks={diffHunks}
 *   onConfirm={() => applyChanges()}
 *   onReject={() => discardChanges()}
 * />
 * ```
 */
export const DiffView: Component<DiffViewProps> = (props) => {
  // ---- 内部状态 ----

  /** Diff 显示模式 */
  const [mode, setMode] = createSignal<DiffMode>('unified')

  /** 各 Hunk 的确认/拒绝状态映射（hunkId → 'confirmed' | 'rejected' | undefined） */
  const [hunkStates, setHunkStates] = createSignal<Map<string, 'confirmed' | 'rejected'>>(
    new Map(),
  )

  /**
   * 计算 Diff 数据
   *
   * 如果 props.hunks 为空，则使用 diff-extension 自动计算；
   * 否则使用传入的 hunks 数据。
   */
  const diffHunks = (): DiffHunk[] => {
    if (props.hunks.length > 0) return props.hunks
    return computeDiff(props.original, props.modified)
  }

  // ---- Hunk 操作 ----

  /**
   * 确认单个 Hunk
   *
   * @param hunkId Hunk ID
   */
  const confirmHunk = (hunkId: string) => {
    setHunkStates((prev) => {
      const next = new Map(prev)
      next.set(hunkId, 'confirmed')
      return next
    })
    props.onConfirmHunk?.(hunkId)
  }

  /**
   * 拒绝单个 Hunk
   *
   * @param hunkId Hunk ID
   */
  const rejectHunk = (hunkId: string) => {
    setHunkStates((prev) => {
      const next = new Map(prev)
      next.set(hunkId, 'rejected')
      return next
    })
    props.onRejectHunk?.(hunkId)
  }

  /**
   * 确认所有 Hunk
   */
  const confirmAll = () => {
    for (const hunk of diffHunks()) {
      setHunkStates((prev) => {
        const next = new Map(prev)
        next.set(hunk.id, 'confirmed')
        return next
      })
    }
    props.onConfirm()
  }

  /**
   * 拒绝所有 Hunk
   */
  const rejectAll = () => {
    for (const hunk of diffHunks()) {
      setHunkStates((prev) => {
        const next = new Map(prev)
        next.set(hunk.id, 'rejected')
        return next
      })
    }
    props.onReject()
  }

  /**
   * 获取 Hunk 状态样式
   *
   * @param hunkId Hunk ID
   * @returns CSS 类名
   */
  const hunkStateClass = (hunkId: string): string => {
    const state = hunkStates().get(hunkId)
    if (state === 'confirmed') return 'opacity-50 bg-green-50/30 dark:bg-green-900/10'
    if (state === 'rejected') return 'opacity-50 bg-red-50/30 dark:bg-red-900/10'
    return ''
  }

  // ---- 渲染 ----

  return (
    <div class="flex flex-col h-full bg-white dark:bg-gray-900">
      {/* 工具栏 */}
      <div class="flex items-center justify-between h-10 px-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        {/* 左侧：文件路径和模式切换 */}
        <div class="flex items-center gap-3">
          <span class="text-xs text-gray-500 dark:text-gray-400 font-mono truncate max-w-[300px]">
            {props.filePath}
          </span>

          {/* 模式切换按钮组 */}
          <div class="flex items-center border border-gray-300 dark:border-gray-600 rounded overflow-hidden">
            <button
              class={`
                px-2 py-0.5 text-xs transition-colors
                ${mode() === 'unified'
                  ? 'bg-blue-500 text-white'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                }
              `}
              onClick={() => setMode('unified')}
            >
              Unified
            </button>
            <button
              class={`
                px-2 py-0.5 text-xs transition-colors border-l border-gray-300 dark:border-gray-600
                ${mode() === 'side-by-side'
                  ? 'bg-blue-500 text-white'
                  : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                }
              `}
              onClick={() => setMode('side-by-side')}
            >
              Side-by-side
            </button>
          </div>
        </div>

        {/* 右侧：确认/拒绝按钮 */}
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1 text-xs bg-green-500 hover:bg-green-600 text-white rounded transition-colors"
            onClick={confirmAll}
          >
            全部接受
          </button>
          <button
            class="px-3 py-1 text-xs bg-red-500 hover:bg-red-600 text-white rounded transition-colors"
            onClick={rejectAll}
          >
            全部拒绝
          </button>
        </div>
      </div>

      {/* Diff 内容区域 */}
      <div class="flex-1 overflow-auto font-mono text-sm leading-relaxed">
        <Switch>
          {/* Unified 模式 */}
          <Match when={mode() === 'unified'}>
            <UnifiedDiffView
              hunks={diffHunks()}
              hunkStates={hunkStates()}
              onConfirmHunk={confirmHunk}
              onRejectHunk={rejectHunk}
              hunkStateClass={hunkStateClass}
            />
          </Match>

          {/* Side-by-side 模式 */}
          <Match when={mode() === 'side-by-side'}>
            <SideBySideDiffView
              hunks={diffHunks()}
              hunkStates={hunkStates()}
              onConfirmHunk={confirmHunk}
              onRejectHunk={rejectHunk}
              hunkStateClass={hunkStateClass}
            />
          </Match>
        </Switch>
      </div>
    </div>
  )
}

// ============================================================
// Unified Diff 子组件
// ============================================================

/**
 * Unified Diff 子组件 Props
 */
interface UnifiedDiffViewProps {
  hunks: DiffHunk[]
  hunkStates: Map<string, 'confirmed' | 'rejected'>
  onConfirmHunk: (hunkId: string) => void
  onRejectHunk: (hunkId: string) => void
  hunkStateClass: (hunkId: string) => string
}

/**
 * Unified 模式 Diff 视图
 *
 * 上下对照显示差异，适合小改动。
 * 每行显示：原始行号 | 修改行号 | 前缀(+/-/ ) | 行内容
 */
const UnifiedDiffView: Component<UnifiedDiffViewProps> = (props) => {
  return (
    <div class="min-w-full">
      <For each={props.hunks}>
        {(hunk) => (
          <div class={`border-b border-gray-200 dark:border-gray-700 ${props.hunkStateClass(hunk.id)}`}>
            {/* Hunk 头部 */}
            <div class="flex items-center justify-between px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
              <span class="text-xs text-gray-500 dark:text-gray-400">
                {formatHunkHeader(hunk)}
              </span>
              {/* Hunk 操作按钮 */}
              <div class="flex items-center gap-1.5">
                <button
                  class="px-2 py-0.5 text-xs bg-green-100 hover:bg-green-200 dark:bg-green-900/30 dark:hover:bg-green-900/50 text-green-700 dark:text-green-400 rounded transition-colors"
                  onClick={() => props.onConfirmHunk(hunk.id)}
                >
                  接受
                </button>
                <button
                  class="px-2 py-0.5 text-xs bg-red-100 hover:bg-red-200 dark:bg-red-900/30 dark:hover:bg-red-900/50 text-red-700 dark:text-red-400 rounded transition-colors"
                  onClick={() => props.onRejectHunk(hunk.id)}
                >
                  拒绝
                </button>
              </div>
            </div>

            {/* 变更行列表 */}
            <For each={hunk.changes}>
              {(change) => (
                <UnifiedDiffLine change={change} />
              )}
            </For>
          </div>
        )}
      </For>

      {/* 空状态 */}
      <Show when={props.hunks.length === 0}>
        <div class="flex items-center justify-center py-16 text-gray-400 dark:text-gray-600 text-sm">
          没有差异
        </div>
      </Show>
    </div>
  )
}

/**
 * Unified 模式单行变更组件
 */
const UnifiedDiffLine: Component<{ change: DiffChange }> = (props) => {
  /** 原始行号（左侧） */
  const origLine = () => props.change.originalLine?.toString() ?? ''
  /** 修改行号（右侧） */
  const modLine = () => props.change.modifiedLine?.toString() ?? ''

  return (
    <div class={`flex ${DIFF_LINE_CLASSES[props.change.type]}`}>
      {/* 原始行号 */}
      <span class="w-12 shrink-0 text-right pr-2 text-xs text-gray-400 dark:text-gray-600 select-none">
        {origLine()}
      </span>
      {/* 修改行号 */}
      <span class="w-12 shrink-0 text-right pr-2 text-xs text-gray-400 dark:text-gray-600 select-none">
        {modLine()}
      </span>
      {/* 前缀标记 */}
      <span class={`
        w-5 shrink-0 text-center select-none font-bold
        ${props.change.type === 'addition' ? 'text-green-600 dark:text-green-400' : ''}
        ${props.change.type === 'deletion' ? 'text-red-600 dark:text-red-400' : ''}
      `}>
        {DIFF_LINE_PREFIX[props.change.type]}
      </span>
      {/* 行内容 */}
      <pre class="flex-1 px-2 overflow-x-auto whitespace-pre">{props.change.content}</pre>
    </div>
  )
}

// ============================================================
// Side-by-side Diff 子组件
// ============================================================

/**
 * Side-by-side Diff 子组件 Props
 */
interface SideBySideDiffViewProps {
  hunks: DiffHunk[]
  hunkStates: Map<string, 'confirmed' | 'rejected'>
  onConfirmHunk: (hunkId: string) => void
  onRejectHunk: (hunkId: string) => void
  hunkStateClass: (hunkId: string) => string
}

/**
 * Side-by-side 模式 Diff 视图
 *
 * 左右对照显示差异，适合大改动。
 * 左侧显示原始文件，右侧显示修改后文件。
 */
const SideBySideDiffView: Component<SideBySideDiffViewProps> = (props) => {
  return (
    <div class="min-w-full">
      {/* 表头 */}
      <div class="flex border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 sticky top-0 z-10">
        <div class="flex-1 px-3 py-1 text-xs text-gray-500 dark:text-gray-400 border-r border-gray-200 dark:border-gray-700">
          原始文件
        </div>
        <div class="flex-1 px-3 py-1 text-xs text-gray-500 dark:text-gray-400">
          修改后
        </div>
      </div>

      <For each={props.hunks}>
        {(hunk) => {
          // 将 changes 分为原始侧和修改侧
          const originalChanges = hunk.changes.filter(
            (c) => c.type === 'context' || c.type === 'deletion',
          )
          const modifiedChanges = hunk.changes.filter(
            (c) => c.type === 'context' || c.type === 'addition',
          )

          return (
            <div class={props.hunkStateClass(hunk.id)}>
              {/* Hunk 头部 */}
              <div class="flex items-center justify-between px-3 py-1.5 bg-gray-100 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <span class="text-xs text-gray-500 dark:text-gray-400">
                  {formatHunkHeader(hunk)}
                </span>
                <div class="flex items-center gap-1.5">
                  <button
                    class="px-2 py-0.5 text-xs bg-green-100 hover:bg-green-200 dark:bg-green-900/30 dark:hover:bg-green-900/50 text-green-700 dark:text-green-400 rounded transition-colors"
                    onClick={() => props.onConfirmHunk(hunk.id)}
                  >
                    接受
                  </button>
                  <button
                    class="px-2 py-0.5 text-xs bg-red-100 hover:bg-red-200 dark:bg-red-900/30 dark:hover:bg-red-900/50 text-red-700 dark:text-red-400 rounded transition-colors"
                    onClick={() => props.onRejectHunk(hunk.id)}
                  >
                    拒绝
                  </button>
                </div>
              </div>

              {/* 双栏对比 */}
              <div class="flex border-b border-gray-200 dark:border-gray-700">
                {/* 左侧：原始文件 */}
                <div class="flex-1 border-r border-gray-200 dark:border-gray-700">
                  <For each={originalChanges}>
                    {(change) => (
                      <div class={`flex ${change.type === 'deletion' ? DIFF_LINE_CLASSES.deletion : ''}`}>
                        <span class="w-10 shrink-0 text-right pr-2 text-xs text-gray-400 select-none">
                          {change.originalLine ?? ''}
                        </span>
                        <span class="w-4 shrink-0 text-center font-bold text-red-600 dark:text-red-400 select-none">
                          {change.type === 'deletion' ? '-' : ''}
                        </span>
                        <pre class="flex-1 px-2 overflow-x-auto whitespace-pre">{change.content}</pre>
                      </div>
                    )}
                  </For>
                </div>

                {/* 右侧：修改后文件 */}
                <div class="flex-1">
                  <For each={modifiedChanges}>
                    {(change) => (
                      <div class={`flex ${change.type === 'addition' ? DIFF_LINE_CLASSES.addition : ''}`}>
                        <span class="w-10 shrink-0 text-right pr-2 text-xs text-gray-400 select-none">
                          {change.modifiedLine ?? ''}
                        </span>
                        <span class="w-4 shrink-0 text-center font-bold text-green-600 dark:text-green-400 select-none">
                          {change.type === 'addition' ? '+' : ''}
                        </span>
                        <pre class="flex-1 px-2 overflow-x-auto whitespace-pre">{change.content}</pre>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </div>
          )
        }}
      </For>

      {/* 空状态 */}
      <Show when={props.hunks.length === 0}>
        <div class="flex items-center justify-center py-16 text-gray-400 dark:text-gray-600 text-sm">
          没有差异
        </div>
      </Show>
    </div>
  )
}
