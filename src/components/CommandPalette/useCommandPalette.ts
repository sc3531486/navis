/**
 * useCommandPalette Hook
 *
 * 封装命令面板的全部交互逻辑，供 CommandPalette 组件使用。
 *
 * 职责：
 * 1. 查询文本管理（输入、清空）
 * 2. 键盘导航（上/下箭头、Enter 确认、Escape 关闭）
 * 3. 命令执行（调用 handler、记录使用历史）
 * 4. AI 推荐引擎（本地关键词匹配 + 排序加权）
 * 5. 前缀触发检测（> @ / #）
 *
 * 设计依据：design/23-command-palette.md 第四章"交互设计" & 第五章"AI 推荐"
 */

import { createSignal, createEffect, on } from 'solid-js'
import {
  commandPaletteAPI,
  commandPaletteState,
  ensureWorktreeSymbolIndex,
  setCommandPaletteState,
  parseQueryPrefix,
  type Command,
  type CommandScope,
} from './store'
import { activeSession, activeSessionId } from '../../stores/session-tree'
import { loadSessionWorktree, worktreeState } from '../../stores/worktree'

/**
 * AI 推荐引擎（本地策略层）
 *
 * 对应 design/23-command-palette.md 5.2 "AI 推荐实现说明"：
 * "前端本地关键词匹配（默认，不消耗 API 额度）"
 *
 * 策略：
 * 1. 对命令名、描述、标签进行模糊匹配
 * 2. 结合最近使用频率加权排序
 * 3. 覆盖 80% 的常见推荐场景
 *
 * @param query 用户输入文本
 * @param commands 所有可用命令
 * @param recentIds 最近使用的命令 ID 列表
 * @returns 排序后的推荐命令列表
 */
export function getAIRecommendations(
  query: string,
  commands: Command[],
  recentIds: string[],
): Command[] {
  // 空查询时，返回最近使用的命令 + 高优先级命令
  if (!query.trim()) {
    const recentCommands = recentIds
      .map((id) => commands.find((c) => c.id === id))
      .filter(Boolean) as Command[]

    // 补充未在最近使用中的命令（保持原始顺序）
    const rest = commands.filter((c) => !recentIds.includes(c.id))
    return [...recentCommands, ...rest]
  }

  /**
   * 计算单个命令的推荐分数
   *
   * 分数组成：
   * - 关键词匹配分（最高 100）
   * - 最近使用加分（最高 40）
   * - 来源权重（内置 > 扩展 > 技能）
   */
  const scored = commands.map((cmd) => {
    let score = 0

    // 1. 关键词匹配（label 权重最高）
    const labelLower = cmd.label.toLowerCase()
    const queryLower = query.toLowerCase()

    // label 前缀匹配 → 最高分
    if (labelLower.startsWith(queryLower)) {
      score += 100
    }
    // label 包含匹配
    else if (labelLower.includes(queryLower)) {
      score += 70
    }
    // 描述包含匹配
    else if (cmd.description?.toLowerCase().includes(queryLower)) {
      score += 50
    }
    // 分类包含匹配
    else if (cmd.category.toLowerCase().includes(queryLower)) {
      score += 30
    }
    // 标签匹配（AI 推荐专用）
    else if (cmd.tags?.some((tag) => tag.toLowerCase().includes(queryLower))) {
      score += 40
    }

    // 2. 最近使用加分（越近越高，第一条 +40，逐条递减 5）
    const recentIndex = recentIds.indexOf(cmd.id)
    if (recentIndex >= 0) {
      score += Math.max(0, 40 - recentIndex * 5)
    }

    // 3. 来源权重（内置命令略优先，因为更常用）
    if (cmd.source === 'builtin') score += 10
    if (cmd.source === 'extension') score += 5

    return { cmd, score }
  })

  return scored
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
    .map((item) => item.cmd)
}

/**
 * useCommandPalette Hook 返回值类型
 */
export interface UseCommandPaletteReturn {
  /** 当前查询文本（响应式信号） */
  query: () => string
  /** 设置查询文本 */
  setQuery: (q: string) => void
  /** 当前选中索引（响应式信号） */
  selectedIndex: () => number
  /** 当前搜索范围（由前缀决定） */
  scope: () => CommandScope | null
  /** 过滤后的命令列表 */
  filteredCommands: () => Command[]
  /** AI 推荐的命令列表（基于查询实时计算） */
  recommendedCommands: () => Command[]
  /** 面板是否打开 */
  isOpen: () => boolean

  /** 处理键盘事件 */
  handleKeyDown: (e: KeyboardEvent) => void
  /** 执行命令 */
  executeCommand: (command: Command) => void
  /** 选中某个命令（鼠标悬停） */
  selectCommand: (index: number) => void
  /** 打开面板 */
  open: (scope?: CommandScope) => void
  /** 关闭面板 */
  close: () => void
}

/**
 * 命令面板核心 Hook
 *
 * 用法：
 * ```tsx
 * const palette = useCommandPalette()
 * // palette.open()          → 打开面板
 * // palette.setQuery('...') → 更新搜索
 * // palette.handleKeyDown() → 处理键盘
 * ```
 */
export function useCommandPalette(): UseCommandPaletteReturn {
  // ---- 本地信号 ----

  /** AI 推荐结果（独立于 filteredCommands，用于默认列表展示） */
  const [recommended, setRecommended] = createSignal<Command[]>([])

  // ---- 响应式派生 ----

  /** 从全局 store 读取响应式状态（getter 函数保证细粒度响应） */
  const query = () => commandPaletteState.query
  const selectedIndex = () => commandPaletteState.selectedIndex
  const scope = () => commandPaletteState.scope
  const filteredCommands = () => commandPaletteState.filteredCommands
  const isOpen = () => commandPaletteState.isOpen

  /**
   * 查询文本变更时，自动执行搜索并更新 AI 推荐
   *
   * 使用 createEffect 监听 query 变化，实现输入即搜索的即时反馈。
   */
  createEffect(
    on(
      () => commandPaletteState.query,
      (q) => {
        // 执行搜索（更新 store 中的 filteredCommands）
        commandPaletteAPI.search(q)

        // 计算 AI 推荐（仅在无前缀时显示推荐列表）
        const { scope: parsedScope } = parseQueryPrefix(q)
        if (parsedScope === 'files' || parsedScope === 'symbols') {
          const sessionId = activeSessionId()
          const worktreeRoot = activeSession()?.worktreeRoot?.trim()
          const needsWorktree =
            Boolean(worktreeRoot) && worktreeState.currentWorktree?.path !== worktreeRoot
          if (sessionId && needsWorktree && !worktreeState.isLoading) {
            void loadSessionWorktree(sessionId).then(() => {
              if (commandPaletteState.query === q) {
                commandPaletteAPI.search(q)
                if (parsedScope === 'symbols') {
                  void ensureWorktreeSymbolIndex().then(() => {
                    if (commandPaletteState.query === q) commandPaletteAPI.search(q)
                  })
                }
              }
            })
          } else if (parsedScope === 'symbols') {
            void ensureWorktreeSymbolIndex().then(() => {
              if (commandPaletteState.query === q) commandPaletteAPI.search(q)
            })
          }
        }

        if (parsedScope === null) {
          const allCommands = commandPaletteState.commands.filter(
            (cmd) => !cmd.isEnabled || cmd.isEnabled(),
          )
          setRecommended(getAIRecommendations(q, allCommands, commandPaletteState.recentCommands))
        } else {
          // 有前缀时清空推荐（使用 filteredCommands 即可）
          setRecommended([])
        }
      },
    ),
  )

  // ---- 操作方法 ----

  /**
   * 更新查询文本
   * 同时重置选中索引为 0（避免索引越界）
   */
  const setQuery = (q: string): void => {
    setCommandPaletteState('query', q)
    setCommandPaletteState('selectedIndex', 0)
  }

  /**
   * 处理键盘事件
   *
   * 键盘映射：
   * - ArrowDown / Ctrl+N  → 选中下一条
   * - ArrowUp   / Ctrl+P  → 选中上一条
   * - Enter               → 执行选中的命令
   * - Escape              → 关闭面板
   */
  const handleKeyDown = (e: KeyboardEvent): void => {
    const commands = getDisplayCommands()
    const currentIdx = selectedIndex()

    switch (e.key) {
      case 'ArrowDown':
      case 'n': {
        // Ctrl+N 等效于 ArrowDown（VSCode 风格）
        if (e.key === 'n' && !e.ctrlKey) break
        e.preventDefault()

        const nextIdx = commands.length > 0 ? (currentIdx + 1) % commands.length : 0
        setCommandPaletteState('selectedIndex', nextIdx)
        break
      }

      case 'ArrowUp':
      case 'p': {
        // Ctrl+P 等效于 ArrowUp（VSCode 风格）
        if (e.key === 'p' && !e.ctrlKey) break
        e.preventDefault()

        const prevIdx =
          commands.length > 0 ? (currentIdx - 1 + commands.length) % commands.length : 0
        setCommandPaletteState('selectedIndex', prevIdx)
        break
      }

      case 'Enter': {
        e.preventDefault()
        if (commands.length > 0 && currentIdx >= 0 && currentIdx < commands.length) {
          executeCommand(commands[currentIdx])
        }
        break
      }

      case 'Escape': {
        e.preventDefault()
        close()
        break
      }
    }
  }

  /**
   * 执行命令
   *
   * 流程：
   * 1. 检查命令是否启用
   * 2. 关闭面板
   * 3. 调用命令 handler（支持同步/异步）
   * 4. 记录使用历史
   */
  const executeCommand = (command: Command): void => {
    // 检查命令是否启用
    if (command.isEnabled && !command.isEnabled()) {
      return
    }

    // 关闭面板（先关闭再执行，避免执行过程中的 UI 残留）
    close()

    // 异步执行命令（不阻塞 UI 渲染）
    try {
      const result = command.handler()
      // 如果返回 Promise，捕获异步错误
      if (result instanceof Promise) {
        result.catch((err) => {
          console.error(`[CommandPalette] 命令 "${command.id}" 执行失败:`, err)
        })
      }
    } catch (err) {
      console.error(`[CommandPalette] 命令 "${command.id}" 执行出错:`, err)
    }

    // 记录使用历史（持久化到 localStorage）
    commandPaletteAPI.recordUsage(command.id)
  }

  /** 选中某个命令（鼠标悬停时调用） */
  const selectCommand = (index: number): void => {
    setCommandPaletteState('selectedIndex', index)
  }

  /** 打开命令面板 */
  const open = (scope?: CommandScope): void => {
    commandPaletteAPI.open(scope)
    // 打开时立即计算 AI 推荐（空查询状态）
    const allCommands = commandPaletteState.commands.filter(
      (cmd) => !cmd.isEnabled || cmd.isEnabled(),
    )
    setRecommended(getAIRecommendations('', allCommands, commandPaletteState.recentCommands))
  }

  /** 关闭命令面板 */
  const close = (): void => {
    commandPaletteAPI.close()
    setRecommended([])
  }

  /**
   * 获取当前应该展示的命令列表
   *
   * 优先级：
   * 1. 有前缀且有搜索文本 → 使用 filteredCommands（精确过滤结果）
   * 2. 无前缀且无搜索文本 → 使用 recommendedCommands（AI 推荐 + 最近使用）
   * 3. 无前缀有搜索文本   → 使用 filteredCommands（模糊搜索结果）
   */
  const getDisplayCommands = (): Command[] => {
    const currentScope = scope()
    const currentQuery = query()

    // 有前缀时，使用 filteredCommands
    if (currentScope !== null) {
      return filteredCommands()
    }

    // 无前缀 + 空查询 → 显示推荐列表
    if (!currentQuery.trim()) {
      return recommended()
    }

    // 无前缀 + 有查询 → 使用 filteredCommands
    return filteredCommands()
  }

  return {
    query,
    setQuery,
    selectedIndex,
    scope,
    filteredCommands: getDisplayCommands,
    recommendedCommands: recommended,
    isOpen,
    handleKeyDown,
    executeCommand,
    selectCommand,
    open,
    close,
  }
}
