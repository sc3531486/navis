/**
 * i18n 国际化模块 - 入口文件
 *
 * 本文件是 i18n 模块的主入口，负责：
 * - 初始化国际化系统（加载语言包、设置默认语言）
 * - 提供响应式语言切换（基于 Solid.js createSignal）
 * - 对外暴露翻译函数 t() 和格式化工具
 * - 管理扩展语言包注册
 *
 * 使用 Solid.js 的响应式系统，当语言切换时，
 * 所有使用 t() 的组件会自动重新渲染。
 *
 * @example
 * // 在组件中使用
 * import { t, setLocale, getLocale } from '@/i18n';
 *
 * function MyComponent() {
 *   return (
 *     <div>
 *       <h1>{t('settings.title')}</h1>
 *       <button onClick={() => setLocale('en-US')}>English</button>
 *     </div>
 *   );
 * }
 */

import { createSignal, createRoot } from 'solid-js';
import type {
  SupportedLocale,
  LocaleMessages,
  ExtensionLocaleMessages,
  ExtensionLocaleBundle,
  DateFormatOptions,
  I18nManager,
} from './types';
import { SUPPORTED_LOCALES, DEFAULT_LOCALE } from './types';
import { translate } from './t';
import { formatDate, formatNumber, formatCurrency, formatRelative, isI18nKey } from './format';

// ============================================================================
// 语言包静态导入
// ============================================================================

/**
 * 静态导入内置语言包
 *
 * 虽然设计文档提到懒加载，但内置语言包体积很小（几 KB），
 * 静态导入可以避免首次使用时的异步等待，提升用户体验。
 * 扩展语言包仍通过 registerExtensionLocale 动态注册。
 */
import zhCN from './locales/zh-CN.json';
import enUS from './locales/en-US.json';

// ============================================================================
// 内置语言包映射表
// ============================================================================

/**
 * 内置语言包映射表
 * key 为语言标识，value 为对应的语言包对象
 */
const BUILTIN_MESSAGES: Record<SupportedLocale, LocaleMessages> = {
  'zh-CN': zhCN as LocaleMessages,
  'en-US': enUS as LocaleMessages,
};

// ============================================================================
// 响应式状态（Solid.js createRoot）
// ============================================================================

/**
 * 使用 createRoot 包裹响应式状态的创建
 *
 * createRoot 确保响应式上下文在模块加载时正确初始化，
 * 而不是在组件内部创建，这样 t() 可以在任何地方调用。
 */
const i18nState = createRoot(() => {
  /**
   * 当前语言标识的响应式信号
   * 初始值为 en-US（默认语言），后续会根据用户配置或浏览器语言自动调整
   */
  const [locale, setLocaleSignal] = createSignal<SupportedLocale>(DEFAULT_LOCALE);

  /**
   * 扩展语言包注册表
   * key 为扩展 ID，value 为该扩展的语言包集合
   * 使用普通对象管理，不需要响应式（扩展注册后内容不变）
   */
  const extensionBundles: Record<string, ExtensionLocaleBundle> = {};

  /**
   * 获取当前语言对应的扩展语言包合集
   * 根据当前语言标识，从各扩展的语言包集合中提取对应语言的翻译
   * 如果扩展没有当前语言的翻译，回退到扩展的默认语言（en-US）
   */
  function getExtensionMessages(): Record<string, ExtensionLocaleMessages> {
    const currentLocale = locale();
    const result: Record<string, ExtensionLocaleMessages> = {};

    for (const [extensionId, bundle] of Object.entries(extensionBundles)) {
      // 优先使用当前语言，回退到 en-US
      result[extensionId] = bundle[currentLocale] ?? bundle[DEFAULT_LOCALE] ?? {};
    }

    return result;
  }

  /**
   * 获取当前语言包
   * 回退链：当前语言 -> en-US
   */
  function getCurrentMessages(): LocaleMessages {
    return BUILTIN_MESSAGES[locale()] ?? BUILTIN_MESSAGES[DEFAULT_LOCALE];
  }

  return {
    locale,
    setLocaleSignal,
    extensionBundles,
    getExtensionMessages,
    getCurrentMessages,
  };
});

// ============================================================================
// 翻译函数
// ============================================================================

/**
 * 翻译函数 - 获取指定 key 的本地化文案
 *
 * 这是 i18n 模块的核心 API，在整个应用中广泛使用。
 * 函数内部读取 Solid.js 响应式信号，因此在组件中调用时
 * 会自动建立依赖关系，语言切换时触发组件重新渲染。
 *
 * @param key - 翻译 key，使用点号分隔命名空间和键名（如 "common.ok"）
 * @param params - 插值参数（如 { count: 42 }）或复数计数（数字）
 * @returns 翻译后的本地化字符串
 *
 * @example
 * // 基本翻译
 * t('common.ok')                    // => "确定"（zh-CN）/ "OK"（en-US）
 *
 * // 插值变量
 * t('session.message_count', { count: 42 })  // => "共 42 条消息"
 *
 * // 复数形式（数字自动映射为 {count} 插值变量）
 * t('notification.unread', 5)       // => "5 条未读通知"
 */
export function t(
  key: string,
  paramsOrCount?: Record<string, string | number> | number,
): string {
  // 读取响应式信号，建立依赖关系
  const currentLocale = i18nState.locale();
  const currentMessages = i18nState.getCurrentMessages();
  const fallbackMessages = BUILTIN_MESSAGES[DEFAULT_LOCALE];
  const extensionMessages = i18nState.getExtensionMessages();

  return translate(key, currentMessages, fallbackMessages, extensionMessages, paramsOrCount);
}

// ============================================================================
// 语言管理 API
// ============================================================================

/**
 * 获取当前语言标识
 * @returns 当前语言标识字符串（如 "zh-CN"）
 */
export function getLocale(): SupportedLocale {
  return i18nState.locale();
}

/**
 * 设置当前语言
 *
 * 触发响应式更新，所有使用 t() 的组件会自动重新渲染。
 * 同时将语言设置持久化到 localStorage，下次启动时自动恢复。
 *
 * 流程：setLocale -> 更新信号 -> 触发重渲染 -> 持久化
 *
 * @param locale - 目标语言标识
 *
 * @example
 * await setLocale('en-US');  // 切换到英文
 * await setLocale('zh-CN');  // 切换回中文
 */
export async function setLocale(locale: SupportedLocale): Promise<void> {
  // 校验语言标识是否合法
  if (!SUPPORTED_LOCALES.includes(locale)) {
    console.warn(`[i18n] Unsupported locale: "${locale}", falling back to "${DEFAULT_LOCALE}"`);
    locale = DEFAULT_LOCALE;
  }

  // 更新响应式信号，触发所有使用 t() 的组件重新渲染
  i18nState.setLocaleSignal(locale);

  // 持久化到 localStorage
  try {
    localStorage.setItem('navis-locale', locale);
  } catch {
    // localStorage 可能不可用（如隐私模式），静默忽略
  }
}

/**
 * 获取所有可用的语言标识列表
 * @returns 支持的语言标识数组
 */
export function getAvailableLocales(): SupportedLocale[] {
  return [...SUPPORTED_LOCALES];
}

// ============================================================================
// 格式化工具（透传，自动使用当前语言）
// ============================================================================

/**
 * 格式化日期为本地化字符串
 *
 * 自动使用当前语言标识，无需手动传入 locale。
 *
 * @param date - 待格式化的日期对象
 * @param options - 格式化选项
 * @returns 格式化后的日期字符串
 *
 * @example
 * formatDate(new Date(), { style: 'long', includeTime: true })
 * // zh-CN: "2024年3月15日 14:30:00"
 * // en-US: "March 15, 2024 at 2:30:00 PM"
 */
export function i18nFormatDate(date: Date, options?: DateFormatOptions): string {
  return formatDate(date, i18nState.locale(), options);
}

/**
 * 格式化数字为本地化字符串
 *
 * @param num - 待格式化的数字
 * @param options - Intl.NumberFormat 选项
 * @returns 格式化后的数字字符串
 */
export function i18nFormatNumber(num: number, options?: Intl.NumberFormatOptions): string {
  return formatNumber(num, i18nState.locale(), options);
}

/**
 * 格式化货币为本地化字符串
 *
 * @param amount - 金额
 * @param currency - 货币代码（默认 'CNY'）
 * @returns 格式化后的货币字符串
 */
export function i18nFormatCurrency(amount: number, currency?: string): string {
  return formatCurrency(amount, i18nState.locale(), currency);
}

/**
 * 格式化相对时间（如 "3分钟前"）
 *
 * 基于 Intl.RelativeTimeFormat API，根据当前 locale 自动输出
 * 对应语言的相对时间文本。
 *
 * @param date - 目标日期
 * @returns 相对时间字符串
 *
 * @example
 * i18nFormatRelative(new Date(Date.now() - 3 * 60 * 1000))
 * // zh-CN: "3分钟前"
 * // en-US: "3 minutes ago"
 */
export function i18nFormatRelative(date: Date): string {
  return formatRelative(date, i18nState.locale());
}

// ============================================================================
// 扩展语言包注册
// ============================================================================

/**
 * 注册扩展语言包
 *
 * 扩展通过此方法将其语言包注册到 i18n 系统中。
 * 注册后，扩展的翻译 key 可以通过 t() 函数访问。
 * 扩展翻译 key 以扩展 ID 为命名空间前缀，避免冲突。
 *
 * @param extensionId - 扩展标识（如 "com.example.my-extension"）
 * @param bundle - 扩展语言包集合（key 为语言标识，value 为翻译内容）
 *
 * @example
 * registerExtensionLocale('com.example.my-extension', {
 *   'zh-CN': {
 *     'com.example.my-extension.hello': '你好',
 *     'com.example.my-extension.config.title': '扩展配置'
 *   },
 *   'en-US': {
 *     'com.example.my-extension.hello': 'Hello',
 *     'com.example.my-extension.config.title': 'Extension Config'
 *   }
 * });
 *
 * // 之后可以在组件中使用
 * t('com.example.my-extension.hello')  // => "你好"
 */
export function registerExtensionLocale(extensionId: string, bundle: ExtensionLocaleBundle): void {
  i18nState.extensionBundles[extensionId] = bundle;
}

// ============================================================================
// 初始化函数
// ============================================================================

/**
 * 初始化 i18n 模块
 *
 * 应用启动时调用一次，负责：
 * 1. 从 localStorage 恢复用户上次选择的语言
 * 2. 如果没有保存的语言，尝试匹配浏览器语言
 * 3. 如果浏览器语言不支持，使用默认语言（en-US）
 *
 * @returns 初始化完成的 Promise
 *
 * @example
 * // 在应用入口调用
 * import { initI18n } from '@/i18n';
 * await initI18n();
 */
export async function initI18n(): Promise<void> {
  let targetLocale: SupportedLocale = DEFAULT_LOCALE;

  try {
    // 第一优先级：从 localStorage 恢复用户选择
    const savedLocale = localStorage.getItem('navis-locale');
    if (savedLocale && SUPPORTED_LOCALES.includes(savedLocale as SupportedLocale)) {
      targetLocale = savedLocale as SupportedLocale;
    } else {
      // 第二优先级：匹配浏览器语言
      const browserLang = navigator.language;

      // 精确匹配（如 "zh-CN"）
      if (SUPPORTED_LOCALES.includes(browserLang as SupportedLocale)) {
        targetLocale = browserLang as SupportedLocale;
      } else {
        // 前缀匹配（如 "zh" 匹配 "zh-CN"）
        const langPrefix = browserLang.split('-')[0];
        const matchedLocale = SUPPORTED_LOCALES.find((l) =>
          l.startsWith(langPrefix),
        );
        if (matchedLocale) {
          targetLocale = matchedLocale;
        }
      }
    }
  } catch {
    // localStorage 或 navigator 不可用时，使用默认语言
  }

  // 设置语言（setLocale 内部会更新信号和持久化）
  await setLocale(targetLocale);
}

// ============================================================================
// 统一导出 I18nManager 接口
// ============================================================================

/**
 * i18n 管理器对象
 *
 * 提供统一的 API 接口，便于其他模块（如 Config 模块）集成使用。
 * 对应设计文档第四章的接口定义。
 */
export const i18n: I18nManager = {
  getLocale,
  setLocale,
  getAvailableLocales,
  formatDate: i18nFormatDate,
  formatNumber: i18nFormatNumber,
  formatRelative: i18nFormatRelative,
  registerExtensionLocale,
};

// ============================================================================
// 工具函数重导出
// ============================================================================

/**
 * 重导出 isI18nKey 工具函数
 * 供 Hotkey 模块等需要判断翻译 key 的场景使用
 */
export { isI18nKey };
export { SUPPORTED_LOCALES };

/**
 * 重导出类型定义，供外部模块引用
 */
export type {
  SupportedLocale,
  LocaleMessages,
  ExtensionLocaleMessages,
  ExtensionLocaleBundle,
  DateFormatOptions,
  I18nManager,
};
