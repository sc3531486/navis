/**
 * Hotkey 全局快捷键模块 - Solid.js 响应式 Hook
 *
 * 提供与 Solid.js 响应式系统深度集成的快捷键使用方式。
 * 将 HotkeyManager 的命令式 API 封装为声明式的响应式接口。
 *
 * 设计原则：
 * - 遵循 Solid.js 的响应式范式（createSignal / onCleanup）
 * - 自动在组件卸载时清理事件订阅（无内存泄漏）
 * - 返回 getter 函数（而非直接值），符合 Solid.js 约定
 *
 * 使用示例：
 * ```tsx
 * // 1. 使用 useHotkeys 获取快捷键列表
 * const hotkeys = useHotkeys();
 * // hotkeys() 返回当前快捷键绑定数组
 *
 * // 2. 使用 useHotkeyCommand 监听命令触发
 * useHotkeyCommand('commandPalette.open', (binding) => {
 *   openCommandPalette();
 * });
 *
 * // 3. 使用 useHotkeyTriggered 监听触发事件
 * useHotkeyTriggered((event) => {
 *   console.log(`快捷键触发: ${event.command}`);
 * });
 * ```
 */

import { createSignal, onCleanup, onMount } from 'solid-js';
import { getHotkeyManager } from './hotkey';
import type { HotkeyBinding, HotkeyEventPayloads } from './types';

/**
 * 获取响应式的快捷键列表
 *
 * 返回一个 getter 函数，返回当前所有已注册的快捷键绑定。
 * 组件挂载时自动加载列表，快捷键变更时自动更新。
 *
 * @returns getter 函数，调用时返回当前快捷键数组
 */
export function useHotkeys(): () => HotkeyBinding[] {
  const manager = getHotkeyManager();
  const [bindings, setBindings] = createSignal<HotkeyBinding[]>([]);

  // 组件挂载时加载初始列表
  onMount(async () => {
    const list = await manager.list();
    setBindings(list);
  });

  // 监听注册/注销/重置事件，自动刷新列表
  const refreshList = async () => {
    const list = await manager.list();
    setBindings(list);
  };

  const unsubRegister = manager.on('hotkey.registered', refreshList);
  const unsubUnregister = manager.on('hotkey.unregistered', refreshList);

  // 组件卸载时取消订阅
  onCleanup(() => {
    unsubRegister();
    unsubUnregister();
  });

  return bindings;
}

/**
 * 监听特定命令的快捷键触发
 *
 * 当指定命令的快捷键被按下时，执行回调函数。
 * 自动在组件卸载时清理。
 *
 * @param command 命令标识，如 "commandPalette.open"
 * @param callback 快捷键触发时的回调函数
 */
export function useHotkeyCommand(
  command: string,
  callback: (binding: HotkeyBinding) => void,
): void {
  const manager = getHotkeyManager();

  // 注册命令回调
  manager.onCommand(command, callback);

  // 组件卸载时注销
  onCleanup(() => {
    manager.offCommand(command);
  });
}

/**
 * 监听所有快捷键触发事件
 *
 * 当任意 App 作用域快捷键被触发时，执行回调函数。
 * 自动在组件卸载时清理。
 *
 * @param callback 触发事件回调
 */
export function useHotkeyTriggered(
  callback: (event: HotkeyEventPayloads['hotkey.triggered']) => void,
): void {
  const manager = getHotkeyManager();

  const unsub = manager.on('hotkey.triggered', callback);

  onCleanup(unsub);
}

/**
 * 获取快捷键冲突检测状态
 *
 * 用于设置页面的快捷键编辑场景：
 * 输入新按键组合时实时检测是否冲突。
 *
 * @returns [冲突检测函数, 当前冲突结果 getter]
 *
 * 使用示例：
 * ```tsx
 * const [checkConflict, conflictResult] = useHotkeyConflict();
 *
 * // 在输入框 onChange 中调用
 * checkConflict('Ctrl+B');
 *
 * // 读取冲突结果
 * const conflict = conflictResult(); // null 或冲突的 HotkeyBinding
 * ```
 */
export function useHotkeyConflict(): [
  (keybinding: string) => Promise<void>,
  () => HotkeyBinding | null,
] {
  const manager = getHotkeyManager();
  const [conflict, setConflict] = createSignal<HotkeyBinding | null>(null);

  /**
   * 检测指定按键组合是否存在冲突
   *
   * @param keybinding 待检测的按键组合
   */
  const check = async (keybinding: string) => {
    const result = await manager.checkConflict(keybinding);
    setConflict(result);
  };

  return [check, conflict];
}

/**
 * 获取指定分类的快捷键列表
 *
 * @param category 分类名称，如 "General"、"Agent"
 * @returns getter 函数，返回该分类下的快捷键数组
 */
export function useHotkeysByCategory(category: string): () => HotkeyBinding[] {
  const hotkeys = useHotkeys();

  return () => hotkeys().filter(b => b.category === category);
}
