/**
 * CommandItem 单个命令项组件
 *
 * 职责：
 * 1. 渲染单个命令的图标、名称、描述、快捷键
 * 2. 高亮当前选中项（键盘/鼠标导航）
 * 3. 鼠标悬停时通知父组件更新选中索引
 * 4. 点击时执行命令
 *
 * 设计依据：design/23-command-palette.md 第四章"交互设计" 中的列表项布局
 */

import { type JSX, Show } from 'solid-js'
import type { Command, CommandSource } from './store'

/**
 * CommandItem 组件属性
 */
export interface CommandItemProps {
  /** 命令数据 */
  command: Command
  /** 是否为当前选中项 */
  isSelected: boolean
  /** 索引（用于鼠标悬停时通知父组件） */
  index: number
  /** 鼠标悬停回调 */
  onMouseEnter: (index: number) => void
  /** 点击回调 */
  onClick: (command: Command) => void
}

/**
 * 命令来源对应的图标和颜色映射
 *
 * 对应 design/23-command-palette.md 第四章的 UI 设计：
 * - Builtin host commands
 * - Skill commands
 * - Extension commands
 * - Lightweight slash commands
 */
const SOURCE_CONFIG: Record<CommandSource, { icon: string; label: string }> = {
  builtin: { icon: '⌘', label: 'Host' },
  extension: { icon: '+', label: 'Extension' },
  skill: { icon: '/', label: 'Skill' },
  command: { icon: '>', label: 'Command' },
  file: { icon: '@', label: 'File' },
  symbol: { icon: '#', label: 'Symbol' },
}

/**
 * CommandItem 组件
 *
 * 渲染单个命令列表项，包含：
 * - 来源图标（左侧）
 * - 命令名称和描述（中间）
 * - 快捷键（右侧，如有）
 *
 * 选中态通过 background 高亮 + border-left 指示器实现。
 */
export function CommandItem(props: CommandItemProps): JSX.Element {
  /** 获取来源配置 */
  const sourceConfig = () => SOURCE_CONFIG[props.command.source]

  /** 判断命令是否禁用 */
  const isDisabled = (): boolean => {
    return props.command.isEnabled ? !props.command.isEnabled() : false
  }

  return (
    <div
      role="option"
      aria-selected={props.isSelected}
      aria-disabled={isDisabled()}
      classList={{
        'navis-command-palette-item flex items-center gap-2 px-3 py-1 cursor-pointer transition-colors duration-100': true,
        'is-selected': props.isSelected,
        // 禁用态
        'opacity-50 cursor-not-allowed': isDisabled(),
      }}
      onMouseEnter={() => props.onMouseEnter(props.index)}
      onClick={() => {
        if (!isDisabled()) {
          props.onClick(props.command)
        }
      }}
    >
      {/* 来源图标 */}
      <span class="navis-command-source-mark shrink-0 select-none" aria-hidden="true">
        {sourceConfig().icon}
      </span>

      {/* 命令信息（名称 + 描述） */}
      <div class="flex-1 min-w-0">
        {/* 命令名称 */}
        <div class="flex items-center gap-2">
          <span class="navis-command-label text-[var(--color-text-primary)] truncate">
            {props.command.label}
          </span>

          {/* 来源标签（非 builtin 时显示） */}
          <Show when={props.command.source !== 'builtin'}>
            <span class="navis-command-source-label px-1.5 py-0.5 rounded bg-[var(--color-bg-secondary)] text-[var(--color-text-secondary)] shrink-0">
              {sourceConfig().label}
            </span>
          </Show>
        </div>

        {/* 命令描述 */}
        <Show when={props.command.description}>
          <div class="navis-command-description text-[var(--color-text-secondary)] truncate">
            {props.command.description}
          </div>
        </Show>
      </div>

      {/* 快捷键显示 */}
      <Show when={props.command.keybinding}>
        <kbd class="navis-command-keybinding inline-flex items-center px-1.5 py-0.5 font-mono rounded border border-[var(--color-border)] bg-[var(--color-bg-secondary)] text-[var(--color-text-secondary)] shrink-0 select-none">
          {props.command.keybinding}
        </kbd>
      </Show>
    </div>
  )
}
