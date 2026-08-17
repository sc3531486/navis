import { appState } from './app';
import { normalizePermissionPolicy } from './composer-menu';
import { gatewayState, preferredGatewayDefaultModelSelection, type GatewayModelSelection } from './gateway';
import { activeSession, type ReasoningEffort } from './session-tree';

export function resolvedComposerPermissionPolicy(): string {
  const session = activeSession();
  if (session) return normalizePermissionPolicy(session.permissionPolicy);
  return normalizePermissionPolicy(appState.pendingStartPermissionPolicy);
}

export function resolvedComposerModelSelection(): GatewayModelSelection | null {
  const session = activeSession();
  if (session) {
    const providerId = session.providerId?.trim();
    const modelId = session.modelId?.trim();
    if (providerId && modelId) return { providerId, modelId };
    return preferredGatewayDefaultModelSelection(gatewayState.config);
  }

  const providerId = appState.pendingStartProviderId?.trim();
  const modelId = appState.pendingStartModelId?.trim();
  if (providerId && modelId) return { providerId, modelId };
  return preferredGatewayDefaultModelSelection(gatewayState.config);
}

export function resolvedComposerReasoningEffort(): ReasoningEffort {
  const session = activeSession();
  if (session) return session.reasoningEffort ?? 'high';
  return appState.pendingStartReasoningEffort ?? 'high';
}
