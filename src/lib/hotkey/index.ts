/**
 * Hotkey 全局快捷键模块 - 入口文件
 *
 * 对应设计文档 design/27-hotkey.md 架构设计中的 mod.rs（模块入口）。
 * 统一导出所有公共 API、类型和 Hook。
 *
 * 模块架构：
 * ```
 * hotkey/
 * ├── types.ts        # TypeScript 类型定义（数据模型 + 事件 + 接口）
 * ├── defaults.ts     # 系统默认快捷键配置（7 条内置绑定）
 * ├── conflict.ts     # 冲突检测（规范化 + 比较 + 查找冲突）
 * ├── notifier.ts     # Hotkey 模块本地通知器
 * ├── store.ts     # 快捷键存储（数据存储 + 增删改查）
 * ├── dispatcher.ts   # 触发分发器（键盘事件监听 + 命令分发）
 * ├── hotkey.ts       # 主模块（协调层 + 全局单例）
 * ├── use-hotkey.ts   # Solid.js 响应式 Hook
 * └── index.ts        # 入口文件（本文件）
 * ```
 *
 * 使用指南：
 *
 * 1. 直接使用管理器 API（命令式）：
 * ```typescript
 * import { getHotkeyManager } from '@/lib/hotkey';
 *
 * const hotkey = getHotkeyManager();
 * const bindings = await hotkey.list();
 * await hotkey.register(binding);
 * ```
 *
 * 2. 使用 Solid.js Hook（声明式）：
 * ```tsx
 * import { useHotkeys, useHotkeyCommand } from '@/lib/hotkey';
 *
 * function MyComponent() {
 *   const hotkeys = useHotkeys();
 *   useHotkeyCommand('commandPalette.open', () => openPalette());
 *   return <div>{hotkeys().length} 个快捷键</div>;
 * }
 * ```
 *
 * 3. 使用类型定义：
 * ```typescript
 * import { HotkeyScope, HotkeyBinding } from '@/lib/hotkey';
 * ```
 */

// ============================================================
// 类型导出
// ============================================================
export {
  HotkeyScope,
  type HotkeyBinding,
  type HotkeyEventPayloads,
  type HotkeyEventName,
  type HotkeyManagerAPI,
  type HotkeyCallback,
  type ParsedKeybinding,
} from './types';

// ============================================================
// 默认配置导出
// ============================================================
export { DEFAULT_HOTKEYS } from './defaults';

// ============================================================
// 冲突检测工具导出
// ============================================================
export { normalizeKeybinding, isKeybindingEqual, findConflict } from './conflict';

// ============================================================
// Hotkey 本地通知器导出
// ============================================================
export { HotkeyNotifier, createHotkeyNotifier } from './notifier';

// ============================================================
// 存储导出
// ============================================================
export { HotkeyStore } from './store';

// ============================================================
// 分发器导出
// ============================================================
export { HotkeyDispatcher } from './dispatcher';

// ============================================================
// 主模块导出
// ============================================================
export { HotkeyManager, getHotkeyManager, resetHotkeyManager } from './hotkey';

// ============================================================
// Solid.js Hook 导出
// ============================================================
export {
  useHotkeys,
  useHotkeyCommand,
  useHotkeyTriggered,
  useHotkeyConflict,
  useHotkeysByCategory,
} from './use-hotkey';
