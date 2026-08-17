/**
 * 通知中心面板组件
 *
 * 持久化通知的管理面板，可查看所有历史通知。
 * 对应 design/25-notification.md 中的通知中心功能。
 *
 * 功能：
 * 1. 展示所有持久化通知列表
 * 2. 未读数徽章显示
 * 3. 标记全部已读
 * 4. 清空所有通知
 * 5. 级别筛选
 * 6. 空状态展示
 *
 * 布局：
 * ┌─────────────────────────────────────────┐
 * │  通知中心                    [全部已读]  │
 * ├─────────────────────────────────────────┤
 * │  [全部] [未读] [信息] [警告] [错误]     │
 * ├─────────────────────────────────────────┤
 * │  通知条目 1                              │
 * │  通知条目 2                              │
 * │  通知条目 3                              │
 * │  ...                                     │
 * ├─────────────────────────────────────────┤
 * │                  [清空所有通知]          │
 * └─────────────────────────────────────────┘
 */

import { Component, Show, For, createSignal, createMemo } from 'solid-js'
import { notificationState } from './store'
import { useNotification } from './hooks'
import NotificationItem from './NotificationItem'
import type { NotificationLevel } from './types'

// ============================================================
// 一、Props 定义
// ============================================================

/** 通知中心面板属性 */
export interface NotificationCenterProps {
  /** 面板是否可见 */
  open: boolean
  /** 关闭面板的回调 */
  onClose: () => void
}

// ============================================================
// 二、筛选标签定义
// ============================================================

/** 筛选标签类型 */
type FilterTab = 'all' | 'unread' | NotificationLevel

/** 筛选标签配置 */
const FILTER_TABS: Array<{ key: FilterTab; label: string }> = [
  { key: 'all', label: '全部' },
  { key: 'unread', label: '未读' },
  { key: 'info', label: '信息' },
  { key: 'warning', label: '警告' },
  { key: 'error', label: '错误' },
]

// ============================================================
// 三、NotificationCenter 组件
// ============================================================

/**
 * 通知中心面板组件
 *
 * @example
 * ```tsx
 * <NotificationCenter
 *   open={isNotificationCenterOpen()}
 *   onClose={() => setIsNotificationCenterOpen(false)}
 * />
 * ```
 */
const NotificationCenter: Component<NotificationCenterProps> = (props) => {
  const notify = useNotification()

  /** 当前筛选标签 */
  const [activeFilter, setActiveFilter] = createSignal<FilterTab>('all')

  /**
   * 根据当前筛选条件过滤通知列表
   *
   * - 'all'：显示所有通知
   * - 'unread'：只显示未读通知
   * - 'info' | 'warning' | 'error'：按级别筛选
   */
  const filteredNotifications = createMemo(() => {
    const filter = activeFilter()
    const notifications = notificationState.notifications

    switch (filter) {
      case 'all':
        return notifications
      case 'unread':
        return notifications.filter((n) => !n.read)
      default:
        // 按级别筛选（filter 即 NotificationLevel）
        return notifications.filter((n) => n.level === filter)
    }
  })

  /**
   * 是否有通知
   */
  const hasNotifications = createMemo(
    () => notificationState.notifications.length > 0,
  )

  /**
   * 处理标记全部已读
   */
  function handleMarkAllRead(): void {
    notify.markAllRead()
  }

  /**
   * 处理清空所有通知
   */
  function handleClearAll(): void {
    notify.clearAll()
  }

  /**
   * 处理通知关闭
   */
  function handleDismiss(id: string): void {
    notify.dismiss(id)
  }

  /**
   * 处理标记已读
   */
  function handleMarkRead(id: string): void {
    notify.markRead(id)
  }

  return (
    <Show when={props.open}>
      {/* ---- 遮罩层（点击关闭面板） ---- */}
      <div
        class="fixed inset-0 z-40"
        onClick={props.onClose}
        aria-hidden="true"
      />

      {/* ---- 面板主体 ---- */}
      <div
        class={`
          fixed top-12 right-4 z-50
          w-96 max-h-[70vh]
          bg-white dark:bg-gray-900
          rounded-xl shadow-2xl
          border border-gray-200 dark:border-gray-700
          flex flex-col
          overflow-hidden
        `}
        role="dialog"
        aria-label="通知中心"
      >
        {/* ---- 头部：标题 + 操作按钮 ---- */}
        <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
          {/* 标题 + 未读数 */}
          <div class="flex items-center gap-2">
            <h2 class="text-sm font-semibold text-gray-900 dark:text-gray-100">
              通知中心
            </h2>

            {/* 未读数徽章 */}
            <Show when={notificationState.unreadCount > 0}>
              <span
                class={`
                  inline-flex items-center justify-center
                  min-w-[1.25rem] h-5 px-1.5
                  text-[10px] font-bold
                  text-white bg-red-500
                  rounded-full
                `}
              >
                {notificationState.unreadCount > 99
                  ? '99+'
                  : notificationState.unreadCount}
              </span>
            </Show>
          </div>

          {/* 操作按钮组 */}
          <div class="flex items-center gap-1">
            {/* 全部已读按钮 */}
            <Show when={notificationState.unreadCount > 0}>
              <button
                type="button"
                class={`
                  px-2 py-1 text-xs
                  text-blue-600 hover:text-blue-800
                  dark:text-blue-400 dark:hover:text-blue-200
                  rounded hover:bg-blue-50 dark:hover:bg-blue-900/30
                  transition-colors duration-150
                  focus:outline-none focus:ring-2 focus:ring-blue-300
                `}
                onClick={handleMarkAllRead}
              >
                全部已读
              </button>
            </Show>

            {/* 关闭按钮 */}
            <button
              type="button"
              class={`
                p-1 rounded
                text-gray-400 hover:text-gray-600
                dark:text-gray-500 dark:hover:text-gray-300
                transition-colors duration-150
                focus:outline-none focus:ring-2 focus:ring-gray-300
              `}
              onClick={props.onClose}
              aria-label="关闭通知中心"
            >
              <svg
                class="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </div>

        {/* ---- 筛选标签栏 ---- */}
        <div class="flex items-center gap-1 px-4 py-2 border-b border-gray-100 dark:border-gray-800">
          <For each={FILTER_TABS}>
            {(tab) => (
              <button
                type="button"
                class={`
                  px-2.5 py-1 text-xs font-medium rounded-full
                  transition-colors duration-150
                  focus:outline-none focus:ring-2 focus:ring-blue-300
                  ${
                    activeFilter() === tab.key
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                      : 'text-gray-500 hover:text-gray-700 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-gray-200 dark:hover:bg-gray-800'
                  }
                `}
                onClick={() => setActiveFilter(tab.key)}
              >
                {tab.label}
              </button>
            )}
          </For>
        </div>

        {/* ---- 通知列表 ---- */}
        <div class="flex-1 overflow-y-auto overscroll-contain">
          <Show
            when={filteredNotifications().length > 0}
            fallback={
              /* ---- 空状态 ---- */
              <div class="flex flex-col items-center justify-center py-12 text-gray-400 dark:text-gray-500">
                <svg
                  class="w-12 h-12 mb-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  aria-hidden="true"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="1.5"
                    d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
                  />
                </svg>
                <p class="text-sm">
                  {hasNotifications() ? '没有符合条件的通知' : '暂无通知'}
                </p>
              </div>
            }
          >
            <div class="divide-y divide-gray-100 dark:divide-gray-800" role="list">
              <For each={filteredNotifications()}>
                {(notification) => (
                  <NotificationItem
                    notification={notification}
                    onDismiss={handleDismiss}
                    onMarkRead={handleMarkRead}
                  />
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* ---- 底部：清空操作 ---- */}
        <Show when={hasNotifications()}>
          <div class="px-4 py-2.5 border-t border-gray-200 dark:border-gray-700">
            <button
              type="button"
              class={`
                w-full px-3 py-1.5
                text-xs font-medium text-center
                text-gray-500 hover:text-red-600
                dark:text-gray-400 dark:hover:text-red-400
                rounded
                hover:bg-red-50 dark:hover:bg-red-900/20
                transition-colors duration-150
                focus:outline-none focus:ring-2 focus:ring-red-300
              `}
              onClick={handleClearAll}
            >
              清空所有通知
            </button>
          </div>
        </Show>
      </div>
    </Show>
  )
}

export default NotificationCenter
