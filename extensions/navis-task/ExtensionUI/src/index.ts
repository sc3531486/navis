/**
 * ============================================================
 * navis-task 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/components/Plan/、
 * src/components/WorkspacePanel/、src/stores/task-projection.ts
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - Plan 组件 → 扩展 navis-task
 *   - WorkspacePanel 组件 → 扩展 navis-task
 *   - task-projection store → 扩展 navis-task
 * ============================================================
 */

// ── Plan 组件 ────────────────────────────────────────────
export { default as PlanPhaseLine } from '@/components/Plan/PlanPhaseLine';

// ── WorkspacePanel 组件 ─────────────────────────────────
export { default as BackgroundTasksPanel } from '@/components/WorkspacePanel/BackgroundTasksPanel';
export { default as BuiltinRightWorkspaceContent } from '@/components/WorkspacePanel/BuiltinRightWorkspaceContent';
export { default as DiffPanel } from '@/components/WorkspacePanel/DiffPanel';
export { default as PlanPanel } from '@/components/WorkspacePanel/PlanPanel';
export { default as SessionTranscriptPanel } from '@/components/WorkspacePanel/SessionTranscriptPanel';
export { default as ToolDiffPanel } from '@/components/WorkspacePanel/ToolDiffPanel';
export { default as WorkspacePanelFrame } from '@/components/WorkspacePanel/WorkspacePanelFrame';
export { default as WorkspacePanelShell } from '@/components/WorkspacePanel/WorkspacePanelShell';
export {
  executeRightWorkspaceMenuItem,
  getOpenRightWorkspaceCommands,
} from '@/components/WorkspacePanel/index';

// ── Task Projection Store ────────────────────────────────
export {
  taskProjectionState,
  setTaskProjectionState,
} from '@/stores/task-projection';

export type {
  TaskProjection,
  TaskProjectionState,
} from '@/stores/task-projection';
