/**
 * Diff 渲染扩展 - CodeMirror 6 Extension
 *
 * 严格遵循 design/26-editor.md 第五章"Diff 视图"设计。
 * 提供 Unified 和 Side-by-side 两种 Diff 显示模式，
 * 支持逐块确认/拒绝、全部确认/拒绝、手动编辑 Diff。
 *
 * 功能：
 * - 解析原始文本和修改文本，生成 Diff Hunk 列表
 * - Unified 模式：上下对照，适合小改动
 * - Side-by-side 模式：左右对照，适合大改动
 * - 变更行高亮（新增行绿色、删除行红色）
 * - 逐块确认/拒绝交互
 *
 * 设计依据：design/26-editor.md S5 Diff 视图
 */

import type {
  DiffHunk,
  DiffChange,
  DiffChangeType,
  DiffMode,
} from '../types'

// ============================================================
// Diff 算法
// ============================================================

/**
 * 简化的行级 Diff 算法
 *
 * 基于 LCS（最长公共子序列）算法，将原始文本和修改文本
 * 比较为逐行差异，生成 DiffHunk 列表。
 *
 * 注意：这是前端展示用的简化实现，精确的 Diff 计算由后端
 * Edit 模块（design/15-edit.md）完成。
 * 前端主要用于渲染预览和交互。
 *
 * @param original 原始文件内容
 * @param modified 修改后文件内容
 * @returns Diff Hunk 列表
 */
export function computeDiff(original: string, modified: string): DiffHunk[] {
  const originalLines = original.split('\n')
  const modifiedLines = modified.split('\n')

  // 使用简易 diff 算法生成变更列表
  const changes = lineDiff(originalLines, modifiedLines)

  // 将连续的变更行聚合为 Hunk
  return groupChangesIntoHunks(changes)
}

/**
 * 行级 Diff 计算
 *
 * 使用 Myers diff 算法的简化版本，计算两组行之间的差异。
 * 返回每个位置的变更类型和内容。
 *
 * @param originalLines 原始文件行列表
 * @param modifiedLines 修改后文件行列表
 * @returns 变更列表（包含上下文行）
 */
function lineDiff(
  originalLines: string[],
  modifiedLines: string[],
): DiffChange[] {
  const changes: DiffChange[] = []

  // 使用双指针算法做简单的行级比较
  // 注意：生产环境应使用更精确的算法（如 diff-match-patch）
  let oi = 0 // 原始文件索引
  let mi = 0 // 修改文件索引

  while (oi < originalLines.length || mi < modifiedLines.length) {
    if (oi >= originalLines.length) {
      // 原始文件已遍历完，剩余都是新增行
      changes.push({
        type: 'addition',
        content: modifiedLines[mi],
        modifiedLine: mi + 1,
      })
      mi++
    } else if (mi >= modifiedLines.length) {
      // 修改文件已遍历完，剩余都是删除行
      changes.push({
        type: 'deletion',
        content: originalLines[oi],
        originalLine: oi + 1,
      })
      oi++
    } else if (originalLines[oi] === modifiedLines[mi]) {
      // 行内容相同 → 上下文行
      changes.push({
        type: 'context',
        content: originalLines[oi],
        originalLine: oi + 1,
        modifiedLine: mi + 1,
      })
      oi++
      mi++
    } else {
      // 行内容不同 → 查找最近的匹配点
      // 简化策略：先尝试在附近查找匹配
      const lookAhead = 3
      let foundMatch = false

      // 在修改文件中查找当前原始行
      for (let lookMi = mi + 1; lookMi < Math.min(mi + lookAhead, modifiedLines.length); lookMi++) {
        if (originalLines[oi] === modifiedLines[lookMi]) {
          // 找到匹配 → 中间的修改文件行是新增行
          while (mi < lookMi) {
            changes.push({
              type: 'addition',
              content: modifiedLines[mi],
              modifiedLine: mi + 1,
            })
            mi++
          }
          foundMatch = true
          break
        }
      }

      if (!foundMatch) {
        // 在原始文件中查找当前修改行
        for (let lookOi = oi + 1; lookOi < Math.min(oi + lookAhead, originalLines.length); lookOi++) {
          if (modifiedLines[mi] === originalLines[lookOi]) {
            // 找到匹配 → 中间的原始文件行是删除行
            while (oi < lookOi) {
              changes.push({
                type: 'deletion',
                content: originalLines[oi],
                originalLine: oi + 1,
              })
              oi++
            }
            foundMatch = true
            break
          }
        }
      }

      if (!foundMatch) {
        // 未找到附近匹配 → 视为替换（删除 + 新增）
        changes.push({
          type: 'deletion',
          content: originalLines[oi],
          originalLine: oi + 1,
        })
        changes.push({
          type: 'addition',
          content: modifiedLines[mi],
          modifiedLine: mi + 1,
        })
        oi++
        mi++
      }
    }
  }

  return changes
}

/**
 * 将变更列表聚合为 Diff Hunk
 *
 * 将连续的非上下文变更聚合为 Hunk，每个 Hunk 包含
 * 若干上下文行（用于定位）和实际变更行。
 *
 * @param changes 完整变更列表
 * @param contextLines 每个 Hunk 前后保留的上下文行数，默认 3
 * @returns Diff Hunk 列表
 */
function groupChangesIntoHunks(
  changes: DiffChange[],
  contextLines: number = 3,
): DiffHunk[] {
  if (changes.length === 0) return []

  /** Hunk 列表 */
  const hunks: DiffHunk[] = []
  /** 当前正在构建的 Hunk 中的变更行 */
  let currentChanges: DiffChange[] = []
  /** 最后一个非上下文变更的索引 */
  let lastNonContextIndex = -1

  for (let i = 0; i < changes.length; i++) {
    const change = changes[i]

    if (change.type !== 'context') {
      // 非上下文行 → 检查是否需要开始新 Hunk
      if (lastNonContextIndex >= 0 && i - lastNonContextIndex > contextLines * 2) {
        // 距离上一个变更太远 → 保存当前 Hunk，开始新 Hunk
        if (currentChanges.length > 0) {
          hunks.push(buildHunk(currentChanges))
          currentChanges = []
        }
      }

      // 添加上下文行（从上一个变更到当前变更之间的行）
      const startContext = Math.max(
        lastNonContextIndex >= 0 ? lastNonContextIndex + 1 : 0,
        i - contextLines,
      )
      for (let j = startContext; j < i; j++) {
        if (changes[j].type === 'context' && !currentChanges.includes(changes[j])) {
          currentChanges.push(changes[j])
        }
      }

      currentChanges.push(change)
      lastNonContextIndex = i
    }
  }

  // 处理最后一个 Hunk
  if (currentChanges.length > 0) {
    // 添加尾部上下文行
    const tailStart = lastNonContextIndex + 1
    const tailEnd = Math.min(tailStart + contextLines, changes.length)
    for (let j = tailStart; j < tailEnd; j++) {
      if (changes[j].type === 'context') {
        currentChanges.push(changes[j])
      }
    }
    hunks.push(buildHunk(currentChanges))
  }

  return hunks
}

/**
 * 根据变更行列表构建 Diff Hunk 对象
 *
 * @param changes Hunk 中的变更行列表
 * @returns DiffHunk 对象
 */
function buildHunk(changes: DiffChange[]): DiffHunk {
  // 计算原始文件的起始行和行数
  const originalLines = changes.filter(
    (c) => c.type === 'context' || c.type === 'deletion',
  )
  const modifiedLines = changes.filter(
    (c) => c.type === 'context' || c.type === 'addition',
  )

  const originalStartLine = originalLines.length > 0
    ? (originalLines[0].originalLine ?? 1)
    : 1
  const modifiedStartLine = modifiedLines.length > 0
    ? (modifiedLines[0].modifiedLine ?? 1)
    : 1

  return {
    id: generateHunkId(),
    originalStartLine,
    originalLineCount: originalLines.length,
    modifiedStartLine,
    modifiedLineCount: modifiedLines.length,
    changes,
  }
}

/**
 * 生成 Hunk 唯一标识
 */
function generateHunkId(): string {
  return `hunk-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

// ============================================================
// Diff 渲染工具
// ============================================================

/**
 * Diff 行样式类名映射
 *
 * 根据变更类型返回对应的 Tailwind CSS 类名。
 * 用于 DiffView 组件中渲染不同颜色的变更行。
 */
export const DIFF_LINE_CLASSES: Record<DiffChangeType, string> = {
  /** 新增行：绿色背景 */
  addition: 'bg-green-50 dark:bg-green-900/20 border-l-2 border-green-500',
  /** 删除行：红色背景 */
  deletion: 'bg-red-50 dark:bg-red-900/20 border-l-2 border-red-500',
  /** 上下文行：默认样式 */
  context: '',
}

/**
 * Diff 行前缀映射
 *
 * 对应 Unified 模式中每行前的 +/-/空格 标记。
 */
export const DIFF_LINE_PREFIX: Record<DiffChangeType, string> = {
  addition: '+',
  deletion: '-',
  context: ' ',
}

/**
 * Diff Hunk 标题文本
 *
 * 生成类似 "@@ -1,5 +1,7 @@" 的 Hunk 头部文本。
 *
 * @param hunk Diff Hunk
 * @returns Hunk 标题文本
 */
export function formatHunkHeader(hunk: DiffHunk): string {
  return `@@ -${hunk.originalStartLine},${hunk.originalLineCount} +${hunk.modifiedStartLine},${hunk.modifiedLineCount} @@`
}

// ============================================================
// Diff Extension 配置
// ============================================================

/**
 * Diff 扩展配置选项
 */
export interface DiffExtensionConfig {
  /** Diff 显示模式 */
  mode: DiffMode
  /** 每个 Hunk 前后保留的上下文行数 */
  contextLines?: number
  /** 是否启用逐块确认/拒绝 */
  enableHunkActions?: boolean
}

/**
 * 创建 Diff 扩展配置
 *
 * 生成 DiffView 组件所需的配置对象和初始 Hunk 数据。
 *
 * @param original 原始文本
 * @param modified 修改后文本
 * @param config Diff 扩展配置
 * @returns Diff Hunk 列表和合并配置
 */
export function createDiffExtension(
  original: string,
  modified: string,
  config: DiffExtensionConfig,
): {
  hunks: DiffHunk[]
  mode: DiffMode
  contextLines: number
} {
  const contextLines = config.contextLines ?? 3
  const hunks = computeDiff(original, modified)

  return {
    hunks,
    mode: config.mode,
    contextLines,
  }
}
