/**
 * i18n 国际化模块 - 类型定义
 *
 * 本文件定义了 i18n 模块所需的全部 TypeScript 类型：
 * - 语言包结构类型（从 JSON 自动推导）
 * - 支持的语言标识类型
 * - 翻译函数签名
 * - 格式化选项类型
 * - i18n 管理器接口
 */

// ============================================================================
// 语言包结构类型定义
// ============================================================================

/**
 * 语言包的完整结构定义
 * 与 locale JSON 文件一一对应，保证类型安全
 */
export interface LocaleMessages {
  /** 通用文案（确认、取消、加载中等） */
  common: {
    ok: string;
    cancel: string;
    save: string;
    delete: string;
    loading: string;
    error: string;
    success: string;
  };

  /** 会话管理相关文案 */
  session: {
    create: string;
    delete: string;
    archive: string;
    switch: string;
    empty: string;
    /** 带插值的消息，如 "共 {count} 条消息" */
    message_count: string;
  };

  /** Agent 状态与操作相关文案 */
  agent: {
    thinking: string;
    executing: string;
    waiting: string;
    streaming: string;
    idle: string;
    error: string;
    cancel: string;
    confirm_tool: string;
  };

  /** 设置页面相关文案 */
  settings: {
    title: string;
    general: string;
    model: string;
    editor: string;
    terminal: string;
    security: string;
    extensions: string;
    about: string;
  };

  /** 通知消息文案 */
  notification: {
    model_switched: string;
    network_offline: string;
    network_online: string;
    update_available: string;
    /** 带插值的未读通知，如 "5 条未读通知" */
    unread: string;
  };

  /** 快捷键描述文案 */
  hotkey: {
    open_command_palette: string;
    new_session: string;
    abort_agent: string;
    toggle_terminal: string;
    toggle_sidebar: string;
    send_and_execute: string;
    open_file: string;
  };

  /** Worktree 相关文案 */
  worktree: {
    open: string;
    recent: string;
    close: string;
  };

  /** 命令面板相关文案 */
  command_palette: {
    placeholder: string;
    no_results: string;
  };

  /** 对话框相关文案 */
  dialog: {
    confirm: string;
    cancel: string;
    yes: string;
    no: string;
    close: string;
  };

  /** 扩展管理相关文案 */
  extension: {
    installed: string;
    enabled: string;
    disabled: string;
    loading: string;
    unloading: string;
    install: string;
    uninstall: string;
    permission_denied: string;
  };

  /** 沙箱相关文案 */
  sandbox: {
    access_denied: string;
    permission_required: string;
  };

  /** 健康监控相关文案 */
  health: {
    healthy: string;
    degraded: string;
    unhealthy: string;
    resource_warning: string;
    resource_exceeded: string;
  };

  /** 文件操作相关文案 */
  file: {
    open: string;
    save: string;
    save_as: string;
    close: string;
    unsaved_changes: string;
  };

  /** 终端相关文案 */
  terminal: {
    new: string;
    clear: string;
    kill: string;
    rename: string;
  };

  /** Git 操作相关文案 */
  git: {
    commit: string;
    push: string;
    pull: string;
    stage: string;
    unstage: string;
    diff: string;
  };

  /** 编辑器相关文案 */
  editor: {
    save: string;
    format: string;
    undo: string;
    redo: string;
    diff_confirm: string;
    diff_reject: string;
    goto_line: string;
    find: string;
    replace: string;
  };
}

// ============================================================================
// 语言标识类型
// ============================================================================

/**
 * 应用支持的语言标识联合类型
 * 所有可用语言必须在此注册
 */
export type SupportedLocale = 'zh-CN' | 'en-US';

/**
 * 所有支持的语言标识列表
 * 用于遍历和校验
 */
export const SUPPORTED_LOCALES: SupportedLocale[] = ['zh-CN', 'en-US'];

/**
 * 默认语言标识
 * 当翻译缺失时回退到此语言
 */
export const DEFAULT_LOCALE: SupportedLocale = 'en-US';

// ============================================================================
// 翻译函数类型
// ============================================================================

/**
 * 翻译函数参数类型
 * 支持插值变量（Record<string, any>）或复数计数（number）
 */
export type TranslateParams = Record<string, string | number> | number;

// ============================================================================
// 格式化选项类型
// ============================================================================

/**
 * 日期格式化预设样式
 * 对应 Intl.DateTimeFormat 的 dateStyle 选项
 */
export type DateFormatStyle = 'full' | 'long' | 'medium' | 'short';

/**
 * 日期格式化选项
 */
export interface DateFormatOptions {
  /** 预设样式 */
  style?: DateFormatStyle;
  /** 是否包含时间 */
  includeTime?: boolean;
  /** 自定义格式化模式（如 "yyyy-MM-dd HH:mm"） */
  pattern?: string;
}

/**
 * 相对时间单位
 * 对应 Intl.RelativeTimeFormat 支持的单位
 */
export type RelativeTimeUnit =
  | 'year'
  | 'month'
  | 'week'
  | 'day'
  | 'hour'
  | 'minute'
  | 'second';

// ============================================================================
// 扩展语言包类型
// ============================================================================

/**
 * 扩展语言包结构
 * 扩展通过 extension.json 声明 i18n 目录，翻译 key 以扩展 ID 为前缀
 */
export interface ExtensionLocaleMessages {
  /** 扩展翻译键值对，key 格式为 "extensionId.xxx" */
  [key: string]: string;
}

/**
 * 扩展语言包集合
 * key 为语言标识，value 为该语言的翻译内容
 */
export interface ExtensionLocaleBundle {
  [locale: string]: ExtensionLocaleMessages;
}

// ============================================================================
// i18n 管理器接口
// ============================================================================

/**
 * i18n 管理器对外暴露的接口
 * 包含语言管理、翻译函数和格式化工具
 */
export interface I18nManager {
  /** 获取当前语言标识 */
  getLocale(): SupportedLocale;

  /**
   * 设置当前语言
   * 触发响应式更新，所有使用 t() 的组件自动重新渲染
   * @param locale - 目标语言标识
   */
  setLocale(locale: SupportedLocale): Promise<void>;

  /** 获取所有可用的语言标识列表 */
  getAvailableLocales(): SupportedLocale[];

  /**
   * 格式化日期
   * @param date - 待格式化的日期对象
   * @param options - 格式化选项
   */
  formatDate(date: Date, options?: DateFormatOptions): string;

  /**
   * 格式化数字
   * @param num - 待格式化的数字
   * @param options - Intl.NumberFormat 选项
   */
  formatNumber(num: number, options?: Intl.NumberFormatOptions): string;

  /**
   * 格式化相对时间（如 "3分钟前"）
   * 基于 Intl.RelativeTimeFormat API
   * @param date - 目标日期
   */
  formatRelative(date: Date): string;

  /**
   * 注册扩展语言包
   * @param extensionId - 扩展标识
   * @param bundle - 扩展语言包集合
   */
  registerExtensionLocale(extensionId: string, bundle: ExtensionLocaleBundle): void;
}
