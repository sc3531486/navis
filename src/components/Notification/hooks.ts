/**
 * Notification 模块 Hooks
 *
 * 提供组件中使用通知系统的 Solid.js Hooks。
 * 封装 store 操作，提供简洁的 API 接口。
 *
 * 严格遵循 design/25-notification.md 第四节接口定义。
 */

import { createSignal, onCleanup, createEffect } from 'solid-js'
import { listen } from '@tauri-apps/api/event'
import type {
  Notification,
  NotificationLevel,
  NotificationAction,
  NotificationChannel,
  ChannelFilterConfig,
} from './types'
import {
  notificationState,
  createNotification,
  addActiveToast,
  removeActiveToast,
  dismissNotification,
  clearAllNotifications,
  markNotificationRead,
  markAllNotificationsRead,
  getUnreadNotifications,
  getAllNotifications,
  registerChannel,
  unregisterChannel,
} from './store'

// ============================================================
// 一、useNotification — 主通知 API Hook
// ============================================================

/**
 * 通知系统主 Hook
 *
 * 返回通知 API 的所有方法，供组件调用。
 * 对应 design/25-notification.md 第四节的完整接口定义。
 *
 * @example
 * ```tsx
 * const notify = useNotification()
 * notify.info('操作成功', '文件已保存')
 * notify.toast('error', '网络断开', '请检查网络连接')
 * ```
 */
export function useNotification() {
  /**
   * 创建 Toast 即时通知
   *
   * Toast 为非持久化通知，仅在界面上短暂显示后自动消失。
   *
   * @param level    - 通知级别
   * @param title    - 标题
   * @param message  - 可选详细消息
   * @param duration - 持续时间（毫秒），不传则使用级别默认值
   * @returns 通知 ID
   */
  function toast(
    level: NotificationLevel,
    title: string,
    message?: string,
    duration?: number,
  ): string {
    const id = createNotification({
      level,
      title,
      message,
      persistent: false,
      duration,
    })

    // 将 Toast 加入活跃列表
    if (id) {
      addActiveToast(id)
    }

    return id
  }

  /**
   * 创建 info 级别持久化通知
   *
   * 通知会同时出现在 Toast 和通知中心中。
   */
  function info(title: string, message?: string): string {
    return createNotification({
      level: 'info',
      title,
      message,
      persistent: true,
    })
  }

  /**
   * 创建 success 级别持久化通知
   */
  function success(title: string, message?: string): string {
    return createNotification({
      level: 'success',
      title,
      message,
      persistent: true,
    })
  }

  /**
   * 创建 warning 级别持久化通知
   */
  function warning(title: string, message?: string): string {
    return createNotification({
      level: 'warning',
      title,
      message,
      persistent: true,
    })
  }

  /**
   * 创建 error 级别持久化通知
   */
  function error(title: string, message?: string): string {
    return createNotification({
      level: 'error',
      title,
      message,
      persistent: true,
    })
  }

  /**
   * 关闭单条通知
   *
   * 同时从 Toast 和通知中心中移除。
   */
  function dismiss(id: string): void {
    removeActiveToast(id)
    dismissNotification(id)
  }

  /**
   * 清空所有通知
   */
  function clearAll(): void {
    clearAllNotifications()
  }

  /**
   * 标记单条通知为已读
   */
  function markRead(id: string): void {
    markNotificationRead(id)
  }

  /**
   * 标记所有通知为已读
   */
  function markAllRead(): void {
    markAllNotificationsRead()
  }

  /**
   * 获取未读通知列表
   */
  function getUnread(): Notification[] {
    return getUnreadNotifications()
  }

  /**
   * 获取所有通知列表
   */
  function getAll(): Notification[] {
    return getAllNotifications()
  }

  /**
   * 注册扩展通知渠道
   */
  function registerExtensionChannel(
    channel: NotificationChannel,
    filter?: ChannelFilterConfig,
  ): void {
    registerChannel(channel, filter)
  }

  /**
   * 注销扩展通知渠道
   */
  function unregisterExtensionChannel(channelId: string): void {
    unregisterChannel(channelId)
  }

  return {
    toast,
    info,
    success,
    warning,
    error,
    dismiss,
    clearAll,
    markRead,
    markAllRead,
    getUnread,
    getAll,
    registerChannel: registerExtensionChannel,
    unregisterChannel: unregisterExtensionChannel,
  }
}

// ============================================================
// 二、useNotificationState — 响应式状态 Hook
// ============================================================

/**
 * 通知响应式状态 Hook
 *
 * 提供组件中可直接读取的响应式信号。
 * Solid.js 的响应式系统会自动追踪依赖并在状态变化时更新组件。
 *
 * @example
 * ```tsx
 * const { notifications, unreadCount, activeToasts } = useNotificationState()
 * // 在 JSX 中直接使用 {unreadCount()} 即可获得自动更新
 * ```
 */
export function useNotificationState() {
  return {
    /** 所有通知列表（响应式） */
    get notifications(): Notification[] {
      return notificationState.notifications
    },
    /** 未读数量（响应式） */
    get unreadCount(): number {
      return notificationState.unreadCount
    },
    /** 活跃 Toast ID 列表（响应式） */
    get activeToasts(): string[] {
      return notificationState.activeToasts
    },
  }
}

// ============================================================
// 三、useToastTimer — Toast 自动消失计时器 Hook
// ============================================================

/** Toast 默认持续时间（毫秒） */
const TOAST_DEFAULT_DURATION: Record<NotificationLevel, number> = {
  info: 4000,
  success: 3000,
  warning: 5000,
  error: 6000,
}

/**
 * Toast 自动消失计时器 Hook
 *
 * 管理单个 Toast 的生命周期：开始计时 -> 到期后自动关闭。
 * 组件卸载时自动清理计时器。
 *
 * @param id       - 通知 ID
 * @param level    - 通知级别（用于确定默认持续时间）
 * @param duration - 自定义持续时间（可选，覆盖级别默认值）
 * @param onExpire - 到期回调（通常调用 dismiss）
 * @returns { progress } - 进度百分比（0-100），可用于进度条动画
 */
export function useToastTimer(
  id: string,
  level: NotificationLevel,
  duration: number | undefined,
  onExpire: (id: string) => void,
) {
  /** 实际使用的持续时间 */
  const effectiveDuration = duration ?? TOAST_DEFAULT_DURATION[level]
  /** 进度百分比信号 */
  const [progress, setProgress] = createSignal(100)
  /** 计时器开始时间 */
  const startTime = Date.now()

  // 使用 requestAnimationFrame 实现平滑进度更新
  let animationFrameId: number | null = null

  /** 更新进度的回调 */
  function tick(): void {
    const elapsed = Date.now() - startTime
    const remaining = Math.max(0, effectiveDuration - elapsed)
    const percent = (remaining / effectiveDuration) * 100

    setProgress(percent)

    if (remaining <= 0) {
      // 计时结束，触发关闭
      onExpire(id)
      return
    }

    // 继续下一帧
    animationFrameId = requestAnimationFrame(tick)
  }

  // 启动计时
  animationFrameId = requestAnimationFrame(tick)

  // 组件卸载时清理
  onCleanup(() => {
    if (animationFrameId !== null) {
      cancelAnimationFrame(animationFrameId)
    }
  })

  return { progress }
}

// ============================================================
// 四、useNotificationEventProjection — 通知事件投影 Hook
// ============================================================

type HostEventEnvelope = {
  id?: string
  topic?: string
  payload?: unknown
}

function projectedEventPayload(value: unknown): Record<string, unknown> {
  const envelope = value as HostEventEnvelope
  const payload = envelope?.payload
  return payload && typeof payload === 'object' && !Array.isArray(payload)
    ? payload as Record<string, unknown>
    : {}
}

/**
 * 通知事件投影 Hook
 *
 * 监听后端投影到 Tauri 的通知事件，根据 design/25-notification.md 第五节规则
 * 自动产生对应通知。
 *
 * 使用方法：在应用顶层组件中调用此 Hook 即可全局生效。
 *
 * @example
 * ```tsx
 * // 在 App.tsx 中
 * useNotificationEventProjection()
 * ```
 *
 * 注意：此 Hook 只依赖后端投影到 Tauri 的事件名。
 */
export function useNotificationEventProjection() {
  /** 存储所有投影事件监听的清理函数 */
  const cleanupFns: Array<() => void> = []

  /**
   * 启动通知事件投影监听
   *
   * 对应设计文档第五节的自动通知触发规则：
   * - 任务完成       -> Toast success
   * - 网络断开       -> Toast warning
   * - 网络恢复       -> Toast success
   * - 模型调用失败   -> Toast error + 通知中心
   * - 存储空间不足   -> Toast warning + 通知中心
   * - 新版本可用     -> 通知中心 info
   * - 安全操作被拦截 -> Toast warning
   * - MCP 连接失败  -> Toast error + 通知中心
   * - RAG 索引完成  -> Toast success
   */
  function start(): void {
    // ---- 事件处理规则映射 ----
    // key: Tauri projected event name
    // value: { level, titleFn, messageFn, persistent, toastOnly }
    const eventRules: Array<{
      eventName: string
      level: NotificationLevel
      titleFn: (payload: Record<string, unknown>) => string
      messageFn?: (payload: Record<string, unknown>) => string
      persistent: boolean
    }> = [
      // 任务完成 -> Toast success（后台任务时）
      {
        eventName: 'task.completed',
        level: 'success',
        titleFn: () => '任务完成',
        messageFn: (p) => (p.taskName as string) ?? '后台任务已完成',
        persistent: false,
      },
      // 网络断开 -> Toast warning
      {
        eventName: 'network.disconnected',
        level: 'warning',
        titleFn: () => '网络断开',
        messageFn: () => '模型请求将等待网络恢复或切换可用 Provider',
        persistent: false,
      },
      // 网络恢复 -> Toast success
      {
        eventName: 'network.reconnected',
        level: 'success',
        titleFn: () => '网络恢复',
        messageFn: () => '已重新连接到服务器',
        persistent: false,
      },
      // 模型调用失败 -> Toast error + 通知中心
      {
        eventName: 'model.call.failed',
        level: 'error',
        titleFn: () => '模型调用失败',
        messageFn: (p) => (p.error as string) ?? '请检查 API Key 配置',
        persistent: true,
      },
      // 存储空间不足 -> Toast warning + 通知中心
      {
        eventName: 'storage.low',
        level: 'warning',
        titleFn: () => '存储空间不足',
        messageFn: () => '请及时清理磁盘空间',
        persistent: true,
      },
      // 新版本可用 -> 仅通知中心 info
      {
        eventName: 'update.available',
        level: 'info',
        titleFn: (p) => `新版本 ${(p.version as string) ?? ''} 可用`,
        messageFn: () => '点击查看详情',
        persistent: true,
      },
      // 安全操作被拦截 -> Toast warning
      {
        eventName: 'security.blocked',
        level: 'warning',
        titleFn: () => '安全操作被拦截',
        messageFn: (p) => (p.reason as string) ?? '操作已被安全策略阻止',
        persistent: false,
      },
      // MCP 连接失败 -> Toast error + 通知中心
      {
        eventName: 'connection.error',
        level: 'error',
        titleFn: () => 'MCP 连接失败',
        messageFn: (p) => (p.error as string) ?? '无法连接到 MCP 服务器',
        persistent: true,
      },
      // 知识检索完成 -> Toast success
      {
        eventName: 'knowledge.search.completed',
        level: 'success',
        titleFn: () => '知识检索完成',
        messageFn: (p) => `找到 ${(p.resultCount as number) ?? 0} 条结果`,
        persistent: false,
      },
    ]

    stop()

    for (const rule of eventRules) {
      let active = true
      listen<HostEventEnvelope>(rule.eventName, (event) => {
        const payload = projectedEventPayload(event.payload)
        const id = createNotification({
          level: rule.level,
          title: rule.titleFn(payload),
          message: rule.messageFn?.(payload),
          persistent: rule.persistent,
          source: event.payload?.topic ?? rule.eventName,
        })
        if (id) {
          addActiveToast(id)
        }
      }).then((unlisten) => {
        if (active) {
          cleanupFns.push(unlisten)
        } else {
          unlisten()
        }
      })

      cleanupFns.push(() => {
        active = false
      })
    }
  }

  /**
   * 停止所有通知事件投影监听
   */
  function stop(): void {
    for (const cleanup of cleanupFns) {
      cleanup()
    }
    cleanupFns.length = 0
  }

  // 组件卸载时自动停止投影监听
  onCleanup(() => {
    stop()
  })

  return { start, stop }
}
