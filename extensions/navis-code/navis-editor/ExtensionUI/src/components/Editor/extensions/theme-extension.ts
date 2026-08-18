/**
 * 主题扩展 - CodeMirror 6 Extension
 *
 * 严格遵循 design/26-editor.md 扩展扩展支持章节中的主题注册机制。
 * 提供编辑器主题管理，支持亮色、暗色和高对比度主题。
 *
 * 功能：
 * - 内置默认主题（亮色/暗色）
 * - 主题目录管理
 * - 动态主题切换
 * - 与应用全局主题联动
 *
 * 设计依据：design/26-editor.md 扩展扩展支持 contributes.themes
 */

import type { ThemeRegistration } from '../types'

// ============================================================
// 类型定义
// ============================================================

/**
 * 主题类型枚举
 *
 * 对应 design/26-editor.md 中 themes 的 type 字段。
 */
export type ThemeType = 'light' | 'dark' | 'highContrast'

/**
 * CodeMirror 6 主题配置
 *
 * CodeMirror 6 使用 EditorView.theme() 创建主题，
 * 此接口定义主题配置的结构化数据，便于动态注册。
 */
export interface EditorThemeConfig {
  /** 主题唯一标识 */
  id: string
  /** 主题显示名称 */
  name: string
  /** 主题类型 */
  type: ThemeType
  /** 是否为默认主题 */
  isDefault?: boolean
  /**
   * CodeMirror 6 主题样式配置
   *
   * 对应 EditorView.theme() 的第一个参数。
   * 键为 CSS 选择器，值为 CSS 属性对象。
   */
  cmTheme: Record<string, Record<string, string>>
  /**
   * 语法高亮样式
   *
   * 对应 HighlightStyle.define() 的参数。
   * 每个条目定义一个语法标记类型的颜色和样式。
   */
  highlightStyle: Array<{
    /** 语法标记类型名称（如 'keyword'、'string'、'comment'） */
    tag: string
    /** 文本颜色 */
    color?: string
    /** 字体粗细 */
    fontWeight?: string
    /** 字体样式（斜体） */
    fontStyle?: string
    /** 文本装饰（如下划线） */
    textDecoration?: string
  }>
}

// ============================================================
// 默认主题定义
// ============================================================

/**
 * 默认亮色主题配置
 *
 * 基于 VS Code Light+ 主题的配色方案。
 * CodeMirror 6 中使用 EditorView.theme() 应用。
 */
export const DEFAULT_LIGHT_THEME: EditorThemeConfig = {
  id: 'default-light',
  name: 'Default Light',
  type: 'light',
  isDefault: true,
  cmTheme: {
    /** 编辑器根容器 */
    '&': {
      backgroundColor: '#ffffff',
      color: '#1e1e1e',
    },
    /** 编辑器内容区域 */
    '.cm-content': {
      caretColor: '#000000',
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace",
      fontSize: '14px',
      lineHeight: '1.6',
    },
    /** 光标 */
    '&.cm-focused .cm-cursor': {
      borderLeftColor: '#000000',
    },
    /** 选中区域 */
    '&.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: '#add6ff',
    },
    /** 非焦点时的选中区域 */
    '.cm-selectionBackground': {
      backgroundColor: '#e5ebf1',
    },
    /** 行号Gutter */
    '.cm-gutters': {
      backgroundColor: '#f8f8f8',
      color: '#999999',
      borderRight: '1px solid #e0e0e0',
    },
    /** 活跃行号 */
    '.cm-activeLineGutter': {
      backgroundColor: '#e8e8e8',
    },
    /** 当前行高亮 */
    '.cm-activeLine': {
      backgroundColor: '#f5f5f5',
    },
    /** 匹配括号高亮 */
    '&.cm-focused .cm-matchingBracket': {
      backgroundColor: '#c9e6ca',
      outline: '1px solid #b4d8b4',
    },
    /** 搜索匹配高亮 */
    '.cm-searchMatch': {
      backgroundColor: '#ffdf5d',
    },
    /** 活跃搜索匹配高亮 */
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: '#ffa60033',
    },
    /** 折叠标记 */
    '.cm-foldPlaceholder': {
      backgroundColor: '#e8e8e8',
      color: '#666666',
      border: 'none',
    },
  },
  highlightStyle: [
    { tag: 'keyword', color: '#0000ff' },
    { tag: 'string', color: '#a31515' },
    { tag: 'comment', color: '#008000', fontStyle: 'italic' },
    { tag: 'number', color: '#098658' },
    { tag: 'typeName', color: '#267f99' },
    { tag: 'variableName', color: '#001080' },
    { tag: 'function(variableName)', color: '#795e26' },
    { tag: 'definition(variableName)', color: '#001080' },
    { tag: 'propertyName', color: '#0451a5' },
    { tag: 'operator', color: '#000000' },
    { tag: 'punctuation', color: '#000000' },
    { tag: 'meta', color: '#ff0000' },
    { tag: 'tagName', color: '#800000' },
    { tag: 'attributeName', color: '#ff0000' },
  ],
}

/**
 * 默认暗色主题配置
 *
 * 基于 VS Code Dark+ 主题的配色方案。
 */
export const DEFAULT_DARK_THEME: EditorThemeConfig = {
  id: 'default-dark',
  name: 'Default Dark',
  type: 'dark',
  isDefault: true,
  cmTheme: {
    '&': {
      backgroundColor: '#1e1e1e',
      color: '#d4d4d4',
    },
    '.cm-content': {
      caretColor: '#aeafad',
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace",
      fontSize: '14px',
      lineHeight: '1.6',
    },
    '&.cm-focused .cm-cursor': {
      borderLeftColor: '#aeafad',
    },
    '&.cm-focused .cm-selectionBackground, ::selection': {
      backgroundColor: '#264f78',
    },
    '.cm-selectionBackground': {
      backgroundColor: '#3a3d41',
    },
    '.cm-gutters': {
      backgroundColor: '#1e1e1e',
      color: '#858585',
      borderRight: '1px solid #333333',
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#2a2a2a',
    },
    '.cm-activeLine': {
      backgroundColor: '#2a2d2e',
    },
    '&.cm-focused .cm-matchingBracket': {
      backgroundColor: '#3a3d41',
      outline: '1px solid #888888',
    },
    '.cm-searchMatch': {
      backgroundColor: '#515c6a',
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: '#61687666',
    },
    '.cm-foldPlaceholder': {
      backgroundColor: '#333333',
      color: '#999999',
      border: 'none',
    },
  },
  highlightStyle: [
    { tag: 'keyword', color: '#569cd6' },
    { tag: 'string', color: '#ce9178' },
    { tag: 'comment', color: '#6a9955', fontStyle: 'italic' },
    { tag: 'number', color: '#b5cea8' },
    { tag: 'typeName', color: '#4ec9b0' },
    { tag: 'variableName', color: '#9cdcfe' },
    { tag: 'function(variableName)', color: '#dcdcaa' },
    { tag: 'definition(variableName)', color: '#9cdcfe' },
    { tag: 'propertyName', color: '#9cdcfe' },
    { tag: 'operator', color: '#d4d4d4' },
    { tag: 'punctuation', color: '#d4d4d4' },
    { tag: 'meta', color: '#569cd6' },
    { tag: 'tagName', color: '#569cd6' },
    { tag: 'attributeName', color: '#9cdcfe' },
  ],
}

// ============================================================
// 主题目录
// ============================================================

/**
 * 编辑器主题目录
 *
 * 管理所有已注册的编辑器主题。
 * 支持通过扩展系统动态注册自定义主题。
 */
class ThemeCatalog {
  /** 已注册主题映射（id → EditorThemeConfig） */
  private themes: Map<string, EditorThemeConfig> = new Map()
  /** 当前活跃主题 ID */
  private activeThemeId: string | null = null

  constructor() {
    // 注册默认主题
    this.register(DEFAULT_LIGHT_THEME)
    this.register(DEFAULT_DARK_THEME)
  }

  /**
   * 注册主题
   *
   * @param theme 主题配置
   */
  register(theme: EditorThemeConfig): void {
    this.themes.set(theme.id, theme)
  }

  /**
   * 注销主题
   *
   * @param id 主题 ID
   */
  unregister(id: string): void {
    // 不允许注销默认主题
    const theme = this.themes.get(id)
    if (theme?.isDefault) {
      console.warn(`[Theme] 不能注销默认主题 "${id}"`)
      return
    }
    this.themes.delete(id)
  }

  /**
   * 设置当前活跃主题
   *
   * @param id 主题 ID
   * @returns 是否设置成功
   */
  setActive(id: string): boolean {
    if (!this.themes.has(id)) {
      console.warn(`[Theme] 主题 "${id}" 不存在`)
      return false
    }
    this.activeThemeId = id
    return true
  }

  /**
   * 根据系统主题类型选择匹配的主题
   *
   * 如果系统为暗色模式，优先使用暗色主题，否则使用亮色主题。
   *
   * @param type 主题类型
   */
  setByType(type: ThemeType): void {
    const candidates = Array.from(this.themes.values()).filter((t) => t.type === type)
    if (candidates.length === 0) return

    // 优先选择默认主题
    const defaultTheme = candidates.find((t) => t.isDefault)
    this.activeThemeId = (defaultTheme ?? candidates[0]).id
  }

  /**
   * 获取当前活跃主题
   *
   * @returns 当前活跃主题配置或 null
   */
  getActive(): EditorThemeConfig | null {
    if (!this.activeThemeId) return null
    return this.themes.get(this.activeThemeId) ?? null
  }

  /**
   * 获取所有已注册主题
   *
   * @returns 主题配置数组
   */
  getAll(): EditorThemeConfig[] {
    return Array.from(this.themes.values())
  }

  /**
   * 按类型获取主题列表
   *
   * @param type 主题类型
   * @returns 匹配的主题配置数组
   */
  getByType(type: ThemeType): EditorThemeConfig[] {
    return this.getAll().filter((t) => t.type === type)
  }
}

// ============================================================
// 全局目录实例
// ============================================================

/**
 * 全局主题目录实例
 *
 * 应用启动时初始化，包含默认亮色和暗色主题。
 * 扩展通过此实例注册自定义主题。
 */
export const themeCatalog = new ThemeCatalog()
