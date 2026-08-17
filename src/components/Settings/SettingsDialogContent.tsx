import { Component, For, Match, Show, Switch, createSignal, onMount } from 'solid-js';

import { gatewayState, loadGatewayCatalog } from '../../stores/gateway';
import { loadExtensions, extensionState } from '../../stores/extension';
import CodingSettingsEditor from './CodingSettingsEditor';
import { GatewayConfigEditor } from './GatewayConfigEditor';
import { ExtensionsManager, type ExtensionsFilter } from './ExtensionsManager';
import { HostViewSurface } from '../HostView';
import { hostViewsForZone } from '../../stores/app';
import { extensionPointsByKind, extensionPointsState, loadExtensionPoints } from '../../stores/extension-points';
import ExtensionConfigurationEditor from './ExtensionConfigurationEditor';

interface SettingsSection {
  id: SettingsTab;
  title: string;
}

export type SettingsTab = 'gateway' | 'coding' | 'extensions';

interface SettingsDialogContentProps {
  initialTab?: SettingsTab;
  extensionsFilter?: ExtensionsFilter;
}

const sections: SettingsSection[] = [
  { id: 'gateway', title: 'Gateway' },
  { id: 'coding', title: 'Coding' },
  { id: 'extensions', title: 'Extensions' },
];

const SettingsDialogContent: Component<SettingsDialogContentProps> = (props) => {
  const [activeTab, setActiveTab] = createSignal<SettingsTab>(props.initialTab ?? 'gateway');

  onMount(() => {
    if (!extensionState.loaded) void loadExtensions();
    if (!gatewayState.loaded) void loadGatewayCatalog();
    if (!extensionPointsState.loaded) void loadExtensionPoints();
  });

  return (
    <div class="navis-settings-dialog">
      <div class="navis-settings-layout">
        <aside class="navis-settings-nav" role="tablist" aria-label="Settings categories">
          <For each={sections}>
            {(section) => (
              <button
                type="button"
                role="tab"
                aria-selected={activeTab() === section.id}
                class={activeTab() === section.id ? 'is-active' : ''}
                onClick={() => setActiveTab(section.id)}
              >
                <span>{section.title}</span>
                <Show when={section.id === 'gateway'}>
                  <small>{gatewayState.models.length} models</small>
                </Show>
                <Show when={section.id === 'extensions'}>
                  <small>{extensionState.extensions.length} installed</small>
                  <small>{hostViewsForZone('settingsSection').length} views</small>
                </Show>
              </button>
            )}
          </For>
        </aside>
        <div class="navis-settings-content">
          <Switch>
            <Match when={activeTab() === 'coding'}>
              <CodingSettingsEditor />
            </Match>
            <Match when={activeTab() === 'gateway'}>
              <GatewayConfigEditor />
            </Match>
            <Match when={activeTab() === 'extensions'}>
              <div class="navis-settings-extension-content">
                <ExtensionsManager initialFilter={props.extensionsFilter} />
                <Show when={extensionPointsByKind('configuration').length > 0}>
                  <div class="navis-settings-section">
                    <div class="navis-settings-section-title">Extension configuration</div>
                    <div class="space-y-3">
                      <For each={extensionPointsByKind('configuration')}>
                        {(point) => <ExtensionConfigurationEditor point={point} />}
                      </For>
                    </div>
                  </div>
                </Show>
                <HostViewSurface zone="settingsSection" title="Extensions" />
              </div>
            </Match>
          </Switch>
        </div>
      </div>
      <Show when={extensionState.error || gatewayState.error}>
        <div class="navis-settings-state-warning">
          {extensionState.error || gatewayState.error}
        </div>
      </Show>
    </div>
  );
};

export default SettingsDialogContent;