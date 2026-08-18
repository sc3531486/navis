/**
 * Minimap 缩略图组件
 *
 * 严格遵循 design/26-editor.md 第二章"架构设计"中的 Minimap.tsx 定义。
 * 渲染编辑器右侧的代码缩略图，提供全局视图和快速定位。
 */

import { Component, For, Show, createMemo, createSignal, onCleanup, onMount } from 'solid-js'
import type { Diagnostic } from '../types'

export interface MinimapProps {
  content: string
  visibleStartLine: number
  visibleEndLine: number
  totalLines: number
  diagnostics?: Diagnostic[]
  onNavigate: (line: number) => void
  width?: number
}

const MINIMAP_LINE_HEIGHT = 2
const MINIMAP_MAX_CHARS = 80
const DIAGNOSTIC_MARK_SIZE = 3
const MAX_PREVIEW_LINES = 900

export const Minimap: Component<MinimapProps> = (props) => {
  let containerRef: HTMLDivElement | undefined

  const [isDragging, setIsDragging] = createSignal(false)
  const [containerHeight, setContainerHeight] = createSignal(0)

  const minimapWidth = () => props.width ?? 60

  const effectiveLineHeight = createMemo(() => {
    const totalLines = Math.max(props.totalLines, 1)
    const fitted = containerHeight() > 0 ? containerHeight() / totalLines : MINIMAP_LINE_HEIGHT
    return Math.max(0.75, Math.min(MINIMAP_LINE_HEIGHT, fitted))
  })

  const minimapHeight = () => props.totalLines * effectiveLineHeight()

  const viewportIndicator = () => ({
    top: props.visibleStartLine * effectiveLineHeight(),
    height: (props.visibleEndLine - props.visibleStartLine) * effectiveLineHeight(),
  })

  const previewLines = createMemo(() => {
    const lines = props.content.split('\n')
    const step = lines.length > MAX_PREVIEW_LINES ? Math.ceil(lines.length / MAX_PREVIEW_LINES) : 1

    return lines.flatMap((line, index) => (
      index % step === 0 || index === lines.length - 1
        ? [{ content: line, index }]
        : []
    ))
  })

  const getLineFromY = (clientY: number): number => {
    if (!containerRef) return 0
    const rect = containerRef.getBoundingClientRect()
    const relativeY = clientY - rect.top
    const line = Math.floor(relativeY / effectiveLineHeight())
    return Math.max(0, Math.min(line, props.totalLines - 1))
  }

  const handleClick = (event: MouseEvent) => {
    props.onNavigate(getLineFromY(event.clientY))
  }

  const handleMouseDown = (event: MouseEvent) => {
    event.preventDefault()
    setIsDragging(true)
    props.onNavigate(getLineFromY(event.clientY))
  }

  const handleMouseMove = (event: MouseEvent) => {
    if (!isDragging()) return
    props.onNavigate(getLineFromY(event.clientY))
  }

  const handleMouseUp = () => {
    setIsDragging(false)
  }

  onMount(() => {
    const syncContainerHeight = () => {
      setContainerHeight(containerRef?.clientHeight ?? 0)
    }

    syncContainerHeight()
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    const resizeObserver = typeof ResizeObserver !== 'undefined' && containerRef
      ? new ResizeObserver(() => syncContainerHeight())
      : null
    resizeObserver?.observe(containerRef!)

    onCleanup(() => {
      resizeObserver?.disconnect()
    })
  })

  onCleanup(() => {
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
  })

  return (
    <div
      ref={containerRef}
      class="relative cursor-pointer select-none overflow-hidden"
      style={{
        width: `${minimapWidth()}px`,
        height: '100%',
        background: '#fafafa',
        'border-left': '1px solid #ececec',
      }}
      onClick={handleClick}
      onMouseDown={handleMouseDown}
      role="slider"
      aria-label="代码缩略图"
      aria-valuemin={0}
      aria-valuemax={props.totalLines - 1}
      aria-valuenow={props.visibleStartLine}
    >
      <div class="absolute inset-0">
        <div class="relative opacity-35" style={{ height: `${minimapHeight()}px` }}>
          <For each={previewLines()}>
            {(line) => (
              <div
                style={{
                  position: 'absolute',
                  top: `${line.index * effectiveLineHeight()}px`,
                  left: '0',
                  height: `${Math.max(effectiveLineHeight() - 0.25, 0.5)}px`,
                  width: `${Math.min(line.content.length / MINIMAP_MAX_CHARS * 100, 100)}%`,
                  'background-color': line.content.trim().length === 0 ? 'transparent' : '#6f6f6f',
                }}
              />
            )}
          </For>
        </div>
      </div>

      <div
        class="absolute left-0 right-0 transition-all duration-100"
        style={{
          top: `${viewportIndicator().top}px`,
          height: `${Math.max(viewportIndicator().height, effectiveLineHeight())}px`,
          background: 'rgba(59, 130, 246, 0.12)',
          'border-top': '1px solid rgba(59, 130, 246, 0.22)',
          'border-bottom': '1px solid rgba(59, 130, 246, 0.22)',
        }}
      />

      <Show when={props.diagnostics && props.diagnostics.length > 0}>
        <For each={props.diagnostics}>
          {(diag) => (
            <div
              style={{
                position: 'absolute',
                top: `${diag.startLine * effectiveLineHeight()}px`,
                left: '4px',
                width: `${DIAGNOSTIC_MARK_SIZE}px`,
                height: `${DIAGNOSTIC_MARK_SIZE}px`,
                'border-radius': '999px',
                'background-color': diag.severity === 1
                  ? '#dc2626'
                  : diag.severity === 2
                    ? '#d97706'
                    : diag.severity === 3
                      ? '#2563eb'
                      : '#9ca3af',
              }}
              title={diag.message}
            />
          )}
        </For>
      </Show>
    </div>
  )
}
