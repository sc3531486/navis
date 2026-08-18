/**
 * HoverTooltip 悬停提示组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 HoverTooltip.tsx 定义。
 * 渲染 LSP 悬停信息的浮动提示框，包含类型签名、文档说明等。
 *
 * 功能：
 * - 类型信息展示（Markdown 格式渲染）
 * - 语法高亮代码块
 * - 浮动定位（跟随鼠标/光标位置）
 * - 自动尺寸调整（避免超出视口）
 *
 * 设计依据：design/26-editor.md S4 LSP 集成 - 悬停链路
 */

import { Component, Show } from 'solid-js'
import type { HoverInfo } from '../types'

// ============================================================
// 类型定义
// ============================================================

/**
 * HoverTooltip 组件 Props
 */
export interface HoverTooltipProps {
  /** 悬停信息（null 表示不显示） */
  info: HoverInfo | null
  /** 提示框在视口中的定位坐标 */
  position: { x: number; y: number }
  /** 是否显示 */
  visible: boolean
}

// ============================================================
// HoverTooltip 组件
// ============================================================

/**
 * 悬停提示组件
 *
 * 在编辑器光标附近浮动显示 LSP 悬停信息。
 * 支持 Markdown 格式的内容渲染（简化实现：保留换行和代码块）。
 *
 * @example
 * ```tsx
 * <HoverTooltip
 *   info={hoverInfo}
 *   position={{ x: mouseX, y: mouseY }}
 *   visible={showHover}
 * />
 * ```
 */
export const HoverTooltip: Component<HoverTooltipProps> = (props) => {
  // ---- 定位计算 ----

  /**
   * 计算提示框位置
   *
   * 确保提示框不会超出视口边界。
   * - 默认显示在光标下方偏移 8px
   * - 如果下方空间不足，显示在光标上方
   * - 如果右侧空间不足，左移
   */
  const tooltipStyle = () => {
    const offset = 8 // 距离光标的偏移量
    const viewportWidth = window.innerWidth
    const viewportHeight = window.innerHeight

    let x = props.position.x
    let y = props.position.y + offset

    // 防止超出右侧边界（假设提示框宽度约 400px）
    if (x + 400 > viewportWidth) {
      x = viewportWidth - 400 - 8
    }

    // 防止超出底部边界（假设提示框高度约 200px）
    if (y + 200 > viewportHeight) {
      // 显示在光标上方
      y = props.position.y - 200 - offset
    }

    return {
      left: `${Math.max(0, x)}px`,
      top: `${Math.max(0, y)}px`,
    }
  }

  // ---- 渲染 ----

  return (
    <Show when={props.visible && props.info !== null}>
      <div
        class="fixed z-50 max-w-[400px] max-h-[300px] overflow-auto rounded-md shadow-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-3"
        style={tooltipStyle()}
        role="tooltip"
      >
        {/* 语言标识（如果有） */}
        <Show when={props.info!.language}>
          <div class="text-xs text-gray-400 dark:text-gray-500 mb-1 font-mono">
            {props.info!.language}
          </div>
        </Show>

        {/* 内容区域 */}
        <div class="text-sm text-gray-800 dark:text-gray-200 leading-relaxed font-mono whitespace-pre-wrap">
          {props.info!.contents}
        </div>
      </div>
    </Show>
  )
}
