import { navisCodeProductState } from '@navis-code/stores/product-app';
import { normalizePermissionPolicy } from './composer-menu';
import { gatewayState, preferredGatewayDefaultModelSelection, type GatewayModelSelection } from '@project-ext/stores/gateway';
import { activeSession, type ReasoningEffort } from '@session/stores/session-tree';

export function resolvedComposerPermissionPolicy(): string {
  const session = activeSession();
  if (session) return normalizePermissionPolicy(session.permissionPolicy);
  return normalizePermissionPolicy(navisCodeProductState.pendingStartPermissionPolicy);
}

export function resolvedComposerModelSelection(): GatewayModelSelection | null {
  const session = activeSession();
  if (session) {
    const providerId = session.providerId?.trim();
    const modelId = session.modelId?.trim();
    if (providerId && modelId) return { providerId, modelId };
    return preferredGatewayDefaultModelSelection(gatewayState.config);
  }

  const providerId = navisCodeProductState.pendingStartProviderId?.trim();
  const modelId = navisCodeProductState.pendingStartModelId?.trim();
  if (providerId && modelId) return { providerId, modelId };
  return preferredGatewayDefaultModelSelection(gatewayState.config);
}

export function resolvedComposerReasoningEffort(): ReasoningEffort {
  const session = activeSession();
  if (session) return session.reasoningEffort ?? 'high';
  return navisCodeProductState.pendingStartReasoningEffort ?? 'high';
}
