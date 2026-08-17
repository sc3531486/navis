/**
 * Editor 编辑器状态管理 Store
 *
 * 严格遵循 design/26-editor.md 第三章"数据模型"中的 EditorState 定义。
 * 使用 Solid.js 的 createStore 实现细粒度响应式状态管理。
 *
 * 职责：
 * 1. 管理编辑器标签页生命周期（打开/关闭/切换/固定）
 * 2. 维护文件脏状态标记
 * 3. 管理分屏模式
 * 4. 存储 LSP 诊断信息
 * 5. 管理图片附件列表
 * 6. 管理文件附件列表
 *
 * 设计依据：design/26-editor.md S3 数据模型、S7 事件定义
 */

import { createStore } from 'solid-js/store'
import type {
  EditorTab,
  EditorState,
  SplitMode,
  Diagnostic,
  ImageAttachment,
  ImageAnnotation,
  FileAttachment,
} from '../types'

// ============================================================
// 常量
// ============================================================

/** 图片输入默认最大文件大小：20MB */
export const IMAGE_MAX_SIZE_BYTES = 20 * 1024 * 1024

/** 文件输入默认最大文件大小（硬限制）：10MB */
export const FILE_HARD_MAX_SIZE_BYTES = 10 * 1024 * 1024

/** 文件输入默认大文件阈值：1MB */
export const FILE_LARGE_THRESHOLD_BYTES = 1 * 1024 * 1024

/** 大文件截取的最大行数 */
export const MAX_FILE_LINES = 500

/** 单条消息最大图片数 */
export const MAX_IMAGE_COUNT = 10

/** 单条消息最大文件数 */
export const MAX_FILE_COUNT = 10

/** 文件读取超时时间（毫秒） */
export const FILE_READ_TIMEOUT_MS = 5000

/** 代码文件大文件行数阈值 */
export const CODE_FILE_LINE_THRESHOLD = 10000

/** PDF 文件大文件页数阈值 */
export const PDF_PAGE_THRESHOLD = 50

/** PDF 大文件截取页数 */
export const PDF_MAX_EXTRACT_PAGES = 20

// ============================================================
// 工具函数
// ============================================================

/**
 * 生成唯一 ID
 * 使用 crypto.randomUUID（如可用）或时间戳 + 随机数
 */
function generateId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2, 11)}`
}

/**
 * 根据文件扩展名推断编程语言标识
 *
 * 映射常见文件扩展名到 CodeMirror 6 语言模式名称。
 * 用于 EditorTab 的 language 字段初始化。
 *
 * @param fileName 文件名（含扩展名）
 * @returns CodeMirror 语言标识字符串
 */
export function detectLanguage(fileName: string): string {
  const ext = fileName.includes('.') ? fileName.split('.').pop()?.toLowerCase() : ''

  /** 扩展名 → 语言标识映射表 */
  const languageMap: Record<string, string> = {
    // TypeScript / JavaScript
    ts: 'typescript',
    tsx: 'typescript',
    js: 'javascript',
    jsx: 'javascript',
    mjs: 'javascript',
    cjs: 'javascript',
    // Python
    py: 'python',
    // Rust
    rs: 'rust',
    // Go
    go: 'go',
    // Java
    java: 'java',
    // C / C++
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    cc: 'cpp',
    cxx: 'cpp',
    // Web
    html: 'html',
    htm: 'html',
    css: 'css',
    scss: 'scss',
    less: 'less',
    // Vue / Svelte
    vue: 'vue',
    svelte: 'svelte',
    // 数据格式
    json: 'json',
    jsonc: 'json',
    yaml: 'yaml',
    yml: 'yaml',
    toml: 'toml',
    xml: 'xml',
    // 标记语言
    md: 'markdown',
    mdx: 'markdown',
    // Shell
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
    // SQL
    sql: 'sql',
    // 其他
    rb: 'ruby',
    php: 'php',
    swift: 'swift',
    kt: 'kotlin',
    dart: 'dart',
    lua: 'lua',
    r: 'r',
    dockerfile: 'dockerfile',
  }

  // 特殊文件名处理
  const lowerName = fileName.toLowerCase()
  if (lowerName === 'dockerfile') return 'dockerfile'
  if (lowerName === '.gitignore') return 'text'
  if (lowerName === '.env' || lowerName.startsWith('.env.')) return 'dotenv'

  return languageMap[ext ?? ''] ?? 'text'
}

// ============================================================
// 编辑器状态 Store
// ============================================================

/**
 * 编辑器响应式状态
 *
 * 使用 Solid.js 的 createStore 实现细粒度响应式。
 * 标签页、诊断信息等数据的变更会自动触发 UI 更新。
 */
const [editorState, setEditorState] = createStore<EditorState>({
  tabs: [],
  activeTabId: null,
  splitMode: 'none',
  diagnostics: new Map(),
})

// ============================================================
// 图片附件 Store
// ============================================================

/**
 * 图片附件响应式状态
 *
 * 管理当前消息上下文中待发送的图片列表。
 */
const [imageState, setImageState] = createStore<{
  /** 待发送的图片列表 */
  pendingImages: ImageAttachment[]
  /** 图片标注映射（attachmentId → annotations） */
  annotations: Map<string, ImageAnnotation[]>
}>({
  pendingImages: [],
  annotations: new Map(),
})

// ============================================================
// 文件附件 Store
// ============================================================

/**
 * 文件附件响应式状态
 *
 * 管理当前消息上下文中待发送的文件列表。
 */
const [fileState, setFileState] = createStore<{
  /** 待发送的文件列表 */
  pendingFiles: FileAttachment[]
}>({
  pendingFiles: [],
})

// ============================================================
// 公开 API - 标签页管理
// ============================================================

/**
 * 编辑器标签页管理 API
 *
 * 对应 design/26-editor.md S3 数据模型中的 EditorState 操作。
 */
export const editorTabsAPI = {
  /**
   * 打开文件（创建新标签或切换到已有标签）
   *
   * @param filePath 文件绝对路径
   * @param fileName 文件名
   * @returns 标签页 ID
   */
  open(filePath: string, fileName: string): string {
    // 检查文件是否已经打开
    const existingTab = editorState.tabs.find((tab) => tab.filePath === filePath)
    if (existingTab) {
      // 已打开 → 切换到该标签
      setEditorState('activeTabId', existingTab.id)
      return existingTab.id
    }

    // 创建新标签
    const newTab: EditorTab = {
      id: generateId(),
      filePath,
      fileName,
      language: detectLanguage(fileName),
      isDirty: false,
      isPinned: false,
    }

    setEditorState('tabs', (prev) => [...prev, newTab])
    setEditorState('activeTabId', newTab.id)

    return newTab.id
  },

  /**
   * 关闭指定标签
   *
   * @param tabId 标签 ID
   */
  close(tabId: string): void {
    const tabIndex = editorState.tabs.findIndex((tab) => tab.id === tabId)
    if (tabIndex === -1) return

    // 不允许关闭固定的标签
    const tab = editorState.tabs[tabIndex]
    if (tab.isPinned) return

    setEditorState('tabs', (prev) => prev.filter((t) => t.id !== tabId))

    // 如果关闭的是当前活跃标签，切换到相邻标签
    if (editorState.activeTabId === tabId) {
      const remaining = editorState.tabs.filter((t) => t.id !== tabId)
      if (remaining.length > 0) {
        // 优先切换到右侧标签，如果没有则切换到左侧
        const newIndex = Math.min(tabIndex, remaining.length - 1)
        setEditorState('activeTabId', remaining[newIndex].id)
      } else {
        setEditorState('activeTabId', null)
      }
    }
  },

  /**
   * 关闭除指定标签外的所有标签
   *
   * @param tabId 保留的标签 ID
   */
  closeOthers(tabId: string): void {
    setEditorState('tabs', (prev) =>
      prev.filter((tab) => tab.id === tabId || tab.isPinned),
    )
    setEditorState('activeTabId', tabId)
  },

  /**
   * 关闭所有未固定的标签
   */
  closeAll(): void {
    setEditorState('tabs', (prev) => prev.filter((tab) => tab.isPinned))
    // 如果当前活跃标签被关闭，切换到第一个固定标签
    const isCurrentPinned = editorState.tabs.some(
      (tab) => tab.id === editorState.activeTabId && tab.isPinned,
    )
    if (!isCurrentPinned) {
      const pinnedTabs = editorState.tabs.filter((tab) => tab.isPinned)
      setEditorState('activeTabId', pinnedTabs.length > 0 ? pinnedTabs[0].id : null)
    }
  },

  /**
   * 切换到指定标签
   *
   * @param tabId 标签 ID
   */
  switchTo(tabId: string): void {
    const exists = editorState.tabs.some((tab) => tab.id === tabId)
    if (exists) {
      setEditorState('activeTabId', tabId)
    }
  },

  /**
   * 切换标签的固定状态
   *
   * @param tabId 标签 ID
   */
  togglePin(tabId: string): void {
    setEditorState('tabs', (tab) => tab.id === tabId, 'isPinned', (prev) => !prev)
  },

  /**
   * 设置标签的脏状态
   *
   * @param tabId 标签 ID
   * @param isDirty 是否有未保存的修改
   */
  setDirty(tabId: string, isDirty: boolean): void {
    setEditorState('tabs', (tab) => tab.id === tabId, 'isDirty', isDirty)
  },

  /**
   * 标记指定文件为已保存（清除脏状态）
   *
   * @param filePath 文件路径
   */
  markSaved(filePath: string): void {
    setEditorState('tabs', (tab) => tab.filePath === filePath, 'isDirty', false)
  },

  /**
   * 获取当前活跃标签
   *
   * @returns 当前活跃标签或 null
   */
  getActiveTab(): EditorTab | null {
    if (!editorState.activeTabId) return null
    return editorState.tabs.find((tab) => tab.id === editorState.activeTabId) ?? null
  },

  /**
   * 获取所有标签列表（响应式）
   */
  getTabs: () => editorState.tabs,

  /**
   * 获取当前活跃标签 ID（响应式）
   */
  getActiveTabId: () => editorState.activeTabId,
}

// ============================================================
// 公开 API - 分屏管理
// ============================================================

/**
 * 编辑器分屏管理 API
 *
 * 对应 design/26-editor.md 中的 splitMode 状态管理。
 */
export const editorSplitAPI = {
  /**
   * 设置分屏模式
   *
   * @param mode 分屏模式：'none' | 'horizontal' | 'vertical'
   */
  setMode(mode: SplitMode): void {
    setEditorState('splitMode', mode)
  },

  /**
   * 获取当前分屏模式（响应式）
   */
  getMode: () => editorState.splitMode,

  /**
   * 在三种模式间循环切换
   * none → vertical → horizontal → none
   */
  toggle(): void {
    const modes: SplitMode[] = ['none', 'vertical', 'horizontal']
    const currentIndex = modes.indexOf(editorState.splitMode)
    const nextIndex = (currentIndex + 1) % modes.length
    setEditorState('splitMode', modes[nextIndex])
  },
}

// ============================================================
// 公开 API - 诊断管理
// ============================================================

/**
 * LSP 诊断信息管理 API
 *
 * 对应 design/26-editor.md S4 LSP 集成中的诊断标记功能。
 */
export const editorDiagnosticsAPI = {
  /**
   * 更新指定文件的诊断信息
   *
   * @param filePath 文件路径
   * @param diagnostics 诊断列表
   */
  update(filePath: string, diagnostics: Diagnostic[]): void {
    setEditorState('diagnostics', (prev) => {
      const next = new Map(prev)
      next.set(filePath, diagnostics)
      return next
    })
  },

  /**
   * 清除指定文件的诊断信息
   *
   * @param filePath 文件路径
   */
  clear(filePath: string): void {
    setEditorState('diagnostics', (prev) => {
      const next = new Map(prev)
      next.delete(filePath)
      return next
    })
  },

  /**
   * 获取指定文件的诊断信息
   *
   * @param filePath 文件路径
   * @returns 诊断列表（可能为空数组）
   */
  get(filePath: string): Diagnostic[] {
    return editorState.diagnostics.get(filePath) ?? []
  },

  /**
   * 获取所有诊断信息（响应式）
   */
  getAll: () => editorState.diagnostics,
}

export function resetEditorState(): void {
  setEditorState({
    tabs: [],
    activeTabId: null,
    splitMode: 'none',
    diagnostics: new Map(),
  })
}

// ============================================================
// 公开 API - 图片附件管理
// ============================================================

/**
 * 图片附件管理 API
 *
 * 对应 design/26-editor.md S6.2 ImageInput 组件设计。
 * 管理待发送的图片列表和图片标注。
 */
export const imageAPI = {
  /**
   * 添加图片附件
   *
   * @param attachment 图片附件数据
   * @returns 是否添加成功（超出数量限制时返回 false）
   */
  add(attachment: ImageAttachment): boolean {
    if (imageState.pendingImages.length >= MAX_IMAGE_COUNT) {
      return false
    }
    setImageState('pendingImages', (prev) => [...prev, attachment])
    return true
  },

  /**
   * 移除图片附件
   *
   * @param attachmentId 图片附件 ID
   */
  remove(attachmentId: string): void {
    setImageState('pendingImages', (prev) =>
      prev.filter((img) => img.id !== attachmentId),
    )
    // 同时清除该图片的标注
    setImageState('annotations', (prev) => {
      const next = new Map(prev)
      next.delete(attachmentId)
      return next
    })
  },

  /**
   * 清空所有图片附件
   */
  clearAll(): void {
    setImageState('pendingImages', [])
    setImageState('annotations', new Map())
  },

  /**
   * 获取待发送图片列表（响应式）
   */
  getPending: () => imageState.pendingImages,

  /**
   * 获取图片标注
   *
   * @param attachmentId 图片附件 ID
   * @returns 标注列表
   */
  getAnnotations(attachmentId: string): ImageAnnotation[] {
    return imageState.annotations.get(attachmentId) ?? []
  },

  /**
   * 更新图片标注
   *
   * @param attachmentId 图片附件 ID
   * @param annotations 新的标注列表
   */
  setAnnotations(attachmentId: string, annotations: ImageAnnotation[]): void {
    setImageState('annotations', (prev) => {
      const next = new Map(prev)
      next.set(attachmentId, annotations)
      return next
    })
  },
}

// ============================================================
// 公开 API - 文件附件管理
// ============================================================

/**
 * 文件附件管理 API
 *
 * 对应 design/26-editor.md S6.5.2 FileInput 组件设计。
 * 管理待发送的文件列表。
 */
export const fileAPI = {
  /**
   * 添加文件附件
   *
   * @param attachment 文件附件数据
   * @returns 是否添加成功（超出数量限制时返回 false）
   */
  add(attachment: FileAttachment): boolean {
    if (fileState.pendingFiles.length >= MAX_FILE_COUNT) {
      return false
    }
    setFileState('pendingFiles', (prev) => [...prev, attachment])
    return true
  },

  /**
   * 移除文件附件
   *
   * @param attachmentId 文件附件 ID
   */
  remove(attachmentId: string): void {
    setFileState('pendingFiles', (prev) =>
      prev.filter((file) => file.id !== attachmentId),
    )
  },

  /**
   * 清空所有文件附件
   */
  clearAll(): void {
    setFileState('pendingFiles', [])
  },

  /**
   * 获取待发送文件列表（响应式）
   */
  getPending: () => fileState.pendingFiles,
}

// ============================================================
// 导出 Store 供组件使用
// ============================================================

export {
  editorState,
  setEditorState,
  imageState,
  setImageState,
  fileState,
  setFileState,
}
