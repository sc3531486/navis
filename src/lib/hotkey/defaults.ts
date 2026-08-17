/**
 * Hotkey 全局快捷键模块 - 默认快捷键配置
 *
 * 严格对应设计文档 design/27-hotkey.md §4 默认快捷键。
 * 包含 7 条系统内置快捷键，均为 App 作用域。
 * 这些默认配置不可被 unregister，只能通过 update 修改按键组合。
 */

import { HotkeyScope } from './types';
import type { HotkeyBinding } from './types';

/**
 * 系统默认快捷键绑定列表
 *
 * 对应设计文档中定义的 7 条默认快捷键：
 * - Ctrl+Shift+P: 打开命令面板
 * - Ctrl+Shift+N: 新建会话
 * - Ctrl+Shift+A: 终止当前任务
 * - Ctrl+`: 切换终端
 * - Ctrl+B: 切换侧边栏
 * - Ctrl+Shift+Enter: 发送消息并执行
 * - Ctrl+O: 打开文件
 *
 * 所有默认快捷键的作用域均为 App（仅应用内生效），
 * id 格式为 "scope:command" 以确保全局唯一。
 */
export const DEFAULT_HOTKEYS: HotkeyBinding[] = [
  {
    id: 'app:commandPalette.open',
    keybinding: 'Ctrl+Shift+P',
    scope: HotkeyScope.App,
    command: 'commandPalette.open',
    description: '打开命令面板',
    category: 'General',
    is_custom: false,
  },
  {
    id: 'app:session.create',
    keybinding: 'Ctrl+Shift+N',
    scope: HotkeyScope.App,
    command: 'session.create',
    description: '新建会话',
    category: 'Session',
    is_custom: false,
  },
  {
    id: 'app:agent.abort',
    keybinding: 'Ctrl+Shift+A',
    scope: HotkeyScope.App,
    command: 'agent.abort',
    description: '终止当前任务',
    category: 'Agent',
    is_custom: false,
  },
  {
    id: 'app:terminal.toggle',
    keybinding: 'Ctrl+`',
    scope: HotkeyScope.App,
    command: 'terminal.toggle',
    description: '切换终端',
    category: 'Terminal',
    is_custom: false,
  },
  {
    id: 'app:sidebar.toggle',
    keybinding: 'Ctrl+B',
    scope: HotkeyScope.App,
    command: 'sidebar.toggle',
    description: '切换侧边栏',
    category: 'General',
    is_custom: false,
  },
  {
    id: 'app:agent.sendAndExecute',
    keybinding: 'Ctrl+Shift+Enter',
    scope: HotkeyScope.App,
    command: 'agent.sendAndExecute',
    description: '发送消息并执行',
    category: 'Agent',
    is_custom: false,
  },
  {
    id: 'app:file.open',
    keybinding: 'Ctrl+O',
    scope: HotkeyScope.App,
    command: 'file.open',
    description: '打开文件',
    category: 'General',
    is_custom: false,
  },
  // 【变更 #4/后端 #9】添加面板切换默认快捷键
  {
    id: 'app:panel.toggle',
    keybinding: 'Ctrl+J',
    scope: HotkeyScope.App,
    command: 'panel.toggle',
    description: '切换面板显示/隐藏',
    category: 'General',
    is_custom: false,
  },
];
