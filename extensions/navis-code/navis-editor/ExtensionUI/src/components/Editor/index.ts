/**
 * Editor 编辑器模块 - 统一导出入口
 *
 * 汇总导出编辑器模块的所有公共类型、组件、扩展和 Store API。
 * 外部模块通过此文件引入编辑器功能。
 *
 * 设计依据：design/26-editor.md 第二章"架构设计"
 */

// ============================================================
// 类型导出
// ============================================================

export type {
  EditorTab,
  EditorState,
  SplitMode,
  Diagnostic,
  DiffHunk,
  DiffChange,
  DiffChangeType,
  DiffMode,
  DiffViewProps,
  ImageAttachment,
  ImageAnnotation,
  ImagePreviewMode,
  FileAttachment,
  FilePreviewProps,
  ImageInputError,
  FileInputError,
  EditorEventPayloads,
  ImageInputEventPayloads,
  FileInputEventPayloads,
  EditorExtensionEventPayloads,
  CompletionItem,
  HoverInfo,
  DefinitionLocation,
  LSPExtensionOptions,
  ThemeRegistration,
  LanguageRegistration,
  EditorExtensionRegistration,
  ImageProcessOptions,
  ImageProcessResult,
  PdfExtractOptions,
  PdfExtractResult,
  FileInfo,
} from './types'

export { DiagnosticSeverity, FileType, CompletionItemKind } from './types'

// ============================================================
// 组件导出
// ============================================================

export { EditorView } from './EditorView'
export type { EditorViewProps } from './EditorView'

export { EditorTabs } from './EditorTabs'

export { DiffView } from './DiffView'

export { CompletionPanel } from './components/CompletionPanel'
export type { CompletionPanelProps } from './components/CompletionPanel'

export { DiagnosticPanel } from './components/DiagnosticPanel'
export type { DiagnosticPanelProps } from './components/DiagnosticPanel'

export { HoverTooltip } from './components/HoverTooltip'
export type { HoverTooltipProps } from './components/HoverTooltip'

export { OutlinePanel } from './components/OutlinePanel'
export type { OutlinePanelProps, DocumentSymbol } from './components/OutlinePanel'
export { SymbolKind } from './components/OutlinePanel'

export { Minimap } from './components/Minimap'
export type { MinimapProps } from './components/Minimap'

export { ImageInput } from './components/ImageInput'
export type { ImageInputProps } from './components/ImageInput'

export { ImagePreview } from './components/ImagePreview'
export type { ImagePreviewProps } from './components/ImagePreview'

export { FileInput } from './components/FileInput'
export type { FileInputProps } from './components/FileInput'

export { FilePreview } from './components/FilePreview'

// ============================================================
// 扩展导出
// ============================================================

export { LSPClient, createLSPExtension, shouldTriggerCompletion, shouldTriggerHover } from './extensions/lsp-extension'
export type { LSPExtensionConfig } from './extensions/lsp-extension'

export { computeDiff, formatHunkHeader, DIFF_LINE_CLASSES, DIFF_LINE_PREFIX, createDiffExtension } from './extensions/diff-extension'
export type { DiffExtensionConfig } from './extensions/diff-extension'

export {
  snippetCatalog,
  parseSnippetTabStops,
  resolveSnippetBody,
  BUILTIN_SNIPPETS,
} from './extensions/snippet-extension'
export type { Snippet, SnippetTabStop } from './extensions/snippet-extension'

export {
  themeCatalog,
  DEFAULT_LIGHT_THEME,
  DEFAULT_DARK_THEME,
} from './extensions/theme-extension'
export type { ThemeType, EditorThemeConfig } from './extensions/theme-extension'

export {
  processImageFile,
  processFile,
  validateImage,
  validateFile,
  detectFileType,
  extractImagesFromClipboard,
  extractImagesFromDrop,
  extractFilesFromDrop,
  extractFilesFromClipboard,
  fileToBase64,
  readTextContent,
} from './extensions/image-drop-ext'

// ============================================================
// Store API 导出
// ============================================================

export {
  editorState,
  setEditorState,
  imageState,
  setImageState,
  fileState,
  setFileState,
  editorTabsAPI,
  editorSplitAPI,
  editorDiagnosticsAPI,
  resetEditorState,
  imageAPI,
  fileAPI,
  detectLanguage,
  IMAGE_MAX_SIZE_BYTES,
  FILE_HARD_MAX_SIZE_BYTES,
  FILE_LARGE_THRESHOLD_BYTES,
  MAX_FILE_LINES,
  MAX_IMAGE_COUNT,
  MAX_FILE_COUNT,
  FILE_READ_TIMEOUT_MS,
  CODE_FILE_LINE_THRESHOLD,
  PDF_PAGE_THRESHOLD,
  PDF_MAX_EXTRACT_PAGES,
} from './stores/editor'
