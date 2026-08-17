import type {
  GatewayApiProtocol,
  GatewayCatalog,
  GatewayConfig,
  GatewayModelConfig,
  GatewayProviderCatalog,
  GatewayProviderConfig,
  GatewayProtocolCatalog,
  GatewayReasoningEffort,
} from '../../stores/gateway';

export interface DiscoveredGatewayModel {
  id: string;
  name: string;
}

export const DEFAULT_GATEWAY_REQUEST_TIMEOUT_SECS = 300;

// Only builtin profile IDs may populate providerType; Extension catalog IDs stay independent.
const BUILTIN_PROVIDER_TYPES = new Set(['openai', 'anthropic', 'custom']);

export const reasoningEffortOptions: Array<{ value: GatewayReasoningEffort; label: string }> = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
  { value: 'extra-high', label: 'Extra high' },
  { value: 'max', label: 'Max' },
];

export function cloneGatewayConfig(config: GatewayConfig): GatewayConfig {
  return {
    ...config,
    providers: config.providers.map((provider) => ({
      ...provider,
      models: provider.models.map((model) => ({ ...model })),
    })),
  };
}

export function gatewayApiProtocolKey(protocol: GatewayApiProtocol): string {
  return typeof protocol === 'string' ? protocol : JSON.stringify(protocol);
}

export function apiProtocolToValue(protocol: GatewayApiProtocol): string {
  return gatewayApiProtocolKey(protocol);
}

export function apiProtocolFromValue(value: string): GatewayApiProtocol {
  try {
    const parsed: unknown = JSON.parse(value);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, string>;
    }
  } catch {
    // Protocol IDs are commonly plain strings; JSON is only used for structured IDs.
  }
  return value;
}

export function providerCatalogFor(
  catalog: GatewayCatalog | null | undefined,
  providerId: string | undefined,
): GatewayProviderCatalog | null {
  return catalog?.providers.find((item) => item.id === providerId) ?? null;
}
export function providerCatalogForProvider(
  catalog: GatewayCatalog | null | undefined,
  provider: GatewayProviderConfig | null | undefined,
): GatewayProviderCatalog | null {
  return providerCatalogFor(catalog, provider?.id);
}

export function builtinProviderTypeForCatalog(provider: GatewayProviderCatalog): string {
  return BUILTIN_PROVIDER_TYPES.has(provider.id) ? provider.id : '';
}

export function protocolCatalogForProvider(
  catalog: GatewayCatalog | null | undefined,
  provider: GatewayProviderConfig | null | undefined,
): GatewayProtocolCatalog[] {
  if (!catalog) return [];
  const providerCatalog = providerCatalogForProvider(catalog, provider);
  if (!providerCatalog) return [];
  const allowed = new Set(providerCatalog.protocols.map(gatewayApiProtocolKey));
  return catalog.protocols.filter((protocol) => allowed.has(gatewayApiProtocolKey(protocol.id)));
}

export function protocolOptions(
  protocols: GatewayProtocolCatalog[],
): Array<{ value: string; label: string }> {
  return protocols.map((protocol) => ({
    value: apiProtocolToValue(protocol.id),
    label: protocol.label,
  }));
}

export function createModel(provider: GatewayProviderCatalog): GatewayModelConfig {
  return {
    id: '',
    name: '',
    contextWindow: 256000,
    maxOutputTokens: 4096,
    supportsTools: provider.supportsTools,
    supportsStreaming: provider.supportsStreaming,
    supportsMultimodal: provider.supportsMultimodal,
    supportsReasoningEffort: provider.supportsReasoningEffort,
    supportsStructuredOutput: provider.supportsStructuredOutput,
    supportsUsage: provider.supportsUsage,
    defaultReasoningEffort: 'high',
    apiProtocol: provider.defaultProtocol,
    costPer1kInput: 0,
    costPer1kOutput: 0,
  };
}

export function createProvider(provider: GatewayProviderCatalog): GatewayProviderConfig {
  return {
    id: provider.id,
    providerType: builtinProviderTypeForCatalog(provider),
    name: provider.label,
    baseUrl: provider.defaultBaseUrl,
    secretRef: null,
    models: [createModel(provider)],
    defaultModel: '',
  };
}

export function normalizeReasoningEffort(value: string | undefined): GatewayReasoningEffort {
  return reasoningEffortOptions.some((option) => option.value === value)
    ? value as GatewayReasoningEffort
    : 'high';
}

export function sanitizeProviderModel(
  provider: GatewayProviderCatalog,
  source?: GatewayModelConfig,
): GatewayModelConfig {
  const defaults = createModel(provider);
  return {
    ...defaults,
    ...source,
    id: source?.id?.trim() ?? '',
    apiProtocol: source?.apiProtocol ?? defaults.apiProtocol,
    supportsTools: source?.supportsTools ?? defaults.supportsTools,
    supportsStreaming: source?.supportsStreaming ?? defaults.supportsStreaming,
    supportsMultimodal: source?.supportsMultimodal ?? defaults.supportsMultimodal,
    supportsReasoningEffort: source?.supportsReasoningEffort ?? defaults.supportsReasoningEffort,
    supportsStructuredOutput: source?.supportsStructuredOutput ?? defaults.supportsStructuredOutput,
    supportsUsage: source?.supportsUsage ?? defaults.supportsUsage,
    defaultReasoningEffort: normalizeReasoningEffort(source?.defaultReasoningEffort),
  };
}

export function normalizeConnectionProvider(
  provider: GatewayProviderCatalog,
  source?: GatewayProviderConfig,
): GatewayProviderConfig {
  const models = source?.models.length
    ? source.models.map((model) => sanitizeProviderModel(provider, model))
    : [createModel(provider)];
  const requestedDefaultModel = source?.defaultModel?.trim() ?? '';

  return {
    id: source?.id?.trim() || provider.id,
    providerType: source?.providerType?.trim() || builtinProviderTypeForCatalog(provider),
    name: source?.name?.trim() || provider.label,
    baseUrl: source?.baseUrl ?? provider.defaultBaseUrl,
    secretRef: source?.secretRef ?? null,
    models,
    defaultModel: requestedDefaultModel,
  };
}

export function stabilizeProviderConfig(
  provider: GatewayProviderConfig,
  providerCatalog: GatewayProviderCatalog,
): GatewayProviderConfig {
  return normalizeConnectionProvider(providerCatalog, provider);
}

export function providerDefaultModelOptions(provider: GatewayProviderConfig | null | undefined): string[] {
  if (!provider) return [];
  const seen = new Set<string>();
  const ids: string[] = [];
  for (const model of provider.models) {
    const id = model.id.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    ids.push(id);
  }
  return ids;
}

export function resolveProviderDefaultModelId(provider: GatewayProviderConfig | null | undefined): string {
  const modelIds = providerDefaultModelOptions(provider);
  const requestedDefaultModel = provider?.defaultModel?.trim() ?? '';
  return requestedDefaultModel && modelIds.includes(requestedDefaultModel)
    ? requestedDefaultModel
    : '';
}

export function normalizeConnectionConfig(
  config: GatewayConfig,
  catalog: GatewayCatalog | null | undefined,
): GatewayConfig {
  const providers = config.providers.length
    ? config.providers.map((provider) => {
      const providerCatalog = providerCatalogFor(catalog, provider.id);
      return providerCatalog ? normalizeConnectionProvider(providerCatalog, provider) : provider;
    })
    : catalog?.providers[0]
      ? [createProvider(catalog.providers[0])]
      : [];
  const defaultProvider = config.defaultProvider && providers.some((provider) => provider.id === config.defaultProvider)
    ? config.defaultProvider
    : providers[0]?.id;

  return {
    ...cloneGatewayConfig(config),
    providers,
    defaultProvider,
    offlineFallbackModel: config.offlineFallbackModel ?? null,
    requestTimeoutSecs: Math.max(1, config.requestTimeoutSecs || DEFAULT_GATEWAY_REQUEST_TIMEOUT_SECS),
  };
}
