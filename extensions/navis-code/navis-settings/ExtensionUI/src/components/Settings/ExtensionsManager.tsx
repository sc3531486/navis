import { Component, For, Show, createSignal } from 'solid-js';
import { open as openDialog } from '@tauri-apps/plugin-dialog';

import {
  installExtension,
  loadExtensions,
  extensionState,
  setExtensionEnabled,
  uninstallExtension,
  type ExtensionContributionCounts,
  type ExtensionRuntimeState,
} from '@/stores/extension';
import { dialog } from '@navis-code/components/Dialog';
import { statusClass } from '@/lib/status';

export type ExtensionsFilter = 'all' | 'modes' | 'connectors';

function selectedExtensionSource(result: string | string[] | null): string | null {
  if (!result) return null;
  return Array.isArray(result) ? result[0] ?? null : result;
}

function contributionSummary(counts: ExtensionContributionCounts): string {
  const parts = [
    counts.workModes ? `${counts.workModes} modes` : '',
    counts.views ? `${counts.views} views` : '',
    counts.menus ? `${counts.menus} menus` : '',
    counts.commands ? `${counts.commands} commands` : '',
    counts.keybindings ? `${counts.keybindings} keys` : '',
    counts.triggers ? `${counts.triggers} triggers` : '',
    counts.mcpServers ? `${counts.mcpServers} MCP servers` : '',
    counts.providers ? `${counts.providers} provider declarations` : '',
  ].filter(Boolean);

  return parts.join(' · ') || 'No contributions';
}

function isConnectorExtension(extension: ExtensionRuntimeState): boolean {
  return extension.contributionCounts.mcpServers > 0;
}

function isModeExtension(extension: ExtensionRuntimeState): boolean {
  return extension.contributionCounts.workModes > 0;
}

function permissionSummary(extension: ExtensionRuntimeState): string {
  const permissions = extension.permissions;
  const parts = [
    permissions.filesystem.length ? `fs ${permissions.filesystem.length}` : '',
    permissions.terminal.length ? `terminal ${permissions.terminal.length}` : '',
    permissions.network.length ? `network ${permissions.network.length}` : '',
    permissions.ipc.length ? `ipc ${permissions.ipc.length}` : '',
    permissions.events.length ? `events ${permissions.events.length}` : '',
  ].filter(Boolean);

  return parts.join(' · ') || 'No declared host permissions';
}

export const ExtensionsManager: Component<{ initialFilter?: ExtensionsFilter }> = (props) => {
  const [status, setStatus] = createSignal('');
  const [filter, setFilter] = createSignal<ExtensionsFilter>(props.initialFilter ?? 'all');

  const visibleExtensions = () =>
    filter() === 'connectors'
      ? extensionState.extensions.filter(isConnectorExtension)
      : filter() === 'modes'
        ? extensionState.extensions.filter(isModeExtension)
        : extensionState.extensions;

  async function chooseExtensionSource(): Promise<void> {
    setStatus('');
    const result = await openDialog({
      multiple: false,
      directory: true,
      title: '选择扩展目录',
    });
    const sourcePath = selectedExtensionSource(result);
    if (!sourcePath) return;

    await installExtension(sourcePath);
    setStatus(`已安装 ${sourcePath}`);
  }

  async function toggleExtension(extension: ExtensionRuntimeState): Promise<void> {
    const enabled = extension.status === 'enabled';
    setStatus('');
    await setExtensionEnabled(extension.id, !enabled);
    setStatus(enabled ? `${extension.name} 已停用` : `${extension.name} 已启用`);
  }

  async function removeExtension(extension: ExtensionRuntimeState): Promise<void> {
    const confirmed = await dialog.confirm({
      title: 'Uninstall extension?',
      message: `${extension.name} will be removed from Navis Go. Disable it first if it is currently enabled.`,
      confirmText: 'Uninstall',
      cancelText: 'Cancel',
      danger: true,
    });
    if (!confirmed) return;

    setStatus('');
    await uninstallExtension(extension.id);
    setStatus(`${extension.name} 已卸载`);
  }

  return (
    <section class="navis-extensions-manager">
      <div class="navis-extensions-manager-head">
        <div>
          <div class="navis-settings-section-title">Extensions</div>
          <p>Install local extension folders, enable contributed modes, menus, right panels and MCP connectors.</p>
        </div>
        <div class="navis-extensions-actions">
          <button type="button" onClick={() => void loadExtensions()} disabled={extensionState.loading}>
            Refresh
          </button>
          <button type="button" onClick={() => void chooseExtensionSource()} disabled={extensionState.loading}>
            Install local
          </button>
        </div>
      </div>

      <div class="navis-extensions-filter" role="tablist" aria-label="Extension filters">
        <button
          type="button"
          role="tab"
          aria-selected={filter() === 'all'}
          class={filter() === 'all' ? 'is-active' : ''}
          onClick={() => setFilter('all')}
        >
          All extensions
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={filter() === 'modes'}
          class={filter() === 'modes' ? 'is-active' : ''}
          onClick={() => setFilter('modes')}
        >
          Mode extensions
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={filter() === 'connectors'}
          class={filter() === 'connectors' ? 'is-active' : ''}
          onClick={() => setFilter('connectors')}
        >
          Connectors
        </button>
      </div>

      <Show when={visibleExtensions().length > 0} fallback={
        <div class="navis-settings-state-warning">
          {filter() === 'connectors'
            ? 'No connector extensions found. Connector extensions contribute MCP servers.'
            : filter() === 'modes'
              ? 'No mode extensions found. Mode extensions contribute custom work modes.'
              : '未安装扩展。点击 Install local 选择包含 extension.json 的扩展目录。'}
        </div>
      }>
        <div class="navis-extension-list">
          <For each={visibleExtensions()}>
            {(extension) => (
              <article class={`navis-extension-card ${statusClass(extension.statusPresentation)}`}>
                <div class="navis-extension-card-main">
                  <div>
                    <div class="navis-extension-title-row">
                      <span class="navis-extension-name">{extension.name}</span>
                      <span class="navis-extension-status">{extension.status}</span>
                    </div>
                    <div class="navis-extension-description">{extension.description || extension.id}</div>
                  </div>
                  <div class="navis-extension-actions">
                    <button
                      type="button"
                      onClick={() => void toggleExtension(extension)}
                      disabled={extensionState.loading || extension.status === 'loading' || extension.status === 'disabling'}
                    >
                      {extension.status === 'enabled' ? 'Disable' : 'Enable'}
                    </button>
                    <button
                      type="button"
                      class="is-danger"
                      onClick={() => void removeExtension(extension)}
                      disabled={extensionState.loading || extension.status === 'enabled'}
                      title={extension.status === 'enabled' ? 'Disable before uninstalling' : 'Uninstall extension'}
                    >
                      Uninstall
                    </button>
                  </div>
                </div>
                <div class="navis-extension-meta-grid">
                  <span>{extension.id}</span>
                  <span>v{extension.version}</span>
                  <span>{extension.author || 'Unknown author'}</span>
                  <span title={extension.installPath}>{extension.installPath}</span>
                </div>
                <div class="navis-extension-contributions">
                  <span>{contributionSummary(extension.contributionCounts)}</span>
                  <span>{permissionSummary(extension)}</span>
                </div>
                <Show when={extension.error}>
                  {(error) => <div class="navis-extension-error">{error()}</div>}
                </Show>
              </article>
            )}
          </For>
        </div>
      </Show>

      <Show when={status()}>
        <div class="navis-extension-status-line">{status()}</div>
      </Show>
    </section>
  );
};
