/**
 * 代码片段扩展 - CodeMirror 6 Extension
 *
 * 严格遵循 design/26-editor.md 第一章"职责边界"中的代码片段（Snippet）功能。
 * 提供代码片段的注册、搜索和插入能力。
 *
 * 功能：
 * - 代码片段目录管理
 * - 按语言过滤片段
 * - 片段插入（支持 Tab Stop 占位符）
 * - 片段搜索（模糊匹配）
 *
 * 设计依据：design/26-editor.md S1 模块概述
 */

// ============================================================
// 类型定义
// ============================================================

/**
 * 代码片段定义
 *
 * 描述一个可复用的代码模板，支持 Tab Stop 占位符。
 * Tab Stop 语法遵循 TextMate 规范：$1, $2, ${1:default}, $0（最终光标位置）。
 */
export interface Snippet {
  /** 片段唯一标识 */
  id: string
  /** 片段名称（用于搜索和显示） */
  name: string
  /** 片段描述 */
  description?: string
  /** 片段适用的编程语言列表（空数组表示通用） */
  languages: string[]
  /** 片段前缀（触发补全的输入文本） */
  prefix: string
  /** 片段内容（支持 Tab Stop 占位符） */
  body: string
  /** 片段来源 */
  source: 'builtin' | 'user' | 'extension'
}

/**
 * 片段解析后的 Tab Stop
 *
 * 将 Tab Stop 语法解析为结构化数据，便于 CodeMirror 扩展处理。
 */
export interface SnippetTabStop {
  /** Tab Stop 索引（0 表示最终光标位置） */
  index: number
  /** 在片段文本中的起始位置 */
  start: number
  /** 在片段文本中的结束位置 */
  end: number
  /** 默认值（${1:default} 中的 default 部分） */
  defaultValue?: string
}

// ============================================================
// 片段目录
// ============================================================

/**
 * 代码片段目录
 *
 * 管理所有注册的代码片段，提供搜索和查询能力。
 * 片段来源包括：
 * - builtin: 内置片段（随编辑器提供）
 * - user: 用户自定义片段
 * - extension: 扩展注册的片段
 */
class SnippetCatalog {
  /** 已注册的片段映射（id → Snippet） */
  private snippets: Map<string, Snippet> = new Map()

  /**
   * 注册一个代码片段
   *
   * @param snippet 片段定义
   */
  register(snippet: Snippet): void {
    if (this.snippets.has(snippet.id)) {
      console.warn(`[Snippet] 片段 "${snippet.id}" 已存在，将被覆盖`)
    }
    this.snippets.set(snippet.id, snippet)
  }

  /**
   * 批量注册代码片段
   *
   * @param snippets 片段定义数组
   */
  registerBatch(snippets: Snippet[]): void {
    for (const snippet of snippets) {
      this.register(snippet)
    }
  }

  /**
   * 注销一个代码片段
   *
   * @param id 片段 ID
   */
  unregister(id: string): void {
    this.snippets.delete(id)
  }

  /**
   * 获取所有已注册片段
   *
   * @returns 片段数组
   */
  getAll(): Snippet[] {
    return Array.from(this.snippets.values())
  }

  /**
   * 按语言过滤片段
   *
   * 返回适用于指定语言的片段（包括通用片段）。
   *
   * @param language 编程语言标识
   * @returns 匹配的片段数组
   */
  getByLanguage(language: string): Snippet[] {
    return this.getAll().filter(
      (snippet) =>
        snippet.languages.length === 0 || // 通用片段
        snippet.languages.includes(language),
    )
  }

  /**
   * 按前缀搜索片段
   *
   * 支持前缀匹配和模糊匹配，返回按匹配度排序的结果。
   *
   * @param prefix 输入前缀
   * @param language 可选的语言过滤
   * @returns 匹配的片段数组（按匹配度排序）
   */
  search(prefix: string, language?: string): Snippet[] {
    const candidates = language ? this.getByLanguage(language) : this.getAll()
    if (!prefix.trim()) return candidates

    const lowerPrefix = prefix.toLowerCase()

    return candidates
      .map((snippet) => ({
        snippet,
        score: computeSnippetScore(lowerPrefix, snippet),
      }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((item) => item.snippet)
  }
}

/**
 * 计算片段匹配分数
 *
 * 优先匹配前缀，其次匹配名称和描述。
 *
 * @param query 查询文本（已转小写）
 * @param snippet 片段定义
 * @returns 匹配分数（0 表示不匹配）
 */
function computeSnippetScore(query: string, snippet: Snippet): number {
  const lowerPrefix = snippet.prefix.toLowerCase()
  const lowerName = snippet.name.toLowerCase()

  // 前缀完全匹配 → 最高分
  if (lowerPrefix === query) return 100

  // 前缀前缀匹配
  if (lowerPrefix.startsWith(query)) return 80

  // 名称包含查询
  if (lowerName.includes(query)) return 60

  // 前缀包含查询
  if (lowerPrefix.includes(query)) return 40

  // 描述包含查询
  if (snippet.description?.toLowerCase().includes(query)) return 20

  return 0
}

// ============================================================
// Tab Stop 解析
// ============================================================

/**
 * 解析片段内容中的 Tab Stop 占位符
 *
 * 支持的 Tab Stop 语法：
 * - $0        → 最终光标位置
 * - $1, $2... → 按索引的 Tab Stop
 * - ${1:text} → 带默认值的 Tab Stop
 *
 * @param body 片段内容文本
 * @returns 解析后的 Tab Stop 列表（按索引排序）
 */
export function parseSnippetTabStops(body: string): SnippetTabStop[] {
  const tabStops: SnippetTabStop[] = []
  /** Tab Stop 正则：匹配 $N 或 ${N:default} */
  const regex = /\$(?:(\d+)|\{(\d+)(?::([^}]*))?\})/g

  let match: RegExpExecArray | null
  while ((match = regex.exec(body)) !== null) {
    const index = parseInt(match[1] ?? match[2], 10)
    tabStops.push({
      index,
      start: match.index,
      end: match.index + match[0].length,
      defaultValue: match[3],
    })
  }

  // 按索引排序（$0 排在最后）
  return tabStops.sort((a, b) => {
    if (a.index === 0) return 1
    if (b.index === 0) return -1
    return a.index - b.index
  })
}

/**
 * 将片段内容中的 Tab Stop 替换为默认值
 *
 * 用于纯文本插入场景（不需要 Tab Stop 跳转）。
 *
 * @param body 片段内容文本
 * @returns 替换后的纯文本
 */
export function resolveSnippetBody(body: string): string {
  return body.replace(/\$(?:(\d+)|\{(\d+)(?::([^}]*))?\})/g, (match, simple, _complex, defaultVal) => {
    // 有默认值 → 使用默认值
    if (defaultVal !== undefined) return defaultVal
    // $0 或无默认值 → 替换为空字符串
    return ''
  })
}

// ============================================================
// 内置片段定义
// ============================================================

/**
 * 内置代码片段列表
 *
 * 提供常用语言的基础代码片段，覆盖高频使用场景。
 */
export const BUILTIN_SNIPPETS: Snippet[] = [
  // TypeScript / JavaScript
  {
    id: 'ts-function',
    name: 'Arrow Function',
    description: '箭头函数声明',
    languages: ['typescript', 'javascript'],
    prefix: 'afn',
    body: 'const ${1:name} = (${2:params}) => {\n\t$0\n}',
    source: 'builtin',
  },
  {
    id: 'ts-async-function',
    name: 'Async Arrow Function',
    description: '异步箭头函数声明',
    languages: ['typescript', 'javascript'],
    prefix: 'aafn',
    body: 'const ${1:name} = async (${2:params}) => {\n\t$0\n}',
    source: 'builtin',
  },
  {
    id: 'ts-interface',
    name: 'Interface',
    description: 'TypeScript 接口声明',
    languages: ['typescript'],
    prefix: 'intf',
    body: 'interface ${1:Name} {\n\t$0\n}',
    source: 'builtin',
  },
  {
    id: 'ts-type',
    name: 'Type Alias',
    description: 'TypeScript 类型别名',
    languages: ['typescript'],
    prefix: 'tp',
    body: 'type ${1:Name} = ${2:type}',
    source: 'builtin',
  },
  {
    id: 'ts-import',
    name: 'Import',
    description: 'ES Module 导入语句',
    languages: ['typescript', 'javascript'],
    prefix: 'imp',
    body: "import { ${2:module} } from '${1:package}'",
    source: 'builtin',
  },
  {
    id: 'ts-console-log',
    name: 'Console Log',
    description: '控制台日志输出',
    languages: ['typescript', 'javascript'],
    prefix: 'cl',
    body: "console.log(${1:value})",
    source: 'builtin',
  },
  // Solid.js
  {
    id: 'solid-component',
    name: 'Solid Component',
    description: 'Solid.js 函数式组件',
    languages: ['typescript', 'javascript'],
    prefix: 'scomp',
    body: "import { Component } from 'solid-js'\n\nconst ${1:Name}: Component = () => {\n\treturn (\n\t\t<div>$0</div>\n\t)\n}\n\nexport default ${1:Name}",
    source: 'builtin',
  },
  // Python
  {
    id: 'py-function',
    name: 'Python Function',
    description: 'Python 函数定义',
    languages: ['python'],
    prefix: 'def',
    body: 'def ${1:name}(${2:params}):\n\t${0:pass}',
    source: 'builtin',
  },
  {
    id: 'py-class',
    name: 'Python Class',
    description: 'Python 类定义',
    languages: ['python'],
    prefix: 'cls',
    body: 'class ${1:Name}:\n\tdef __init__(self${2:, params}):\n\t\t${0:pass}',
    source: 'builtin',
  },
  // Rust
  {
    id: 'rs-function',
    name: 'Rust Function',
    description: 'Rust 函数定义',
    languages: ['rust'],
    prefix: 'fn',
    body: 'fn ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t$0\n}',
    source: 'builtin',
  },
  {
    id: 'rs-struct',
    name: 'Rust Struct',
    description: 'Rust 结构体定义',
    languages: ['rust'],
    prefix: 'st',
    body: 'struct ${1:Name} {\n\t${0:field}: Type,\n}',
    source: 'builtin',
  },
  {
    id: 'rs-impl',
    name: 'Rust Impl Block',
    description: 'Rust impl 块',
    languages: ['rust'],
    prefix: 'imp',
    body: 'impl ${1:Type} {\n\t$0\n}',
    source: 'builtin',
  },
]

// ============================================================
// 全局目录实例
// ============================================================

/**
 * 全局代码片段目录实例
 *
 * 应用启动时注册内置片段，用户和扩展可通过此实例注册自定义片段。
 */
export const snippetCatalog = new SnippetCatalog()

// 初始化：注册内置片段
snippetCatalog.registerBatch(BUILTIN_SNIPPETS)
