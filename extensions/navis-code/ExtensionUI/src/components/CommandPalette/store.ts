/**
 * CommandPalette 命令目录与状态管理
 *
 * 职责：
 * 1. 维护全局命令目录（register / unregister / registerBatch）
 * 2. 管理命令面板的 UI 状态（打开/关闭、查询、选中索引等）
 * 3. 提供命令搜索能力（模糊搜索、按类型过滤）
 * 4. 持久化最近使用记录（localStorage）
 *
 * 设计依据：design/23-command-palette.md 第三章"数据模型" & 第六章"接口定义"
 */

import { createStore } from 'solid-js/store'
import { activeSession, activeSessionId } from '@session/stores/session-tree'
import { readSessionWorktreeFile, worktreeState } from '@session/stores/worktree'
import { requestEditorWorktreeFileOpen } from '@editor-ext/components/Editor/stores/editor-worktree'
import { openRightWorkspacePanel } from '@/stores/host'

// ============================================================
// 类型定义
// ============================================================

/**
 * 命令来源类型
 * - builtin: 框架内置命令
 * - extension:  扩展注册的命令
 * - skill:   技能（Skill）触发命令
 * - command: 轻量命令（.navis/commands/ 下的 .md 文件）
 */
export type CommandSource = 'builtin' | 'extension' | 'skill' | 'command' | 'file' | 'symbol'

/**
 * 命令面板的搜索范围（由前缀触发器决定）
 * - commands: ">" 前缀 → 仅搜索命令
 * - files:    "@" 前缀 → 仅搜索文件
 * - slash:    "/" 前缀 → 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
 * - symbols:  "#" 前缀 → 仅搜索符号
 */
export type CommandScope = 'commands' | 'files' | 'slash' | 'symbols'

/**
 * 单个命令的完整定义
 *
 * 对应 design/23-command-palette.md 中的 Command 接口。
 * 所有模块通过 register() 将 Command 注册到命令面板。
 */
export interface Command {
  /** 命令唯一标识符，如 "session.create"、"extension.format" */
  id: string
  /** 命令显示名称，如 "新建会话" */
  label: string
  /** 命令描述（可选），用于搜索时匹配 */
  description?: string
  /** 命令分类，如 "Session"、"Terminal"、"General" */
  category: string
  /** 快捷键显示文本，如 "Ctrl+Shift+N"（仅用于展示） */
  keybinding?: string
  /** 图标名称（Lucide icon 名），如 "terminal"、"settings" */
  icon?: string
  /** 命令执行函数，支持同步/异步 */
  handler: () => void | Promise<void>
  /** 是否启用（可选），返回 false 时命令不可用 */
  isEnabled?: () => boolean
  /** 命令来源 */
  source: CommandSource
  /** 搜索标签（可选），额外的搜索关键词，用于 AI 推荐匹配 */
  tags?: string[]
}

/**
 * 文件搜索结果（@ 前缀触发）
 */
export interface FileResult {
  /** 文件路径 */
  path: string
  /** 文件名 */
  name: string
  /** 文件类型图标 */
  icon?: string
}

/**
 * 符号搜索结果（# 前缀触发）
 */
export interface SymbolResult {
  /** 符号名称 */
  name: string
  /** 符号类型（函数、类、变量等） */
  kind: string
  /** 所在文件路径 */
  filePath: string
  /** 所在行号 */
  line: number
}

/**
 * 命令面板 UI 状态
 *
 * 对应 design/23-command-palette.md 中的 CommandPaletteState 接口。
 * 使用 Solid.js 的 createStore 实现响应式状态管理。
 */
export interface CommandPaletteState {
  /** 面板是否打开 */
  isOpen: boolean
  /** 用户输入的搜索查询 */
  query: string
  /** 当前选中的命令索引（用于键盘导航） */
  selectedIndex: number
  /** 所有已注册的命令 */
  commands: Command[]
  /** 经过搜索过滤后的命令列表 */
  filteredCommands: Command[]
  /** 最近使用的命令 ID 列表（持久化到 localStorage） */
  recentCommands: string[]
  /** 当前搜索范围（由前缀决定） */
  scope: CommandScope | null
}

// ============================================================
// 常量
// ============================================================

/** localStorage 键名：最近使用的命令 */
const STORAGE_KEY_RECENT_COMMANDS = 'navis.commandPalette.recentCommands'
/** 最近使用记录的最大条数 */
const MAX_RECENT_COMMANDS = 20

// ============================================================
// 工具函数
// ============================================================

/**
 * 从 localStorage 加载最近使用的命令列表
 * 如果读取失败（如 SSR、隐私模式），返回空数组
 */
function loadRecentCommands(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_RECENT_COMMANDS)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    // 防御性检查：确保是字符串数组
    if (Array.isArray(parsed) && parsed.every((item) => typeof item === 'string')) {
      return parsed.slice(0, MAX_RECENT_COMMANDS)
    }
    return []
  } catch {
    return []
  }
}

/**
 * 将最近使用的命令列表持久化到 localStorage
 */
function saveRecentCommands(ids: string[]): void {
  try {
    localStorage.setItem(
      STORAGE_KEY_RECENT_COMMANDS,
      JSON.stringify(ids.slice(0, MAX_RECENT_COMMANDS)),
    )
  } catch {
    // 静默失败，不影响主流程
  }
}

/**
 * 模糊搜索算法
 *
 * 策略：将查询拆分为多个 token，每个 token 必须在目标文本中按序出现。
 * 匹配时优先返回：
 * 1. 前缀匹配（权重最高）
 * 2. 连续匹配
 * 3. 分散匹配（权重最低）
 *
 * @param query  用户输入的搜索查询
 * @param target 目标文本（命令名/描述/分类等）
 * @returns 匹配分数，0 表示不匹配，正数越大匹配度越高
 */
export function fuzzyMatch(query: string, target: string): number {
  // 空查询匹配所有内容
  if (!query.trim()) return 1

  const q = query.toLowerCase()
  const t = target.toLowerCase()

  // 完全包含查询字符串 → 高分
  if (t.includes(q)) {
    // 前缀匹配 → 最高分
    if (t.startsWith(q)) return 100
    // 单词前缀匹配（如 "nc" 匹配 "new conversation"）
    const words = t.split(/\s+/)
    if (words.some((w) => w.startsWith(q))) return 80
    return 60
  }

  // 子序列匹配：检查查询的每个字符是否按序出现在目标中
  let qi = 0
  let score = 0
  let consecutiveCount = 0 // 连续匹配计数

  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      qi++
      consecutiveCount++
      // 连续匹配加分
      score += consecutiveCount * 2
      // 单词首字符匹配额外加分
      if (ti === 0 || t[ti - 1] === ' ' || t[ti - 1] === '.' || t[ti - 1] === '-') {
        score += 5
      }
    } else {
      consecutiveCount = 0
    }
  }

  // 所有查询字符都匹配了 → 返回分数
  if (qi === q.length) {
    // 优先匹配较短的目标文本（更精确的匹配）
    score += Math.max(0, 20 - (t.length - q.length))
    return score
  }

  return 0
}

function fileNameFromPath(path: string): string {
  return path.split(/[\\/]+/).filter(Boolean).at(-1) ?? path
}

function openActiveSessionFilePanel(): void {
  const sessionId = activeSessionId()
  if (!sessionId) return

  openRightWorkspacePanel({
    id: 'editor',
    title: 'File',
    viewId: 'editor',
    sessionId,
  })
}

function worktreeFileCommands(query: string): Command[] {
  const text = query.trim()
  const files = worktreeState.worktreeFiles
  const worktreeRoot = activeSession()?.worktreeRoot?.trim()
  if (!activeSessionId() || !worktreeRoot || worktreeState.currentWorktree?.path !== worktreeRoot || files.length === 0) return []

  return files
    .map((path) => {
      const fileName = fileNameFromPath(path)
      const score = text
        ? Math.max(
            fuzzyMatch(text, fileName),
            fuzzyMatch(text, path),
          )
        : 1
      return { path, fileName, score }
    })
    .filter((item) => item.score > 0)
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score
      return left.path.localeCompare(right.path)
    })
    .slice(0, 50)
    .map((item) => ({
      id: `file:${item.path}`,
      label: item.fileName,
      description: item.path,
      category: 'File',
      source: 'file' as const,
      tags: item.path.split(/[\\/]+/).filter(Boolean),
      handler: () => {
        requestEditorWorktreeFileOpen(item.path)
        openActiveSessionFilePanel()
      },
    }))
}

const SYMBOL_FILE_EXTENSIONS = new Set([
  'ts',
  'tsx',
  'js',
  'jsx',
  'mjs',
  'cjs',
  'rs',
  'go',
  'py',
  'java',
  'kt',
  'kts',
  'cs',
  'cpp',
  'cc',
  'cxx',
  'c',
  'h',
  'hpp',
  'vue',
  'svelte',
])
const MAX_SYMBOL_INDEX_FILES = 500
const SYMBOL_INDEX_CONCURRENCY = 8

let symbolIndexKey: string | null = null
let symbolIndex: SymbolResult[] = []
let symbolIndexLoadKey: string | null = null
let symbolIndexLoadPromise: Promise<void> | null = null

function fileExtension(path: string): string {
  const name = fileNameFromPath(path)
  const dot = name.lastIndexOf('.')
  return dot >= 0 ? name.slice(dot + 1).toLowerCase() : ''
}

function shouldIndexSymbols(path: string): boolean {
  const normalized = path.replace(/\\/g, '/')
  if (/(^|\/)(node_modules|dist|target|build|\.git|coverage)\//.test(normalized)) return false
  return SYMBOL_FILE_EXTENSIONS.has(fileExtension(path))
}

function currentWorktreeSymbolKey(): string | null {
  const sessionId = activeSessionId()
  const worktreeRoot = activeSession()?.worktreeRoot?.trim()
  if (!sessionId || !worktreeRoot || worktreeState.currentWorktree?.path !== worktreeRoot) return null
  return `${sessionId}:${worktreeRoot}`
}

function firstMatch(line: string, pattern: RegExp, kind: string, nameGroup = 1): { kind: string; name: string; column: number } | null {
  const match = line.match(pattern)
  const name = match?.[nameGroup]
  if (!name) return null
  return { kind, name, column: Math.max(0, line.indexOf(name)) }
}

function symbolFromLine(line: string): { kind: string; name: string; column: number } | null {
  return (
    firstMatch(line, /^\s*(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][\w$]*)/, 'class') ??
    firstMatch(line, /^\s*(?:export\s+)?interface\s+([A-Za-z_$][\w$]*)/, 'interface') ??
    firstMatch(line, /^\s*(?:export\s+)?type\s+([A-Za-z_$][\w$]*)\s*=/, 'type') ??
    firstMatch(line, /^\s*(?:export\s+)?enum\s+([A-Za-z_$][\w$]*)/, 'enum') ??
    firstMatch(line, /^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/, 'function') ??
    firstMatch(line, /^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*=>/, 'function') ??
    firstMatch(line, /^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)/, 'function') ??
    firstMatch(line, /^\s*(?:pub\s+)?(?:struct|enum|trait)\s+([A-Za-z_]\w*)/, 'type') ??
    firstMatch(line, /^\s*func\s+(?:\([^)]+\)\s*)?([A-Za-z_]\w*)\s*\(/, 'function') ??
    firstMatch(line, /^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)\s*\(/, 'function') ??
    firstMatch(line, /^\s*class\s+([A-Za-z_]\w*)/, 'class') ??
    firstMatch(line, /^\s*(?:public|private|protected|internal|static|final|abstract|sealed|\s)*\s*(?:class|interface|enum|record)\s+([A-Za-z_]\w*)/, 'class') ??
    firstMatch(line, /^\s*(?:public|private|protected|internal|static|final|async|override|virtual|suspend|\s)+[\w<>\[\],.?]+\s+([A-Za-z_]\w*)\s*\([^;{}]*\)\s*(?:\{|=>|$)/, 'method')
  )
}

function extractSymbols(relativePath: string, content: string): SymbolResult[] {
  return content
    .split(/\r?\n/)
    .flatMap((line, index) => {
      const symbol = symbolFromLine(line)
      return symbol
        ? [{
            name: symbol.name,
            kind: symbol.kind,
            filePath: relativePath,
            line: index,
          }]
        : []
    })
}

async function mapWithLimit<T, R>(
  items: T[],
  limit: number,
  mapper: (item: T) => Promise<R>,
): Promise<R[]> {
  const results: R[] = []
  let nextIndex = 0
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (nextIndex < items.length) {
      const index = nextIndex++
      results[index] = await mapper(items[index])
    }
  })
  await Promise.all(workers)
  return results
}

export async function ensureWorktreeSymbolIndex(): Promise<void> {
  const key = currentWorktreeSymbolKey()
  const sessionId = activeSessionId()
  if (!key || !sessionId) {
    symbolIndexKey = null
    symbolIndex = []
    return
  }
  if (symbolIndexKey === key) return
  if (symbolIndexLoadKey === key && symbolIndexLoadPromise) return symbolIndexLoadPromise

  const files = worktreeState.worktreeFiles
    .filter(shouldIndexSymbols)
    .slice(0, MAX_SYMBOL_INDEX_FILES)

  symbolIndexLoadKey = key
  symbolIndexLoadPromise = mapWithLimit(files, SYMBOL_INDEX_CONCURRENCY, async (path) => {
    try {
      const document = await readSessionWorktreeFile(sessionId, path)
      return extractSymbols(document.relativePath, document.content)
    } catch {
      return []
    }
  }).then((results) => {
    if (currentWorktreeSymbolKey() !== key) return
    symbolIndexKey = key
    symbolIndex = results.flat()
  }).finally(() => {
    if (symbolIndexLoadKey === key) {
      symbolIndexLoadKey = null
      symbolIndexLoadPromise = null
    }
  })

  return symbolIndexLoadPromise
}

function worktreeSymbolCommands(query: string): Command[] {
  const key = currentWorktreeSymbolKey()
  const text = query.trim()
  if (!key || symbolIndexKey !== key) return []

  return symbolIndex
    .map((symbol) => {
      const location = `${symbol.filePath}:${symbol.line + 1}`
      const score = text
        ? Math.max(
            fuzzyMatch(text, symbol.name),
            fuzzyMatch(text, symbol.kind),
            fuzzyMatch(text, location),
          )
        : 1
      return { symbol, location, score }
    })
    .filter((item) => item.score > 0)
    .sort((left, right) => {
      if (right.score !== left.score) return right.score - left.score
      return left.location.localeCompare(right.location)
    })
    .slice(0, 50)
    .map(({ symbol, location }) => ({
      id: `symbol:${symbol.filePath}:${symbol.line}:${symbol.name}`,
      label: symbol.name,
      description: `${symbol.kind} · ${location}`,
      category: 'Symbol',
      source: 'symbol' as const,
      tags: [symbol.kind, symbol.filePath, location],
      handler: () => {
        requestEditorWorktreeFileOpen(symbol.filePath, { line: symbol.line })
        openActiveSessionFilePanel()
      },
    }))
}

/**
 * 检测查询中的前缀并提取实际搜索内容
 *
 * 对应 design/23-command-palette.md 第四章"交互设计"：
 * - ">" → 仅搜索命令
 * - "@" → 搜索文件
 * - "/" → 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
 * - "#" → 搜索符号
 *
 * @param query 用户输入
 * @returns 解析后的范围和实际查询文本
 */
export function parseQueryPrefix(query: string): {
  scope: CommandScope | null
  text: string
} {
  const trimmed = query.trimStart()

  if (trimmed.startsWith('>')) {
    return { scope: 'commands', text: trimmed.slice(1).trimStart() }
  }
  if (trimmed.startsWith('@')) {
    return { scope: 'files', text: trimmed.slice(1).trimStart() }
  }
  if (trimmed.startsWith('/')) {
    return { scope: 'slash', text: trimmed.slice(1).trimStart() }
  }
  if (trimmed.startsWith('#')) {
    return { scope: 'symbols', text: trimmed.slice(1).trimStart() }
  }

  // 无前缀 → 搜索所有命令
  return { scope: null, text: trimmed }
}

// ============================================================
// 全局命令目录 Store
// ============================================================

/**
 * 命令目录的响应式状态
 *
 * 使用 Solid.js 的 createStore 实现细粒度响应式更新。
 * 命令注册/注销操作会直接修改此 store。
 */
const [commandPaletteState, setCommandPaletteState] = createStore<CommandPaletteState>({
  isOpen: false,
  query: '',
  selectedIndex: 0,
  commands: [],
  filteredCommands: [],
  recentCommands: loadRecentCommands(),
  scope: null,
})

// ============================================================
// 公开 API
// ============================================================

/**
 * 命令面板 API 对象
 *
 * 对应 design/23-command-palette.md 第六章"接口定义"中的命令面板接口。
 * 所有模块通过此 API 与命令面板交互。
 */
export const commandPaletteAPI = {
  // ---- 状态访问器 ----

  /** 获取命令面板的响应式状态（只读） */
  getState: () => commandPaletteState,

  // ---- 命令注册 ----

  /**
   * 注册单个命令
   *
   * @param command 命令定义
   * @throws 如果命令 ID 已存在则打印警告（不覆盖，避免冲突）
   */
  register(command: Command): void {
    const existing = commandPaletteState.commands.find((c) => c.id === command.id)
    if (existing) {
      console.warn(
        `[CommandPalette] 命令 "${command.id}" 已存在，跳过注册。` +
          `如需更新命令，请先调用 unregister("${command.id}")。`,
      )
      return
    }
    setCommandPaletteState('commands', (prev) => [...prev, command])
  },

  /**
   * 注销命令
   *
   * @param id 要注销的命令 ID
   */
  unregister(id: string): void {
    setCommandPaletteState('commands', (prev) => prev.filter((c) => c.id !== id))
  },

  /**
   * 批量注册命令
   *
   * 对应 design/23-command-palette.md 中 extension.json commands 注册链路：
   * Extension 加载 → 解析 extension.json 中的 commands 定义 → commandPalette.registerBatch()
   *
   * @param commands 命令定义数组
   */
  registerBatch(commands: Command[]): void {
    for (const cmd of commands) {
      commandPaletteAPI.register(cmd)
    }
  },

  // ---- 面板控制 ----

  /**
   * 打开命令面板
   *
   * @param scope 可选的搜索范围，决定初始前缀
   *   - 'commands' → 预填 ">" 前缀
   *   - 'files'    → 预填 "@" 前缀
   *   - 'slash'    → 预填 "/" 前缀
   *   - 'symbols'  → 预填 "#" 前缀
   */
  open(scope?: CommandScope): void {
    // 根据 scope 设置初始查询前缀
    const prefixMap: Record<CommandScope, string> = {
      commands: '>',
      files: '@',
      slash: '/',
      symbols: '#',
    }
    const initialQuery = scope ? prefixMap[scope] : ''

    setCommandPaletteState({
      isOpen: true,
      query: initialQuery,
      selectedIndex: 0,
      scope: scope ?? null,
    })

    // 立即执行一次搜索，确保 filteredCommands 就绪
    commandPaletteAPI.search(initialQuery)
  },

  /**
   * 关闭命令面板
   */
  close(): void {
    setCommandPaletteState({
      isOpen: false,
      query: '',
      selectedIndex: 0,
      scope: null,
    })
  },

  // ---- 搜索 ----

  /**
   * 执行搜索并更新 filteredCommands
   *
   * 搜索策略（优先级从高到低）：
   * 1. 前缀过滤：根据 > @ / # 限定搜索范围
   * 2. 模糊匹配：对命令名、描述、分类、快捷键进行模糊搜索
   * 3. 最近使用加权：最近使用的命令排序靠前
   * 4. 启用状态过滤：禁用的命令不出现在结果中
   *
   * @param query 搜索查询文本（可选，默认使用当前状态中的 query）
   */
  search(query?: string): void {
    const q = query ?? commandPaletteState.query
    const { scope, text } = parseQueryPrefix(q)

    // 更新 scope 状态
    setCommandPaletteState('scope', scope)

    // 获取所有启用的命令
    const enabledCommands = commandPaletteState.commands.filter(
      (cmd) => !cmd.isEnabled || cmd.isEnabled(),
    )

    let results: Command[]

    if (scope === null) {
      // 无前缀 → 搜索所有命令，同时展示最近使用的命令在前面
      results = enabledCommands
        .map((cmd) => ({
          cmd,
          score: text
            ? Math.max(
                fuzzyMatch(text, cmd.label),
                fuzzyMatch(text, cmd.description ?? ''),
                fuzzyMatch(text, cmd.category),
                fuzzyMatch(text, cmd.keybinding ?? ''),
                // 搜索标签也参与匹配（AI 推荐用）
                ...(cmd.tags ?? []).map((tag) => fuzzyMatch(text, tag)),
              )
            : 1, // 空查询时所有命令都显示
        }))
        .filter((item) => item.score > 0)
        .sort((a, b) => {
          // 最近使用的命令优先
          const aRecent = commandPaletteState.recentCommands.indexOf(a.cmd.id)
          const bRecent = commandPaletteState.recentCommands.indexOf(b.cmd.id)
          const aRecentScore = aRecent >= 0 ? MAX_RECENT_COMMANDS - aRecent : 0
          const bRecentScore = bRecent >= 0 ? MAX_RECENT_COMMANDS - bRecent : 0

          return b.score + bRecentScore * 2 - (a.score + aRecentScore * 2)
        })
        .map((item) => item.cmd)
    } else if (scope === 'commands') {
      // ">" 前缀 → 仅搜索命令
      results = enabledCommands
        .map((cmd) => ({
          cmd,
          score: text
            ? Math.max(
                fuzzyMatch(text, cmd.label),
                fuzzyMatch(text, cmd.description ?? ''),
                fuzzyMatch(text, cmd.category),
                fuzzyMatch(text, cmd.keybinding ?? ''),
                ...(cmd.tags ?? []).map((tag) => fuzzyMatch(text, tag)),
              )
            : 1,
        }))
        .filter((item) => item.score > 0)
        .sort((a, b) => b.score - a.score)
        .map((item) => item.cmd)
    } else if (scope === 'slash') {
      // "/" 前缀 → 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
      results = enabledCommands
        .filter((cmd) => cmd.source === 'skill' || cmd.source === 'command' || cmd.source === 'extension')
        .map((cmd) => ({
          cmd,
          score: text
            ? Math.max(
                fuzzyMatch(text, cmd.label),
                fuzzyMatch(text, cmd.description ?? ''),
                ...(cmd.tags ?? []).map((tag) => fuzzyMatch(text, tag)),
              )
            : 1,
        }))
        .filter((item) => item.score > 0)
        .sort((a, b) => b.score - a.score)
        .map((item) => item.cmd)
    } else if (scope === 'files') {
      results = worktreeFileCommands(text)
    } else {
      results = worktreeSymbolCommands(text)
    }

    setCommandPaletteState('filteredCommands', results)

    // 重置选中索引（搜索变更时从第一条开始）
    setCommandPaletteState('selectedIndex', 0)
  },

  /**
   * 搜索命令（仅返回匹配结果，不修改状态）
   *
   * 对应 design/23-command-palette.md 接口：searchCommands(query)
   *
   * @param query 搜索查询
   * @returns 匹配的命令列表
   */
  searchCommands(query: string): Command[] {
    const { text } = parseQueryPrefix(query)
    if (!text) return commandPaletteState.commands

    return commandPaletteState.commands
      .map((cmd) => ({
        cmd,
        score: Math.max(
          fuzzyMatch(text, cmd.label),
          fuzzyMatch(text, cmd.description ?? ''),
          fuzzyMatch(text, cmd.category),
        ),
      }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((item) => item.cmd)
  },

  searchSymbols(query: string): SymbolResult[] {
    const { text } = parseQueryPrefix(query)
    const key = currentWorktreeSymbolKey()
    if (!key || symbolIndexKey !== key) return []

    return symbolIndex
      .map((symbol) => ({
        symbol,
        score: text
          ? Math.max(
              fuzzyMatch(text, symbol.name),
              fuzzyMatch(text, symbol.kind),
              fuzzyMatch(text, `${symbol.filePath}:${symbol.line + 1}`),
            )
          : 1,
      }))
      .filter((item) => item.score > 0)
      .sort((left, right) => right.score - left.score)
      .map((item) => item.symbol)
  },

  /**
   * 搜索 Slash commands
   *
   * 搜索来源为 'skill'、'command' 或 'extension' 的命令，
   * 与 "/" 前缀触发的搜索逻辑一致，但不修改状态。
   *
   * @param query 搜索查询文本
   * @returns 匹配的 Slash command 列表
   */
  searchSlashCommands(query: string): Command[] {
    const slashCommands = commandPaletteState.commands.filter(
      (cmd) => cmd.source === 'skill' || cmd.source === 'command' || cmd.source === 'extension',
    )

    if (!query.trim()) return slashCommands

    return slashCommands
      .map((cmd) => ({
        cmd,
        score: Math.max(
          fuzzyMatch(query, cmd.label),
          fuzzyMatch(query, cmd.description ?? ''),
          ...(cmd.tags ?? []).map((tag) => fuzzyMatch(query, tag)),
        ),
      }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((item) => item.cmd)
  },

  // ---- 最近使用记录 ----

  /**
   * 记录命令使用（执行后调用）
   *
   * 将命令 ID 移到最近使用列表的最前面，并持久化。
   *
   * @param commandId 被执行的命令 ID
   */
  recordUsage(commandId: string): void {
    const recent = commandPaletteState.recentCommands.filter((id) => id !== commandId)
    const updated = [commandId, ...recent].slice(0, MAX_RECENT_COMMANDS)

    setCommandPaletteState('recentCommands', updated)
    saveRecentCommands(updated)
  },

  /**
   * 清除最近使用记录
   */
  clearRecentCommands(): void {
    setCommandPaletteState('recentCommands', [])
    saveRecentCommands([])
  },
}

// 导出 store 供 Hook 和组件使用
export { commandPaletteState, setCommandPaletteState }
