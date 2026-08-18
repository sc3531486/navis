/**
 * Editor 编辑器模块 - 核心类型定义
 *
 * 严格遵循 design/26-editor.md 第三章"数据模型" & 第七章"事件定义"。
 * 包含编辑器标签、状态、Diff 视图、图片附件、文件附件等所有类型声明。
 */

// ============================================================
// 一、数据模型 - 对应设计文档 S3 数据模型
// ============================================================

/**
 * 编辑器标签页数据结构
 *
 * 描述一个打开的代码文件标签，包含文件路径、语言、脏标记等元信息。
 */
export interface EditorTab {
  /** 标签唯一标识符 */
  id: string
  /** 文件绝对路径 */
  filePath: string
  /** 文件名（含扩展名） */
  fileName: string
  /** 编程语言标识，如 'typescript'、'python'、'rust' */
  language: string
  /** 是否有未保存的修改 */
  isDirty: boolean
  /** 是否已固定（固定标签不会被自动关闭） */
  isPinned: boolean
}

/**
 * 分屏模式
 * - none:       不分屏，单编辑器
 * - horizontal: 上下分屏
 * - vertical:   左右分屏
 */
export type SplitMode = 'none' | 'horizontal' | 'vertical'

/**
 * LSP 诊断条目
 *
 * 对应 LSP 协议的 Diagnostic 结构，用于标记代码中的错误和警告。
 */
export interface Diagnostic {
  /** 诊断唯一标识 */
  id: string
  /** 诊断所属文件路径 */
  filePath: string
  /** 诊断严重级别 */
  severity: DiagnosticSeverity
  /** 诊断消息文本 */
  message: string
  /** 诊断来源（如 'typescript'、'eslint'） */
  source: string
  /** 诊断代码（可选，如错误码 'TS2322'） */
  code?: string | number
  /** 起始行号（0-based） */
  startLine: number
  /** 起始列号（0-based） */
  startColumn: number
  /** 结束行号（0-based） */
  endLine: number
  /** 结束列号（0-based） */
  endColumn: number
}

/**
 * 诊断严重级别
 * 对应 LSP 协议的 DiagnosticSeverity 枚举
 */
export enum DiagnosticSeverity {
  Error = 1,
  Warning = 2,
  Information = 3,
  Hint = 4,
}

/**
 * 编辑器全局状态
 *
 * 管理所有打开的标签、当前活跃标签、分屏模式和诊断信息。
 * 使用 Solid.js 的 createStore 实现响应式状态管理。
 */
export interface EditorState {
  /** 所有打开的标签页 */
  tabs: EditorTab[]
  /** 当前活跃标签页 ID */
  activeTabId: string | null
  /** 分屏模式 */
  splitMode: SplitMode
  /** 按文件路径索引的诊断信息 */
  diagnostics: Map<string, Diagnostic[]>
}

// ============================================================
// 二、Diff 视图 - 对应设计文档 S5 Diff 视图
// ============================================================

/**
 * Diff 视图显示模式
 * - unified:     上下对照（适合小改动）
 * - side-by-side: 左右对照（适合大改动）
 */
export type DiffMode = 'unified' | 'side-by-side'

/**
 * Diff Hunk（差异块）
 *
 * 表示原始文件和修改文件之间的一段连续差异。
 */
export interface DiffHunk {
  /** Hunk 唯一标识 */
  id: string
  /** 原始文件起始行号（1-based） */
  originalStartLine: number
  /** 原始文件影响行数 */
  originalLineCount: number
  /** 修改文件起始行号（1-based） */
  modifiedStartLine: number
  /** 修改文件影响行数 */
  modifiedLineCount: number
  /** 变更行列表 */
  changes: DiffChange[]
  /** 用户是否已确认该 Hunk */
  confirmed?: boolean
  /** 用户是否已拒绝该 Hunk */
  rejected?: boolean
}

/**
 * Diff 变更行类型
 * - addition: 新增行
 * - deletion: 删除行
 * - context:  上下文（未变更的行）
 */
export type DiffChangeType = 'addition' | 'deletion' | 'context'

/**
 * Diff 变更行
 */
export interface DiffChange {
  /** 变更类型 */
  type: DiffChangeType
  /** 行内容（不含 +/-/空格 前缀） */
  content: string
  /** 原始文件行号（仅 context/deletion 有效，1-based） */
  originalLine?: number
  /** 修改文件行号（仅 context/addition 有效，1-based） */
  modifiedLine?: number
}

/**
 * Diff 视图组件 Props
 *
 * 对应 design/26-editor.md 第五章 Diff 视图接口定义。
 */
export interface DiffViewProps {
  /** 文件路径 */
  filePath: string
  /** 原始文件内容 */
  original: string
  /** 修改后文件内容 */
  modified: string
  /** Diff Hunk 列表 */
  hunks: DiffHunk[]
  /** 确认全部修改的回调 */
  onConfirm: () => void
  /** 拒绝全部修改的回调 */
  onReject: () => void
  /** 确认单个 Hunk 的回调（可选） */
  onConfirmHunk?: (hunkId: string) => void
  /** 拒绝单个 Hunk 的回调（可选） */
  onRejectHunk?: (hunkId: string) => void
}

// ============================================================
// 三、图片附件 - 对应设计文档 S6.2 ImageInput/ImagePreview
// ============================================================

/**
 * 图片附件（用户上传或粘贴的图片）
 *
 * 包含图片的二进制数据（Base64）、尺寸信息和来源方式。
 * 作为 Chat 消息的多模态内容一并提交给 Agent。
 */
export interface ImageAttachment {
  /** 附件唯一标识 */
  id: string
  /** 原始文件名 */
  fileName: string
  /** MIME 类型：image/png, image/jpeg, image/gif, image/webp */
  mimeType: string
  /** 图片二进制数据（Base64 编码，用于前端预览和传输） */
  dataBase64: string
  /** 图片宽度（像素） */
  width: number
  /** 图片高度（像素） */
  height: number
  /** 文件大小（字节） */
  sizeBytes: number
  /** 来源方式 */
  source: 'paste' | 'drop' | 'file-picker'
  /** 创建时间（Unix 时间戳毫秒） */
  createdAt: number
}

/**
 * 图片标注对象
 *
 * 在 ImagePreview 的 Lightbox 模式下，用户可以对图片进行标注，
 * 标注结果随消息发送给 Agent，作为二次修改指令。
 */
export interface ImageAnnotation {
  /** 标注唯一标识 */
  id: string
  /** 所属图片附件 ID */
  attachmentId: string
  /** 标注类型 */
  type: 'rect' | 'arrow' | 'text'
  /** 标注坐标 X（相对于图片原始尺寸的比例 0-1） */
  x: number
  /** 标注坐标 Y（相对于图片原始尺寸的比例 0-1） */
  y: number
  /** 标注宽度（仅 rect 类型，比例值 0-1） */
  width?: number
  /** 标注高度（仅 rect 类型，比例值 0-1） */
  height?: number
  /** 箭头终点坐标 X（仅 type=arrow 时有效，比例值） */
  endX?: number
  /** 箭头终点坐标 Y（仅 type=arrow 时有效，比例值） */
  endY?: number
  /** 文字内容（仅 type=text 时有效） */
  text?: string
  /** 标注颜色（CSS 颜色值，如 '#ff0000'） */
  color: string
}

/**
 * 图片预览渲染模式
 * - thumbnail: 内联缩略图，自适应容器宽度
 * - lightbox:  全屏大图，支持缩放平移
 * - annotate:  标注模式，显示标注工具栏
 */
export type ImagePreviewMode = 'thumbnail' | 'lightbox' | 'annotate'

// ============================================================
// 四、文件附件 - 对应设计文档 S6.5 FileInput/FilePreview
// ============================================================

/**
 * 文件类型分类
 *
 * 基于文件扩展名映射到类型分类，决定后续的处理策略。
 */
export enum FileType {
  /** 代码文件：.ts/.js/.py/.rs/.go/.java/.c/.cpp/.html/.css/.vue/.svelte 等 */
  Code = 'code',
  /** 文档文件：.md/.txt/.rst/.pdf/.docx */
  Document = 'document',
  /** 配置文件：.env/.gitignore/Dockerfile/.json/.yaml/.toml */
  Config = 'config',
  /** 图片文件（走已有图片处理流程） */
  Image = 'image',
  /** 未识别类型 */
  Unknown = 'unknown',
}

/**
 * 文件附件（用户拖拽或选择的非图片文件）
 *
 * 包含文件元信息和解析后的文本内容。
 * 对于大文件，文本内容会被截取并标记 isTruncated。
 */
export interface FileAttachment {
  /** 附件唯一标识 */
  id: string
  /** 原始文件名 */
  fileName: string
  /** 文件绝对路径（本地文件拖拽时提供） */
  filePath: string
  /** MIME 类型 */
  mimeType: string
  /** 文件大小（字节） */
  sizeBytes: number
  /** 文件类型分类 */
  fileType: FileType
  /** 来源方式 */
  source: 'paste' | 'drop' | 'picker'
  /** 创建时间（Unix 时间戳毫秒） */
  createdAt: number
  /** 文件文本内容（代码/文档/配置文件解析后填充，大文件为截取内容） */
  textContent?: string
  /** 内容是否被截取（大文件策略生效时为 true） */
  isTruncated?: boolean
  /** 原始总行数（大文件截取时记录） */
  totalLines?: number
}

/**
 * 文件预览组件 Props
 *
 * 对应 design/26-editor.md S6.5.2 FilePreview 组件设计。
 */
export interface FilePreviewProps {
  /** 文件附件数据 */
  attachment: FileAttachment
  /** 是否可移除，默认 false */
  removable?: boolean
  /** 预览行数，默认 10 */
  maxPreviewLines?: number
  /** 移除附件的回调 */
  onRemove?: (id: string) => void
  /** 点击展开/收起的回调 */
  onExpand?: (id: string) => void
}

// ============================================================
// 五、错误类型 - 对应设计文档 S6.5.5 约束与限制
// ============================================================

/**
 * 图片输入错误类型
 *
 * 对应 design/26-editor.md S7 事件定义中的 ImageInputError。
 */
export type ImageInputError =
  | 'format-unsupported'   // 不支持的图片格式
  | 'size-exceeded'        // 图片大小超限
  | 'count-exceeded'       // 图片数量超限
  | 'read-failed'          // 图片读取失败

/**
 * 文件输入错误类型
 *
 * 对应 design/26-editor.md S7 事件定义中的 FileInputError。
 */
export type FileInputError =
  | 'format-unsupported'   // 不支持的文件格式
  | 'size-exceeded'        // 文件大小超限
  | 'count-exceeded'       // 文件数量超限
  | 'read-failed'          // 文件读取失败
  | 'read-timeout'         // 文件读取超时
  | 'parse-failed'         // 文件解析失败（PDF/DOCX 提取失败）

// ============================================================
// 六、事件类型 - 对应设计文档 S7 事件定义
// ============================================================

/**
 * 编辑器事件负载映射表
 *
 * 键为事件名称，值为该事件携带的负载数据类型。
 * 用于类型安全的事件订阅和分发。
 */
export interface EditorEventPayloads {
  /** 文件打开事件 */
  'editor.file.opened': { sessionId: string; filePath: string }
  /** 文件关闭事件 */
  'editor.file.closed': { sessionId: string; filePath: string }
  /** 文件保存事件 */
  'editor.file.saved': { sessionId: string; filePath: string }
  /** 文件脏状态变更事件 */
  'editor.file.dirty': { sessionId: string; filePath: string; isDirty: boolean }
  /** 标签切换事件 */
  'editor.tab.changed': { sessionId: string; tabId: string }
  /** 分屏模式变更事件 */
  'editor.split.changed': { sessionId: string; mode: string }
  /** 光标位置变更事件 */
  'editor.cursor.changed': { sessionId: string; line: number; character: number }
}

/**
 * 图片输入事件负载映射表
 *
 * 对应 design/26-editor.md S7 事件定义中的 ImageInputEvents。
 */
export interface ImageInputEventPayloads {
  /** 图片添加到消息上下文 */
  'image.added': { sessionId: string; attachment: ImageAttachment }
  /** 图片从消息上下文移除 */
  'image.removed': { sessionId: string; attachmentId: string }
  /** 图片处理出错（格式不支持、大小超限等） */
  'image.error': { sessionId: string; fileName: string; error: ImageInputError }
  /** 图片标注变更 */
  'image.annotation.changed': { sessionId: string; attachmentId: string; annotations: ImageAnnotation[] }
  /** 图片标注导出（用于发送给 Agent） */
  'image.annotation.exported': { sessionId: string; attachmentId: string; imageData: string; annotations: ImageAnnotation[] }
}

/**
 * 文件输入事件负载映射表
 *
 * 对应 design/26-editor.md S7 事件定义中的 FileInputEvents。
 */
export interface FileInputEventPayloads {
  /** 文件添加到消息上下文 */
  'file.added': { sessionId: string; attachment: FileAttachment }
  /** 文件从消息上下文移除 */
  'file.removed': { sessionId: string; attachmentId: string }
  /** 文件解析完成（内容已填充到 attachment.textContent） */
  'file.parsed': { sessionId: string; attachmentId: string; lineCount: number; isTruncated: boolean }
  /** 文件处理出错（格式不支持、大小超限、读取失败等） */
  'file.error': { sessionId: string; fileName: string; error: FileInputError }
}

/**
 * 编辑器扩展事件负载映射表
 *
 * 对应 design/26-editor.md 扩展扩展支持章节中的 EditorExtensionEvents。
 */
export interface EditorExtensionEventPayloads {
  /** 主题注册事件 */
  'editor.theme.registered': { themeId: string; name: string; type: 'light' | 'dark' | 'highContrast' }
  /** 语言模式注册事件 */
  'editor.language.registered': { languageId: string; name: string; extensions: string[] }
  /** 扩展注册事件 */
  'editor.extension.registered': { extensionId: string; name: string }
  /** 扩展激活事件 */
  'editor.extension.activated': { sessionId: string; extensionId: string }
  /** 扩展停用事件 */
  'editor.extension.deactivated': { sessionId: string; extensionId: string }
}

// ============================================================
// 七、LSP 集成接口 - 对应设计文档 S4 LSP 集成
// ============================================================

/**
 * LSP 补全项
 *
 * 对应 LSP 协议的 CompletionItem 结构。
 */
export interface CompletionItem {
  /** 补全项标签（显示文本） */
  label: string
  /** 补全项类型 */
  kind: CompletionItemKind
  /** 补全项详细信息 */
  detail?: string
  /** 补全项文档说明（Markdown 格式） */
  documentation?: string
  /** 插入文本 */
  insertText: string
  /** 排序优先级（数字越小越靠前） */
  sortText?: string
  /** 是否已弃用 */
  deprecated?: boolean
}

/**
 * 补全项类型枚举
 * 对应 LSP 协议的 CompletionItemKind
 */
export enum CompletionItemKind {
  Text = 1,
  Method = 2,
  Function = 3,
  Constructor = 4,
  Field = 5,
  Variable = 6,
  Class = 7,
  Interface = 8,
  Module = 9,
  Property = 10,
  Unit = 11,
  Value = 12,
  Enum = 13,
  Keyword = 14,
  Snippet = 15,
  Color = 16,
  File = 17,
  Reference = 18,
  Folder = 19,
  EnumMember = 20,
  Constant = 21,
  Struct = 22,
  Event = 23,
  Operator = 24,
  TypeParameter = 25,
}

/**
 * LSP 悬停信息
 *
 * 对应 LSP 协议的 Hover 响应结构。
 */
export interface HoverInfo {
  /** 悬停内容（Markdown 格式） */
  contents: string
  /** 内容类型：纯文本或 Markdown */
  contentType: 'plaintext' | 'markdown'
  /** 语言标识（用于语法高亮） */
  language?: string
}

/**
 * LSP 跳转位置
 *
 * 对应 LSP 协议的 Location 结构。
 */
export interface DefinitionLocation {
  /** 目标文件路径 */
  filePath: string
  /** 目标行号（0-based） */
  line: number
  /** 目标列号（0-based） */
  column: number
  /** 目标行的预览文本（可选） */
  previewText?: string
}

/**
 * LSP 集成扩展的配置选项
 */
export interface LSPExtensionOptions {
  /** 会话 ID */
  sessionId: string
  /** 文件路径 */
  filePath: string
  /** 编程语言标识 */
  language: string
  /** 补全触发回调 */
  onCompletion: (items: CompletionItem[]) => void
  /** 悬停信息回调 */
  onHover: (info: HoverInfo | null) => void
  /** 跳转位置回调 */
  onDefinition: (locations: DefinitionLocation[]) => void
  /** 诊断更新回调 */
  onDiagnostics: (diagnostics: Diagnostic[]) => void
  /** 格式化结果回调 */
  onFormat: (formatted: string) => void
}

// ============================================================
// 八、扩展扩展注册 - 对应设计文档 扩展扩展支持
// ============================================================

/**
 * 扩展主题注册信息
 *
 * 对应 design/26-editor.md contributes.themes 定义。
 */
export interface ThemeRegistration {
  /** 主题唯一标识 */
  id: string
  /** 主题显示名称 */
  name: string
  /** 主题类型 */
  type: 'light' | 'dark' | 'highContrast'
  /** 主题模块路径（CodeMirror 6 Extension 导出） */
  module: string
}

/**
 * 扩展语言注册信息
 *
 * 对应 design/26-editor.md contributes.editorLanguages 定义。
 */
export interface LanguageRegistration {
  /** 语言唯一标识 */
  id: string
  /** 语言显示名称 */
  name: string
  /** 关联的文件扩展名列表 */
  extensions: string[]
  /** 语言模块路径（Lezer 语法定义或 StreamLanguage 导出） */
  module: string
}

/**
 * 扩展编辑器扩展注册信息
 *
 * 对应 design/26-editor.md contributes.editorExtensions 定义。
 */
export interface EditorExtensionRegistration {
  /** 扩展唯一标识 */
  id: string
  /** 扩展显示名称 */
  name: string
  /** 扩展描述 */
  description?: string
  /** 扩展模块路径（CodeMirror 6 Extension 导出） */
  module: string
  /** 激活事件列表 */
  activationEvents: string[]
}

// ============================================================
// 九、IPC 接口 - 对应设计文档 多模态后端处理
// ============================================================

/**
 * 图片处理选项
 *
 * 对应 design/26-editor.md IPC 接口 file.processImage 的 options 参数。
 */
export interface ImageProcessOptions {
  /** 最大宽度（像素），默认 1920 */
  maxWidth?: number
  /** 最大高度（像素），默认 1080 */
  maxHeight?: number
  /** 输出格式，默认 webp */
  format?: 'webp' | 'avif' | 'jpeg' | 'png'
  /** 压缩质量（0-100），默认 80 */
  quality?: number
}

/**
 * 图片处理结果
 *
 * 对应 design/26-editor.md IPC 接口 file.processImage 的返回值。
 */
export interface ImageProcessResult {
  /** Base64 编码的图片数据 */
  base64: string
  /** 处理后图片宽度 */
  width: number
  /** 处理后图片高度 */
  height: number
  /** 处理后文件大小（字节） */
  size: number
}

/**
 * PDF 解析选项
 *
 * 对应 design/26-editor.md IPC 接口 file.extractPdfText 的 options 参数。
 */
export interface PdfExtractOptions {
  /** 最大页数，默认 50 */
  maxPages?: number
  /** 是否包含元数据，默认 true */
  includeMetadata?: boolean
}

/**
 * PDF 解析结果
 *
 * 对应 design/26-editor.md IPC 接口 file.extractPdfText 的返回值。
 */
export interface PdfExtractResult {
  /** 提取的纯文本内容 */
  text: string
  /** PDF 总页数 */
  pageCount: number
  /** 文档元数据（可选） */
  metadata?: Record<string, string>
}

/**
 * 文件信息
 *
 * 对应 design/26-editor.md IPC 接口 file.getFileInfo 的返回值。
 */
export interface FileInfo {
  /** 文件名 */
  name: string
  /** 文件大小（字节） */
  size: number
  /** MIME 类型 */
  mimeType: string
  /** 是否支持作为多模态输入 */
  isSupported: boolean
}
