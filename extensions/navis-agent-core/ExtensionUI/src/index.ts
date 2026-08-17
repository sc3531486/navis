/**
 * ============================================================
 * navis-agent-core 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/components/Composer/、
 * src/components/AgentTimeline/、src/stores/composer-*、
 * src/stores/agent*、src/lib/agent-timeline/
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - Composer 组件 → 扩展 navis-agent-core
 *   - AgentTimeline 组件 → 扩展 navis-agent-core
 *   - composer-*/agent* stores → 扩展 navis-agent-core
 *   - agent-timeline lib → 扩展 navis-agent-core
 *   - composer CSS → 扩展 navis-agent-core
 * ============================================================
 */

// ── Composer 组件 ────────────────────────────────────────
export { Composer } from '@/components/Composer';
export { default as ComposerApprovalPanel } from '@/components/Composer/ComposerApprovalPanel';
export { default as ComposerInputShell } from '@/components/Composer/ComposerInputShell';
export { default as ComposerMenus } from '@/components/Composer/ComposerMenus';
export { default as ComposerRunStack } from '@/components/Composer/ComposerRunStack';
export { default as ComposerToolbar } from '@/components/Composer/ComposerToolbar';
export { default as ComposerWorktreeSelector } from '@/components/Composer/ComposerWorktreeSelector';
export { default as ContextRing } from '@/components/Composer/ContextRing';
export { composerInstructions } from '@/components/Composer/composer-instructions';
export { useComposerAttachments } from '@/components/Composer/useComposerAttachments';
export { useComposerPromptInput } from '@/components/Composer/useComposerPromptInput';
export { useComposerRunControls } from '@/components/Composer/useComposerRunControls';
export { useComposerSession } from '@/components/Composer/useComposerSession';
export { useComposerSubmission } from '@/components/Composer/useComposerSubmission';
export { useComposerTaskRunner } from '@/components/Composer/useComposerTaskRunner';

// ── AgentTimeline 组件 ──────────────────────────────────
export { default as AgentTimelineView } from '@/components/AgentTimeline/AgentTimelineView';
export { default as GenericToolStep } from '@/components/AgentTimeline/GenericToolStep';
export { default as SidechainToolStep } from '@/components/AgentTimeline/SidechainToolStep';
export { default as TerminalToolStep } from '@/components/AgentTimeline/TerminalToolStep';
export { default as TimelineToolLabel } from '@/components/AgentTimeline/TimelineToolLabel';
export { default as TimelineToolTarget } from '@/components/AgentTimeline/TimelineToolTarget';
export { default as TraceIcon } from '@/components/AgentTimeline/TraceIcon';
export { builtinAgentTimelineRenderers } from '@/components/AgentTimeline/builtin-agent-timeline-renderers';
export { builtinToolRenderers } from '@/components/AgentTimeline/builtin-tool-renderers';
export { toolDetail } from '@/components/AgentTimeline/tool-detail';
export { toolKind } from '@/components/AgentTimeline/tool-kind';
export { toolLabel } from '@/components/AgentTimeline/tool-label';
export { toolPath } from '@/components/AgentTimeline/tool-path';
export { toolPresentation } from '@/components/AgentTimeline/tool-presentation';
export type { ToolRecord } from '@/components/AgentTimeline/tool-record';
export { timelineFileTarget } from '@/components/AgentTimeline/timeline-file-target';
export { timelinePanelActions } from '@/components/AgentTimeline/timeline-panel-actions';

// ── Composer Stores ──────────────────────────────────────
export {
  composerInputState,
  setComposerInputState,
} from '@/stores/composer-input';

export type {
  ComposerInputState,
} from '@/stores/composer-input';

export {
  composerMenuState,
  setComposerMenuState,
} from '@/stores/composer-menu';

export type {
  ComposerMenuState,
} from '@/stores/composer-menu';

export {
  composerRunState,
  setComposerRunState,
} from '@/stores/composer-run';

export type {
  ComposerRunState,
} from '@/stores/composer-run';

export {
  composerSessionState,
  setComposerSessionState,
} from '@/stores/composer-session';

export type {
  ComposerSessionState,
} from '@/stores/composer-session';

// ── Agent Stores ─────────────────────────────────────────
export {
  agentState,
  setAgentState,
  setWorkMode,
} from '@/stores/agent';

export type {
  AgentState,
  WorkMode,
} from '@/stores/agent';

export {
  agentRuntimeStatus,
} from '@/stores/agent-runtime';

export type {
  AgentRuntimeStatus,
} from '@/stores/agent-runtime';

// ── Agent Timeline Lib ───────────────────────────────────
export {
  createAgentTimeline,
  mergeAgentTimeline,
} from '@/lib/agent-timeline';

export type {
  AgentTimeline,
  AgentTimelineEntry,
} from '@/lib/agent-timeline';

export { timelineOrder } from '@/lib/agent-timeline/timeline-order';
export { toolRendererCatalog } from '@/lib/agent-timeline/tool-renderer-catalog';
export { messageReducer } from '@/lib/agent-timeline/message-reducer';
