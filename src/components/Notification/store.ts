/**
 * Notification 状态管理
 *
 * 使用 Solid.js 的 createStore 管理通知列表、未读数等状态。
 * 所有通知操作（创建、关闭、标记已读等）通过此 store 进行。
 *
 * 严格遵循 design/25-notification.md 第四节接口定义。
 */

import { createStore, produce } from 'solid-js/store'
import type {
  Notification,
  CreateNotificationOptions,
  ChannelFilterConfig,
  NotificationChannel,
  NotificationPayload,
  NotificationLevel,
} from './types'

// ============================================================
// 一、ID 生成器
// ============================================================

/** 自增计数器，配合时间戳生成唯一 ID */
let idCounter = 0

/**
 * 生成通知唯一 ID
 * 格式：notif-{timestamp}-{counter}
 */
function generateId(): string {
  idCounter += 1
  return `notif-${Date.now()}-${idCounter}`
}

// ============================================================
// 二、去重窗口配置
// ============================================================

/**
 * 去重时间窗口（毫秒）
 * 在此时间窗口内，相同标题+级别的通知不会重复创建。
 */
const DEDUP_WINDOW_MS = 3000

/**
 * 最近通知记录，用于去重
 * key = "level:title"，value = 上次创建时间戳
 */
const recentNotifications = new Map<string, number>()

// ============================================================
// 三、Store 状态定义
// ============================================================

/** 通知 store 的响应式状态 */
interface NotificationState {
  /** 所有通知列表（按时间倒序） */
  notifications: Notification[]
  /** 当前活跃的 Toast 通知 ID 列表 */
  activeToasts: string[]
  /** 未读通知数量 */
  unreadCount: number
  /** 已注册的扩展渠道列表 */
  channels: RegisteredChannel[]
}

/** 已注册的扩展渠道信息 */
interface RegisteredChannel {
  /** 渠道实例 */
  channel: NotificationChannel
  /** 渠道过滤配置 */
  filter: ChannelFilterConfig
  /** 是否已初始化 */
  initialized: boolean
}

// ============================================================
// 四、创建 Store
// ============================================================

/** 通知中心最大存储通知数量 */
const MAX_NOTIFICATIONS = 500

/** 创建通知响应式 store */
const [state, setState] = createStore<NotificationState>({
  notifications: [],
  activeToasts: [],
  unreadCount: 0,
  channels: [],
})

// ============================================================
// 五、Store 操作函数
// ============================================================

/**
 * 检查通知是否重复
 *
 * 在 DEDUP_WINDOW_MS 时间窗口内，相同 level + title 的通知视为重复。
 *
 * @param level - 通知级别
 * @param title - 通知标题
 * @returns true 表示是重复通知，应跳过
 */
function isDuplicate(level: NotificationLevel, title: string): boolean {
  const key = `${level}:${title}`
  const lastTime = recentNotifications.get(key)
  if (lastTime && Date.now() - lastTime < DEDUP_WINDOW_MS) {
    return true
  }
  // 记录本次通知时间
  recentNotifications.set(key, Date.now())
  return false
}

/**
 * 清理过期的去重记录
 * 防止 Map 无限增长
 */
function cleanupDedupRecords(): void {
  const now = Date.now()
  for (const [key, time] of recentNotifications) {
    if (now - time > DEDUP_WINDOW_MS * 2) {
      recentNotifications.delete(key)
    }
  }
}

/**
 * 创建一条通知
 *
 * 核心函数，所有通知创建都经过此函数。
 * 负责去重、生成 ID、更新 store、分发到扩展渠道。
 *
 * @param options - 通知创建选项
 * @returns 通知 ID，若因去重被跳过则返回空字符串
 */
export function createNotification(options: CreateNotificationOptions): string {
  // 去重检查（仅对持久化通知做去重，Toast 允许快速重复以显示最新状态）
  if (options.persistent && isDuplicate(options.level, options.title)) {
    return ''
  }

  // 生成唯一 ID
  const id = generateId()

  // 构造通知对象
  const notification: Notification = {
    id,
    level: options.level,
    title: options.title,
    message: options.message,
    timestamp: Date.now(),
    read: false,
    persistent: options.persistent,
    duration: options.duration,
    actions: options.actions,
    source: options.source,
  }

  // 更新 store：插入到列表头部
  setState(
    produce((s) => {
      s.notifications.unshift(notification)

      // 超出上限时淘汰最旧的已读通知
      if (s.notifications.length > MAX_NOTIFICATIONS) {
        let lastReadIndex = -1
        for (let i = s.notifications.length - 1; i >= 0; i--) {
          if (s.notifications[i].read) {
            lastReadIndex = i
            break
          }
        }
        if (lastReadIndex !== -1) {
          s.notifications.splice(lastReadIndex, 1)
        } else {
          // 没有已读的则淘汰末尾
          s.notifications.pop()
        }
      }

      // 更新未读计数
      s.unreadCount = s.notifications.filter((n) => !n.read).length
    }),
  )

  // 分发到扩展渠道（异步，不阻塞主流程）
  if (options.persistent) {
    dispatchToChannels(notification)
  }

  // 定期清理去重记录
  cleanupDedupRecords()

  return id
}

/**
 * 添加 Toast 活跃记录
 *
 * @param id - 通知 ID
 */
export function addActiveToast(id: string): void {
  setState('activeToasts', (prev) => [...prev, id])
}

/**
 * 移除 Toast 活跃记录
 *
 * @param id - 通知 ID
 */
export function removeActiveToast(id: string): void {
  setState('activeToasts', (prev) => prev.filter((toastId) => toastId !== id))
}

/**
 * 关闭（删除）单条通知
 *
 * 同时从通知列表和活跃 Toast 中移除。
 *
 * @param id - 通知 ID
 */
export function dismissNotification(id: string): void {
  setState(
    produce((s) => {
      const index = s.notifications.findIndex((n) => n.id === id)
      if (index !== -1) {
        // 如果是未读通知，更新未读计数
        if (!s.notifications[index].read) {
          s.unreadCount = Math.max(0, s.unreadCount - 1)
        }
        s.notifications.splice(index, 1)
      }
      // 同时从活跃 Toast 中移除
      s.activeToasts = s.activeToasts.filter((toastId) => toastId !== id)
    }),
  )
}

/**
 * 清空所有通知
 */
export function clearAllNotifications(): void {
  setState({
    notifications: [],
    activeToasts: [],
    unreadCount: 0,
  })
}

/**
 * 标记单条通知为已读
 *
 * @param id - 通知 ID
 */
export function markNotificationRead(id: string): void {
  setState(
    produce((s) => {
      const notification = s.notifications.find((n) => n.id === id)
      if (notification && !notification.read) {
        notification.read = true
        s.unreadCount = Math.max(0, s.unreadCount - 1)
      }
    }),
  )
}

/**
 * 标记所有通知为已读
 */
export function markAllNotificationsRead(): void {
  setState(
    produce((s) => {
      for (const notification of s.notifications) {
        notification.read = true
      }
      s.unreadCount = 0
    }),
  )
}

/**
 * 获取未读通知列表
 *
 * @returns 未读通知数组（按时间倒序）
 */
export function getUnreadNotifications(): Notification[] {
  return state.notifications.filter((n) => !n.read)
}

/**
 * 获取所有通知列表
 *
 * @returns 全部通知数组（按时间倒序）
 */
export function getAllNotifications(): Notification[] {
  return state.notifications
}

// ============================================================
// 六、扩展渠道管理
// ============================================================

/**
 * 注册扩展通知渠道
 *
 * @param channel - 渠道实例（实现 NotificationChannel 接口）
 * @param filter  - 渠道过滤配置（接收哪些级别的通知）
 */
export function registerChannel(
  channel: NotificationChannel,
  filter?: ChannelFilterConfig,
): void {
  const defaultFilter: ChannelFilterConfig = {
    channelId: channel.id,
    levels: [], // 空数组 = 接收所有级别
  }

  setState(
    produce((s) => {
      // 防止重复注册
      const exists = s.channels.some((c) => c.channel.id === channel.id)
      if (exists) return

      s.channels.push({
        channel,
        filter: filter ?? defaultFilter,
        initialized: false,
      })
    }),
  )
}

/**
 * 注销扩展通知渠道
 *
 * @param channelId - 要注销的渠道 ID
 */
export function unregisterChannel(channelId: string): void {
  setState(
    produce((s) => {
      const index = s.channels.findIndex((c) => c.channel.id === channelId)
      if (index !== -1) {
        // 调用渠道的 dispose 清理资源
        const entry = s.channels[index]
        if (entry.initialized) {
          entry.channel.dispose().catch(() => {
            // dispose 失败不影响注销流程
          })
        }
        s.channels.splice(index, 1)
      }
    }),
  )
}

/**
 * 初始化所有已注册但未初始化的渠道
 *
 * @param configProvider - 提供渠道配置的函数（从 Config 模块读取）
 */
export async function initializeChannels(
  configProvider: (channelId: string) => Record<string, unknown>,
): Promise<void> {
  for (const entry of state.channels) {
    if (!entry.initialized) {
      try {
        const config = configProvider(entry.channel.id)
        await entry.channel.initialize(config)
        // 标记为已初始化
        setState(
          'channels',
          (c) => c.channel.id === entry.channel.id,
          'initialized',
          true,
        )
      } catch {
        // 初始化失败，记录但不阻断其他渠道
      }
    }
  }
}

// ============================================================
// 七、通知分发（Dispatch to Channels）
// ============================================================

/**
 * 将通知分发到所有已注册且匹配过滤条件的扩展渠道
 *
 * 分发是异步的、并行的，单个渠道失败不影响其他渠道。
 *
 * @param notification - 通知对象
 */
async function dispatchToChannels(notification: Notification): Promise<void> {
  const payload: NotificationPayload = {
    level: notification.level,
    title: notification.title,
    message: notification.message ?? '',
    source: notification.source ?? 'unknown',
    timestamp: notification.timestamp,
    actions: notification.actions?.map((a, i) => ({
      id: `action-${i}`,
      label: a.label,
    })),
  }

  // 并行分发到所有匹配的渠道
  const tasks = state.channels
    .filter((entry) => {
      // 渠道必须已初始化
      if (!entry.initialized) return false
      // 检查级别过滤：空数组 = 接收所有级别
      const { levels } = entry.filter
      if (levels.length === 0) return true
      return levels.includes(notification.level)
    })
    .map(async (entry) => {
      try {
        await entry.channel.send(payload)
      } catch (error) {
        const id = createNotification({
          level: 'warning',
          title: 'Notification channel failed',
          message: error instanceof Error ? error.message : String(error),
          persistent: false,
          source: 'notification.channel.failed',
        })
        if (id) {
          addActiveToast(id)
        }
      }
    })

  await Promise.allSettled(tasks)
}

// ============================================================
// 八、导出 Store 只读引用
// ============================================================

/**
 * 获取通知状态的只读引用
 *
 * 组件中通过 `state.xxx` 读取响应式数据，Solid.js 会自动追踪依赖。
 * 不可直接修改，必须通过上面导出的操作函数修改。
 */
export { state as notificationState }

// ============================================================
// 九、便捷通知方法（变更 #10 补充）
//
// 提供 .info() / .success() / .warning() / .error() 快捷方法，
// 不再保留通用的 toast() 方法（已删除）。
// ============================================================

/**
 * 发送 info 级别通知
 *
 * @param title - 通知标题
 * @param message - 通知内容（可选）
 * @param persistent - 是否持久化（默认 false，即 Toast 行为）
 * @returns 通知 ID
 */
export function info(title: string, message?: string, persistent = false): string {
  return createNotification({ level: 'info', title, message, persistent })
}

/**
 * 发送 success 级别通知
 *
 * @param title - 通知标题
 * @param message - 通知内容（可选）
 * @param persistent - 是否持久化（默认 false，即 Toast 行为）
 * @returns 通知 ID
 */
export function success(title: string, message?: string, persistent = false): string {
  return createNotification({ level: 'success', title, message, persistent })
}

/**
 * 发送 warning 级别通知
 *
 * @param title - 通知标题
 * @param message - 通知内容（可选）
 * @param persistent - 是否持久化（默认 false，即 Toast 行为）
 * @returns 通知 ID
 */
export function warning(title: string, message?: string, persistent = false): string {
  return createNotification({ level: 'warning', title, message, persistent })
}

/**
 * 发送 error 级别通知
 *
 * @param title - 通知标题
 * @param message - 通知内容（可选）
 * @param persistent - 是否持久化（默认 true，错误默认持久化）
 * @returns 通知 ID
 */
export function error(title: string, message?: string, persistent = true): string {
  return createNotification({ level: 'error', title, message, persistent })
}
