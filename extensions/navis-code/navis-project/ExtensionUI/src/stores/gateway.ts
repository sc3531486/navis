import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';

export interface GatewayProvider {
  id: string;
  providerType: string;
  name: string;
  baseUrl: string;
  defaultModel: string;
  modelCount: number;
}

export interface GatewayModel {
  id: string;
  providerId: string;
  name: string;
  contextWindow: number;
  maxOutputTokens: number;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsMultimodal: boolean;
  supportsReasoningEffort: boolean;
  supportsStructuredOutput: boolean;
  supportsUsage: boolean;
  defaultReasoningEffort: GatewayReasoningEffort;
  apiProtocol: GatewayApiProtocol;
  costPer1kInput: number;
  costPer1kOutput: number;
}

export interface GatewayModelSelection {
  providerId: string;
  modelId: string;
}

export type GatewayApiProtocol = string | Record<string, string>;
export type GatewayReasoningEffort = 'low' | 'medium' | 'high' | 'extra-high' | 'max';

export interface GatewayProtocolCatalog {
  id: GatewayApiProtocol;
  runtimeId: string;
  label: string;
  description: string;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsMultimodal: boolean;
  supportsReasoningEffort: boolean;
  supportsStructuredOutput: boolean;
  supportsUsage: boolean;
}

export type GatewayProviderStatus = 'catalogOnly' | 'available' | 'partiallyAvailable' | 'unavailable';

export interface GatewayCapabilitySet {
  tools: boolean;
  streaming: boolean;
  multimodal: boolean;
  reasoning: boolean;
  structuredOutput: boolean;
  usage: boolean;
  modelCatalog: boolean;
  maxOutputTokens?: number;
  maxToolCount?: number;
  inputContentTypes?: string[];
  outputContentTypes?: string[];
  reasoningEfforts?: string[];
  maxImageSize?: number;
}

export interface GatewayCapabilityDiagnostic {
  field: string;
  constraints: Array<{ kind: string; sourceId: string }>;
  reason: string;
}

export interface GatewayProviderCatalog {
  id: string;
  label: string;
  description: string;
  defaultBaseUrl: string;
  defaultProtocol: GatewayApiProtocol;
  protocols: GatewayApiProtocol[];
  requiresSecret: boolean;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsMultimodal: boolean;
  supportsReasoningEffort: boolean;
  supportsStructuredOutput: boolean;
  supportsUsage: boolean;
  capabilities: GatewayCapabilitySet;
  capabilityVersion: { major: number; minor: number };
  diagnostics: GatewayCapabilityDiagnostic[];
  configured: boolean;
  modelCount: number;
  availableModelCount: number;
  status: GatewayProviderStatus;
}

export interface GatewayCatalog {
  protocols: GatewayProtocolCatalog[];
  providers: GatewayProviderCatalog[];
  models: GatewayModel[];
}

export interface GatewayModelConfig {
  id: string;
  name: string;
  contextWindow: number;
  maxOutputTokens: number;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsMultimodal: boolean;
  supportsReasoningEffort: boolean;
  supportsStructuredOutput: boolean;
  supportsUsage: boolean;
  defaultReasoningEffort: GatewayReasoningEffort;
  apiProtocol: GatewayModel['apiProtocol'];
  costPer1kInput: number;
  costPer1kOutput: number;
}

export interface GatewayProviderConfig {
  id: string;
  providerType: string;
  name: string;
  baseUrl: string;
  secretRef?: string | null;
  models: GatewayModelConfig[];
  defaultModel: string;
}

export interface GatewayConfig {
  providers: GatewayProviderConfig[];
  defaultProvider?: string | null;
  offlineFallbackModel?: string | null;
  requestTimeoutSecs: number;
  maxRetries: number;
}

interface GatewayState {
  providers: GatewayProvider[];
  models: GatewayModel[];
  catalog: GatewayCatalog | null;
  config: GatewayConfig | null;
  loaded: boolean;
  loading: boolean;
  error: string | null;
}

export const [gatewayState, setGatewayState] = createStore<GatewayState>({
  providers: [],
  models: [],
  catalog: null,
  config: null,
  loaded: false,
  loading: false,
  error: null,
});

export async function loadGatewayCatalog(): Promise<void> {
  setGatewayState('loading', true);
  setGatewayState('error', null);

  try {
    const [catalog, config] = await Promise.all([
      invoke<GatewayCatalog>('ui_get_gateway_catalog'),
      invoke<GatewayConfig>('ui_get_gateway_config'),
    ]);

    setGatewayState({
      providers: gatewayProvidersFromConfig(config),
      models: catalog.models,
      catalog,
      config,
      loaded: true,
      loading: false,
      error: null,
    });
  } catch (error) {
    setGatewayState('loading', false);
    setGatewayState('loaded', true);
    setGatewayState('error', error instanceof Error ? error.message : String(error));
  }
}

export async function saveGatewayConfig(config: GatewayConfig): Promise<GatewayConfig> {
  setGatewayState('loading', true);
  setGatewayState('error', null);

  try {
    const saved = await invoke<GatewayConfig>('ui_save_gateway_config', { payload: config });
    const [catalog, refreshedConfig] = await Promise.all([
      invoke<GatewayCatalog>('ui_get_gateway_catalog'),
      invoke<GatewayConfig>('ui_get_gateway_config'),
    ]);
    const nextConfig = refreshedConfig ?? saved;

    setGatewayState({
      providers: gatewayProvidersFromConfig(nextConfig),
      models: catalog.models,
      catalog,
      config: nextConfig,
      loaded: true,
      loading: false,
      error: null,
    });
    return nextConfig;
  } catch (error) {
    setGatewayState('loading', false);
    setGatewayState('loaded', true);
    setGatewayState('error', error instanceof Error ? error.message : String(error));
    throw error;
  }
}

function gatewayProvidersFromConfig(config: GatewayConfig): GatewayProvider[] {
  return config.providers.map((provider) => ({
    id: provider.id,
    providerType: provider.providerType,
    name: provider.name,
    baseUrl: provider.baseUrl,
    defaultModel: provider.defaultModel,
    modelCount: provider.models.length,
  }));
}

export function preferredGatewayDefaultModelSelection(
  config: GatewayConfig | null | undefined = gatewayState.config,
): GatewayModelSelection | null {
  if (!config) return null;

  const validDefaultModelForProvider = (provider: GatewayProviderConfig | undefined): GatewayModelSelection | null => {
    const modelId = provider?.defaultModel?.trim() ?? '';
    const providerId = provider?.id.trim() ?? '';
    if (!providerId || !modelId) return null;
    return provider?.models.some((model) => model.id.trim() === modelId)
      ? { providerId, modelId }
      : null;
  };

  const defaultProviderId = config.defaultProvider?.trim();
  const defaultProvider = defaultProviderId
    ? config.providers.find((provider) => provider.id === defaultProviderId)
    : undefined;
  const preferredProvider = defaultProvider ?? config.providers[0];
  const preferredModel = validDefaultModelForProvider(preferredProvider);
  if (preferredModel) return preferredModel;

  return config.providers
    .map((provider) => validDefaultModelForProvider(provider))
    .find((selection): selection is GatewayModelSelection => selection !== null) ?? null;
}
