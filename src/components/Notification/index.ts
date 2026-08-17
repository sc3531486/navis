/**
 * Notification 模块统一导出
 *
 * 集中导出通知系统的所有公开 API、类型和组件。
 * 其他模块通过此入口访问通知系统。
 *
 * @example
 * ```tsx
 * // 导入组件
 * import { Toast, ToastContainer, NotificationCenter, NotificationItem } from '@/components/Notification'
 *
 * // 导入 Hook
 * import { useNotification, useNotificationState } from '@/components/Notification'
 *
 * // 导入类型
 * import type { Notification, NotificationLevel, NotificationChannel } from '@/components/Notification'
 * ```
 */

// ---- 组件 ----
export { default as Toast } from './Toast'
export { default as ToastContainer } from './ToastContainer'
export { default as NotificationCenter } from './NotificationCenter'
export { default as NotificationItem } from './NotificationItem'

// ---- Hooks ----
export {
  useNotification,
  useNotificationState,
  useToastTimer,
  useNotificationEventProjection,
} from './hooks'

// ---- Store（只读状态 + 操作函数） ----
export {
  notificationState,
  createNotification,
  dismissNotification,
  clearAllNotifications,
  markNotificationRead,
  markAllNotificationsRead,
  getUnreadNotifications,
  getAllNotifications,
  registerChannel,
  unregisterChannel,
} from './store'

// ---- 渠道管理 ----
export {
  registerNotificationChannel,
  unregisterNotificationChannel,
  initializeChannel,
  initializeAllChannels,
  dispatchToAllChannels,
  getRegisteredChannels,
  updateChannelFilter,
  disposeAllChannels,
} from './channel'

// ---- 类型 ----
export type {
  Notification,
  NotificationLevel,
  NotificationAction,
  CreateNotificationOptions,
  NotificationEventMap,
  NotificationEventName,
  NotificationChannel,
  NotificationPayload,
  ChannelFilterConfig,
  NotificationAPI,
} from './types'

// ---- 组件 Props 类型 ----
export type { ToastProps } from './Toast'
export type { NotificationItemProps } from './NotificationItem'
export type { NotificationCenterProps } from './NotificationCenter'
