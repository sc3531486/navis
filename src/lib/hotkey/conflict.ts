/**
 * Hotkey 全局快捷键模块 - 冲突检测
 *
 * 对应设计文档 design/27-hotkey.md §7.2 冲突处理策略：
 * - 后注册被拒绝：不自动覆盖已有绑定
 * - 提示用户：返回冲突信息，告知哪个快捷键已被哪个命令占用
 * - 用户可手动修改冲突方的快捷键后再重新注册
 *
 * 冲突判定规则：
 * 1. 按键组合完全匹配（规范化后比较）
 * 2. 作用域在同一范围内（同为 App 或同为 Global）
 * 3. 忽略绑定自身的 id（用于 update 场景）
 */

import type { HotkeyBinding } from './types';

/**
 * 规范化按键组合字符串
 *
 * 将用户输入或配置中的按键组合统一为标准格式：
 * - 修饰键统一为首字母大写（Ctrl、Shift、Alt、Meta）
 * - 修饰键按固定顺序排列：Ctrl > Shift > Alt > Meta
 * - 主键统一为小写
 * - 去除多余空格
 *
 * 示例：
 *   "ctrl+shift+a" -> "Ctrl+Shift+a"
 *   "SHIFT+CTRL+b" -> "Ctrl+Shift+b"
 *   "Alt + F4"     -> "Alt+f4"
 *
 * @param keybinding 原始按键组合字符串
 * @returns 规范化后的按键组合字符串
 */
export function normalizeKeybinding(keybinding: string): string {
  // 按 "+" 分割并去除每部分的首尾空格
  const parts = keybinding.split('+').map(p => p.trim().toLowerCase());

  // 修饰键优先级映射表（用于排序）
  const modifierOrder: Record<string, number> = {
    ctrl: 0,
    shift: 1,
    alt: 2,
    meta: 3,
  };

  // 将修饰键和主键分离
  const modifiers: string[] = [];
  const keys: string[] = [];

  for (const part of parts) {
    if (part in modifierOrder) {
      // 修饰键：首字母大写
      modifiers.push(part.charAt(0).toUpperCase() + part.slice(1));
    } else {
      // 主键：保持小写
      keys.push(part);
    }
  }

  // 按固定顺序排序修饰键：Ctrl > Shift > Alt > Meta
  modifiers.sort((a, b) => {
    const orderA = modifierOrder[a.toLowerCase()] ?? 99;
    const orderB = modifierOrder[b.toLowerCase()] ?? 99;
    return orderA - orderB;
  });

  // 组合为最终规范化字符串
  return [...modifiers, ...keys].join('+');
}

/**
 * 检测两条按键组合是否等价
 *
 * 通过规范化后进行字符串精确比较。
 * 两条组合即使输入格式不同（如大小写、空格、顺序），规范化后也会一致。
 *
 * @param a 第一条按键组合
 * @param b 第二条按键组合
 * @returns 是否等价
 */
export function isKeybindingEqual(a: string, b: string): boolean {
  return normalizeKeybinding(a) === normalizeKeybinding(b);
}

/**
 * 在已有绑定列表中查找冲突
 *
 * 对应设计文档 §7.2 的冲突检测逻辑：
 * - 遍历所有已注册绑定
 * - 按键组合匹配 + 作用域相同 = 冲突
 * - 可选排除指定 id（用于 update 时排除自身）
 *
 * @param keybinding 待检测的按键组合
 * @param scope 待检测的作用域
 * @param bindings 已注册的绑定列表
 * @param excludeId 排除的绑定 id（可选，用于更新场景排除自身）
 * @returns 冲突的绑定，无冲突返回 null
 */
export function findConflict(
  keybinding: string,
  scope: string,
  bindings: HotkeyBinding[],
  excludeId?: string,
): HotkeyBinding | null {
  const normalizedTarget = normalizeKeybinding(keybinding);

  for (const binding of bindings) {
    // 跳过自身（用于 update 场景）
    if (excludeId && binding.id === excludeId) {
      continue;
    }

    // 冲突条件：作用域相同 且 按键组合等价
    if (
      binding.scope === scope &&
      normalizeKeybinding(binding.keybinding) === normalizedTarget
    ) {
      return binding;
    }
  }

  return null;
}
