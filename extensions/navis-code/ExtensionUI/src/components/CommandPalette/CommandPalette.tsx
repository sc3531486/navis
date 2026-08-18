/**
 * CommandPalette 主组件
 *
 * 命令面板是快速访问入口，支持命令搜索、文件搜索、Slash commands、AI 推荐。
 *
 * 架构：
 * - 基于 Kobalte Dialog 实现可访问的模态弹窗
 * - 通过 useCommandPalette Hook 管理所有交互逻辑
 * - SearchInput 处理搜索输入
 * - CommandList 渲染命令列表
 *
 * 触发方式：
 * - Ctrl+Shift+P → 打开命令面板（默认所有命令）
 * - programmatic: commandPaletteAPI.open(scope) → 按指定范围打开
 *
 * 前缀触发器：
 * - ">" → 仅搜索命令
 * - "@" → 搜索文件
 * - "/" → 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
 * - "#" → 搜索符号
 *
 * 设计依据：design/23-command-palette.md 全文
 */

import { type JSX, createEffect } from 'solid-js'
import { useCommandPalette } from './useCommandPalette'
import { CommandList } from './CommandList'
import { SearchSurface } from '@agent-core/components/SearchSurface'

/**
 * CommandPalette 组件属性
 */
export interface CommandPaletteProps {
  /**
   * 外部控制面板打开状态（可选）
   * 如果提供，则使用受控模式；否则使用 Hook 内部状态
   */
  open?: boolean
  /**
   * 面板打开状态变更回调（受控模式下使用）
   */
  onOpenChange?: (open: boolean) => void
}

const SCOPE_PLACEHOLDERS = {
  commands: 'Search commands',
  files: 'Search files',
  slash: 'Search slash commands',
  symbols: 'Search symbols',
} as const

const SCOPE_INDICATORS = {
  commands: '> Commands',
  files: '@ Files',
  slash: '/ Slash',
  symbols: '# Symbols',
} as const

/**
 * CommandPalette 命令面板主组件
 *
 * 使用 Kobalte Dialog 实现：
 * - 模态弹窗（focus trap + scroll lock）
 * - ESC 键关闭
 * - 点击遮罩层关闭
 * - 可访问性（ARIA 属性自动管理）
 *
 * 用法：
 * ```tsx
 * // 在 App 中挂载（全局单例）
 * <CommandPalette />
 *
 * // 或使用受控模式
 * <CommandPalette open={isOpen()} onOpenChange={setIsOpen} />
 * ```
 */
export function CommandPalette(props: CommandPaletteProps): JSX.Element {
  /** 使用命令面板 Hook 获取所有状态和方法 */
  const palette = useCommandPalette()

  /**
   * 受控模式：同步外部 open 状态到内部 store
   * 当外部通过 props.open 控制时，同步到 Hook 内部状态
   */
  createEffect(() => {
    if (props.open !== undefined) {
      if (props.open && !palette.isOpen()) {
        palette.open()
      } else if (!props.open && palette.isOpen()) {
        palette.close()
      }
    }
  })

  /**
   * 处理 Dialog 的 onOpenChange 回调
   * 当 Dialog 自身触发关闭（ESC、遮罩点击）时同步状态
   */
  const handleOpenChange = (isOpen: boolean): void => {
    if (isOpen) {
      palette.open()
    } else {
      palette.close()
    }
    // 通知外部（受控模式）
    props.onOpenChange?.(isOpen)
  }

  /**
   * 获取当前应该展示的命令列表
   *
   * 优先级：
   * 1. 有 scope（前缀触发）→ 使用 filteredCommands
   * 2. 无 scope + 空查询 → 使用 recommendedCommands（AI 推荐 + 最近使用）
   * 3. 无 scope + 有查询 → 使用 filteredCommands（模糊搜索结果）
   */
  const displayCommands = () => {
    return palette.filteredCommands()
  }

  const placeholder = () => {
    const scope = palette.scope()
    return scope ? SCOPE_PLACEHOLDERS[scope] : 'Search commands and files'
  }

  const scopeAccessory = () => {
    const scope = palette.scope()
    if (!scope) return undefined
    return <span class="navis-search-surface-scope">{SCOPE_INDICATORS[scope]}</span>
  }

  return (
    <SearchSurface
      open={palette.isOpen()}
      title="Command Palette"
      description="Search commands, files, slash commands, or symbols. Use arrow keys to navigate and Enter to select."
      placeholder={placeholder()}
      query={palette.query()}
      onOpenChange={handleOpenChange}
      onQueryChange={palette.setQuery}
      onKeyDown={palette.handleKeyDown}
      leadingAccessory={scopeAccessory()}
    >
      <CommandList
        commands={displayCommands()}
        selectedIndex={palette.selectedIndex()}
        scope={palette.scope()}
        onMouseEnter={palette.selectCommand}
        onClick={palette.executeCommand}
      />
    </SearchSurface>
  )
}

/**
 * 重新导出所有公开 API，供外部使用
 *
 * 使用方式：
 * ```tsx
 * // 在应用初始化时注册命令
 * import { commandPaletteAPI } from '@navis-code/components/CommandPalette'
 * commandPaletteAPI.register({ id: '...', label: '...', ... })
 *
 * // 打开命令面板
 * commandPaletteAPI.open()
 * commandPaletteAPI.open('slash')  // 直接进入 Slash commands 搜索
 * ```
 */
export { commandPaletteAPI, type Command, type CommandSource, type CommandScope } from './store'
export { useCommandPalette, getAIRecommendations } from './useCommandPalette'
export { SearchInput } from './SearchInput'
export { CommandItem } from './CommandItem'
export { CommandList } from './CommandList'


