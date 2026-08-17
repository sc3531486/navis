/**
 * CommandPalette 模块入口
 *
 * 统一导出所有公开的组件、Hook、类型和 API。
 *
 * 使用方式：
 * ```tsx
 * // 导入主组件（在 App 中全局挂载）
 * import { CommandPalette } from '@/components/CommandPalette'
 *
 * // 导入命令注册 API（各模块注册命令时使用）
 * import { commandPaletteAPI } from '@/components/CommandPalette'
 *
 * // 导入类型（TypeScript 类型检查）
 * import type { Command, CommandSource, CommandScope } from '@/components/CommandPalette'
 *
 * // 导入 Hook（自定义命令面板 UI 时使用）
 * import { useCommandPalette } from '@/components/CommandPalette'
 * ```
 */

// 主组件
export { CommandPalette } from './CommandPalette'
export type { CommandPaletteProps } from './CommandPalette'

// 子组件（高级场景下可单独使用）
export { SearchInput } from './SearchInput'
export type { SearchInputProps } from './SearchInput'
export { CommandItem } from './CommandItem'
export type { CommandItemProps } from './CommandItem'
export { CommandList } from './CommandList'
export type { CommandListProps } from './CommandList'

// Hook
export { useCommandPalette, getAIRecommendations } from './useCommandPalette'
export type { UseCommandPaletteReturn } from './useCommandPalette'

// Store API（命令注册/注销/搜索）
export {
  commandPaletteAPI,
  commandPaletteState,
  setCommandPaletteState,
  fuzzyMatch,
  parseQueryPrefix,
} from './store'

// 类型
export type {
  Command,
  CommandSource,
  CommandScope,
  CommandPaletteState,
  FileResult,
  SymbolResult,
} from './store'
