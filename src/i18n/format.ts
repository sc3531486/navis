/**
 * i18n 国际化模块 - 格式化工具
 *
 * 提供日期、数字、货币及相对时间的本地化格式化能力。
 * 全部基于浏览器原生 Intl API 实现，无额外依赖。
 *
 * 核心功能：
 * - formatDate: 日期格式化（支持预设样式和自定义模式）
 * - formatNumber: 数字格式化（支持千分位、小数位等选项）
 * - formatCurrency: 货币格式化
 * - formatRelative: 相对时间格式化（如 "3分钟前"）
 */

import type { SupportedLocale, DateFormatOptions, RelativeTimeUnit } from './types';

// ============================================================================
// 相对时间单位阈值配置
// ============================================================================

/**
 * 相对时间各单位的秒数阈值与对应的 Intl 单位
 * 按从大到小排列，用于自动选择最合适的时间单位
 */
const RELATIVE_TIME_THRESHOLDS: Array<{
  /** 秒数阈值上限 */
  threshold: number;
  /** 对应的 Intl.RelativeTimeFormat 单位 */
  unit: RelativeTimeUnit;
  /** 每单位包含的秒数 */
  divisor: number;
}> = [
  { threshold: 31536000, unit: 'year', divisor: 31536000 },     // 365 天
  { threshold: 2592000, unit: 'month', divisor: 2592000 },      // 30 天
  { threshold: 604800, unit: 'week', divisor: 604800 },          // 7 天
  { threshold: 86400, unit: 'day', divisor: 86400 },             // 1 天
  { threshold: 3600, unit: 'hour', divisor: 3600 },              // 1 小时
  { threshold: 60, unit: 'minute', divisor: 60 },                // 1 分钟
  { threshold: 0, unit: 'second', divisor: 1 },                  // 秒
];

// ============================================================================
// 日期格式化
// ============================================================================

/**
 * 格式化日期为本地化字符串
 *
 * 使用 Intl.DateTimeFormat API，根据当前语言环境输出对应格式。
 *
 * @param date - 待格式化的日期对象
 * @param locale - 当前语言标识
 * @param options - 格式化选项（样式、是否包含时间等）
 * @returns 格式化后的日期字符串
 *
 * @example
 * formatDate(new Date(), 'zh-CN', { style: 'long', includeTime: true })
 * // => "2024年3月15日 14:30:00"
 *
 * formatDate(new Date(), 'en-US', { style: 'short' })
 * // => "3/15/24"
 */
export function formatDate(
  date: Date,
  locale: SupportedLocale,
  options?: DateFormatOptions,
): string {
  // 默认使用 medium 样式
  const style = options?.style ?? 'medium';
  const includeTime = options?.includeTime ?? false;

  // 构建 Intl.DateTimeFormat 选项
  const formatOptions: Intl.DateTimeFormatOptions = {
    dateStyle: style,
  };

  // 如果需要包含时间，添加 timeStyle
  if (includeTime) {
    formatOptions.timeStyle = style;
  }

  return new Intl.DateTimeFormat(locale, formatOptions).format(date);
}

// ============================================================================
// 数字格式化
// ============================================================================

/**
 * 格式化数字为本地化字符串
 *
 * 使用 Intl.NumberFormat API，支持千分位分隔符、小数位数等选项。
 *
 * @param num - 待格式化的数字
 * @param locale - 当前语言标识
 * @param options - Intl.NumberFormatOptions 选项
 * @returns 格式化后的数字字符串
 *
 * @example
 * formatNumber(1234567.89, 'zh-CN')
 * // => "1,234,567.89"
 *
 * formatNumber(1234567.89, 'zh-CN', { maximumFractionDigits: 0 })
 * // => "1,234,568"
 */
export function formatNumber(
  num: number,
  locale: SupportedLocale,
  options?: Intl.NumberFormatOptions,
): string {
  return new Intl.NumberFormat(locale, options).format(num);
}

// ============================================================================
// 货币格式化
// ============================================================================

/**
 * 格式化数字为货币字符串
 *
 * @param amount - 金额
 * @param locale - 当前语言标识
 * @param currency - 货币代码（如 'CNY', 'USD'）
 * @returns 格式化后的货币字符串
 *
 * @example
 * formatCurrency(99.5, 'zh-CN', 'CNY')
 * // => "¥99.50"
 *
 * formatCurrency(99.5, 'en-US', 'USD')
 * // => "$99.50"
 */
export function formatCurrency(
  amount: number,
  locale: SupportedLocale,
  currency: string = 'CNY',
): string {
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
  }).format(amount);
}

// ============================================================================
// 相对时间格式化
// ============================================================================

/**
 * 格式化日期为相对时间字符串
 *
 * 基于 Intl.RelativeTimeFormat API，根据当前 locale 自动输出
 * 对应语言的相对时间文本（如中文 "3分钟前"、英文 "3 minutes ago"）。
 *
 * 自动选择最合适的时间单位（年、月、周、天、小时、分钟、秒）。
 *
 * @param date - 目标日期（相对于当前时间的过去或未来）
 * @param locale - 当前语言标识
 * @returns 相对时间字符串
 *
 * @example
 * // 假设当前时间是 2024-03-15 14:30:00
 * formatRelative(new Date('2024-03-15T14:27:00'), 'zh-CN')
 * // => "3分钟前"
 *
 * formatRelative(new Date('2024-03-15T14:27:00'), 'en-US')
 * // => "3 minutes ago"
 */
export function formatRelative(date: Date, locale: SupportedLocale): string {
  // 计算与当前时间的差值（秒），正数表示过去，负数表示未来
  const now = Date.now();
  const diffInSeconds = Math.floor((now - date.getTime()) / 1000);

  // 创建 RelativeTimeFormat 实例，使用 "auto" 数值以获得自然的文本（如 "yesterday"）
  const rtf = new Intl.RelativeTimeFormat(locale, {
    numeric: 'auto',
    style: 'long',
  });

  // 遍历阈值配置，找到最合适的时间单位
  for (const { threshold, unit, divisor } of RELATIVE_TIME_THRESHOLDS) {
    if (Math.abs(diffInSeconds) >= threshold) {
      // 计算该单位下的数值（向下取整）
      const value = Math.floor(diffInSeconds / divisor);

      // 负数表示过去（如 -3 minutes ago => "3 minutes ago"）
      // 正数表示未来（如 3 minutes from now => "in 3 minutes"）
      return rtf.format(-value, unit);
    }
  }

  // 默认返回 "刚刚" / "just now"
  return rtf.format(0, 'second');
}

// ============================================================================
// 热键描述翻译判断
// ============================================================================

/**
 * 判断一个字符串是否是 i18n 翻译 key
 *
 * 根据设计文档第五章，热键描述可以是普通文本或 i18n key。
 * i18n key 的特征是以已知命名空间前缀开头，用点号分隔。
 *
 * 已知命名空间：common, session, agent, settings, notification,
 * hotkey, worktree, command_palette, dialog, extension, sandbox,
 * health, file, terminal, git, editor
 *
 * @param description - 待检测的字符串
 * @returns 是否是 i18n key
 *
 * @example
 * isI18nKey('hotkey.open_command_palette')  // => true
 * isI18nKey('打开命令面板')                    // => false
 */
export function isI18nKey(description: string): boolean {
  // 已知的语言包命名空间前缀列表
  const NAMESPACES = [
    'common', 'session', 'agent', 'settings', 'notification',
    'hotkey', 'worktree', 'command_palette', 'dialog', 'extension',
    'sandbox', 'health', 'file', 'terminal', 'git', 'editor',
  ];

  // 检查是否以已知命名空间开头
  return NAMESPACES.some((ns) => description.startsWith(`${ns}.`));
}
