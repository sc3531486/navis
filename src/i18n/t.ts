/**
 * i18n 国际化模块 - 翻译函数
 *
 * 提供核心的 t() 翻译函数，支持：
 * - 基本翻译（通过点号分隔的 key 访问嵌套属性）
 * - 插值变量（如 {count} 占位符替换）
 * - 复数形式（通过 count 参数选择复数形式）
 * - 缺失翻译回退（当前语言 -> en-US -> 输出 key 本身）
 *
 * 翻译函数本身是纯函数，不持有状态。
 * 语言包数据由 i18n/index.ts 中的响应式 store 管理，
 * 翻译函数通过参数接收当前语言包。
 */

import type { LocaleMessages, ExtensionLocaleMessages } from './types';

// ============================================================================
// 内部辅助函数
// ============================================================================

/**
 * 根据点号分隔的路径从对象中获取嵌套值
 *
 * @param obj - 源对象
 * @param path - 点号分隔的路径（如 "agent.thinking"）
 * @returns 找到的值，未找到则返回 undefined
 *
 * @example
 * getByPath({ agent: { thinking: "思考中..." } }, "agent.thinking")
 * // => "思考中..."
 */
function getByPath(obj: Record<string, any>, path: string): string | undefined {
  // 按点号拆分路径
  const keys = path.split('.');
  let current: any = obj;

  // 逐层深入查找
  for (const key of keys) {
    if (current == null || typeof current !== 'object') {
      return undefined;
    }
    current = current[key];
  }

  // 最终值必须是字符串
  return typeof current === 'string' ? current : undefined;
}

/**
 * 执行字符串插值
 *
 * 将模板字符串中的 {key} 占位符替换为参数中对应的值。
 * 如果参数中没有对应的 key，保留原始占位符。
 *
 * @param template - 包含占位符的模板字符串（如 "共 {count} 条消息"）
 * @param params - 插值参数对象（如 { count: 42 }）
 * @returns 替换后的字符串
 *
 * @example
 * interpolate("共 {count} 条消息", { count: 42 })
 * // => "共 42 条消息"
 */
function interpolate(template: string, params: Record<string, string | number>): string {
  // 匹配 {key} 占位符的正则表达式
  return template.replace(/\{(\w+)\}/g, (match, key: string) => {
    // 如果参数中存在对应的 key，替换为参数值；否则保留原始占位符
    if (key in params) {
      return String(params[key]);
    }
    return match;
  });
}

// ============================================================================
// 翻译函数实现
// ============================================================================

/**
 * 核心翻译函数
 *
 * 根据翻译 key 从语言包中查找对应的文案，支持插值和复数。
 *
 * 查找策略（回退链）：
 * 1. 当前语言包（如 zh-CN）
 * 2. 扩展语言包（如果 key 以扩展 ID 为前缀）
 * 3. 默认语言包（en-US）
 * 4. 输出 key 本身 + 控制台警告
 *
 * @param key - 翻译 key（点号分隔，如 "agent.thinking"）
 * @param currentMessages - 当前语言包
 * @param fallbackMessages - 回退语言包（en-US）
 * @param extensionMessages - 已加载的扩展语言包合集
 * @param paramsOrCount - 插值参数对象或复数计数
 * @returns 翻译后的字符串
 *
 * @example
 * // 基本翻译
 * translate('common.ok', zhCN, enUS, {}) => "确定"
 *
 * // 插值
 * translate('session.message_count', zhCN, enUS, {}, { count: 42 }) => "共 42 条消息"
 *
 * // 复数（count 数字会映射到 {count} 插值变量）
 * translate('notification.unread', zhCN, enUS, {}, 5) => "5 条未读通知"
 */
export function translate(
  key: string,
  currentMessages: LocaleMessages,
  fallbackMessages: LocaleMessages,
  extensionMessages: Record<string, ExtensionLocaleMessages>,
  paramsOrCount?: Record<string, string | number> | number,
): string {
  let result: string | undefined;

  // --- 第一步：从当前语言包中查找 ---
  result = getByPath(currentMessages as Record<string, any>, key);

  // --- 第二步：如果未找到，从扩展语言包中查找 ---
  // 扩展 ID 可以包含点号（如 com.example.ext），因此不能只截取第一段。
  if (result === undefined) {
    for (const [extensionId, extensionBundle] of Object.entries(extensionMessages)) {
      if (key === extensionId || key.startsWith(`${extensionId}.`)) {
        result = extensionBundle[key];
        if (result !== undefined) break;
      }
    }
  }

  // --- 第三步：从回退语言包（en-US）中查找 ---
  if (result === undefined) {
    result = getByPath(fallbackMessages as Record<string, any>, key);
  }

  // --- 第四步：回退链全部未命中，输出 key 本身并打印警告 ---
  if (result === undefined) {
    if (typeof console !== 'undefined') {
      console.warn(`[i18n] Missing translation key: "${key}"`);
    }
    result = key;
  }

  // --- 处理插值参数 ---
  if (paramsOrCount !== undefined) {
    // 如果是数字，映射为 {count} 插值变量（用于复数场景）
    const params: Record<string, string | number> =
      typeof paramsOrCount === 'number'
        ? { count: paramsOrCount }
        : paramsOrCount;

    result = interpolate(result, params);
  }

  return result;
}
