import { Component, For, Show } from 'solid-js';
import type { GatewayProviderConfig } from '../../stores/gateway';

export const GatewayProviderRail: Component<{
  providers: GatewayProviderConfig[];
  selectedProviderId: string;
  defaultProviderId: string | null;
  loading: boolean;
  onSelectProvider: (providerId: string) => void;
  onSetDefault: (providerId: string) => void;
  onDeleteProvider: (providerId: string) => void;
  onAddProvider: () => void;
}> = (props) => (
  <div class="navis-gateway-provider-rail" aria-label="Gateway providers">
    <div class="navis-gateway-provider-rail-head">
      <span>Providers</span>
      <small>{props.providers.length}</small>
    </div>
    <For each={props.providers}>
      {(provider) => (
        <div
          class={`navis-gateway-provider-card ${provider.id === props.selectedProviderId ? 'is-active' : ''}`}
        >
          <button
            type="button"
            class="navis-gateway-provider-card-main"
            onClick={() => props.onSelectProvider(provider.id)}
          >
            <span>{provider.id}</span>
            <small>{provider.models.length} models · {provider.providerType}</small>
          </button>
          <div class="navis-gateway-provider-card-actions">
            <Show
              when={provider.id === props.defaultProviderId}
              fallback={(
                <button
                  type="button"
                  class="navis-gateway-provider-default-action"
                  disabled={props.loading}
                  onClick={() => props.onSetDefault(provider.id)}
                >
                  Set default
                </button>
              )}
            >
              <em>Default</em>
            </Show>
            <button
              type="button"
              class="navis-gateway-provider-delete-action"
              disabled={props.loading}
              onClick={() => props.onDeleteProvider(provider.id)}
            >
              Delete
            </button>
          </div>
        </div>
      )}
    </For>
    <div class="navis-gateway-provider-add">
      <button type="button" onClick={props.onAddProvider}>
        Add Provider
      </button>
    </div>
  </div>
);
