// CSS imports
import '../styles/composer/index.css';
import '../styles/search-surface.css';

// navis-agent-core Extension UI

export { Composer } from '@agent-core/components/Composer';
export { default as ComposerApprovalPanel } from '@agent-core/components/Composer/ComposerApprovalPanel';
export { default as ComposerInputShell } from '@agent-core/components/Composer/ComposerInputShell';
export { default as ComposerRunStack } from '@agent-core/components/Composer/ComposerRunStack';
export { default as ComposerToolbar } from '@agent-core/components/Composer/ComposerToolbar';
export { default as ComposerWorktreeSelector } from '@agent-core/components/Composer/ComposerWorktreeSelector';
export { default as ContextRing } from '@agent-core/components/Composer/ContextRing';
export { useComposerAttachments } from '@agent-core/components/Composer/useComposerAttachments';
export { useComposerRunControls } from '@agent-core/components/Composer/useComposerRunControls';
export { useComposerSession } from '@agent-core/components/Composer/useComposerSession';
export { useComposerSubmission } from '@agent-core/components/Composer/useComposerSubmission';
export { useComposerTaskRunner } from '@agent-core/components/Composer/useComposerTaskRunner';

export { agentState, setAgentState, setWorkMode } from '@agent-core/stores/agent';
export type { AgentState, WorkMode } from '@agent-core/stores/agent';
export { agentRuntimeStatus } from '@agent-core/stores/agent-runtime';
export type { AgentRuntimeStatus } from '@agent-core/stores/agent-runtime';

export { mergeAgentTimelinePart } from '@agent-core/lib/agent-timeline';
export type { AgentTimelineItem } from '@agent-core/lib/agent-timeline';
