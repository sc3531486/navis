/**
 * Hotkey 全局快捷键模块 - 主模块
 *
 * 对应设计文档 design/27-hotkey.md 架构设计中的 mod.rs（模块入口）。
 * 作为快捷键系统的统一门面（Facade），协调以下组件：
 *
 * - HotkeyStore: 快捷键存储（数据存储层）
 * - HotkeyDispatcher: 触发分发器（事件监听层）
 * - HotkeyNotifier: 快捷键本地通知器（仅限前端模块内分发）
 *
 * 对外暴露设计文档 §5 定义的全部 API 接口：
 * - list / register / unregister / update / checkConflict / reset
 *
 * 生命周期：
 * 1. 构造 -> 内部创建 emitter、store、dispatcher
 * 2. init()  -> 加载默认快捷键 + 启动事件监听
 * 3. 对外提供 API 方法
 * 4. destroy() -> 清理所有资源
 *
 * 全局单例模式：
 * 使用模块级变量保存单例实例，通过 getHotkeyManager() 获取。
 */

import { HotkeyScope } from './types';
import type { HotkeyBinding, HotkeyManagerAPI } from './types';
import { HotkeyStore } from './store';
import { HotkeyDispatcher } from './dispatcher';
import { createHotkeyNotifier } from './notifier';
import { findConflict } from './conflict';
import type { HotkeyNotifier } from './notifier';
import type { HotkeyEventPayloads } from './types';

/**
 * 快捷键管理器主类
 *
 * 实现 HotkeyManagerAPI 接口，作为快捷键模块的统一入口。
 * 聚合 Store（数据层）、Dispatcher（监听层）、Notifier（本地通知层）。
 */
export class HotkeyManager implements HotkeyManagerAPI {
  /** 本地通知器 */
  private notifier: HotkeyNotifier;

  /** 快捷键存储 */
  private store: HotkeyStore;

  /** 快捷键分发器 */
  private dispatcher: HotkeyDispatcher;

  /** 是否已初始化 */
  private initialized = false;

  constructor() {
    // 按依赖顺序创建组件：notifier -> store -> dispatcher
    this.notifier = createHotkeyNotifier();
    this.store = new HotkeyStore(this.notifier);
    this.dispatcher = new HotkeyDispatcher(this.notifier, this.store);
  }

  // ============================================================
  // 生命周期管理
  // ============================================================

  /**
   * 初始化快捷键管理器
   *
   * 执行流程：
   * 1. 加载系统默认快捷键到存储
   * 2. 启动事件分发器（开始监听键盘事件）
   *
   * 幂等：重复调用是安全的。
   */
  init(): void {
    if (this.initialized) return;

    // 步骤1：加载系统默认快捷键
    this.store.loadDefaults();

    // 步骤2：启动 App 作用域的键盘事件监听
    this.dispatcher.start();

    this.initialized = true;
  }

  /**
   * 销毁快捷键管理器，释放所有资源
   *
   * 执行流程：
   * 1. 停止事件分发器
   * 2. 清理本地通知器的所有监听器
   * 3. 标记为未初始化
   */
  destroy(): void {
    this.dispatcher.destroy();
    this.notifier.clear();
    this.initialized = false;
  }

  // ============================================================
  // 设计文档 §5 接口实现
  // ============================================================

  /**
   * 获取所有已注册的快捷键列表
   *
   * 对应接口：hotkey.list(): Promise<HotkeyBinding[]>
   *
   * @returns 包含默认和自定义快捷键的完整列表
   */
  async list(): Promise<HotkeyBinding[]> {
    return this.store.getAll();
  }

  /**
   * 注册一条新的快捷键绑定
   *
   * 对应接口：hotkey.register(binding: HotkeyBinding): Promise<void>
   *
   * 约束：
   * - 扩展/自定义只能注册 App 作用域（§7.1）
   * - 按键冲突时拒绝注册并抛出错误（§7.2）
   *
   * @param binding 要注册的快捷键绑定
   * @throws Error 当注册失败时（冲突/数据不合法/权限不足）
   */
  async register(binding: HotkeyBinding): Promise<void> {
    this.store.register(binding);
  }

  /**
   * 注销一条快捷键绑定
   *
   * 对应接口：hotkey.unregister(id: string): Promise<void>
   *
   * @param id 要注销的绑定 id
   * @throws Error 当 id 不存在时
   */
  async unregister(id: string): Promise<void> {
    this.store.unregister(id);
  }

  /**
   * 更新指定快捷键的按键组合
   *
   * 对应接口：hotkey.update(id: string, keybinding: string): Promise<void>
   *
   * 更新流程：
   * 1. 在存储中更新绑定
   * 2. 冲突检测在存储内部完成
   *
   * @param id 要更新的绑定 id
   * @param keybinding 新的按键组合
   * @throws Error 当 id 不存在或按键冲突时
   */
  async update(id: string, keybinding: string): Promise<void> {
    this.store.update(id, keybinding);
  }

  /**
   * 检测给定按键组合是否与已有绑定冲突
   *
   * 对应接口：hotkey.checkConflict(keybinding: string): Promise<HotkeyBinding | null>
   *
   * 在 App 作用域中查找冲突（Global 作用域需通过 Tauri 后端检测）。
   *
   * @param keybinding 待检测的按键组合
   * @returns 冲突的绑定，无冲突返回 null
   */
  async checkConflict(keybinding: string): Promise<HotkeyBinding | null> {
    return findConflict(keybinding, HotkeyScope.App, this.store.getAll());
  }

  /**
   * 重置所有快捷键为系统默认值
   *
   * 对应接口：hotkey.reset(): Promise<void>
   *
   * 清除所有自定义配置，恢复到 DEFAULT_HOTKEYS 初始状态。
   */
  async reset(): Promise<void> {
    this.store.reset();
  }

  // ============================================================
  // 扩展 API
  // ============================================================

  /**
   * 订阅快捷键事件
   *
   * 对应设计文档 §6 本地通知定义。
   * 返回 unsubscribe 函数，便于 Solid.js 的 onCleanup 使用。
   *
   * @param event 事件名称
   * @param callback 事件回调
   * @returns 取消订阅函数
   */
  on<K extends keyof HotkeyEventPayloads & string>(
    event: K,
    callback: (payload: HotkeyEventPayloads[K]) => void,
  ): () => void {
    return this.notifier.on(event, callback);
  }

  /**
   * 取消订阅快捷键事件
   *
   * @param event 事件名称
   * @param callback 要移除的回调（可选）
   */
  off<K extends keyof HotkeyEventPayloads & string>(
    event: K,
    callback?: (payload: HotkeyEventPayloads[K]) => void,
  ): void {
    this.notifier.off(event, callback);
  }

  /**
   * 为指定命令注册执行回调
   *
   * 当快捷键被触发时，分发器除了发出事件外，
   * 还会直接调用通过此方法注册的回调。
   *
   * @param command 命令标识
   * @param callback 命令执行回调
   */
  onCommand(command: string, callback: (binding: HotkeyBinding) => void): void {
    this.dispatcher.onCommand(command, callback);
  }

  /**
   * 注销命令执行回调
   *
   * @param command 命令标识
   */
  offCommand(command: string): void {
    this.dispatcher.offCommand(command);
  }

  /**
   * 获取指定分类的快捷键列表
   *
   * @param category 分类名称
   * @returns 该分类下的快捷键绑定数组
   */
  getByCategory(category: string): HotkeyBinding[] {
    return this.store.getByCategory(category);
  }

  /**
   * 获取所有分类列表
   *
   * @returns 分类名称数组
   */
  getCategories(): string[] {
    return this.store.getCategories();
  }

  /**
   * 获取内部存储引用（高级用法）
   *
   * 仅供需要直接操作存储的场景使用。
   * 常规使用应通过 HotkeyManager 的公共 API。
   */
  getStore(): HotkeyStore {
    return this.store;
  }

  /**
   * 获取内部本地通知器引用（高级用法）
   *
   * 仅供需要自定义 Hotkey 模块本地通知监听的场景使用。
   * 常规使用应通过 HotkeyManager 的 on/off 方法。
   */
  getNotifier(): HotkeyNotifier {
    return this.notifier;
  }
}

// ============================================================
// 全局单例
// ============================================================

/** 模块级单例实例 */
let hotkeyManagerInstance: HotkeyManager | null = null;

/**
 * 获取快捷键管理器单例
 *
 * 全局唯一的快捷键管理器实例。
 * 首次调用时自动创建并初始化。
 *
 * 使用方式：
 * ```typescript
 * const hotkey = getHotkeyManager();
 * await hotkey.list();
 * await hotkey.register(binding);
 * ```
 *
 * @returns HotkeyManager 单例实例
 */
export function getHotkeyManager(): HotkeyManager {
  if (!hotkeyManagerInstance) {
    hotkeyManagerInstance = new HotkeyManager();
    hotkeyManagerInstance.init();
  }
  return hotkeyManagerInstance;
}

/**
 * 重置快捷键管理器单例
 *
 * 主要用于测试环境，销毁当前单例并重置。
 * 下次调用 getHotkeyManager() 时会创建新实例。
 */
export function resetHotkeyManager(): void {
  if (hotkeyManagerInstance) {
    hotkeyManagerInstance.destroy();
    hotkeyManagerInstance = null;
  }
}
