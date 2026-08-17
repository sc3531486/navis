import { Component, For, Show, createEffect, createSignal } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';

import {
  gatewayState,
  saveGatewayConfig,
  type GatewayConfig,
  type GatewayModelConfig,
  type GatewayProviderConfig,
} from '../../stores/gateway';
import {
  cloneGatewayConfig,
  createModel,
  createProvider,
  DEFAULT_GATEWAY_REQUEST_TIMEOUT_SECS,
  normalizeConnectionConfig,
  protocolCatalogForProvider,
  providerCatalogFor,
  providerCatalogForProvider,
  builtinProviderTypeForCatalog,
  resolveProviderDefaultModelId,
  sanitizeProviderModel,
  stabilizeProviderConfig,
  type DiscoveredGatewayModel,
} from './gateway-config-model';
import type { GatewayProviderCatalog } from '../../stores/gateway';
import { GatewayModelList } from './GatewayModelList';
import { GatewayProviderRail } from './GatewayProviderRail';
export const GatewayConfigEditor: Component = () => {
  const [localConfig, setLocalConfig] = createSignal<GatewayConfig | null>(null);
  const [selectedProviderId, setSelectedProviderId] = createSignal('');
  const [selectedProviderCatalogId, setSelectedProviderCatalogId] = createSignal('');
  const [discoveredModels, setDiscoveredModels] = createSignal<DiscoveredGatewayModel[]>([]);
  const [modelPickerEnabled, setModelPickerEnabled] = createSignal(false);
  const [discoveringModels, setDiscoveringModels] = createSignal(false);
  const [status, setStatus] = createSignal('');
  const [statusDetail, setStatusDetail] = createSignal('');
  const [statusDetailOpen, setStatusDetailOpen] = createSignal(false);
  const [statusTone, setStatusTone] = createSignal<'neutral' | 'success' | 'error'>('neutral');

  createEffect(() => {
    if (!gatewayState.config || localConfig()) return;
    const normalized = normalizeConnectionConfig(gatewayState.config, gatewayState.catalog);
    const provider = normalized.providers.find((item) => item.id === normalized.defaultProvider) ?? normalized.providers[0];
    setSelectedProviderId(provider?.id ?? '');
    setSelectedProviderCatalogId(provider ? (providerCatalogForProvider(gatewayState.catalog, provider)?.id ?? '') : '');
    setLocalConfig(normalized);
  });

  createEffect(() => {
    if (!statusDetail()) setStatusDetailOpen(false);
  });

  function updateConfig(updater: (config: GatewayConfig) => GatewayConfig): void {
    setLocalConfig((current) => {
      const base = current ?? gatewayState.config;
      if (!base) return current;
      return updater(cloneGatewayConfig(base));
    });
  }

  const currentProvider = () => {
    const config = localConfig();
    if (!config) return null;
    return config.providers.find((provider) => provider.id === selectedProviderId()) ?? config.providers[0] ?? null;
  };
  const currentProviderDefaultModelId = () => resolveProviderDefaultModelId(currentProvider());
  const currentProviderCatalog = () => providerCatalogFor(gatewayState.catalog, selectedProviderCatalogId());

  function updateCurrentProvider(
    updater: (provider: GatewayProviderConfig, providerCatalog: GatewayProviderCatalog) => GatewayProviderConfig,
    providerCatalogOverride?: GatewayProviderCatalog,
  ): void {
    updateConfig((config) => {
      const providerCatalog = providerCatalogOverride ?? currentProviderCatalog();
      if (!providerCatalog) return config;
      const current = currentProvider() ?? createProvider(providerCatalog);
      const nextProvider = stabilizeProviderConfig(updater(current, providerCatalog), providerCatalog);
      const nextProviders = config.providers.length
        ? config.providers.map((provider) => provider.id === current.id ? nextProvider : provider)
        : [nextProvider];
      config.providers = nextProviders;
      if (!config.defaultProvider || config.defaultProvider === current.id) {
        config.defaultProvider = nextProvider.id || providerCatalog.id;
      }
      setSelectedProviderId(nextProvider.id);
      return config;
    });
  }

  function selectProviderCatalog(providerCatalogId: string): void {
    const providerCatalog = gatewayState.catalog?.providers.find((item) => item.id === providerCatalogId);
    if (!providerCatalog) return;
    setSelectedProviderCatalogId(providerCatalog.id);
    setDiscoveredModels([]);
    setModelPickerEnabled(false);
    setStatus('');
    setStatusDetail('');
    setStatusTone('neutral');
    updateCurrentProvider((provider) => ({
      ...provider,
      providerType: builtinProviderTypeForCatalog(providerCatalog) || provider.providerType,
      name: provider.name || providerCatalog.label,
      baseUrl: provider.baseUrl || providerCatalog.defaultBaseUrl,
      models: provider.models.map((model) => sanitizeProviderModel(providerCatalog, model)),
    }), providerCatalog);
  }
  function selectProvider(providerId: string): void {
    const provider = localConfig()?.providers.find((item) => item.id === providerId);
    if (!provider) return;
    setSelectedProviderId(provider.id);
    setSelectedProviderCatalogId(providerCatalogForProvider(gatewayState.catalog, provider)?.id ?? '');
  }

  function addProvider(): void {
    const providerCatalog = gatewayState.catalog?.providers[0];
    if (!providerCatalog) return;
    updateConfig((config) => {
      const existingIds = new Set(config.providers.map((provider) => provider.id));
      let id = providerCatalog.id;
      let index = 2;
      while (existingIds.has(id)) {
        id = `${providerCatalog.id}-${index}`;
        index += 1;
      }
      const provider = { ...createProvider(providerCatalog), id };
      setSelectedProviderId(id);
      setSelectedProviderCatalogId(providerCatalog.id);
      return {
        ...config,
        providers: [...config.providers, provider],
        defaultProvider: config.defaultProvider || id,
      };
    });
  }
  function applySavedGatewayConfig(
    saved: GatewayConfig,
    selectedProviderHint?: string | null,
  ): void {
    const normalized = normalizeConnectionConfig(saved, gatewayState.catalog);
    const provider = (selectedProviderHint
      ? normalized.providers.find((item) => item.id === selectedProviderHint)
      : null)
      ?? normalized.providers.find((item) => item.id === normalized.defaultProvider)
      ?? normalized.providers[0];
    setSelectedProviderCatalogId(provider ? (providerCatalogForProvider(gatewayState.catalog, provider)?.id ?? '') : '');
    setSelectedProviderId(provider?.id ?? '');
    setLocalConfig(normalized);
    setDiscoveredModels([]);
    setModelPickerEnabled(false);
  }

  async function persistGatewayConfig(
    config: GatewayConfig,
    selectedProviderHint: string | null,
    successMessage: string,
  ): Promise<void> {
    const payload = normalizeConnectionConfig(config, gatewayState.catalog);
    setStatus('Saving Gateway settings');
    setStatusDetail('');
    setStatusTone('neutral');
    try {
      const saved = await saveGatewayConfig(payload);
      applySavedGatewayConfig(saved, selectedProviderHint);
      setStatus(successMessage);
      setStatusDetail('');
      setStatusTone('success');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setStatus('');
      setStatusDetail(message);
      setStatusTone('error');
    }
  }

  async function setProviderAsDefault(providerId: string): Promise<void> {
    const base = localConfig() ?? gatewayState.config;
    if (!base || !base.providers.some((provider) => provider.id === providerId)) return;
    const next = {
      ...cloneGatewayConfig(base),
      defaultProvider: providerId,
    };
    setLocalConfig(next);
    await persistGatewayConfig(next, providerId, 'Default Provider saved');
  }

  async function deleteProvider(providerId: string): Promise<void> {
    const base = localConfig() ?? gatewayState.config;
    if (!base || !base.providers.some((provider) => provider.id === providerId)) return;
    const next = cloneGatewayConfig(base);
    next.providers = next.providers.filter((item) => item.id !== providerId);
    const nextProvider = next.providers[0] ?? null;
    next.defaultProvider = next.defaultProvider === providerId
      ? nextProvider?.id ?? null
      : next.defaultProvider;
    if (selectedProviderId() === providerId) {
      setSelectedProviderId(nextProvider?.id ?? '');
      setSelectedProviderCatalogId(nextProvider ? (providerCatalogForProvider(gatewayState.catalog, nextProvider)?.id ?? '') : '');
    }
    setLocalConfig(next);
    await persistGatewayConfig(next, nextProvider?.id ?? null, 'Provider deleted');
  }
  function cancelChanges(): void {
    const source = gatewayState.config;
    if (!source) return;
    const normalized = normalizeConnectionConfig(source, gatewayState.catalog);
    const provider = normalized.providers.find((item) => item.id === selectedProviderId())
      ?? normalized.providers.find((item) => item.id === normalized.defaultProvider)
      ?? normalized.providers[0];
    setLocalConfig(normalized);
    setSelectedProviderId(provider?.id ?? '');
    setSelectedProviderCatalogId(provider ? (providerCatalogForProvider(gatewayState.catalog, provider)?.id ?? '') : '');
    setDiscoveredModels([]);
    setModelPickerEnabled(false);
    setStatus('');
    setStatusDetail('');
    setStatusTone('neutral');
  }

  function updateProvider(patch: Partial<GatewayProviderConfig>): void {
    updateCurrentProvider((provider) => {
      return {
        ...provider,
        ...patch,
        defaultModel:
          patch.defaultModel !== undefined
            ? patch.defaultModel.trim()
            : provider.defaultModel,
      };
    });
  }

  function applyDiscoveredModel(modelIndex: number, modelId: string): void {
    const discovered = discoveredModels().find((model) => model.id === modelId);
    updateModel(modelIndex, {
      id: modelId,
      name: discovered?.name && discovered.name !== modelId ? discovered.name : modelId,
    });
  }

  function updateModel(modelIndex: number, patch: Partial<GatewayModelConfig>): void {
    updateCurrentProvider((provider, providerCatalog) => {
      const existingModel = provider.models[modelIndex];
      if (!existingModel) return provider;

      const nextModels = provider.models.map((model, index) =>
        index === modelIndex
          ? sanitizeProviderModel(providerCatalog, { ...model, ...patch })
          : model,
      );

      return {
        ...provider,
        models: nextModels,
        defaultModel: provider.defaultModel,
      };
    });
  }

  function addModel(): void {
    updateCurrentProvider((provider, providerCatalog) => {
      return {
        ...provider,
        models: [...provider.models, createModel(providerCatalog)],
      };
    });
  }

  function removeModel(modelIndex: number): void {
    updateCurrentProvider((provider) => {
      const nextModels = provider.models.filter((_, index) => index !== modelIndex);
      return {
        ...provider,
        models: nextModels,
        defaultModel: provider.defaultModel,
      };
    });
  }

  async function save(): Promise<void> {
    const config = localConfig();
    if (!config) return;
    const selectedProvider = currentProvider();
    await persistGatewayConfig(config, selectedProvider?.id ?? null, 'Gateway settings saved');
  }

  async function discoverModels(): Promise<void> {
    const provider = currentProvider();
    if (!provider) return;
    setDiscoveringModels(true);
    setStatus('正在获取模型');
    setStatusDetail('');
    setStatusTone('neutral');
    try {
      const models = await invoke<DiscoveredGatewayModel[]>('ui_discover_gateway_models', { provider });
      setDiscoveredModels(models);
      setModelPickerEnabled(models.length > 0);
      setStatus(models.length > 0 ? `已获取 ${models.length} 个模型` : '未获取到模型');
      setStatusDetail('');
      setStatusTone(models.length > 0 ? 'success' : 'error');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setModelPickerEnabled(false);
      setStatus('');
      setStatusDetail(message);
      setStatusTone('error');
    } finally {
      setDiscoveringModels(false);
    }
  }

  return (
    <section class="navis-gateway-config-editor">
      <div>
        <div class="navis-settings-section-title">Connection</div>
      </div>
      {localConfig() ? (
        <div class="navis-gateway-connection-layout">
          <GatewayProviderRail
            providers={localConfig()?.providers ?? []}
            selectedProviderId={selectedProviderId()}
            defaultProviderId={localConfig()?.defaultProvider ?? null}
            loading={gatewayState.loading}
            onSelectProvider={selectProvider}
            onSetDefault={(providerId) => void setProviderAsDefault(providerId)}
            onDeleteProvider={(providerId) => void deleteProvider(providerId)}
            onAddProvider={addProvider}
          />
          <div class="navis-gateway-connection-detail">
            {currentProvider() ? (
              <>
                <article class="navis-gateway-connection-card">
                  <div class="navis-gateway-card-head">
                    <div>
                      <div class="navis-gateway-card-title">{currentProvider()!.name || currentProviderCatalog()?.label || currentProvider()!.providerType}</div>
                      <div class="navis-gateway-card-subtitle">{selectedProviderCatalogId()} connection</div>
                    </div>
                    <div class="navis-gateway-card-actions">
                      <span>{selectedProviderCatalogId()}</span>
                    </div>
                  </div>
                  <div class="navis-gateway-form-grid">
                    <label>
                      Provider ID
                      <input
                        value={currentProvider()!.id}
                        placeholder="provider id"
                        onInput={(event) => updateProvider({ id: event.currentTarget.value })}
                      />
                    </label>
                    <label>
                      Provider catalog
                      <select
                        value={selectedProviderCatalogId()}
                        onChange={(event) => selectProviderCatalog(event.currentTarget.value)}
                      >
                        <For each={gatewayState.catalog?.providers ?? []}>
                          {(option) => <option value={option.id}>{option.label}</option>}
                        </For>
                      </select>
                    </label>
                    <label>
                      Name
                      <input
                        value={currentProvider()!.name}
                        placeholder={currentProviderCatalog()?.label ?? currentProvider()!.name}
                        onInput={(event) => updateProvider({ name: event.currentTarget.value })}
                      />
                    </label>
                    <label>
                      Base URL
                      <input
                        value={currentProvider()!.baseUrl}
                        placeholder={currentProviderCatalog()?.defaultBaseUrl ?? ''}
                        onInput={(event) => updateProvider({ baseUrl: event.currentTarget.value })}
                      />
                    </label>
                    <label class="navis-gateway-secret-ref-field">
                      Secret reference
                      <input
                        type="password"
                        value={currentProvider()!.secretRef ?? ''}
                        placeholder="Paste Secret reference"
                        onInput={(event) => updateProvider({ secretRef: event.currentTarget.value || null })}
                      />
                    </label>
                  </div>
                </article>
                <article class="navis-gateway-connection-card">
                  <div class="navis-gateway-card-head">
                    <div>
                      <div class="navis-gateway-card-title">
                        Runtime
                        <span class="navis-gateway-card-title-note">
                          Provider request timeout; stream reads stop only after this value plus 2 minutes without new chunks.
                        </span>
                      </div>
                    </div>
                  </div>
                  <div class="navis-gateway-runtime-grid">
                    <label>
                      Request timeout
                      <input
                        type="number"
                        min="1"
                        value={localConfig()?.requestTimeoutSecs ?? DEFAULT_GATEWAY_REQUEST_TIMEOUT_SECS}
                        onInput={(event) =>
                          updateConfig((next) => ({
                            ...next,
                            requestTimeoutSecs: Math.max(1, Number(event.currentTarget.value) || DEFAULT_GATEWAY_REQUEST_TIMEOUT_SECS),
                          }))
                        }
                      />
                    </label>
                    <label>
                      Max retries
                      <input
                        type="number"
                        min="0"
                        value={localConfig()?.maxRetries ?? 0}
                        onInput={(event) =>
                          updateConfig((next) => ({ ...next, maxRetries: Number(event.currentTarget.value) || 0 }))
                        }
                      />
                    </label>
                  </div>
                </article>
                <GatewayModelList
                  provider={currentProvider()!}
                  protocols={protocolCatalogForProvider(gatewayState.catalog, currentProvider())}
                  defaultModelId={currentProviderDefaultModelId()}
                  discoveredModels={discoveredModels()}
                  modelPickerEnabled={modelPickerEnabled()}
                  discoveringModels={discoveringModels()}
                  onDiscoverModels={() => void discoverModels()}
                  onAddModel={addModel}
                  onRemoveModel={removeModel}
                  onApplyDiscoveredModel={applyDiscoveredModel}
                  onUpdateModel={updateModel}
                  onDefaultModelChange={(modelId) => updateProvider({ defaultModel: modelId })}
                />
                <div class="navis-gateway-config-footer">
                  <button type="button" class="is-secondary" onClick={cancelChanges} disabled={gatewayState.loading}>
                    Cancel
                  </button>
                  <button type="button" onClick={() => void save()} disabled={gatewayState.loading}>
                    Save changes
                  </button>
                  <Show when={statusDetail()}>
                    <span class="is-error navis-gateway-error-inline" title={statusDetail()}>{statusDetail()}</span>
                  </Show>
                  <Show when={!statusDetail() && status()}>
                    <span class={`is-${statusTone()}`}>{status()}</span>
                  </Show>
                  <Show when={statusDetail()}>
                    <button
                      type="button"
                      class="navis-gateway-status-detail"
                      title={statusDetail()}
                      aria-expanded={statusDetailOpen()}
                      onClick={() => setStatusDetailOpen((open) => !open)}
                    >
                      Details
                    </button>
                  </Show>
                  <Show when={statusDetail() && statusDetailOpen()}>
                    <div class="navis-gateway-error-popover" role="status">
                      {statusDetail()}
                    </div>
                  </Show>
                </div>
              </>
            ) : (
              <article class="navis-gateway-connection-card">
                <div class="navis-gateway-card-title">No provider selected</div>
                <div class="navis-gateway-card-subtitle">Add a Provider on the left, then configure its type, endpoint and models here.</div>
                <div class="navis-gateway-config-footer">
                  <button type="button" onClick={() => addProvider()}>
                    Add Provider
                  </button>
                  <button type="button" class="is-secondary" onClick={cancelChanges} disabled={gatewayState.loading}>
                    Cancel
                  </button>
                  <button type="button" onClick={() => void save()} disabled={gatewayState.loading}>
                    Save changes
                  </button>
                </div>
              </article>
            )}
          </div>
        </div>
      ) : (
        <div class="navis-settings-state-warning">正在读取 Gateway 配置</div>
      )}
    </section>
  );
};
