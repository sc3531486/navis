/**
 * Notification 模块类型定义
 *
 * 包含通知数据模型、事件类型、扩展渠道接口等所有 TypeScript 类型。
 * 严格遵循 design/25-notification.md 设计文档。
 */

// ============================================================
// 一、通知级别枚举
// ============================================================

/** 通知级别：信息 / 成功 / 警告 / 错误 */
export type NotificationLevel = 'info' | 'success' | 'warning' | 'error'

// ============================================================
// 二、通知动作（Toast 中的可点击按钮）
// ============================================================

/** 单条通知动作，例如「去设置」「查看详情」 */
export interface NotificationAction {
  /** 按钮显示文本 */
  label: string
  /** 点击回调 */
  handler: () => void
}

// ============================================================
// 三、通知数据模型（核心）
// ============================================================

/**
 * 通知对象
 *
 * @property id          - 唯一标识符
 * @property level       - 通知级别
 * @property title       - 标题
 * @property message     - 可选的详细消息
 * @property timestamp   - 创建时间戳（毫秒）
 * @property read        - 是否已读
 * @property persistent  - 是否持久化到通知中心（false = 仅 Toast）
 * @property duration    - Toast 持续时间（毫秒），undefined 使用默认值
 * @property actions     - 可选的动作按钮列表
 * @property source      - 来源模块名称
 */
export interface Notification {
  id: string
  level: NotificationLevel
  title: string
  message?: string
  timestamp: number
  read: boolean
  persistent: boolean
  duration?: number
  actions?: NotificationAction[]
  source?: string
}

// ============================================================
// 四、创建通知的选项（简化入参）
// ============================================================

/**
 * 创建通知时的选项
 * 内部使用，由 API 层构造后传入 store
 */
export interface CreateNotificationOptions {
  level: NotificationLevel
  title: string
  message?: string
  persistent: boolean
  duration?: number
  actions?: NotificationAction[]
  source?: string
}

// ============================================================
// 五、通知内部事件类型
// ============================================================

/** 通知系统内部事件类型（对应 design/25-notification.md 第七节） */
export type NotificationEventMap = {
  /** 发出通知 */
  'notification.show': {
    id: string
    level: NotificationLevel
    title: string
    message?: string
    persistent: boolean
    source: string
    actions?: Array<{ id: string; label: string }>
  }
  /** 关闭通知 */
  'notification.dismiss': { id: string }
  /** 清空所有通知 */
  'notification.clear': {}
  /** 未读数变化 */
  'notification.unread_count.changed': { count: number }
  /** 扩展渠道已注册 */
  'notification.channel.registered': { channelId: string; channelName: string }
  /** 扩展渠道已注销 */
  'notification.channel.unregistered': { channelId: string }
  /** 扩展渠道发送成功 */
  'notification.channel.sent': { channelId: string; level: string; title: string }
  /** 扩展渠道发送失败 */
  'notification.channel.failed': { channelId: string; error: string }
}

/** 事件名称联合类型 */
export type NotificationEventName = keyof NotificationEventMap

// ============================================================
// 六、扩展渠道接口（Extension Channel Extension）
// ============================================================

/**
 * 通知分发载荷（发送到扩展渠道的数据）
 *
 * 与 Notification 的区别：不含 read/persistent 等 UI 状态字段，
 * 仅包含发送到外部渠道所需的核心信息。
 */
export interface NotificationPayload {
  level: NotificationLevel
  title: string
  message: string
  /** 来源模块名 */
  source: string
  timestamp: number
  actions?: Array<{ id: string; label: string }>
}

/**
 * 扩展通知渠道接口
 *
 * 扩展需实现此接口以注册自定义通知渠道（如 Slack、Webhook、邮件等）。
 * 对应 design/25-notification.md「扩展扩展支持」章节。
 */
export interface NotificationChannel {
  /** 渠道唯一标识 */
  id: string
  /** 渠道显示名称 */
  name: string

  /** 初始化（扩展启用时调用，传入用户配置） */
  initialize(config: Record<string, unknown>): Promise<void>

  /** 发送通知到外部服务 */
  send(notification: NotificationPayload): Promise<void>

  /** 清理资源（扩展禁用时调用） */
  dispose(): Promise<void>
}

/**
 * 渠道过滤配置
 *
 * 用户可为每个渠道配置接收的通知级别。
 * 例如：Slack 只接收 warning + error，Webhook 接收所有级别。
 */
export interface ChannelFilterConfig {
  /** 渠道 ID */
  channelId: string
  /** 接收的通知级别，为空数组表示接收所有级别 */
  levels: NotificationLevel[]
}

// ============================================================
// 七、通知 API 接口（对应 design/25-notification.md 第四节）
// ============================================================

/**
 * 通知系统对外 API 接口
 *
 * 通过 createNotification() 工厂函数创建实例。
 */
export interface NotificationAPI {
  // ---- Toast 即时通知 ----
  /** 创建 Toast 通知，返回通知 ID */
  toast(
    level: NotificationLevel,
    title: string,
    message?: string,
    duration?: number,
  ): string

  // ---- 通知中心持久化通知 ----
  /** 创建 info 级别持久化通知 */
  info(title: string, message?: string): string
  /** 创建 success 级别持久化通知 */
  success(title: string, message?: string): string
  /** 创建 warning 级别持久化通知 */
  warning(title: string, message?: string): string
  /** 创建 error 级别持久化通知 */
  error(title: string, message?: string): string

  // ---- 通知管理 ----
  /** 关闭单条通知（Toast 移除 + 从通知中心删除） */
  dismiss(id: string): void
  /** 清空所有通知 */
  clearAll(): void
  /** 标记单条为已读 */
  markRead(id: string): void
  /** 标记所有为已读 */
  markAllRead(): void
  /** 获取未读通知列表 */
  getUnread(): Notification[]
  /** 获取所有通知列表 */
  getAll(): Notification[]

  // ---- 扩展渠道管理 ----
  /** 注册扩展通知渠道 */
  registerChannel(channel: NotificationChannel, filter?: ChannelFilterConfig): void
  /** 注销扩展通知渠道 */
  unregisterChannel(channelId: string): void

  // ---- 通知事件投影 ----
  /** 启动通知模块关心的 Tauri 投影事件监听，自动产生通知（见设计文档第五节规则） */
  start(): void
  /** 停止通知模块关心的 Tauri 投影事件监听 */
  stop(): void
}
