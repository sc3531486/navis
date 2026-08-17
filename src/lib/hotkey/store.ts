/**
 * Hotkey 全局快捷键模块 - 快捷键存储
 *
 * 对应设计文档 design/27-hotkey.md 架构设计中的 store.ts。
 * 负责管理所有已注册的快捷键绑定的增删改查。
 *
 * 职责：
 * - 存储和管理所有 HotkeyBinding 数据
 * - 加载系统默认快捷键
 * - 提供按 id / 按 command / 按分类的查询能力
 * - 与本地通知器协作，注册/注销时发出对应通知
 * - 序列化/反序列化用户自定义配置（为持久化做准备）
 *
 * 设计约束（对应文档 §7.1）：
 * - 扩展只能注册 App 作用域的快捷键，不能注册 Global 作用域
 * - Global 作用域仅核心模块可使用
 */

import { HotkeyScope } from './types';
import type { HotkeyBinding, HotkeyEventPayloads } from './types';
import { DEFAULT_HOTKEYS } from './defaults';
import { findConflict, normalizeKeybinding } from './conflict';
import type { HotkeyNotifier } from './notifier';

/**
 * 快捷键存储类
 *
 * 核心数据存储层，管理所有快捷键绑定的生命周期。
 * 与 HotkeyNotifier 协作，确保每次注册/注销/冲突都发出对应本地通知。
 */
export class HotkeyStore {
  /** 已注册的快捷键绑定映射表（id -> binding） */
  private bindings = new Map<string, HotkeyBinding>();

  /** 本地通知器引用，用于发出注册/注销/冲突通知 */
  private notifier: HotkeyNotifier;

  /**
   * @param notifier 快捷键本地通知器实例
   */
  constructor(notifier: HotkeyNotifier) {
    this.notifier = notifier;
  }

  // ============================================================
  // 初始化
  // ============================================================

  /**
   * 加载系统默认快捷键
   *
   * 将 DEFAULT_HOTKEYS 中定义的所有默认绑定注册到存储。
   * 如果已有用户自定义的同 id 绑定，不会覆盖（用户自定义优先）。
   */
  loadDefaults(): void {
    for (const binding of DEFAULT_HOTKEYS) {
      // 仅在该 id 尚未被注册时才添加（避免覆盖用户自定义配置）
      if (!this.bindings.has(binding.id)) {
        this.bindings.set(binding.id, { ...binding });
      }
    }
  }

  // ============================================================
  // 增删改查
  // ============================================================

  /**
   * 注册一条新的快捷键绑定
   *
   * 对应设计文档 §5 接口：hotkey.register(binding)
   *
   * 注册流程：
   * 1. 校验绑定数据完整性
   * 2. 检查 id 是否已存在
   * 3. 检测按键冲突（同作用域内）
   * 4. 存入存储
   * 5. 发出 hotkey.registered 事件
   *
   * @param binding 要注册的快捷键绑定
   * @throws Error 当 id 已存在、数据不合法或存在冲突时
   */
  register(binding: HotkeyBinding): void {
    // --- 数据校验 ---
    if (!binding.id || !binding.keybinding || !binding.command) {
      throw new Error(
        `[Hotkey] 注册失败：绑定数据不完整，缺少 id/keybinding/command 字段`,
      );
    }

    // --- 注册约束校验（§7.1）：扩展不能注册 Global 作用域 ---
    // is_custom 为 true 表示来自扩展/用户，此时不允许 Global 作用域
    if (binding.is_custom && binding.scope === HotkeyScope.Global) {
      throw new Error(
        `[Hotkey] 注册失败：扩展/用户自定义快捷键不能使用 Global 作用域，` +
          `仅核心模块可注册系统级全局快捷键（command: ${binding.command}）`,
      );
    }

    // --- id 唯一性检查 ---
    if (this.bindings.has(binding.id)) {
      throw new Error(
        `[Hotkey] 注册失败：id "${binding.id}" 已存在，请使用 unregister 先注销或 update 更新`,
      );
    }

    // --- 冲突检测（§7.2） ---
    const conflict = findConflict(
      binding.keybinding,
      binding.scope,
      this.getAll(),
    );
    if (conflict) {
      // 发出冲突通知
      this.notifier.notify('hotkey.conflict', {
        id: binding.id,
        keybinding: binding.keybinding,
        conflictWith: conflict.id,
      });
      throw new Error(
        `[Hotkey] 注册失败：快捷键 "${binding.keybinding}" 与已注册的 ` +
          `"${conflict.command}"（${conflict.description}）冲突`,
      );
    }

    // --- 存入存储 ---
    this.bindings.set(binding.id, { ...binding });

    // --- 发出注册成功通知 ---
    this.notifier.notify('hotkey.registered', {
      id: binding.id,
      keybinding: binding.keybinding,
    });
  }

  /**
   * 注销一条快捷键绑定
   *
   * 对应设计文档 §5 接口：hotkey.unregister(id)
   *
   * @param id 要注销的绑定 id
   * @throws Error 当 id 不存在时
   */
  unregister(id: string): void {
    if (!this.bindings.has(id)) {
      throw new Error(`[Hotkey] 注销失败：id "${id}" 不存在`);
    }

    this.bindings.delete(id);

    // 发出注销通知
    this.notifier.notify('hotkey.unregistered', { id });
  }

  /**
   * 更新指定快捷键的按键组合
   *
   * 对应设计文档 §5 接口：hotkey.update(id, keybinding)
   *
   * 更新流程：
   * 1. 校验 id 存在性
   * 2. 规范化新的按键组合
   * 3. 检测冲突（排除自身）
   * 4. 更新绑定
   * 5. 发出 hotkey.registered 通知（更新视同重新注册）
   *
   * @param id 要更新的绑定 id
   * @param keybinding 新的按键组合字符串
   * @throws Error 当 id 不存在或新按键组合存在冲突时
   */
  update(id: string, keybinding: string): void {
    const binding = this.bindings.get(id);
    if (!binding) {
      throw new Error(`[Hotkey] 更新失败：id "${id}" 不存在`);
    }

    // 规范化按键组合
    const normalizedNew = normalizeKeybinding(keybinding);

    // 如果按键组合未变化，直接返回（幂等操作）
    if (normalizeKeybinding(binding.keybinding) === normalizedNew) {
      return;
    }

    // 冲突检测（排除自身）
    const conflict = findConflict(
      keybinding,
      binding.scope,
      this.getAll(),
      id,
    );
    if (conflict) {
      this.notifier.notify('hotkey.conflict', {
        id,
        keybinding: normalizedNew,
        conflictWith: conflict.id,
      });
      throw new Error(
        `[Hotkey] 更新失败：新快捷键 "${normalizedNew}" 与已注册的 ` +
          `"${conflict.command}"（${conflict.description}）冲突`,
      );
    }

    // 更新绑定
    this.bindings.set(id, { ...binding, keybinding: normalizedNew });
  }

  /**
   * 获取指定 id 的快捷键绑定
   *
   * @param id 绑定 id
   * @returns 绑定数据，不存在返回 undefined
   */
  get(id: string): HotkeyBinding | undefined {
    return this.bindings.get(id);
  }

  /**
   * 获取所有已注册的快捷键绑定列表
   *
   * 对应设计文档 §5 接口：hotkey.list()
   *
   * @returns 快捷键绑定数组的深拷贝
   */
  getAll(): HotkeyBinding[] {
    return Array.from(this.bindings.values()).map(b => ({ ...b }));
  }

  /**
   * 按分类获取快捷键绑定
   *
   * 用于快捷键帮助文档的分类展示。
   *
   * @param category 分类名称
   * @returns 该分类下的绑定列表
   */
  getByCategory(category: string): HotkeyBinding[] {
    return this.getAll().filter(b => b.category === category);
  }

  /**
   * 按命令名查找快捷键绑定
   *
   * @param command 命令标识
   * @returns 匹配的绑定，未找到返回 undefined
   */
  getByCommand(command: string): HotkeyBinding | undefined {
    for (const binding of this.bindings.values()) {
      if (binding.command === command) {
        return { ...binding };
      }
    }
    return undefined;
  }

  /**
   * 获取所有不重复的分类列表
   *
   * @returns 分类名称数组
   */
  getCategories(): string[] {
    const categories = new Set<string>();
    for (const binding of this.bindings.values()) {
      categories.add(binding.category);
    }
    return Array.from(categories);
  }

  // ============================================================
  // 重置
  // ============================================================

  /**
   * 重置所有快捷键为系统默认值
   *
   * 对应设计文档 §5 接口：hotkey.reset()
   *
   * 操作：
   * 1. 清空所有已注册绑定
   * 2. 重新加载系统默认快捷键
   */
  reset(): void {
    this.bindings.clear();
    this.loadDefaults();
  }

  // ============================================================
  // 序列化（持久化支持）
  // ============================================================

  /**
   * 导出用户自定义的快捷键配置
   *
   * 仅导出 is_custom 为 true 的绑定，用于持久化存储。
   * 默认快捷键不需要持久化，每次启动时从 DEFAULT_HOTKEYS 加载。
   *
   * @returns 可序列化的用户自定义绑定数组
   */
  exportCustom(): HotkeyBinding[] {
    return this.getAll().filter(b => b.is_custom);
  }

  /**
   * 导入用户自定义的快捷键配置
   *
   * 从持久化存储恢复用户自定义绑定。
   * 在 loadDefaults() 之后调用，用户自定义会覆盖默认的按键组合。
   *
   * @param customBindings 用户自定义绑定数组
   */
  importCustom(customBindings: HotkeyBinding[]): void {
    for (const binding of customBindings) {
      // 验证必要字段
      if (!binding.id || !binding.keybinding || !binding.command) {
        console.warn('[Hotkey] 跳过无效的自定义绑定:', binding);
        continue;
      }

      // 检查是否覆盖默认快捷键（同 id）
      const existing = this.bindings.get(binding.id);
      if (existing) {
        // 更新按键组合，保留其他默认属性
        this.bindings.set(binding.id, {
          ...existing,
          keybinding: normalizeKeybinding(binding.keybinding),
          is_custom: true,
        });
      } else {
        // 新增自定义绑定
        this.bindings.set(binding.id, {
          ...binding,
          keybinding: normalizeKeybinding(binding.keybinding),
          is_custom: true,
        });
      }
    }
  }

  /**
   * 获取绑定总数
   */
  get size(): number {
    return this.bindings.size;
  }
}
