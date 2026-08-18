/**
 * FileInput 文件输入组件
 *
 * 严格遵循 design/26-editor.md 第六章"多模态输入渲染"中的 FileInput.tsx 定义。
 * 统一处理非图片文件的粘贴、拖拽和文件选择器三种输入方式。
 *
 * 功能：
 * - 监听 dragover / drop 事件，接收拖拽文件
 * - 监听 paste 事件，提取剪贴板中的文件
 * - 提供文件选择器入口（按钮触发 input[type=file]）
 * - 文件类型识别（基于扩展名映射到 FileType 枚举）
 * - 图片文件自动转交 ImageInput 处理
 * - 格式校验 + 大小校验 + 数量校验 + 错误提示
 * - 内容读取与解析（调用 FileReader API）
 * - 输出 FileAttachment 对象到消息上下文
 *
 * 设计依据：design/26-editor.md S6.5.2 FileInput 组件设计
 */

import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js'
import type { FileAttachment, FileInputError, ImageAttachment } from '../types'
import {
  processFile,
  processImageFile,
  extractFilesFromDrop,
  extractImagesFromDrop,
  extractFilesFromClipboard,
  extractImagesFromClipboard,
  detectFileType,
} from '../extensions/image-drop-ext'
import {
  MAX_FILE_COUNT,
  FILE_HARD_MAX_SIZE_BYTES,
} from '@editor-ext/components/Editor/stores/editor'
// ============================================================
// 类型定义
// ============================================================

/**
 * FileInput 组件 Props
 *
 * 对应 design/26-editor.md S6.5.2 FileInput 组件的 Props 定义。
 */
export interface FileInputProps {
  /** 单个文件最大大小（字节），默认 10MB（硬限制） */
  maxSizeBytes?: number
  /** 单条消息最大文件数，默认 10 */
  maxCount?: number
  /** 支持的文件扩展名列表（默认全量支持） */
  accept?: string[]
  /** 文件列表变更回调 */
  onFilesChange: (files: FileAttachment[]) => void
  /** 图片转交回调（图片文件转交 ImageInput 处理） */
  onImageTransfer?: (images: ImageAttachment[]) => void
  /** 错误回调 */
  onError?: (error: { fileName: string; error: FileInputError }) => void
  /** 容器类名（可选） */
  class?: string
}

/**
 * 文件类型图标映射
 *
 * 根据文件类型返回对应的图标文本和颜色。
 */
const FILE_TYPE_ICONS: Record<string, { icon: string; color: string }> = {
  code: { icon: '<>', color: 'text-blue-500 bg-blue-50 dark:bg-blue-900/20' },
  document: { icon: 'D', color: 'text-green-500 bg-green-50 dark:bg-green-900/20' },
  config: { icon: 'C', color: 'text-yellow-500 bg-yellow-50 dark:bg-yellow-900/20' },
  unknown: { icon: '?', color: 'text-gray-500 bg-gray-50 dark:bg-gray-800' },
}

// ============================================================
// FileInput 组件
// ============================================================

/**
 * 文件输入组件
 *
 * 提供三种文件输入方式：
 * 1. 粘贴（Ctrl+V / Cmd+V）
 * 2. 拖拽（拖入区域）
 * 3. 文件选择器（点击按钮）
 *
 * 图片文件自动转交 ImageInput 处理。
 *
 * @example
 * ```tsx
 * <FileInput
 *   onFilesChange={(files) => setPendingFiles(files)}
 *   onImageTransfer={(images) => addImages(images)}
 *   onError={(err) => showToast(err.error)}
 * />
 * ```
 */
export const FileInput: Component<FileInputProps> = (props) => {
  // ---- 内部状态 ----

  /** 待发送的文件列表 */
  const [pendingFiles, setPendingFiles] = createSignal<FileAttachment[]>([])

  /** 是否正在拖拽 */
  const [isDragging, setIsDragging] = createSignal(false)

  /** 是否正在处理文件 */
  const [processing, setProcessing] = createSignal(false)

  /** 隐藏的文件选择器 input 元素引用 */
  let fileInputRef: HTMLInputElement | undefined

  // ---- 配置 ----

  const maxSizeBytes = () => props.maxSizeBytes ?? FILE_HARD_MAX_SIZE_BYTES
  const maxCount = () => props.maxCount ?? MAX_FILE_COUNT

  // ---- 事件处理 ----

  /**
   * 处理粘贴事件
   *
   * 从剪贴板提取文件数据，分类处理（图片和非图片）。
   */
  const handlePaste = async (e: ClipboardEvent) => {
    const { images, files } = extractFilesFromClipboard(e)

    // 图片转交 ImageInput
    if (images.length > 0 && props.onImageTransfer) {
      const imageAttachments: ImageAttachment[] = []
      for (const file of images) {
        const result = await processImageFile(file, 'paste')
        if (result.attachment) {
          imageAttachments.push(result.attachment)
        }
      }
      if (imageAttachments.length > 0) {
        props.onImageTransfer(imageAttachments)
      }
    }

    // 非图片文件处理
    if (files.length > 0) {
      e.preventDefault()
      await processFiles(files, 'paste')
    }
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
   */
  const handleDrop = async (e: DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)

    const imageFiles = extractImagesFromDrop(e)
    const nonImageFiles = extractFilesFromDrop(e)

    // 图片转交 ImageInput
    if (imageFiles.length > 0 && props.onImageTransfer) {
      const imageAttachments: ImageAttachment[] = []
      for (const file of imageFiles) {
        const result = await processImageFile(file, 'drop')
        if (result.attachment) {
          imageAttachments.push(result.attachment)
        }
      }
      if (imageAttachments.length > 0) {
        props.onImageTransfer(imageAttachments)
      }
    }

    // 非图片文件处理
    if (nonImageFiles.length > 0) {
      await processFiles(nonImageFiles, 'drop')
    }
  }

  /**
   * 处理文件选择器变更
   */
  const handleFileSelect = async (e: Event) => {
    const input = e.target as HTMLInputElement
    const files = input.files
    if (!files || files.length === 0) return

    const fileList = Array.from(files)

    // 分类：图片和非图片
    const images: File[] = []
    const nonImages: File[] = []
    for (const file of fileList) {
      const fileType = detectFileType(file.name)
      if (fileType === ('image' as any)) {
        images.push(file)
      } else {
        nonImages.push(file)
      }
    }

    // 图片转交
    if (images.length > 0 && props.onImageTransfer) {
      const imageAttachments: ImageAttachment[] = []
      for (const file of images) {
        const result = await processImageFile(file, 'file-picker')
        if (result.attachment) {
          imageAttachments.push(result.attachment)
        }
      }
      if (imageAttachments.length > 0) {
        props.onImageTransfer(imageAttachments)
      }
    }

    // 非图片文件处理
    if (nonImages.length > 0) {
      await processFiles(nonImages, 'picker')
    }

    // 重置文件选择器
    input.value = ''
  }

  /**
   * 打开文件选择器
   */
  const openFilePicker = () => {
    fileInputRef?.click()
  }

  /**
   * 移除待发送文件
   */
  const removeFile = (id: string) => {
    setPendingFiles((prev) => prev.filter((f) => f.id !== id))
    notifyChange()
  }

  // ---- 文件处理 ----

  /**
   * 批量处理文件
   */
  const processFiles = async (
    files: File[],
    source: 'paste' | 'drop' | 'picker',
  ) => {
    setProcessing(true)

    const currentCount = pendingFiles().length
    const allowedCount = maxCount() - currentCount

    if (allowedCount <= 0) {
      props.onError?.({ fileName: '', error: 'count-exceeded' })
      setProcessing(false)
      return
    }

    const filesToProcess = files.slice(0, allowedCount)

    if (files.length > allowedCount) {
      props.onError?.({ fileName: '', error: 'count-exceeded' })
    }

    for (const file of filesToProcess) {
      const result = await processFile(file, source)

      if (result.attachment) {
        setPendingFiles((prev) => [...prev, result.attachment!])
      } else if (result.error) {
        props.onError?.({ fileName: file.name, error: result.error })
      }
    }

    setProcessing(false)
    notifyChange()
  }

  /**
   * 通知外部文件列表变更
   */
  const notifyChange = () => {
    props.onFilesChange(pendingFiles())
  }

  /**
   * 格式化文件大小
   */
  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  // ---- 生命周期 ----

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
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <p class="text-sm text-blue-600 dark:text-blue-400">释放以添加文件</p>
            </div>
          </div>
        </Show>

        {/* 待发送文件列表 */}
        <Show when={pendingFiles().length > 0}>
          <div class="flex flex-wrap gap-2 p-2">
            <For each={pendingFiles()}>
              {(file) => {
                const iconConfig = FILE_TYPE_ICONS[file.fileType] ?? FILE_TYPE_ICONS.unknown
                return (
                  <div class="flex items-center gap-2 px-2.5 py-1.5 rounded-md border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 group max-w-[200px]">
                    {/* 文件类型图标 */}
                    <span class={`w-6 h-6 flex items-center justify-center rounded text-xs font-bold shrink-0 ${iconConfig.color}`}>
                      {iconConfig.icon}
                    </span>

                    {/* 文件信息 */}
                    <div class="flex-1 min-w-0">
                      <div class="text-xs text-gray-800 dark:text-gray-200 truncate">
                        {file.fileName}
                      </div>
                      <div class="text-[10px] text-gray-400 dark:text-gray-500">
                        {formatSize(file.sizeBytes)}
                        {file.isTruncated && (
                          <span class="text-orange-400 ml-1">已截取</span>
                        )}
                      </div>
                    </div>

                    {/* 移除按钮 */}
                    <button
                      class="w-4 h-4 flex items-center justify-center text-gray-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                      onClick={() => removeFile(file.id)}
                      title="移除文件"
                    >
                      <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M3 3l6 6M9 3l-6 6" />
                      </svg>
                    </button>
                  </div>
                )
              }}
            </For>
          </div>
        </Show>

        {/* 操作按钮区 */}
        <div class="flex items-center gap-2 p-2">
          {/* 文件选择器按钮 */}
          <button
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
            onClick={openFilePicker}
            disabled={processing() || pendingFiles().length >= maxCount()}
          >
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="12" y1="18" x2="12" y2="12" />
              <line x1="9" y1="15" x2="15" y2="15" />
            </svg>
            添加文件
          </button>

          {/* 提示文本 */}
          <span class="text-xs text-gray-400 dark:text-gray-500">
            拖拽 / 粘贴文件
          </span>

          {/* 文件计数 */}
          <span class="text-xs text-gray-400 dark:text-gray-500 ml-auto">
            {pendingFiles().length} / {maxCount()}
          </span>
        </div>

        {/* 隐藏的文件选择器 input */}
        <input
          ref={fileInputRef}
          type="file"
          multiple
          class="hidden"
          onChange={handleFileSelect}
        />
      </div>
    </div>
  )
}
