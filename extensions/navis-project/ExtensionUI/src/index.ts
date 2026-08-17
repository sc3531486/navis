/**
 * ============================================================
 * navis-project 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/stores/project.ts、
 * src/components/StartWorkspace/
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - project store → 扩展 navis-project
 *   - StartWorkspace 组件 → 扩展 navis-project
 * ============================================================
 */

// ── Project Store ────────────────────────────────────────
export {
  projectState,
  setProjectState,
  setCurrentProject,
  loadRecentWorktrees,
  addRecentWorktree,
  removeRecentBoundWorktree,
  resetProjectState,
} from '@/stores/project';

export type {
  RecentWorktree,
  ProjectState,
} from '@/stores/project';

// ── StartWorkspace 组件 ─────────────────────────────────
export { StartWorkspace } from '@/components/StartWorkspace';
export { default as StartWorkspaceComponent } from '@/components/StartWorkspace/StartWorkspace';
