/**
 * 通知中心单条通知组件
 *
 * 用于通知中心面板中展示单条通知。
 * 与 Toast 的区别：NotificationItem 不会自动消失，显示在持久化列表中。
 * 对应 design/25-notification.md 中的 NotificationCenter 布局。
 *
 * 样式结构：
 * ┌─────────────────────────────────────────────────────┐
 * │  [图标]  标题                        [时间]  [操作] │
 * │          详细消息                                    │
 * │          [动作按钮1] [动作按钮2]                     │
 * └─────────────────────────────────────────────────────┘
 */

import { Component, Show, For, createMemo } from 'solid-js'
import type { Notification, NotificationLevel } from './types'
import { i18nFormatRelative } from '../../i18n'

// ============================================================
// 一、Props 定义
// ============================================================

/** 通知条目组件属性 */
export interface NotificationItemProps {
  /** 通知数据 */
  notification: Notification
  /** 标记为已读的回调 */
  onMarkRead: (id: string) => void
  /** 删除通知的回调 */
  onDismiss: (id: string) => void
}

// ============================================================
// 二、级别样式映射
// ============================================================

/** 各级别通知的图标 */
const LEVEL_ICONS: Record<NotificationLevel, string> = {
  info: 'ℹ️',
  success: '✅',
  warning: '⚠️',
  error: '❌',
}

/** 各级别图标背景色（圆形底色） */
const LEVEL_ICON_BG: Record<NotificationLevel, string> = {
  info: 'bg-blue-100 text-blue-600 dark:bg-blue-900 dark:text-blue-300',
  success: 'bg-green-100 text-green-600 dark:bg-green-900 dark:text-green-300',
  warning: 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900 dark:text-yellow-300',
  error: 'bg-red-100 text-red-600 dark:bg-red-900 dark:text-red-300',
}

/** 各级别通知的左侧边框颜色 */
const LEVEL_LEFT_BORDER: Record<NotificationLevel, string> = {
  info: 'border-l-blue-400',
  success: 'border-l-green-400',
  warning: 'border-l-yellow-400',
  error: 'border-l-red-400',
}

// ============================================================
// 三、NotificationItem 组件
// ============================================================

/**
 * 通知中心单条通知组件
 *
 * 功能：
 * 1. 显示通知图标、标题、消息、时间
 * 2. 未读通知左侧有高亮边框 + 未读圆点
 * 3. 点击时自动标记为已读
 * 4. 支持动作按钮
 * 5. 支持删除操作
 *
 * @example
 * ```tsx
 * <NotificationItem
 *   notification={notif}
 *   onMarkRead={(id) => notify.markRead(id)}
 *   onDismiss={(id) => notify.dismiss(id)}
 * />
 * ```
 */
const NotificationItem: Component<NotificationItemProps> = (props) => {
  const notification = () => props.notification
  const level = () => notification().level
  const isUnread = createMemo(() => !notification().read)
  const hasActions = createMemo(() => (notification().actions?.length ?? 0) > 0)

  /**
   * 处理点击事件
   * 点击时标记为已读
   */
  function handleClick(): void {
    if (isUnread()) {
      props.onMarkRead(notification().id)
    }
  }

  return (
    <div
      class={`
        group relative
        flex items-start gap-3 p-3
        border-l-4 ${LEVEL_LEFT_BORDER[level()]}
        rounded-r-lg
        transition-colors duration-150
        hover:bg-gray-50 dark:hover:bg-gray-800/50
        cursor-pointer
        ${isUnread() ? 'bg-blue-50/30 dark:bg-blue-950/20' : 'bg-white dark:bg-gray-900'}
      `}
      onClick={handleClick}
      role="listitem"
      aria-label={`${notification().title}${isUnread() ? '（未读）' : ''}`}
    >
      {/* ---- 未读圆点指示器 ---- */}
      <Show when={isUnread()}>
        <span
          class="absolute top-3 left-1.5 w-1.5 h-1.5 rounded-full bg-blue-500"
          aria-label="未读"
        />
      </Show>

      {/* ---- 级别图标 ---- */}
      <div
        class={`
          flex-shrink-0 w-8 h-8
          flex items-center justify-center
          rounded-full text-sm
          ${LEVEL_ICON_BG[level()]}
        `}
      >
        {LEVEL_ICONS[level()]}
      </div>

      {/* ---- 内容区域 ---- */}
      <div class="flex-1 min-w-0">
        {/* 标题行 */}
        <div class="flex items-start justify-between gap-2">
          <p
            class={`
              text-sm font-medium truncate
              ${isUnread() ? 'text-gray-900 dark:text-gray-100' : 'text-gray-600 dark:text-gray-400'}
            `}
          >
            {notification().title}
          </p>

          {/* 时间 + 操作按钮 */}
          <div class="flex items-center gap-1 flex-shrink-0">
            <time
              class="text-xs text-gray-400 dark:text-gray-500 whitespace-nowrap"
              datetime={new Date(notification().timestamp).toISOString()}
              title={new Date(notification().timestamp).toLocaleString()}
            >
              {i18nFormatRelative(new Date(notification().timestamp))}
            </time>

            {/* 删除按钮（hover 时显示） */}
            <button
              type="button"
              class={`
                opacity-0 group-hover:opacity-100
                p-1 rounded
                text-gray-400 hover:text-gray-600
                dark:text-gray-500 dark:hover:text-gray-300
                transition-opacity duration-150
                focus:outline-none focus:ring-2 focus:ring-gray-300
              `}
              onClick={(e) => {
                e.stopPropagation() // 阻止冒泡到 handleClick
                props.onDismiss(notification().id)
              }}
              aria-label="删除通知"
            >
              <svg
                class="w-3.5 h-3.5"
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

        {/* 消息文本 */}
        <Show when={notification().message}>
          <p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400 line-clamp-2">
            {notification().message}
          </p>
        </Show>

        {/* 来源标签 */}
        <Show when={notification().source}>
          <span class={`
            inline-block mt-1.5 px-1.5 py-0.5
            text-[10px] font-medium
            rounded
            bg-gray-100 text-gray-500
            dark:bg-gray-800 dark:text-gray-400
          `}>
            {notification().source}
          </span>
        </Show>

        {/* 动作按钮 */}
        <Show when={hasActions()}>
          <div class="flex items-center gap-2 mt-2">
            <For each={notification().actions}>
              {(action) => (
                <button
                  type="button"
                  class={`
                    px-2.5 py-1
                    text-xs font-medium rounded
                    bg-gray-100 hover:bg-gray-200
                    dark:bg-gray-800 dark:hover:bg-gray-700
                    text-gray-700 dark:text-gray-300
                    transition-colors duration-150
                    focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-gray-300
                  `}
                  onClick={(e) => {
                    e.stopPropagation() // 阻止冒泡
                    action.handler()
                  }}
                >
                  {action.label}
                </button>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  )
}

export default NotificationItem
