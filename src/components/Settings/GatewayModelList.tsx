import { Component, For, Index, Show } from 'solid-js';
import type {
  GatewayModelConfig,
  GatewayProviderConfig,
  GatewayProtocolCatalog,
  GatewayReasoningEffort,
} from '../../stores/gateway';
import {
  apiProtocolFromValue,
  apiProtocolToValue,
  providerDefaultModelOptions,
  protocolOptions,
  reasoningEffortOptions,
  type DiscoveredGatewayModel,
} from './gateway-config-model';

export const GatewayModelList: Component<{
  provider: GatewayProviderConfig;
  protocols: GatewayProtocolCatalog[];
  defaultModelId: string;
  discoveredModels: DiscoveredGatewayModel[];
  modelPickerEnabled: boolean;
  discoveringModels: boolean;
  onDiscoverModels: () => void;
  onAddModel: () => void;
  onRemoveModel: (modelIndex: number) => void;
  onApplyDiscoveredModel: (modelIndex: number, modelId: string) => void;
  onUpdateModel: (modelIndex: number, patch: Partial<GatewayModelConfig>) => void;
  onDefaultModelChange: (modelId: string) => void;
}> = (props) => (
  <article class="navis-gateway-connection-card">
    <div class="navis-gateway-card-head">
      <div>
        <div class="navis-gateway-card-title">Models</div>
        <div class="navis-gateway-card-subtitle">Only real model IDs entered here appear in chat.</div>
      </div>
      <div class="navis-gateway-model-actions">
        <button type="button" onClick={props.onDiscoverModels} disabled={props.discoveringModels}>
          {props.discoveringModels ? 'Fetching' : 'Get models'}
        </button>
        <button type="button" class="navis-gateway-add-model" onClick={props.onAddModel}>
          Add model
        </button>
      </div>
    </div>
    <div class="navis-gateway-model-list">
      <div class="navis-gateway-model-row navis-gateway-model-row-head">
        <span>Model ID</span>
        <span>Display name</span>
        <span>Context</span>
        <span>Protocol</span>
        <span>Effort</span>
        <span>Quality</span>
        <span>Action</span>
      </div>
      <Index each={props.provider.models}>
        {(model, modelIndex) => (
          <div class="navis-gateway-model-row">
            <Show
              when={props.modelPickerEnabled && props.discoveredModels.length > 0}
              fallback={(
                <input
                  value={model().id}
                  placeholder="model id"
                  aria-label="Model ID"
                  onInput={(event) => props.onUpdateModel(modelIndex, { id: event.currentTarget.value })}
                />
              )}
            >
              <select
                value={model().id}
                aria-label="Model ID"
                onChange={(event) => props.onApplyDiscoveredModel(modelIndex, event.currentTarget.value)}
              >
                <option value="">Choose model</option>
                <For each={props.discoveredModels}>
                  {(item) => <option value={item.id}>{item.name || item.id}</option>}
                </For>
              </select>
            </Show>
            <input
              value={model().name}
              placeholder="Display name"
              aria-label="Model name"
              onInput={(event) => props.onUpdateModel(modelIndex, { name: event.currentTarget.value })}
            />
            <input
              type="number"
              value={model().contextWindow}
              aria-label="Context window"
              onInput={(event) =>
                props.onUpdateModel(modelIndex, { contextWindow: Number(event.currentTarget.value) || 1 })
              }
            />
            <select
              value={apiProtocolToValue(model().apiProtocol)}
              onChange={(event) =>
                props.onUpdateModel(modelIndex, {
                  apiProtocol: apiProtocolFromValue(event.currentTarget.value),
                })
              }
            >
              <For each={protocolOptions(props.protocols)}>
                {(option) => <option value={option.value}>{option.label}</option>}
              </For>
            </select>
            <button
              type="button"
              class={`navis-gateway-effort-toggle ${model().supportsReasoningEffort ? 'is-on' : ''}`}
              aria-pressed={model().supportsReasoningEffort}
              title="Provider accepts reasoning effort"
              onClick={() =>
                props.onUpdateModel(modelIndex, {
                  supportsReasoningEffort: !model().supportsReasoningEffort,
                  defaultReasoningEffort: model().defaultReasoningEffort || 'high',
                })
              }
            >
              {model().supportsReasoningEffort ? 'On' : 'Off'}
            </button>
            <Show when={model().supportsReasoningEffort} fallback={<span />}>
              <select
                value={model().defaultReasoningEffort || 'high'}
                aria-label="Default quality"
                onChange={(event) =>
                  props.onUpdateModel(modelIndex, {
                    defaultReasoningEffort: event.currentTarget.value as GatewayReasoningEffort,
                  })
                }
              >
                <For each={reasoningEffortOptions}>
                  {(option) => <option value={option.value}>{option.label}</option>}
                </For>
              </select>
            </Show>
            <button type="button" onClick={() => props.onRemoveModel(modelIndex)}>Remove</button>
          </div>
        )}
      </Index>
    </div>
    <div class="navis-gateway-default-model">
      <label>
        Default model
        <select
          value={props.defaultModelId}
          onChange={(event) => props.onDefaultModelChange(event.currentTarget.value.trim())}
        >
          <option value="">Choose after adding a model</option>
          <For each={providerDefaultModelOptions(props.provider)}>
            {(modelId) => (
              <option value={modelId}>
                {modelId}
              </option>
            )}
          </For>
        </select>
      </label>
    </div>
  </article>
);
