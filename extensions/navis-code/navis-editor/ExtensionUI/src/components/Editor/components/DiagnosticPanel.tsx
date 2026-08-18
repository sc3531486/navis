/**
 * DiagnosticPanel 诊断面板组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 DiagnosticPanel.tsx 定义。
 * 渲染文件的 LSP 诊断信息（错误、警告、信息、提示），支持点击跳转。
 *
 * 功能：
 * - 诊断列表渲染（按严重级别分组和排序）
 * - 严重级别图标和颜色区分
 * - 点击诊断条目跳转到对应位置
 * - 诊断数量统计
 * - 过滤（仅显示错误/警告等）
 *
 * 设计依据：design/26-editor.md S4 LSP 集成 - 诊断标记链路
 */

import { Component, createSignal, For, Show, Match, Switch, createMemo } from 'solid-js'
import type { Diagnostic } from '../types'
import { DiagnosticSeverity } from '../types'

// ============================================================
// 类型定义
// ============================================================

/**
 * DiagnosticPanel 组件 Props
 */
export interface DiagnosticPanelProps {
  /** 诊断列表 */
  diagnostics: Diagnostic[]
  /** 文件路径 */
  filePath: string
  /** 点击诊断条目的跳转回调 */
  onNavigate: (diagnostic: Diagnostic) => void
  /** 是否显示面板 */
  visible?: boolean
}

// ============================================================
// 常量
// ============================================================

/**
 * 严重级别配置映射
 *
 * 包含图标文本、颜色类名和排序权重。
 */
const SEVERITY_CONFIG: Record<
  DiagnosticSeverity,
  { icon: string; colorClass: string; label: string; weight: number }
> = {
  [DiagnosticSeverity.Error]: {
    icon: '✕',
    colorClass: 'text-red-500 bg-red-50 dark:bg-red-900/20',
    label: '错误',
    weight: 1,
  },
  [DiagnosticSeverity.Warning]: {
    icon: '▲',
    colorClass: 'text-yellow-500 bg-yellow-50 dark:bg-yellow-900/20',
    label: '警告',
    weight: 2,
  },
  [DiagnosticSeverity.Information]: {
    icon: '●',
    colorClass: 'text-blue-500 bg-blue-50 dark:bg-blue-900/20',
    label: '信息',
    weight: 3,
  },
  [DiagnosticSeverity.Hint]: {
    icon: '○',
    colorClass: 'text-gray-400 bg-gray-50 dark:bg-gray-800',
    label: '提示',
    weight: 4,
  },
}

// ============================================================
// DiagnosticPanel 组件
// ============================================================

/**
 * 诊断面板组件
 *
 * @example
 * ```tsx
 * <DiagnosticPanel
 *   diagnostics={diagnostics}
 *   filePath="/src/app.ts"
 *   onNavigate={(diag) => jumpToLocation(diag)}
 * />
 * ```
 */
export const DiagnosticPanel: Component<DiagnosticPanelProps> = (props) => {
  // ---- 内部状态 ----

  /** 严重级别过滤器（null 表示显示全部） */
  const [severityFilter, setSeverityFilter] = createSignal<DiagnosticSeverity | null>(null)

  // ---- 计算属性 ----

  /**
   * 过滤并排序后的诊断列表
   *
   * 排序规则：按严重级别（错误 > 警告 > 信息 > 提示），
   * 同级别内按行号排序。
   */
  const filteredDiagnostics = createMemo(() => {
    let diags = [...props.diagnostics]

    // 按严重级别过滤
    if (severityFilter() !== null) {
      diags = diags.filter((d) => d.severity === severityFilter())
    }

    // 按严重级别和行号排序
    return diags.sort((a, b) => {
      const weightDiff =
        (SEVERITY_CONFIG[a.severity]?.weight ?? 99) -
        (SEVERITY_CONFIG[b.severity]?.weight ?? 99)
      if (weightDiff !== 0) return weightDiff
      return a.startLine - b.startLine
    })
  })

  /**
   * 各级别诊断数量统计
   */
  const severityCounts = createMemo(() => {
    const counts: Record<number, number> = {
      [DiagnosticSeverity.Error]: 0,
      [DiagnosticSeverity.Warning]: 0,
      [DiagnosticSeverity.Information]: 0,
      [DiagnosticSeverity.Hint]: 0,
    }
    for (const diag of props.diagnostics) {
      counts[diag.severity] = (counts[diag.severity] ?? 0) + 1
    }
    return counts
  })

  // ---- 渲染 ----

  return (
    <Show when={props.visible !== false}>
      <div class="flex flex-col h-full bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700">
        {/* 面板头部 */}
        <div class="flex items-center justify-between h-8 px-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
          <span class="text-xs font-medium text-gray-600 dark:text-gray-400">
            问题
          </span>

          {/* 严重级别过滤按钮组 */}
          <div class="flex items-center gap-2">
            <For each={[DiagnosticSeverity.Error, DiagnosticSeverity.Warning, DiagnosticSeverity.Information, DiagnosticSeverity.Hint]}>
              {(severity) => {
                const config = SEVERITY_CONFIG[severity]
                const count = severityCounts()[severity] ?? 0
                return (
                  <button
                    class={`
                      flex items-center gap-1 px-1.5 py-0.5 rounded text-xs
                      transition-colors duration-100
                      ${
                        severityFilter() === severity
                          ? config.colorClass
                          : 'text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                      }
                    `}
                    onClick={() =>
                      setSeverityFilter((prev) => (prev === severity ? null : severity))
                    }
                    title={`筛选${config.label}`}
                  >
                    <span>{config.icon}</span>
                    <span>{count}</span>
                  </button>
                )
              }}
            </For>
          </div>
        </div>

        {/* 诊断列表 */}
        <div class="flex-1 overflow-y-auto">
          <Show
            when={filteredDiagnostics().length > 0}
            fallback={
              <div class="flex items-center justify-center py-8 text-xs text-gray-400 dark:text-gray-600">
                {props.diagnostics.length === 0 ? '没有诊断信息' : '过滤结果为空'}
              </div>
            }
          >
            <For each={filteredDiagnostics()}>
              {(diagnostic) => {
                const config = SEVERITY_CONFIG[diagnostic.severity]
                return (
                  <div
                    class={`
                      flex items-start gap-2 px-3 py-1.5 cursor-pointer
                      hover:bg-gray-50 dark:hover:bg-gray-800
                      transition-colors duration-75
                    `}
                    onClick={() => props.onNavigate(diagnostic)}
                    title={`${diagnostic.source}: ${diagnostic.message}`}
                  >
                    {/* 严重级别图标 */}
                    <span class={`shrink-0 w-4 text-center text-xs ${config.colorClass.split(' ')[0]}`}>
                      {config.icon}
                    </span>

                    {/* 诊断消息 */}
                    <span class="flex-1 text-xs text-gray-800 dark:text-gray-200 line-clamp-2">
                      {diagnostic.message}
                    </span>

                    {/* 诊断代码 */}
                    <Show when={diagnostic.code}>
                      <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
                        {diagnostic.code}
                      </span>
                    </Show>

                    {/* 位置信息 */}
                    <span class="shrink-0 text-xs text-gray-400 dark:text-gray-500">
                      {diagnostic.startLine + 1}:{diagnostic.startColumn + 1}
                    </span>
                  </div>
                )
              }}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  )
}
