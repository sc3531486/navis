/**
 * Toast 单条通知组件
 *
 * 显示一条即时通知（Toast），支持自动消失、手动关闭、动作按钮。
 * 对应 design/25-notification.md 第六节的 Toast 样式规范。
 *
 * 布局结构：
 * ┌──────────────────────────────────────┐
 * │  [图标] 标题                          │
 * │         详细消息                       │
 * │                    [动作按钮] [关闭]   │
 * └──────────────────────────────────────┘
 */

import { Component, Show, For, createMemo } from 'solid-js'
import type { Notification, NotificationLevel } from './types'
import { useToastTimer } from './hooks'

// ============================================================
// 一、Props 定义
// ============================================================

/** Toast 组件属性 */
export interface ToastProps {
  /** 通知数据 */
  notification: Notification
  /** 关闭回调 */
  onDismiss: (id: string) => void
  /** 点击动作按钮的回调（已封装在 notification.actions[].handler 中） */
}

// ============================================================
// 二、级别样式映射
// ============================================================

/** 各级别通知的图标（Unicode emoji，后续可替换为 Icon 组件） */
const LEVEL_ICONS: Record<NotificationLevel, string> = {
  info: 'ℹ️',    // ℹ️
  success: '✅',        // ✅
  warning: '⚠️',  // ⚠️
  error: '❌',          // ❌
}

/** 各级别通知的背景色 Tailwind 类 */
const LEVEL_BG_COLORS: Record<NotificationLevel, string> = {
  info: 'bg-blue-50 border-blue-200 dark:bg-blue-950 dark:border-blue-800',
  success: 'bg-green-50 border-green-200 dark:bg-green-950 dark:border-green-800',
  warning: 'bg-yellow-50 border-yellow-200 dark:bg-yellow-950 dark:border-yellow-800',
  error: 'bg-red-50 border-red-200 dark:bg-red-950 dark:border-red-800',
}

/** 各级别通知的标题文字颜色 */
const LEVEL_TITLE_COLORS: Record<NotificationLevel, string> = {
  info: 'text-blue-800 dark:text-blue-200',
  success: 'text-green-800 dark:text-green-200',
  warning: 'text-yellow-800 dark:text-yellow-200',
  error: 'text-red-800 dark:text-red-200',
}

/** 各级别通知的消息文字颜色 */
const LEVEL_MESSAGE_COLORS: Record<NotificationLevel, string> = {
  info: 'text-blue-600 dark:text-blue-300',
  success: 'text-green-600 dark:text-green-300',
  warning: 'text-yellow-600 dark:text-yellow-300',
  error: 'text-red-600 dark:text-red-300',
}

/** 进度条颜色 */
const LEVEL_PROGRESS_COLORS: Record<NotificationLevel, string> = {
  info: 'bg-blue-400 dark:bg-blue-500',
  success: 'bg-green-400 dark:bg-green-500',
  warning: 'bg-yellow-400 dark:bg-yellow-500',
  error: 'bg-red-400 dark:bg-red-500',
}

// ============================================================
// 三、Toast 组件
// ============================================================

/**
 * Toast 单条通知组件
 *
 * 功能：
 * 1. 显示通知图标、标题、可选消息
 * 2. 支持可选的动作按钮（如「去设置」）
 * 3. 自动消失（通过 useToastTimer 控制）
 * 4. 手动关闭按钮
 * 5. 底部进度条动画（显示剩余时间）
 *
 * @example
 * ```tsx
 * <Toast
 *   notification={notificationData}
 *   onDismiss={(id) => notify.dismiss(id)}
 * />
 * ```
 */
const Toast: Component<ToastProps> = (props) => {
  // 解构常用属性，避免重复访问
  const notification = () => props.notification
  const level = () => notification().level
  const id = () => notification().id

  // 计时器：自动消失
  const { progress } = useToastTimer(
    id(),
    level(),
    notification().duration,
    props.onDismiss,
  )

  // 是否有动作按钮
  const hasActions = createMemo(() => (notification().actions?.length ?? 0) > 0)

  return (
    <div
      class={`
        relative overflow-hidden
        w-80 rounded-lg border shadow-lg
        transform transition-all duration-300 ease-in-out
        hover:shadow-xl
        ${LEVEL_BG_COLORS[level()]}
      `}
      role="alert"
      aria-live={level() === 'error' ? 'assertive' : 'polite'}
    >
      {/* ---- 主体内容区域 ---- */}
      <div class="flex items-start gap-3 p-4">
        {/* 图标 */}
        <span class="flex-shrink-0 text-lg leading-none mt-0.5">
          {LEVEL_ICONS[level()]}
        </span>

        {/* 文本内容 */}
        <div class="flex-1 min-w-0">
          {/* 标题 */}
          <p class={`text-sm font-semibold ${LEVEL_TITLE_COLORS[level()]}`}>
            {notification().title}
          </p>

          {/* 详细消息（可选） */}
          <Show when={notification().message}>
            <p class={`mt-1 text-xs ${LEVEL_MESSAGE_COLORS[level()]}`}>
              {notification().message}
            </p>
          </Show>
        </div>

        {/* 关闭按钮 */}
        <button
          type="button"
          class={`
            flex-shrink-0 ml-2 p-1 rounded
            text-gray-400 hover:text-gray-600
            dark:text-gray-500 dark:hover:text-gray-300
            transition-colors duration-150
            focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-gray-300
          `}
          onClick={() => props.onDismiss(id())}
          aria-label="关闭通知"
        >
          {/* 关闭图标（X） */}
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

      {/* ---- 动作按钮区域（可选） ---- */}
      <Show when={hasActions()}>
        <div class="flex items-center gap-2 px-4 pb-3">
          {/* 占位，将动作按钮推到右侧 */}
          <div class="flex-1" />

          <For each={notification().actions}>
            {(action) => (
              <button
                type="button"
                class={`
                  px-3 py-1 text-xs font-medium rounded
                  transition-colors duration-150
                  ${LEVEL_TITLE_COLORS[level()]}
                  hover:bg-black/5 dark:hover:bg-white/10
                  focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-gray-300
                `}
                onClick={() => action.handler()}
              >
                {action.label}
              </button>
            )}
          </For>
        </div>
      </Show>

      {/* ---- 底部进度条（倒计时动画） ---- */}
      <div class="absolute bottom-0 left-0 right-0 h-0.5 bg-gray-200/50 dark:bg-gray-700/50">
        <div
          class={`h-full transition-none ${LEVEL_PROGRESS_COLORS[level()]}`}
          style={{ width: `${progress()}%` }}
          role="progressbar"
          aria-valuenow={Math.round(progress())}
          aria-valuemin={0}
          aria-valuemax={100}
        />
      </div>
    </div>
  )
}

export default Toast
