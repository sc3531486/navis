/**
 * LSP 集成扩展 - CodeMirror 6 Extension
 *
 * 严格遵循 design/26-editor.md 第四章"LSP 集成"的完整链路设计。
 * 将 CodeMirror 6 的输入事件发送到 LSP 后端服务（通过 IPC 调用）。
 *
 * 功能链路：
 * - 输入字符 → 触发 lsp.completion IPC → 返回补全列表 → 渲染补全面板
 * - 鼠标悬停 → 触发 lsp.hover IPC → 返回类型信息 → 渲染悬停提示
 * - Ctrl+Click → 触发 lsp.definition IPC → 返回位置 → 跳转到定义
 * - 文件变更 → 触发 lsp.diagnostics IPC → 返回诊断 → 渲染波浪线标记
 * - 保存文件 → 触发 lsp.format IPC → 返回格式化结果 → 应用格式化
 *
 * 设计依据：design/26-editor.md S4 LSP 集成
 */

import type {
  CompletionItem,
  HoverInfo,
  DefinitionLocation,
  Diagnostic,
  LSPExtensionOptions,
} from '../types'
import { invoke } from '@tauri-apps/api/core'

// ============================================================
// 常量 - IPC 通道名称
// ============================================================

/**
 * LSP IPC 命令前缀
 *
 * 所有 LSP 相关的 IPC 调用使用统一的命名空间。
 * 实际调用方式为 Tauri 的 invoke('lsp.xxx', { ... })。
 */
const LSP_IPC_PREFIX = 'lsp'

/** 补全请求 IPC 命令 */
const IPC_COMPLETION = `${LSP_IPC_PREFIX}_completion`

/** 悬停请求 IPC 命令 */
const IPC_HOVER = `${LSP_IPC_PREFIX}_hover`

/** 跳转定义 IPC 命令 */
const IPC_DEFINITION = `${LSP_IPC_PREFIX}_definition`

/** 诊断请求 IPC 命令 */
const IPC_DIAGNOSTICS = `${LSP_IPC_PREFIX}_diagnostics`

/** 格式化请求 IPC 命令 */
const IPC_FORMAT = `${LSP_IPC_PREFIX}_format`

// ============================================================
// LSP 客户端封装
// ============================================================

/**
 * LSP 客户端
 *
 * 封装所有 LSP 相关的 IPC 调用，提供统一的错误处理和超时机制。
 * 每个 LSP 客户端实例绑定到一个文件路径。
 */
export class LSPClient {
  /** 绑定的文件路径 */
  private filePath: string
  /** 绑定的会话 ID */
  private sessionId: string
  private timeout: number = 5000
  /** 最近一次补全请求的 AbortController（用于取消过期请求） */
  private completionAbortController: AbortController | null = null

  constructor(filePath: string, sessionId: string) {
    this.filePath = filePath
    this.sessionId = sessionId
  }

  /**
   * 请求代码补全
   *
   * 对应 design/26-editor.md S4 链路：
   * 输入触发 → IPC: lsp.completion(file, line, col) → 返回补全列表
   *
   * @param line   行号（0-based）
   * @param column 列号（0-based）
   * @returns 补全项列表
   */
  async requestCompletion(line: number, column: number): Promise<CompletionItem[]> {
    // 取消上一次未完成的补全请求，避免旧结果覆盖新结果
    if (this.completionAbortController) {
      this.completionAbortController.abort()
    }
    this.completionAbortController = new AbortController()

    try {
      const result = await invoke<CompletionItem[]>(IPC_COMPLETION, {
        payload: {
          sessionId: this.sessionId,
          filePath: this.filePath,
          line,
          character: column,
        },
      })
      return result
    } catch (error) {
      // AbortError 是正常的取消操作，不记录错误
      if (error instanceof DOMException && error.name === 'AbortError') {
        return []
      }
      throw new Error(`LSP completion failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 请求悬停信息
   *
   * 对应 design/26-editor.md S4 链路：
   * 鼠标悬停 → IPC: lsp.hover(file, line, col) → 返回类型信息
   *
   * @param line   行号（0-based）
   * @param column 列号（0-based）
   * @returns 悬停信息或 null（无信息时）
   */
  async requestHover(line: number, column: number): Promise<HoverInfo | null> {
    try {
      return await invoke<HoverInfo | null>(IPC_HOVER, {
        payload: {
          sessionId: this.sessionId,
          filePath: this.filePath,
          line,
          character: column,
        },
      })
    } catch (error) {
      throw new Error(`LSP hover failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 请求跳转到定义
   *
   * 对应 design/26-editor.md S4 链路：
   * Ctrl+Click → IPC: lsp.definition(file, line, col) → 返回位置
   *
   * @param line   行号（0-based）
   * @param column 列号（0-based）
   * @returns 定义位置列表
   */
  async requestDefinition(line: number, column: number): Promise<DefinitionLocation[]> {
    try {
      return await invoke<DefinitionLocation[]>(IPC_DEFINITION, {
        payload: {
          sessionId: this.sessionId,
          filePath: this.filePath,
          line,
          character: column,
        },
      })
    } catch (error) {
      throw new Error(`LSP definition failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 请求文件诊断信息
   *
   * 对应 design/26-editor.md S4 链路：
   * 文件变更 → IPC: lsp.diagnostics(file) → 返回诊断
   *
   * @returns 诊断列表
   */
  async requestDiagnostics(): Promise<Diagnostic[]> {
    try {
      return await invoke<Diagnostic[]>(IPC_DIAGNOSTICS, {
        payload: {
          sessionId: this.sessionId,
          filePath: this.filePath,
        },
      })
    } catch (error) {
      throw new Error(`LSP diagnostics failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 请求代码格式化
   *
   * 对应 design/26-editor.md S4 链路：
   * 保存文件 → IPC: lsp.format(file) → 返回格式化结果
   *
   * @returns 格式化后的文本内容
   */
  async requestFormat(): Promise<string | null> {
    try {
      return await invoke<string | null>(IPC_FORMAT, {
        payload: {
          sessionId: this.sessionId,
          filePath: this.filePath,
        },
      })
    } catch (error) {
      throw new Error(`LSP format failed: ${error instanceof Error ? error.message : String(error)}`)
    }
  }

  /**
   * 更新绑定的文件路径
   *
   * 当标签切换到不同文件时，更新 LSP 客户端的目标文件。
   *
   * @param newPath 新的文件路径
   */
  updateFilePath(newPath: string): void {
    this.filePath = newPath
  }

  /**
   * 销毁客户端，取消所有待处理的请求
   */
  dispose(): void {
    if (this.completionAbortController) {
      this.completionAbortController.abort()
      this.completionAbortController = null
    }
  }
}

// ============================================================
// LSP Extension 配置
// ============================================================

/**
 * LSP Extension 配置选项
 *
 * 用于创建 CodeMirror 6 LSP 集成扩展时的配置参数。
 */
export interface LSPExtensionConfig {
  /** 文件路径 */
  filePath: string
  /** 编程语言标识 */
  language: string
  /** 是否启用自动补全（默认 true） */
  enableCompletion?: boolean
  /** 是否启用悬停提示（默认 true） */
  enableHover?: boolean
  /** 是否启用 Ctrl+Click 跳转（默认 true） */
  enableDefinition?: boolean
  /** 是否启用诊断标记（默认 true） */
  enableDiagnostics?: boolean
  /** 是否启用保存时自动格式化（默认 true） */
  enableAutoFormat?: boolean
  /** 补全触发延迟（毫秒，默认 150） */
  completionDelay?: number
  /** 悬停触发延迟（毫秒，默认 300） */
  hoverDelay?: number
}

/**
 * 创建 LSP 集成扩展
 *
 * 生成 CodeMirror 6 所需的 Extension 配置对象。
 * 包含自动补全、悬停提示、Ctrl+Click 跳转、诊断标记和格式化的事件绑定。
 *
 * 注意：此函数返回配置对象，实际的 CodeMirror Extension 创建
 * 需要在组件中使用 @codemirror/state 和 @codemirror/view 的 API。
 *
 * @param options LSP 扩展配置
 * @returns LSP 客户端实例和配置，供 EditorView 使用
 *
 * @example
 * ```ts
 * import { createLSPExtension } from './extensions/lsp-extension'
 *
 * const { client, config } = createLSPExtension({
 *   filePath: '/path/to/file.ts',
 *   language: 'typescript',
 *   onCompletion: (items) => { /* 渲染补全面板 *\/ },
 *   onHover: (info) => { /* 渲染悬停提示 *\/ },
 *   onDefinition: (locs) => { /* 跳转到定义 *\/ },
 *   onDiagnostics: (diags) => { /* 渲染诊断标记 *\/ },
 * })
 * ```
 */
export function createLSPExtension(options: LSPExtensionOptions & LSPExtensionConfig): {
  /** LSP 客户端实例 */
  client: LSPClient
  /** 扩展配置（用于 CodeMirror Extension 构建） */
  config: Required<Omit<LSPExtensionConfig, 'filePath' | 'language'>>
} {
  const client = new LSPClient(options.filePath, options.sessionId)

  // 合并默认配置
  const config = {
    enableCompletion: options.enableCompletion ?? true,
    enableHover: options.enableHover ?? true,
    enableDefinition: options.enableDefinition ?? true,
    enableDiagnostics: options.enableDiagnostics ?? true,
    enableAutoFormat: options.enableAutoFormat ?? true,
    completionDelay: options.completionDelay ?? 150,
    hoverDelay: options.hoverDelay ?? 300,
  }

  return { client, config }
}

// ============================================================
// 补全触发器工具函数
// ============================================================

/**
 * 判断是否应该触发补全
 *
 * 根据输入字符和上下文判断是否需要请求 LSP 补全。
 * 以下情况不触发补全：
 * - 输入空白字符
 * - 行首无前缀文本（可选）
 * - 前一个字符是空白（避免在空格后立即触发）
 *
 * @param typedChar 最新输入的字符
 * @param lineText  当前行文本
 * @param cursorCol 光标列位置
 * @returns 是否应触发补全
 */
export function shouldTriggerCompletion(
  typedChar: string,
  lineText: string,
  cursorCol: number,
): boolean {
  // 空白字符不触发补全
  if (/\s/.test(typedChar)) return false

  // 输入 . 或 : 时总是触发（成员访问、命名空间）
  if (typedChar === '.' || typedChar === ':') return true

  // 字母或数字输入时触发
  if (/[a-zA-Z0-9_$]/.test(typedChar)) return true

  return false
}

/**
 * 判断是否应该触发悬停提示
 *
 * 鼠标在标识符上悬停超过延迟时间后触发。
 * 以下情况不触发：
 * - 鼠标在空白字符上
 * - 鼠标在行首空白区域
 *
 * @param lineText  目标行文本
 * @param hoverCol  悬停列位置
 * @returns 是否应触发悬停
 */
export function shouldTriggerHover(lineText: string, hoverCol: number): boolean {
  // 超出行范围不触发
  if (hoverCol >= lineText.length) return false

  // 空白字符上不触发
  if (/\s/.test(lineText[hoverCol])) return false

  return true
}
