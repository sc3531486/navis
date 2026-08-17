/**
 * Hotkey 全局快捷键模块 - 触发分发器
 *
 * 对应设计文档 design/27-hotkey.md 架构设计中的 dispatcher.ts。
 * 负责监听键盘事件，将实际按键组合与存储匹配，触发对应命令。
 *
 * 工作原理：
 * 1. 监听浏览器 keydown/keyup 事件
 * 2. 维护当前按下的修饰键和主键状态
 * 3. 在 keydown 时构建完整按键组合字符串
 * 4. 在存储中查找匹配的 App 作用域绑定
 * 5. 匹配成功则阻止浏览器默认行为，发出触发通知
 *
 * 仅处理 App 作用域的快捷键：
 * - Global 作用域由 Tauri 后端通过系统级 API 处理
 * - App 作用域通过前端 keydown 事件处理
 *
 * 设计要点：
 * - 忽略 input/textarea/select/contenteditable 中的快捷键（避免干扰输入）
 * - 支持快捷键的 preventDefault 阻止浏览器默认行为
 * - 提供 start/stop 生命周期管理
 */

import { HotkeyScope } from './types';
import type { HotkeyBinding, HotkeyEventPayloads } from './types';
import { normalizeKeybinding } from './conflict';
import type { HotkeyNotifier } from './notifier';
import type { HotkeyStore } from './store';

/**
 * 将键盘事件转换为标准化的按键组合字符串
 *
 * 从 KeyboardEvent 中提取修饰键状态和主键名，
 * 组合为与存储格式一致的按键组合字符串。
 *
 * @param event 键盘事件
 * @returns 标准化按键组合字符串，如 "Ctrl+Shift+a"
 */
function keyboardEventToKeybinding(event: KeyboardEvent): string {
  const parts: string[] = [];

  // 收集修饰键（按固定顺序）
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  if (event.metaKey) parts.push('Meta');

  // 获取主键名
  // 对于特殊键（如 Backquote、Backslash）使用 event.code
  // 对于普通字母/数字键使用 event.key（小写）
  let mainKey: string;

  // 特殊键映射：event.code -> 存储使用的键名
  const specialKeyMap: Record<string, string> = {
    Backquote: '`',
    Backslash: '\\',
    Slash: '/',
    Period: '.',
    Comma: ',',
    Semicolon: ';',
    Quote: "'",
    BracketLeft: '[',
    BracketRight: ']',
    Equal: '=',
    Minus: '-',
  };

  // 修饰键列表（这些不应该作为主键）
  const modifierKeys = new Set([
    'Control',
    'Shift',
    'Alt',
    'Meta',
    'ControlLeft',
    'ControlRight',
    'ShiftLeft',
    'ShiftRight',
    'AltLeft',
    'AltRight',
    'MetaLeft',
    'MetaRight',
  ]);

  if (modifierKeys.has(event.key) || modifierKeys.has(event.code)) {
    // 仅按下修饰键，无法构成完整组合
    return '';
  }

  // 优先使用特殊键映射（如 "`" 对应 Backquote）
  if (event.code in specialKeyMap) {
    mainKey = specialKeyMap[event.code];
  } else {
    // 普通键：统一小写
    mainKey = event.key.toLowerCase();
  }

  parts.push(mainKey);

  return parts.join('+');
}

/**
 * 检查事件目标是否为可输入元素
 *
 * 当焦点在 input/textarea/select/contenteditable 元素中时，
 * 不拦截快捷键，避免干扰正常文本输入。
 *
 * @param target 事件目标元素
 * @returns 是否为可输入元素
 */
function isInputElement(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) {
    return false;
  }

  const tagName = target.tagName.toLowerCase();

  // 输入类元素
  if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
    return true;
  }

  // contenteditable 元素
  if (target.isContentEditable) {
    return true;
  }

  // 检查 role="textbox" 等无障碍角色
  const role = target.getAttribute('role');
  if (role === 'textbox' || role === 'searchbox' || role === 'combobox') {
    return true;
  }

  return false;
}

/**
 * 快捷键触发分发器类
 *
 * 管理键盘事件监听器的生命周期，将键盘事件与存储匹配并分发触发。
 */
export class HotkeyDispatcher {
  /** 本地通知器引用 */
  private notifier: HotkeyNotifier;

  /** 存储引用 */
  private store: HotkeyStore;

  /** 键盘按下事件处理器（bound 引用，用于 addEventListener/removeEventListener） */
  private handleKeyDown: ((event: KeyboardEvent) => void) | null = null;

  /** 是否已启动 */
  private started = false;

  /** 已注册的命令回调映射（command -> callback） */
  private commandCallbacks = new Map<string, (binding: HotkeyBinding) => void>();

  /**
   * @param notifier 快捷键本地通知器实例
   * @param store 快捷键存储实例
   */
  constructor(
    notifier: HotkeyNotifier,
    store: HotkeyStore,
  ) {
    this.notifier = notifier;
    this.store = store;
  }

  /**
   * 启动快捷键分发器
   *
   * 在 document 上注册 keydown 事件监听器，开始拦截和分发快捷键。
   * 重复调用是安全的（幂等）。
   */
  start(): void {
    if (this.started) return;

    // 创建事件处理器
    this.handleKeyDown = (event: KeyboardEvent) => {
      this.processKeyboardEvent(event);
    };

    // 在 document 上监听 keydown 事件
    // 使用捕获阶段（第三个参数 true）确保在其他处理器之前拦截
    document.addEventListener('keydown', this.handleKeyDown, true);
    this.started = true;
  }

  /**
   * 停止快捷键分发器
   *
   * 移除 document 上的事件监听器，停止快捷键拦截。
   * 重复调用是安全的（幂等）。
   */
  stop(): void {
    if (!this.started || !this.handleKeyDown) return;

    document.removeEventListener('keydown', this.handleKeyDown, true);
    this.handleKeyDown = null;
    this.started = false;
  }

  /**
   * 注册命令回调
   *
   * 为特定命令绑定执行回调。当快捷键被触发时，除了发出本地通知外，
   * 还会直接调用对应的回调函数。
   *
   * @param command 命令标识
   * @param callback 回调函数，接收匹配的 HotkeyBinding
   */
  onCommand(command: string, callback: (binding: HotkeyBinding) => void): void {
    this.commandCallbacks.set(command, callback);
  }

  /**
   * 注销命令回调
   *
   * @param command 命令标识
   */
  offCommand(command: string): void {
    this.commandCallbacks.delete(command);
  }

  /**
   * 处理键盘事件的核心逻辑
   *
   * @param event 键盘事件
   */
  private processKeyboardEvent(event: KeyboardEvent): void {
    // --- 检查事件目标 ---
    // 如果焦点在输入元素中，不拦截快捷键
    if (isInputElement(event.target)) {
      return;
    }

    // --- 构建按键组合字符串 ---
    const keybinding = keyboardEventToKeybinding(event);
    if (!keybinding) {
      // 仅按下修饰键，不构成完整组合，跳过
      return;
    }

    // --- 在存储中查找匹配的 App 作用域绑定 ---
    const normalizedInput = normalizeKeybinding(keybinding);
    const allBindings = this.store.getAll();

    for (const binding of allBindings) {
      // 仅匹配 App 作用域（Global 由 Tauri 后端处理）
      if (binding.scope !== HotkeyScope.App) {
        continue;
      }

      // 按键组合匹配
      if (normalizeKeybinding(binding.keybinding) === normalizedInput) {
        // --- 匹配成功，执行分发 ---

        // 阻止浏览器默认行为（如 Ctrl+B 不应触发加粗）
        event.preventDefault();
        // 阻止事件继续传播
        event.stopPropagation();

        // 发出触发通知
        this.notifier.notify('hotkey.triggered', {
          id: binding.id,
          keybinding: binding.keybinding,
          command: binding.command,
        });

        // 调用已注册的命令回调（如果存在）
        const callback = this.commandCallbacks.get(binding.command);
        if (callback) {
          try {
            callback(binding);
          } catch (error) {
            console.error(
              `[Hotkey] 命令回调执行出错 (${binding.command}):`,
              error,
            );
          }
        }

        // 匹配到第一个后立即返回（避免同一按键触发多个命令）
        return;
      }
    }
  }

  /**
   * 获取分发器运行状态
   */
  get isRunning(): boolean {
    return this.started;
  }

  /**
   * 销毁分发器，释放所有资源
   */
  destroy(): void {
    this.stop();
    this.commandCallbacks.clear();
  }
}
