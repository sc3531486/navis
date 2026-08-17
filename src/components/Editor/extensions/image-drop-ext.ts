/**
 * 图片拖拽/粘贴扩展 - CodeMirror 6 Extension
 *
 * 严格遵循 design/26-editor.md 第六章"多模态输入渲染"中的
 * image-drop-ext.ts 扩展设计。
 *
 * 职责：
 * - 统一拦截 paste / drop / 文件选择事件
 * - 提取图片 File 对象
 * - 校验格式（PNG / JPEG / GIF / WebP）
 * - 校验大小（单张 <= 20MB）
 * - 转交 ImageInput 组件处理
 *
 * 设计依据：design/26-editor.md S6.1 多模态输入链路
 */

import type { ImageAttachment, ImageInputError, FileAttachment, FileInputError, FileType } from '../types'
import { IMAGE_MAX_SIZE_BYTES, FILE_HARD_MAX_SIZE_BYTES, MAX_IMAGE_COUNT, MAX_FILE_COUNT, MAX_FILE_LINES } from '../stores/editor'

// ============================================================
// 常量
// ============================================================

/** 支持的图片 MIME 类型 */
const SUPPORTED_IMAGE_TYPES = new Set([
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
])

/** 支持的图片文件扩展名（用于从文件扩展名判断） */
const SUPPORTED_IMAGE_EXTENSIONS = new Set([
  '.png', '.jpg', '.jpeg', '.gif', '.webp',
])

/**
 * 代码文件扩展名映射
 *
 * 对应 design/26-editor.md S6.5.3 支持的文件类型-代码文件。
 */
const CODE_FILE_EXTENSIONS = new Set([
  '.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs',
  '.py',
  '.rs',
  '.go',
  '.java',
  '.c', '.cpp', '.h', '.hpp', '.cc', '.cxx',
  '.html', '.htm', '.css', '.scss', '.less',
  '.vue', '.svelte',
  '.rb', '.php', '.swift', '.kt', '.dart',
  '.lua', '.r', '.sql',
  '.sh', '.bash', '.zsh',
])

/**
 * 文档文件扩展名映射
 *
 * 对应 design/26-editor.md S6.5.3 支持的文件类型-文档文件。
 */
const DOCUMENT_FILE_EXTENSIONS = new Set([
  '.md', '.mdx', '.txt', '.rst',
  // .pdf 和 .docx 需要后端处理，此处仅标记
])

/**
 * 配置文件扩展名映射
 *
 * 对应 design/26-editor.md S6.5.3 支持的文件类型-配置文件。
 */
const CONFIG_FILE_EXTENSIONS = new Set([
  '.env', '.gitignore', '.gitattributes',
  '.json', '.jsonc', '.yaml', '.yml', '.toml',
  '.ini', '.cfg', '.editorconfig',
  '.xml',
])

/** 不支持的二进制文件扩展名 */
const UNSUPPORTED_BINARY_EXTENSIONS = new Set([
  '.exe', '.bin', '.zip', '.tar', '.gz', '.rar',
  '.7z', '.dll', '.so', '.dylib', '.iso',
])

// ============================================================
// 工具函数
// ============================================================

/**
 * 根据文件扩展名判断文件类型分类
 *
 * 对应 design/26-editor.md S6.5.1 文件类型识别逻辑。
 *
 * @param fileName 文件名（含扩展名）
 * @returns 文件类型分类枚举值
 */
export function detectFileType(fileName: string): FileType {
  const ext = getFileExtension(fileName)

  // 图片文件 → 转交 ImageInput 处理
  if (SUPPORTED_IMAGE_EXTENSIONS.has(ext)) {
    return 'image' as FileType
  }

  // 代码文件
  if (CODE_FILE_EXTENSIONS.has(ext)) {
    return 'code' as FileType
  }

  // 文档文件
  if (DOCUMENT_FILE_EXTENSIONS.has(ext) || ext === '.pdf' || ext === '.docx') {
    return 'document' as FileType
  }

  // 配置文件（包括无扩展名的特殊文件如 Dockerfile）
  if (CONFIG_FILE_EXTENSIONS.has(ext) || fileName === 'Dockerfile' || fileName === 'docker-compose.yml') {
    return 'config' as FileType
  }

  return 'unknown' as FileType
}

/**
 * 获取文件扩展名（含点号，全小写）
 *
 * @param fileName 文件名
 * @returns 扩展名，如 '.ts'、'.py'
 */
function getFileExtension(fileName: string): string {
  const lastDot = fileName.lastIndexOf('.')
  if (lastDot === -1) return ''
  return fileName.slice(lastDot).toLowerCase()
}

/**
 * 生成唯一 ID
 */
function generateId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2, 11)}`
}

/**
 * 将 File 对象转为 Base64 编码
 *
 * 使用 FileReader API 读取文件内容并转为 Base64。
 *
 * @param file File 对象
 * @returns Base64 编码字符串（不含 data URL 前缀）
 */
export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = reader.result as string
      // 去除 data:mime;base64, 前缀
      const base64 = result.split(',')[1] ?? ''
      resolve(base64)
    }
    reader.onerror = () => {
      reject(new Error('文件读取失败'))
    }
    reader.readAsDataURL(file)
  })
}

/**
 * 读取文件文本内容
 *
 * 使用 FileReader API 读取文本文件内容。
 * 支持大文件截取策略（前 MAX_FILE_LINES 行）。
 *
 * @param file File 对象
 * @param maxLines 最大读取行数，默认 MAX_FILE_LINES
 * @returns 文本内容和是否被截取
 */
export function readTextContent(
  file: File,
  maxLines: number = MAX_FILE_LINES,
): Promise<{ text: string; isTruncated: boolean; totalLines: number }> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const fullText = reader.result as string
      const lines = fullText.split('\n')
      const totalLines = lines.length

      if (totalLines > maxLines) {
        // 大文件截取：仅保留前 maxLines 行
        const truncatedText = lines.slice(0, maxLines).join('\n')
        resolve({ text: truncatedText, isTruncated: true, totalLines })
      } else {
        resolve({ text: fullText, isTruncated: false, totalLines })
      }
    }
    reader.onerror = () => {
      reject(new Error('文件读取失败'))
    }
    reader.readAsText(file, 'utf-8')
  })
}

// ============================================================
// 图片处理
// ============================================================

/**
 * 校验图片文件
 *
 * 对应 design/26-editor.md S6.2 ImageInput 组件的格式校验 + 大小校验逻辑。
 *
 * @param file File 对象
 * @returns 校验结果（null 表示通过，否则返回错误类型）
 */
export function validateImage(file: File): ImageInputError | null {
  // 格式校验：检查 MIME 类型
  if (!SUPPORTED_IMAGE_TYPES.has(file.type)) {
    return 'format-unsupported'
  }

  // 大小校验：单张图片 <= 20MB
  if (file.size > IMAGE_MAX_SIZE_BYTES) {
    return 'size-exceeded'
  }

  return null
}

/**
 * 处理图片文件，生成 ImageAttachment 对象
 *
 * 完整链路：
 * 1. 校验格式和大小
 * 2. 读取图片数据为 Base64
 * 3. 获取图片尺寸（通过 Image 元素）
 * 4. 生成 ImageAttachment 对象
 *
 * @param file File 对象
 * @param source 图片来源方式
 * @returns ImageAttachment 对象或错误
 */
export async function processImageFile(
  file: File,
  source: 'paste' | 'drop' | 'file-picker',
): Promise<{ attachment?: ImageAttachment; error?: ImageInputError }> {
  // 校验
  const validationError = validateImage(file)
  if (validationError) {
    return { error: validationError }
  }

  try {
    // 读取为 Base64
    const base64 = await fileToBase64(file)

    // 获取图片尺寸
    const { width, height } = await getImageDimensions(file)

    const attachment: ImageAttachment = {
      id: generateId(),
      fileName: file.name,
      mimeType: file.type,
      dataBase64: base64,
      width,
      height,
      sizeBytes: file.size,
      source,
      createdAt: Date.now(),
    }

    return { attachment }
  } catch {
    return { error: 'read-failed' }
  }
}

/**
 * 获取图片尺寸
 *
 * 通过创建 Image 元素加载图片数据，读取自然宽高。
 *
 * @param file File 对象
 * @returns 图片宽高（像素）
 */
function getImageDimensions(file: File): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file)
    const img = new Image()

    img.onload = () => {
      resolve({ width: img.naturalWidth, height: img.naturalHeight })
      URL.revokeObjectURL(url)
    }

    img.onerror = () => {
      reject(new Error('图片尺寸读取失败'))
      URL.revokeObjectURL(url)
    }

    img.src = url
  })
}

// ============================================================
// 文件处理
// ============================================================

/**
 * 校验文件
 *
 * 对应 design/26-editor.md S6.5.5 约束与限制。
 *
 * @param file File 对象
 * @returns 校验结果（null 表示通过，否则返回错误类型）
 */
export function validateFile(file: File): FileInputError | null {
  const ext = getFileExtension(file.name)

  // 不支持二进制文件
  if (UNSUPPORTED_BINARY_EXTENSIONS.has(ext)) {
    return 'format-unsupported'
  }

  // 硬大小限制：10MB
  if (file.size > FILE_HARD_MAX_SIZE_BYTES) {
    return 'size-exceeded'
  }

  return null
}

/**
 * 处理非图片文件，生成 FileAttachment 对象
 *
 * 完整链路：
 * 1. 文件类型识别（扩展名 → FileType 映射）
 * 2. 格式校验 + 大小校验
 * 3. 内容读取（代码/文档/配置文件读取文本内容）
 * 4. 大文件截取策略
 * 5. 生成 FileAttachment 对象
 *
 * @param file File 对象
 * @param source 文件来源方式
 * @returns FileAttachment 对象或错误
 */
export async function processFile(
  file: File,
  source: 'paste' | 'drop' | 'picker',
): Promise<{ attachment?: FileAttachment; error?: FileInputError }> {
  // 文件类型识别
  const fileType = detectFileType(file.name)

  // 图片文件 → 应转交 ImageInput 处理
  if (fileType === ('image' as FileType)) {
    return { error: 'format-unsupported' }
  }

  // 未知类型 → 提示不支持
  if (fileType === ('unknown' as FileType)) {
    return { error: 'format-unsupported' }
  }

  // 校验
  const validationError = validateFile(file)
  if (validationError) {
    return { error: validationError }
  }

  try {
    // 读取文本内容（带超时）
    const readPromise = readTextContent(file)
    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error('读取超时')), 5000)
    })

    const { text, isTruncated, totalLines } = await Promise.race([
      readPromise,
      timeoutPromise,
    ])

    const attachment: FileAttachment = {
      id: generateId(),
      fileName: file.name,
      filePath: '', // 前端 File API 不提供完整路径，拖拽本地文件时由 Tauri 提供
      mimeType: file.type || 'text/plain',
      sizeBytes: file.size,
      fileType,
      source,
      createdAt: Date.now(),
      textContent: text,
      isTruncated,
      totalLines,
    }

    return { attachment }
  } catch (error) {
    if (error instanceof Error && error.message === '读取超时') {
      return { error: 'read-timeout' }
    }
    return { error: 'read-failed' }
  }
}

// ============================================================
// 事件拦截器
// ============================================================

/**
 * 从 ClipboardEvent 中提取图片文件列表
 *
 * @param event 剪贴板事件
 * @returns 图片 File 对象列表
 */
export function extractImagesFromClipboard(event: ClipboardEvent): File[] {
  const files: File[] = []

  if (event.clipboardData) {
    const items = Array.from(event.clipboardData.items)
    for (const item of items) {
      // 仅处理图片类型的剪贴板条目
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile()
        if (file) {
          files.push(file)
        }
      }
    }
  }

  return files
}

/**
 * 从 DragEvent 中提取图片文件列表
 *
 * @param event 拖拽事件
 * @returns 图片 File 对象列表
 */
export function extractImagesFromDrop(event: DragEvent): File[] {
  const files: File[] = []

  if (event.dataTransfer) {
    const items = Array.from(event.dataTransfer.files)
    for (const file of items) {
      if (SUPPORTED_IMAGE_TYPES.has(file.type)) {
        files.push(file)
      }
    }
  }

  return files
}

/**
 * 从 DragEvent 中提取非图片文件列表
 *
 * @param event 拖拽事件
 * @returns 非图片 File 对象列表
 */
export function extractFilesFromDrop(event: DragEvent): File[] {
  const files: File[] = []

  if (event.dataTransfer) {
    const items = Array.from(event.dataTransfer.files)
    for (const file of items) {
      if (!SUPPORTED_IMAGE_TYPES.has(file.type)) {
        files.push(file)
      }
    }
  }

  return files
}

/**
 * 从 ClipboardEvent 中提取文件列表（图片和非图片分开）
 *
 * @param event 剪贴板事件
 * @returns 分类后的文件列表
 */
export function extractFilesFromClipboard(event: ClipboardEvent): {
  images: File[]
  files: File[]
} {
  const images: File[] = []
  const files: File[] = []

  if (event.clipboardData) {
    const items = Array.from(event.clipboardData.items)
    for (const item of items) {
      if (item.kind === 'file') {
        const file = item.getAsFile()
        if (!file) continue

        if (item.type.startsWith('image/')) {
          images.push(file)
        } else {
          files.push(file)
        }
      }
    }
  }

  return { images, files }
}
