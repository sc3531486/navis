/**
 * ============================================================
 * navis-editor 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/components/Editor/
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - Editor 组件 + 子组件 + extensions + stores → 扩展 navis-editor
 * ============================================================
 */

// ── Editor 主组件 ────────────────────────────────────────
export { default as WorktreeEditor } from '@/components/Editor/WorktreeEditor';
export { default as EditorView } from '@/components/Editor/EditorView';
export { default as EditorTabs } from '@/components/Editor/EditorTabs';
export { default as DiffView } from '@/components/Editor/DiffView';
export type { EditorTypes } from '@/components/Editor/types';

// ── Editor 子组件 ────────────────────────────────────────
export { default as CompletionPanel } from '@/components/Editor/components/CompletionPanel';
export { default as DiagnosticPanel } from '@/components/Editor/components/DiagnosticPanel';
export { default as FileInput } from '@/components/Editor/components/FileInput';
export { default as FilePreview } from '@/components/Editor/components/FilePreview';
export { default as HoverTooltip } from '@/components/Editor/components/HoverTooltip';
export { default as ImageInput } from '@/components/Editor/components/ImageInput';
export { default as ImagePreview } from '@/components/Editor/components/ImagePreview';
export { default as Minimap } from '@/components/Editor/components/Minimap';
export { default as OutlinePanel } from '@/components/Editor/components/OutlinePanel';

// ── Editor 扩展 ──────────────────────────────────────────
export { diffExtension } from '@/components/Editor/extensions/diff-extension';
export { imageDropExt } from '@/components/Editor/extensions/image-drop-ext';
export { lspExtension } from '@/components/Editor/extensions/lsp-extension';
export { snippetExtension } from '@/components/Editor/extensions/snippet-extension';
export { themeExtension } from '@/components/Editor/extensions/theme-extension';

// ── Editor Stores ────────────────────────────────────────
export {
  editorState,
  setEditorState,
  openEditorFile,
  closeEditorFile,
  setActiveEditorTab,
} from '@/components/Editor/stores/editor';

export type {
  EditorState,
} from '@/components/Editor/stores/editor';

export {
  editorWorktreeState,
  setEditorWorktreeState,
} from '@/components/Editor/stores/editor-worktree';

export type {
  EditorWorktreeState,
} from '@/components/Editor/stores/editor-worktree';

export {
  editorCloseGuard,
} from '@/components/Editor/stores/editor-close-guard';

export {
  editorLifecycleGuard,
} from '@/components/Editor/stores/editor-lifecycle-guard';

export {
  editorUnsavedGuard,
} from '@/components/Editor/stores/editor-unsaved-guard';
