/**
 * ImagePreview 图片预览组件
 *
 * 严格遵循 design/26-editor.md 第六章"多模态输入渲染"中的 ImagePreview.tsx 定义。
 * 提供图片缩略图展示、Lightbox 全屏查看和标注工具。
 *
 * 功能：
 * - 内联缩略图展示（自适应容器宽度）
 * - Lightbox 全屏查看（支持滚轮缩放 / 拖拽平移）
 * - 标注工具栏（圈选矩形、箭头、文字）
 * - 标注对象管理（添加/删除/修改）
 * - 标注导出（返回标注元数据）
 *
 * 设计依据：design/26-editor.md S6.2 ImagePreview 组件设计
 */

import { Component, createSignal, Show, For, onMount, onCleanup } from 'solid-js'
import type { ImageAttachment, ImageAnnotation, ImagePreviewMode } from '../types'

// ============================================================
// 类型定义
// ============================================================

/**
 * ImagePreview 组件 Props
 *
 * 对应 design/26-editor.md S6.2 ImagePreview 组件的 Props 定义。
 */
export interface ImagePreviewProps {
  /** 图片附件数据 */
  attachment: ImageAttachment
  /** 已有的标注列表 */
  annotations?: ImageAnnotation[]
  /** 是否启用标注模式，默认 false */
  editable?: boolean
  /** 缩略图最大宽度，默认 400px */
  maxWidth?: number
  /** 标注变更回调 */
  onAnnotationChange?: (annotations: ImageAnnotation[]) => void
  /** 标注导出回调 */
  onExport?: (exportedImage: Blob, annotations: ImageAnnotation[]) => void
}

// ============================================================
// 常量
// ============================================================

/** 默认标注颜色列表 */
const ANNOTATION_COLORS = [
  '#ef4444', // red
  '#f97316', // orange
  '#eab308', // yellow
  '#22c55e', // green
  '#3b82f6', // blue
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#ffffff', // white
]

/** 默认标注颜色 */
const DEFAULT_ANNOTATION_COLOR = '#ef4444'

// ============================================================
// ImagePreview 组件
// ============================================================

/**
 * 图片预览组件
 *
 * @example
 * ```tsx
 * <ImagePreview
 *   attachment={imageAttachment}
 *   editable={true}
 *   onAnnotationChange={(anns) => updateAnnotations(anns)}
 * />
 * ```
 */
export const ImagePreview: Component<ImagePreviewProps> = (props) => {
  // ---- 内部状态 ----

  /** 当前渲染模式 */
  const [mode, setMode] = createSignal<ImagePreviewMode>('thumbnail')

  /** Lightbox 缩放比例 */
  const [scale, setScale] = createSignal(1)

  /** Lightbox 平移偏移 */
  const [pan, setPan] = createSignal({ x: 0, y: 0 })

  /** 当前正在绘制的标注类型 */
  const [annotationTool, setAnnotationTool] = createSignal<'rect' | 'arrow' | 'text' | null>(null)

  /** 当前标注颜色 */
  const [annotationColor, setAnnotationColor] = createSignal(DEFAULT_ANNOTATION_COLOR)

  /** 内部标注列表 */
  const [annotations, setAnnotations] = createSignal<ImageAnnotation[]>(
    props.annotations ?? [],
  )

  /** Lightbox 拖拽状态 */
  let isPanning = false
  let panStart = { x: 0, y: 0 }

  /** 图片容器引用 */
  let imageContainerRef: HTMLDivElement | undefined

  // ---- 计算属性 ----

  /** 图片 Data URL */
  const imageUrl = () =>
    `data:${props.attachment.mimeType};base64,${props.attachment.dataBase64}`

  /** 缩略图显示尺寸 */
  const thumbnailSize = () => {
    const maxWidth = props.maxWidth ?? 400
    const ratio = props.attachment.width / props.attachment.height
    const width = Math.min(props.attachment.width, maxWidth)
    const height = width / ratio
    return { width, height }
  }

  // ---- Lightbox 交互 ----

  /**
   * 打开 Lightbox
   */
  const openLightbox = () => {
    setMode('lightbox')
    setScale(1)
    setPan({ x: 0, y: 0 })
  }

  /**
   * 关闭 Lightbox / 标注模式
   */
  const closeOverlay = () => {
    setMode('thumbnail')
    setAnnotationTool(null)
  }

  /**
   * 滚轮缩放
   */
  const handleWheel = (e: WheelEvent) => {
    if (mode() === 'thumbnail') return
    e.preventDefault()

    const delta = e.deltaY > 0 ? -0.1 : 0.1
    setScale((prev) => Math.max(0.1, Math.min(5, prev + delta)))
  }

  /**
   * 拖拽平移开始
   */
  const handlePanStart = (e: MouseEvent) => {
    if (mode() === 'thumbnail' || annotationTool()) return
    isPanning = true
    panStart = { x: e.clientX - pan().x, y: e.clientY - pan().y }
  }

  /**
   * 拖拽平移中
   */
  const handlePanMove = (e: MouseEvent) => {
    if (!isPanning) return
    setPan({
      x: e.clientX - panStart.x,
      y: e.clientY - panStart.y,
    })
  }

  /**
   * 拖拽平移结束
   */
  const handlePanEnd = () => {
    isPanning = false
  }

  // ---- 标注工具 ----

  /**
   * 开始标注模式
   */
  const startAnnotate = () => {
    setMode('annotate')
  }

  /**
   * 图片点击（标注模式下添加标注）
   */
  const handleImageClick = (e: MouseEvent) => {
    if (mode() !== 'annotate' || !annotationTool()) return

    const target = e.currentTarget as HTMLElement
    const rect = target.getBoundingClientRect()

    // 计算相对坐标（0-1 比例值）
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height

    const newAnnotation: ImageAnnotation = {
      id: `ann-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      attachmentId: props.attachment.id,
      type: annotationTool()!,
      x,
      y,
      color: annotationColor(),
      // 默认尺寸（rect 类型）
      ...(annotationTool() === 'rect' ? { width: 0.1, height: 0.1 } : {}),
      // 默认文本（text 类型）
      ...(annotationTool() === 'text' ? { text: '注释' } : {}),
    }

    const updated = [...annotations(), newAnnotation]
    setAnnotations(updated)
    props.onAnnotationChange?.(updated)
  }

  /**
   * 删除标注
   */
  const removeAnnotation = (id: string) => {
    const updated = annotations().filter((a) => a.id !== id)
    setAnnotations(updated)
    props.onAnnotationChange?.(updated)
  }

  // ---- 生命周期 ----

  onMount(() => {
    document.addEventListener('mousemove', handlePanMove)
    document.addEventListener('mouseup', handlePanEnd)
  })

  onCleanup(() => {
    document.removeEventListener('mousemove', handlePanMove)
    document.removeEventListener('mouseup', handlePanEnd)
  })

  // ---- 渲染 ----

  return (
    <div class="relative">
      {/* ========== 缩略图模式 ========== */}
      <div
        ref={imageContainerRef}
        class="relative inline-block cursor-pointer rounded-md overflow-hidden border border-gray-200 dark:border-gray-700 hover:border-blue-400 transition-colors"
        style={{
          'max-width': `${thumbnailSize().width}px`,
        }}
        onClick={openLightbox}
      >
        <img
          src={imageUrl()}
          alt={props.attachment.fileName}
          class="block max-w-full h-auto"
          loading="lazy"
        />

        {/* 图片信息覆盖层 */}
        <div class="absolute bottom-0 left-0 right-0 px-2 py-1 bg-gradient-to-t from-black/50 to-transparent">
          <span class="text-xs text-white truncate block">
            {props.attachment.fileName}
          </span>
          <span class="text-[10px] text-white/70">
            {props.attachment.width} x {props.attachment.height} | {(props.attachment.sizeBytes / 1024).toFixed(1)} KB
          </span>
        </div>
      </div>

      {/* ========== Lightbox / 标注模式 ========== */}
      <Show when={mode() !== 'thumbnail'}>
        <div
          class="fixed inset-0 z-50 bg-black/80 flex flex-col"
          onWheel={handleWheel}
        >
          {/* 工具栏 */}
          <div class="flex items-center justify-between h-12 px-4 bg-gray-900/90 border-b border-gray-700">
            {/* 左侧信息 */}
            <div class="flex items-center gap-3">
              <span class="text-sm text-white font-medium">{props.attachment.fileName}</span>
              <span class="text-xs text-gray-400">
                {props.attachment.width} x {props.attachment.height}
              </span>
            </div>

            {/* 中间标注工具（仅标注模式） */}
            <Show when={mode() === 'annotate'}>
              <div class="flex items-center gap-2">
                {/* 标注类型按钮 */}
                <For each={[
                  { type: 'rect' as const, label: '矩形', icon: '▢' },
                  { type: 'arrow' as const, label: '箭头', icon: '→' },
                  { type: 'text' as const, label: '文字', icon: 'T' },
                ]}>
                  {(tool) => (
                    <button
                      class={`
                        w-8 h-8 flex items-center justify-center rounded text-sm
                        transition-colors duration-100
                        ${annotationTool() === tool.type
                          ? 'bg-blue-500 text-white'
                          : 'text-gray-300 hover:bg-gray-700'
                        }
                      `}
                      onClick={() => setAnnotationTool(
                        annotationTool() === tool.type ? null : tool.type,
                      )}
                      title={tool.label}
                    >
                      {tool.icon}
                    </button>
                  )}
                </For>

                {/* 颜色选择器 */}
                <div class="flex items-center gap-1 ml-2">
                  <For each={ANNOTATION_COLORS}>
                    {(color) => (
                      <button
                        class={`
                          w-5 h-5 rounded-full border-2 transition-transform
                          ${annotationColor() === color ? 'border-white scale-110' : 'border-transparent'}
                        `}
                        style={{ 'background-color': color }}
                        onClick={() => setAnnotationColor(color)}
                      />
                    )}
                  </For>
                </div>
              </div>
            </Show>

            {/* 右侧操作按钮 */}
            <div class="flex items-center gap-2">
              <Show when={props.editable && mode() !== 'annotate'}>
                <button
                  class="px-3 py-1 text-xs bg-blue-500 hover:bg-blue-600 text-white rounded transition-colors"
                  onClick={startAnnotate}
                >
                  标注
                </button>
              </Show>

              {/* 缩放比例 */}
              <span class="text-xs text-gray-400">
                {Math.round(scale() * 100)}%
              </span>

              {/* 关闭按钮 */}
              <button
                class="w-8 h-8 flex items-center justify-center text-gray-400 hover:text-white rounded hover:bg-gray-700 transition-colors"
                onClick={closeOverlay}
              >
                <svg class="w-5 h-5" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M3 3l6 6M9 3l-6 6" />
                </svg>
              </button>
            </div>
          </div>

          {/* 图片查看区域 */}
          <div
            class="flex-1 flex items-center justify-center overflow-hidden"
            onMouseDown={handlePanStart}
          >
            <div
              class="relative"
              style={{
                transform: `translate(${pan().x}px, ${pan().y}px) scale(${scale()})`,
                'transform-origin': 'center center',
                transition: isPanning ? 'none' : 'transform 0.1s ease-out',
              }}
            >
              <img
                src={imageUrl()}
                alt={props.attachment.fileName}
                class="max-w-[90vw] max-h-[80vh] object-contain"
                onClick={handleImageClick}
                draggable={false}
              />

              {/* 标注层 */}
              <svg
                class="absolute inset-0 w-full h-full pointer-events-none"
                viewBox={`0 0 ${props.attachment.width} ${props.attachment.height}`}
              >
                <For each={annotations()}>
                  {(annotation) => (
                    <g>
                      {/* 矩形标注 */}
                      <Show when={annotation.type === 'rect'}>
                        <rect
                          x={annotation.x * props.attachment.width}
                          y={annotation.y * props.attachment.height}
                          width={(annotation.width ?? 0.1) * props.attachment.width}
                          height={(annotation.height ?? 0.1) * props.attachment.height}
                          fill="none"
                          stroke={annotation.color}
                          stroke-width="3"
                          rx="2"
                        />
                      </Show>

                      {/* 箭头标注 */}
                      <Show when={annotation.type === 'arrow'}>
                        <line
                          x1={annotation.x * props.attachment.width}
                          y1={annotation.y * props.attachment.height}
                          x2={(annotation.endX ?? annotation.x + 0.1) * props.attachment.width}
                          y2={(annotation.endY ?? annotation.y + 0.1) * props.attachment.height}
                          stroke={annotation.color}
                          stroke-width="3"
                          marker-end="url(#arrowhead)"
                        />
                      </Show>

                      {/* 文字标注 */}
                      <Show when={annotation.type === 'text' && annotation.text}>
                        <text
                          x={annotation.x * props.attachment.width}
                          y={annotation.y * props.attachment.height}
                          fill={annotation.color}
                          font-size="16"
                          font-weight="bold"
                        >
                          {annotation.text}
                        </text>
                      </Show>
                    </g>
                  )}
                </For>

                {/* 箭头标记定义 */}
                <defs>
                  <marker
                    id="arrowhead"
                    markerWidth="10"
                    markerHeight="7"
                    refX="9"
                    refY="3.5"
                    orient="auto"
                  >
                    <polygon points="0 0, 10 3.5, 0 7" fill="#ef4444" />
                  </marker>
                </defs>
              </svg>
            </div>
          </div>

          {/* 标注列表面板（标注模式下显示） */}
          <Show when={mode() === 'annotate' && annotations().length > 0}>
            <div class="h-24 bg-gray-900/90 border-t border-gray-700 px-4 py-2 overflow-x-auto">
              <div class="flex gap-2">
                <For each={annotations()}>
                  {(annotation) => (
                    <div class="flex items-center gap-2 px-2 py-1 bg-gray-800 rounded text-xs text-gray-300 shrink-0">
                      <span
                        class="w-3 h-3 rounded-full"
                        style={{ 'background-color': annotation.color }}
                      />
                      <span>
                        {annotation.type === 'rect' ? '矩形' : annotation.type === 'arrow' ? '箭头' : '文字'}
                        {annotation.text ? `: ${annotation.text}` : ''}
                      </span>
                      <button
                        class="text-gray-500 hover:text-red-400"
                        onClick={() => removeAnnotation(annotation.id)}
                      >
                        <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M3 3l6 6M9 3l-6 6" />
                        </svg>
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  )
}
