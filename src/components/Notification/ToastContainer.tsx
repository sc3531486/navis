/**
 * Toast 容器组件
 *
 * 管理所有活跃 Toast 的堆叠展示。
 * 固定在视口右下角，新 Toast 从底部向上堆叠。
 * 对应 design/25-notification.md 中的 Toast 堆叠管理。
 *
 * 容器职责：
 * 1. 定位在视口右下角（固定定位）
 * 2. 渲染所有活跃 Toast
 * 3. 管理 Toast 的出入场动画
 * 4. 限制最大同时显示数量（防止屏幕被淹没）
 */

import { Component, For, Show, createMemo } from 'solid-js'
import { notificationState } from './store'
import { useNotification } from './hooks'
import Toast from './Toast'

// ============================================================
// 一、配置常量
// ============================================================

/** 最大同时显示的 Toast 数量 */
const MAX_VISIBLE_TOASTS = 5

/** Toast 之间的间距（Tailwind 间距类） */
const TOAST_GAP_CLASS = 'gap-3'

// ============================================================
// 二、ToastContainer 组件
// ============================================================

/**
 * Toast 容器组件
 *
 * 在应用顶层挂载，自动管理所有 Toast 通知的展示。
 *
 * @example
 * ```tsx
 * // 在 App.tsx 中使用
 * <div>
 *   <MainContent />
 *   <ToastContainer />
 * </div>
 * ```
 */
const ToastContainer: Component = () => {
  const notify = useNotification()

  /**
   * 当前需要渲染的 Toast 列表
   *
   * 从活跃 Toast ID 列表中获取对应的通知数据。
   * 限制最多显示 MAX_VISIBLE_TOASTS 条，防止界面拥挤。
   */
  const visibleToasts = createMemo(() => {
    // 获取所有活跃 Toast 对应的通知数据
    const toasts = notificationState.activeToasts
      .map((id) =>
        notificationState.notifications.find((n) => n.id === id),
      )
      .filter(Boolean)
      .slice(0, MAX_VISIBLE_TOASTS)

    return toasts
  })

  /**
   * 是否有溢出的 Toast（超出最大显示数量的）
   *
   * 当有溢出时，显示一条简要提示。
   */
  const overflowCount = createMemo(() => {
    return Math.max(
      0,
      notificationState.activeToasts.length - MAX_VISIBLE_TOASTS,
    )
  })

  /**
   * 处理 Toast 关闭
   */
  function handleDismiss(id: string): void {
    notify.dismiss(id)
  }

  return (
    <div
      class={`
        fixed bottom-4 right-4 z-50
        flex flex-col-reverse ${TOAST_GAP_CLASS}
        pointer-events-none
      `}
      aria-label="通知区域"
      role="region"
    >
      {/* ---- 渲染可见 Toast ---- */}
      <For each={visibleToasts()}>
        {(notification) => (
          <div class="pointer-events-auto">
            <Toast
              notification={notification!}
              onDismiss={handleDismiss}
            />
          </div>
        )}
      </For>

      {/* ---- 溢出提示 ---- */}
      <Show when={overflowCount() > 0}>
        <div class="pointer-events-auto">
          <div
            class={`
              w-80 px-4 py-2 rounded-lg
              bg-gray-100 dark:bg-gray-800
              border border-gray-200 dark:border-gray-700
              text-xs text-gray-500 dark:text-gray-400
              text-center
              shadow
            `}
          >
            还有 {overflowCount()} 条通知...
          </div>
        </div>
      </Show>
    </div>
  )
}

export default ToastContainer
