import { navisCodeProductState, resetPendingStartSessionDefaults } from '@navis-code/stores/product-app';
import { agentState } from '@agent-core/stores/agent';
import {
  activeSessionId,
  createSession,
  setSessionPermissionPolicy,
  setSessionReasoningEffort,
} from '@session/stores/session-tree';
import { resolvedComposerModelSelection } from '@agent-core/stores/composer-session';

export function currentComposerModeKey(): string {
  const workMode = agentState.workMode;
  if (workMode.type === 'custom') return `custom:${workMode.runtimeId}`;
  return workMode.type;
}

export function useComposerSession() {
  async function ensureComposerSession(): Promise<string | null> {
    const current = activeSessionId();
    if (current) return current;

    const modeKey = currentComposerModeKey();
    const kind = navisCodeProductState.pendingStartKind ?? (modeKey === 'cowork' ? 'task' : 'session');
    const modelSelection = resolvedComposerModelSelection();
    const permissionPolicy = navisCodeProductState.pendingStartPermissionPolicy;
    const reasoningEffort = navisCodeProductState.pendingStartReasoningEffort;
    const sessionId = await createSession(
      modeKey,
      kind === 'task' ? 'New task' : 'New session',
      modelSelection,
    );
    const resolvedSessionId = sessionId ?? activeSessionId();
    if (!resolvedSessionId) return null;
    if (permissionPolicy) {
      await setSessionPermissionPolicy(resolvedSessionId, permissionPolicy);
    }
    if (reasoningEffort) {
      await setSessionReasoningEffort(resolvedSessionId, reasoningEffort);
    }
    resetPendingStartSessionDefaults();
    return activeSessionId() ?? resolvedSessionId;
  }

  return { ensureComposerSession };
}
