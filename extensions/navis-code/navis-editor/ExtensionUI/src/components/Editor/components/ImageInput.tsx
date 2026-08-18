/**
 * ImageInput 图片输入组件
 *
 * 严格遵循 design/26-editor.md 第六章"多模态输入渲染"中的 ImageInput.tsx 定义。
 * 统一处理图片的粘贴、拖拽和文件选择器三种输入方式。
 *
 * 功能：
 * - 监听 Chat 输入框的 paste 事件，提取剪贴板图片
 * - 监听 dragover / drop 事件，接收拖拽图片
 * - 提供文件选择器入口（按钮触发 input[type=file]）
 * - 格式校验（PNG / JPEG / GIF / WebP）+ 大小校验（单张 <= 20MB）
 * - 生成缩略图预览（上传前确认）
 * - 输出 ImageAttachment 对象到消息上下文
 *
 * 设计依据：design/26-editor.md S6.2 ImageInput 组件设计
 */

import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js'
import type { ImageAttachment, ImageInputError } from '../types'
import {
  processImageFile,
  extractImagesFromClipboard,
  extractImagesFromDrop,
} from '../extensions/image-drop-ext'
import { IMAGE_MAX_SIZE_BYTES, MAX_IMAGE_COUNT } from '@editor-ext/components/Editor/stores/editor'
// ============================================================
// 类型定义
// ============================================================

/**
 * ImageInput 组件 Props
 *
 * 对应 design/26-editor.md S6.2 ImageInput 组件的 Props 定义。
 */
export interface ImageInputProps {
  /** 单张图片最大大小（字节），默认 20MB */
  maxSizeBytes?: number
  /** 支持的 MIME 类型列表，默认 ['image/png','image/jpeg','image/gif','image/webp'] */
  accept?: string[]
  /** 单条消息最大图片数，默认 10 */
  maxCount?: number
  /** 图片列表变更回调 */
  onImagesChange: (images: ImageAttachment[]) => void
  /** 错误回调 */
  onError?: (error: { fileName: string; error: ImageInputError }) => void
  /** 容器类名（可选） */
  class?: string
}

// ============================================================
// ImageInput 组件
// ============================================================

/**
 * 图片输入组件
 *
 * 提供三种图片输入方式：
 * 1. 粘贴（Ctrl+V / Cmd+V）
 * 2. 拖拽（拖入区域）
 * 3. 文件选择器（点击按钮）
 *
 * @example
 * ```tsx
 * <ImageInput
 *   onImagesChange={(images) => setPendingImages(images)}
 *   onError={(err) => showToast(err.error)}
 * />
 * ```
 */
export const ImageInput: Component<ImageInputProps> = (props) => {
  // ---- 内部状态 ----

  /** 待发送的图片列表 */
  const [pendingImages, setPendingImages] = createSignal<ImageAttachment[]>([])

  /** 是否正在拖拽 */
  const [isDragging, setIsDragging] = createSignal(false)

  /** 是否正在处理文件 */
  const [processing, setProcessing] = createSignal(false)

  /** 隐藏的文件选择器 input 元素引用 */
  let fileInputRef: HTMLInputElement | undefined

  /** 拖拽区域容器引用 */
  let dropZoneRef: HTMLDivElement | undefined

  // ---- 配置 ----

  const maxSizeBytes = () => props.maxSizeBytes ?? IMAGE_MAX_SIZE_BYTES
  const maxCount = () => props.maxCount ?? MAX_IMAGE_COUNT
  const acceptTypes = () =>
    props.accept ?? ['image/png', 'image/jpeg', 'image/gif', 'image/webp']

  // ---- 事件处理 ----

  /**
   * 处理粘贴事件
   *
   * 从剪贴板提取图片数据，校验后转为 ImageAttachment。
   */
  const handlePaste = async (e: ClipboardEvent) => {
    const imageFiles = extractImagesFromClipboard(e)
    if (imageFiles.length === 0) return

    e.preventDefault()
    await processFiles(imageFiles, 'paste')
  }

  /**
   * 处理拖拽进入事件
   */
  const handleDragOver = (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(true)
  }

  /**
   * 处理拖拽离开事件
   */
  const handleDragLeave = (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
  }

  /**
   * 处理拖放事件
   *
   * 从拖放数据中提取图片文件，校验后转为 ImageAttachment。
   */
  const handleDrop = async (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)

    const imageFiles = extractImagesFromDrop(e)
    if (imageFiles.length === 0) return

    await processFiles(imageFiles, 'drop')
  }

  /**
   * 处理文件选择器变更
   *
   * 用户通过文件选择器选择图片后触发。
   */
  const handleFileSelect = async (e: Event) => {
    const input = e.target as HTMLInputElement
    const files = input.files
    if (!files || files.length === 0) return

    await processFiles(Array.from(files), 'file-picker')

    // 重置文件选择器（允许重复选择同一文件）
    input.value = ''
  }

  /**
   * 打开文件选择器
   */
  const openFilePicker = () => {
    fileInputRef?.click()
  }

  /**
   * 移除待发送图片
   */
  const removeImage = (id: string) => {
    setPendingImages((prev) => prev.filter((img) => img.id !== id))
    notifyChange()
  }

  // ---- 文件处理 ----

  /**
   * 批量处理图片文件
   *
   * 依次处理每个文件：校验 → 读取 → 生成 Attachment。
   * 超出数量限制时停止处理并通知。
   */
  const processFiles = async (
    files: File[],
    source: 'paste' | 'drop' | 'file-picker',
  ) => {
    setProcessing(true)

    const currentCount = pendingImages().length
    const allowedCount = maxCount() - currentCount

    if (allowedCount <= 0) {
      props.onError?.({ fileName: '', error: 'count-exceeded' })
      setProcessing(false)
      return
    }

    // 只处理允许数量内的文件
    const filesToProcess = files.slice(0, allowedCount)

    if (files.length > allowedCount) {
      props.onError?.({ fileName: '', error: 'count-exceeded' })
    }

    for (const file of filesToProcess) {
      const result = await processImageFile(file, source)

      if (result.attachment) {
        setPendingImages((prev) => [...prev, result.attachment!])
      } else if (result.error) {
        props.onError?.({ fileName: file.name, error: result.error })
      }
    }

    setProcessing(false)
    notifyChange()
  }

  /**
   * 通知外部图片列表变更
   */
  const notifyChange = () => {
    props.onImagesChange(pendingImages())
  }

  // ---- 生命周期 ----

  /**
   * 注册全局粘贴事件监听
   *
   * 注意：实际使用时应将此组件放置在 Chat 输入框附近，
   * 仅在输入框聚焦时才拦截粘贴事件。
   */
  const handlePasteEvent = (event: Event) => void handlePaste(event as ClipboardEvent)

  onMount(() => {
    document.addEventListener('paste', handlePasteEvent)
  })

  onCleanup(() => {
    document.removeEventListener('paste', handlePasteEvent)
  })

  // ---- 渲染 ----

  return (
    <div class={props.class}>
      {/* 拖拽区域 */}
      <div
        ref={dropZoneRef}
        class={`
          relative rounded-lg border-2 border-dashed transition-colors duration-200
          ${isDragging()
            ? 'border-blue-400 bg-blue-50 dark:bg-blue-900/20'
            : 'border-gray-300 dark:border-gray-600 hover:border-gray-400 dark:hover:border-gray-500'
          }
        `}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {/* 拖拽提示 */}
        <Show when={isDragging()}>
          <div class="absolute inset-0 flex items-center justify-center bg-blue-50/80 dark:bg-blue-900/30 rounded-lg z-10">
            <div class="text-center">
              <svg class="mx-auto mb-2 w-8 h-8 text-blue-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="17 8 12 3 7 8" />
                <line x1="12" y1="3" x2="12" y2="15" />
              </svg>
              <p class="text-sm text-blue-600 dark:text-blue-400">释放以添加图片</p>
            </div>
          </div>
        </Show>

        {/* 待发送图片缩略图列表 */}
        <Show when={pendingImages().length > 0}>
          <div class="flex flex-wrap gap-2 p-2">
            <For each={pendingImages()}>
              {(image) => (
                <div class="relative group w-20 h-20 rounded-md overflow-hidden border border-gray-200 dark:border-gray-700">
                  {/* 缩略图 */}
                  <img
                    src={`data:${image.mimeType};base64,${image.dataBase64}`}
                    alt={image.fileName}
                    class="w-full h-full object-cover"
                  />

                  {/* 移除按钮 */}
                  <button
                    class="absolute top-0.5 right-0.5 w-5 h-5 flex items-center justify-center rounded-full bg-black/50 text-white opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => removeImage(image.id)}
                    title="移除图片"
                  >
                    <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                      <path d="M3 3l6 6M9 3l-6 6" />
                    </svg>
                  </button>

                  {/* 文件名提示 */}
                  <div class="absolute bottom-0 left-0 right-0 px-1 py-0.5 bg-black/50 text-white text-[10px] truncate">
                    {image.fileName}
                  </div>
                </div>
              )}
            </For>
          </div>
        </Show>

        {/* 操作按钮区 */}
        <div class="flex items-center gap-2 p-2">
          {/* 文件选择器按钮 */}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
            onClick={openFilePicker}
            disabled={processing() || pendingImages().length >= maxCount()}
          >
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <circle cx="8.5" cy="8.5" r="1.5" />
              <polyline points="21 15 16 10 5 21" />
            </svg>
            添加图片
          </button>

          {/* 粘贴提示 */}
          <span class="text-xs text-gray-400 dark:text-gray-500">
            粘贴(Ctrl+V) / 拖拽
          </span>

          {/* 图片计数 */}
          <span class="text-xs text-gray-400 dark:text-gray-500 ml-auto">
            {pendingImages().length} / {maxCount()}
          </span>
        </div>

        {/* 隐藏的文件选择器 input */}
        <input
          ref={fileInputRef}
          type="file"
          accept={acceptTypes().join(',')}
          multiple
          class="hidden"
          onChange={handleFileSelect}
        />
      </div>
    </div>
  )
}
