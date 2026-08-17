/**
 * Hotkey 全局快捷键模块 - 类型定义
 *
 * 严格遵循设计文档 design/27-hotkey.md 中的数据模型和接口定义。
 * 包含快捷键绑定、作用域、事件、API 接口等所有类型声明。
 */

// ============================================================
// 一、数据模型 - 对应设计文档 §3 数据模型
// ============================================================

/**
 * 快捷键作用域枚举
 *
 * - Global: 系统级全局快捷键，应用不在前台也能触发（仅核心模块可使用）
 * - App: 仅应用内生效，通过浏览器 keydown/keyup 事件监听
 */
export enum HotkeyScope {
  /** 系统级全局快捷键，需要操作系统级权限（如 Windows 全局热键 API） */
  Global = 'global',
  /** 仅应用内生效的快捷键 */
  App = 'app',
}

/**
 * 快捷键绑定数据结构
 *
 * 完整描述一条快捷键映射：按键组合 -> 命令，包含元数据信息。
 */
export interface HotkeyBinding {
  /** 唯一标识符，格式为 "scope:command" 或自定义 UUID */
  id: string;
  /** 按键组合字符串，如 "Ctrl+Shift+A"、"Ctrl+`" */
  keybinding: string;
  /** 快捷键作用域：系统级全局 或 仅应用内 */
  scope: HotkeyScope;
  /** 触发的命令标识，如 "commandPalette.open"、"session.create" */
  command: string;
  /** 快捷键功能的可读描述，用于帮助文档和命令面板 */
  description: string;
  /** 分类标签，如 "General"、"Agent"、"Editor"、"Terminal"、"Session" */
  category: string;
  /** 是否为用户自定义（vs 系统内置默认） */
  is_custom: boolean;
}

// ============================================================
// 二、事件类型 - 对应设计文档 §6 事件定义
// ============================================================

/**
 * 快捷键事件负载映射表
 *
 * 键为事件名称，值为该事件携带的负载数据类型。
 * 用于类型安全的事件订阅和分发。
 */
export interface HotkeyEventPayloads {
  /** 快捷键被触发时发出，携带完整绑定信息 */
  'hotkey.triggered': {
    id: string;
    keybinding: string;
    command: string;
  };
  /** 新快捷键注册成功时发出 */
  'hotkey.registered': {
    id: string;
    keybinding: string;
  };
  /** 快捷键被注销时发出 */
  'hotkey.unregistered': {
    id: string;
  };
  /** 检测到快捷键冲突时发出，包含冲突双方信息 */
  'hotkey.conflict': {
    id: string;
    keybinding: string;
    conflictWith: string;
  };
}

/** 快捷键事件名称联合类型 */
export type HotkeyEventName = keyof HotkeyEventPayloads;

// ============================================================
// 三、API 接口 - 对应设计文档 §5 接口定义
// ============================================================

/**
 * 快捷键管理器的公共 API 接口
 *
 * 完整对应设计文档定义的 6 个方法：
 * - list / register / unregister / update / checkConflict / reset
 */
export interface HotkeyManagerAPI {
  /** 获取所有已注册的快捷键列表（默认 + 自定义） */
  list(): Promise<HotkeyBinding[]>;
  /** 注册一条快捷键绑定，冲突时抛出错误 */
  register(binding: HotkeyBinding): Promise<void>;
  /** 根据 id 注销一条快捷键绑定 */
  unregister(id: string): Promise<void>;
  /** 更新指定快捷键的按键组合 */
  update(id: string, keybinding: string): Promise<void>;
  /** 检测给定按键组合是否与已有绑定冲突，返回冲突绑定或 null */
  checkConflict(keybinding: string): Promise<HotkeyBinding | null>;
  /** 重置所有快捷键为系统默认值 */
  reset(): Promise<void>;
}

// ============================================================
// 四、内部工具类型
// ============================================================

/** 键盘事件监听器回调类型 */
export type HotkeyListener = (event: KeyboardEvent) => void;

/** 快捷键触发回调函数类型 */
export type HotkeyCallback = (binding: HotkeyBinding) => void;

/**
 * 解析后的按键组合结构
 *
 * 用于内部比较和匹配，将 "Ctrl+Shift+A" 解析为结构化数据。
 */
export interface ParsedKeybinding {
  /** 是否按下 Ctrl 键 */
  ctrl: boolean;
  /** 是否按下 Shift 键 */
  shift: boolean;
  /** 是否按下 Alt 键 */
  alt: boolean;
  /** 是否按下 Meta 键（Mac 的 Cmd / Windows 的 Win） */
  meta: boolean;
  /** 主键名（规范化后的小写），如 "a"、"p"、"backquote" */
  key: string;
}
