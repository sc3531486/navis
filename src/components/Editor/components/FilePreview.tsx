/**
 * FilePreview 文件预览组件
 *
 * 严格遵循 design/26-editor.md 第六章"多模态输入渲染"中的 FilePreview.tsx 定义。
 * 在 Chat 消息气泡内渲染文件预览卡片，展示文件名、大小、类型图标和内容摘要。
 *
 * 功能：
 * - 文件卡片渲染（文件名、文件大小、文件类型图标）
 * - 内容摘要展示（代码文件展示前几行预览）
 * - 点击展开/收起查看完整内容
 * - 大文件截取提示（"内容已截取，共 N 行，显示前 MAX_FILE_LINES 行"）
 * - 支持移除附件
 *
 * 设计依据：design/26-editor.md S6.5.2 FilePreview 组件设计
 */

import { Component, createSignal, Show, createMemo } from 'solid-js'
import type { FileAttachment, FilePreviewProps } from '../types'
import { MAX_FILE_LINES } from '../stores/editor'

// ============================================================
// 常量
// ============================================================

/**
 * 文件类型图标和颜色映射
 *
 * 根据文件类型分类返回对应的图标文本和颜色类名。
 */
const FILE_TYPE_CONFIG: Record<string, { icon: string; colorClass: string; label: string }> = {
  code: {
    icon: '</>',
    colorClass: 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400',
    label: '代码文件',
  },
  document: {
    icon: 'D',
    colorClass: 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400',
    label: '文档文件',
  },
  config: {
    icon: 'C',
    colorClass: 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400',
    label: '配置文件',
  },
  unknown: {
    icon: '?',
    colorClass: 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400',
    label: '未知类型',
  },
}

// ============================================================
// FilePreview 组件
// ============================================================

/**
 * 文件预览组件
 *
 * @example
 * ```tsx
 * <FilePreview
 *   attachment={fileAttachment}
 *   removable={true}
 *   maxPreviewLines={10}
 *   onRemove={(id) => removeFile(id)}
 *   onExpand={(id) => expandFile(id)}
 * />
 * ```
 */
export const FilePreview: Component<FilePreviewProps> = (props) => {
  // ---- 内部状态 ----

  /** 是否展开显示完整内容 */
  const [isExpanded, setIsExpanded] = createSignal(false)

  // ---- 计算属性 ----

  /** 文件类型配置 */
  const typeConfig = () =>
    FILE_TYPE_CONFIG[props.attachment.fileType] ?? FILE_TYPE_CONFIG.unknown

  /** 格式化文件大小 */
  const formattedSize = createMemo(() => {
    const bytes = props.attachment.sizeBytes
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  })

  /** 预览行数 */
  const maxPreviewLines = () => props.maxPreviewLines ?? 10

  /**
   * 预览文本内容
   *
   * 根据展开状态决定显示的文本行数：
   * - 收起状态：显示前 maxPreviewLines 行
   * - 展开状态：显示全部（截取后的内容）
   */
  const previewContent = createMemo(() => {
    const text = props.attachment.textContent
    if (!text) return null

    const lines = text.split('\n')

    if (isExpanded()) {
      return lines.join('\n')
    }

    // 收起状态：仅显示前 N 行
    return lines.slice(0, maxPreviewLines()).join('\n')
  })

  /**
   * 是否有更多内容可以展开
   */
  const hasMoreContent = createMemo(() => {
    const text = props.attachment.textContent
    if (!text) return false
    const lines = text.split('\n')
    return lines.length > maxPreviewLines()
  })

  // ---- 事件处理 ----

  /**
   * 切换展开/收起状态
   */
  const toggleExpand = () => {
    setIsExpanded((prev) => !prev)
    props.onExpand?.(props.attachment.id)
  }

  /**
   * 移除文件
   */
  const handleRemove = () => {
    props.onRemove?.(props.attachment.id)
  }

  // ---- 渲染 ----

  return (
    <div class="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-hidden">
      {/* 文件信息头部 */}
      <div class="flex items-center gap-3 px-3 py-2">
        {/* 文件类型图标 */}
        <div
          class={`
            w-9 h-9 flex items-center justify-center rounded-md text-xs font-bold shrink-0
            ${typeConfig().colorClass}
          `}
        >
          {typeConfig().icon}
        </div>

        {/* 文件名和大小 */}
        <div class="flex-1 min-w-0">
          <div class="text-sm text-gray-800 dark:text-gray-200 truncate font-medium">
            {props.attachment.fileName}
          </div>
          <div class="flex items-center gap-2 text-xs text-gray-400 dark:text-gray-500">
            <span>{formattedSize()}</span>
            <span class="text-gray-300 dark:text-gray-600">|</span>
            <span>{typeConfig().label}</span>
            <Show when={props.attachment.totalLines}>
              <span class="text-gray-300 dark:text-gray-600">|</span>
              <span>{props.attachment.totalLines} 行</span>
            </Show>
          </div>
        </div>

        {/* 操作按钮 */}
        <div class="flex items-center gap-1 shrink-0">
          {/* 展开/收起按钮 */}
          <Show when={props.attachment.textContent && (hasMoreContent() || isExpanded())}>
            <button
              class="w-7 h-7 flex items-center justify-center text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              onClick={toggleExpand}
              title={isExpanded() ? '收起' : '展开'}
            >
              <svg
                class={`w-4 h-4 transition-transform duration-200 ${isExpanded() ? 'rotate-180' : ''}`}
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <polyline points="6 9 12 15 18 9" />
              </svg>
            </button>
          </Show>

          {/* 移除按钮 */}
          <Show when={props.removable}>
            <button
              class="w-7 h-7 flex items-center justify-center text-gray-400 hover:text-red-500 rounded hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
              onClick={handleRemove}
              title="移除文件"
            >
              <svg class="w-4 h-4" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M3 3l6 6M9 3l-6 6" />
              </svg>
            </button>
          </Show>
        </div>
      </div>

      {/* 大文件截取提示 */}
      <Show when={props.attachment.isTruncated}>
        <div class="px-3 py-1.5 bg-orange-50 dark:bg-orange-900/20 border-t border-orange-200 dark:border-orange-800 text-xs text-orange-600 dark:text-orange-400">
          内容已截取，共 {props.attachment.totalLines} 行，显示前 {MAX_FILE_LINES} 行
        </div>
      </Show>

      {/* 内容预览区域 */}
      <Show when={previewContent() !== null}>
        <div class={`
          border-t border-gray-100 dark:border-gray-700
          ${isExpanded() ? 'max-h-[400px] overflow-y-auto' : ''}
        `}>
          <pre class="px-3 py-2 text-xs text-gray-700 dark:text-gray-300 font-mono leading-relaxed whitespace-pre-wrap break-all overflow-x-auto">
            {previewContent()}
          </pre>

          {/* 收起状态下的渐变遮罩 */}
          <Show when={!isExpanded() && hasMoreContent()}>
            <div class="relative h-8 -mt-8 bg-gradient-to-t from-white dark:from-gray-800 to-transparent pointer-events-none" />
          </Show>
        </div>
      </Show>
    </div>
  )
}
