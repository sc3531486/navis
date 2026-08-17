import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Component, For, Show, createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { openSettingsDialog } from '../Settings/openSettingsDialog';
import type { HostViewRendererProps } from './types';

interface HostPanelDataSource {
  kind: 'event' | 'stream' | 'storage';
  pattern?: string;
  transform?: string;
  scope?: string;
  key?: string;
  worktree?: string;
}

function asRecord(value: unknown): Record<string, unknown> {
  return value && typeof value === 'object' ? value as Record<string, unknown> : {};
}

function readDataSource(config: unknown): HostPanelDataSource | null {
  const record = asRecord(config);
  const value = record.data_source ?? record.dataSource;
  if (!value || typeof value !== 'object') return null;
  const source = asRecord(value);
  const kind = source.kind;
  if (kind !== 'event' && kind !== 'stream' && kind !== 'storage') return null;
  return {
    kind,
    pattern: typeof source.pattern === 'string' ? source.pattern.trim() : undefined,
    transform: typeof source.transform === 'string' ? source.transform.trim() : undefined,
    scope: typeof source.scope === 'string' ? source.scope : undefined,
    key: typeof source.key === 'string' ? source.key : undefined,
    worktree: typeof source.worktree === 'string' ? source.worktree : undefined,
  };
}

function applyTransform(value: unknown, transform?: string): unknown {
  if (!transform) return value;
  const normalized = transform.replace(/^(payload|data)\.?/, '');
  if (normalized === transform && transform !== 'payload' && transform !== 'data') {
    throw new Error('Only payload.foo.bar transform paths are supported');
  }
  if (!normalized) return value;
  const parts = normalized.split('.');
  if (parts.some((part) => !/^[A-Za-z_][A-Za-z0-9_]*$/.test(part))) {
    throw new Error('Invalid data transform path');
  }
  let current: unknown = value;
  for (const part of parts) {
    if (!current || typeof current !== 'object' || !(part in (current as Record<string, unknown>))) {
      return undefined;
    }
    current = (current as Record<string, unknown>)[part];
  }
  return current;
}

function formatData(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2) ?? 'undefined';
  } catch {
    return 'Unable to serialize data';
  }
}

const contributionCountLines = (counts: HostViewRendererProps['view']['contributionCounts']): string[] => [
  counts.workModes ? `${counts.workModes} work modes` : '',
  counts.views ? `${counts.views} views` : '',
  counts.menus ? `${counts.menus} menus` : '',
  counts.commands ? `${counts.commands} commands` : '',
  counts.triggers ? `${counts.triggers} triggers` : '',
  counts.mcpServers ? `${counts.mcpServers} MCP servers` : '',
  counts.providers ? `${counts.providers} provider declarations` : '',
].filter(Boolean);

const HostPanelRenderer: Component<HostViewRendererProps> = (props) => {
  const countLines = () => contributionCountLines(props.view.contributionCounts);
  const dataSource = createMemo(() => readDataSource(props.view.config));
  const [data, setData] = createSignal<unknown>(null);
  const [dataError, setDataError] = createSignal<string | null>(null);
  const [dataLoading, setDataLoading] = createSignal(false);
  let unlisten: UnlistenFn | undefined;

  const updateData = (value: unknown): void => {
    try {
      setData(applyTransform(value, dataSource()?.transform));
      setDataError(null);
    } catch (error) {
      setDataError(error instanceof Error ? error.message : String(error));
    }
  };

  onMount(() => {
    const source = dataSource();
    if (!source) return;
    if (source.kind === 'stream') {
      // 后端当前没有 extension stream 订阅命令（无 stream/subscribeSource），
      // 前端 runChannelStream/useChannel 只能驱动声明了 Channel 参数的后端命令，
      // 故此处 fail-closed：明确报错而非调用不存在的命令。
      setDataError('stream data source is unavailable: the backend exposes no extension stream command; host panel fails closed until one ships');
      return;
    }
    if (source.kind === 'event') {
      if (!source.pattern) {
        setDataError('event data source requires pattern');
        return;
      }
      void listen<unknown>(source.pattern, (event) => updateData(event.payload))
        .then((stop) => { unlisten = stop; })
        .catch((error) => setDataError(error instanceof Error ? error.message : String(error)));
      return;
    }
    if (!source.key) {
      setDataError('storage data source requires key');
      return;
    }
    setDataLoading(true);
    void invoke<{ value?: unknown }>('ui_extension_storage_get', {
      request: {
        extensionId: props.view.extensionId,
        scope: source.scope ?? 'global',
        key: source.key,
        worktree: source.worktree,
      },
    })
      .then((response) => updateData(response.value))
      .catch((error) => setDataError(error instanceof Error ? error.message : String(error)))
      .finally(() => setDataLoading(false));
  });

  onCleanup(() => unlisten?.());

  return (
    <div class="navis-host-panel">
      <section class="navis-host-view-section">
        <div class="navis-host-view-section-title">{props.view.name}</div>
        <p>{props.view.extensionDescription || `${props.view.extensionName} contributes this Navis Go host panel.`}</p>
        <button type="button" class="navis-host-view-inline-action" onClick={() => void openSettingsDialog('extensions')}>
          Manage extension
        </button>
      </section>

      <section class="navis-host-view-section">
        <div class="navis-host-view-section-title">Extension</div>
        <p>{props.view.extensionName}</p>
        <p>{props.view.extensionId}</p>
      </section>

      <section class="navis-host-view-section">
        <div class="navis-host-view-section-title">View contract</div>
        <p>{props.view.viewId}</p>
        <p>{props.view.zone || props.view.placement}</p>
        <p>{props.view.renderer}</p>
        <Show when={props.surface}><p>Surface {props.surface}</p></Show>
      </section>

      <Show when={dataSource()}>
        {(source) => (
          <section class="navis-host-view-section">
            <div class="navis-host-view-section-title">Live data</div>
            <p>{source().kind}{source().pattern ? ` · ${source().pattern}` : ''}</p>
            <Show when={dataLoading()}><p>Loading…</p></Show>
            <Show when={dataError()} fallback={<pre class="max-h-48 overflow-auto rounded bg-[#f5f5f5] p-2 text-[11px]">{formatData(data())}</pre>}>
              {(error) => <p class="text-red-600">{error()}</p>}
            </Show>
          </section>
        )}
      </Show>

      <section class="navis-host-view-section">
        <div class="navis-host-view-section-title">Contributions</div>
        <Show when={countLines().length > 0} fallback={<p>This extension has no additional registered contributions.</p>}>
          <For each={countLines()}>{(line) => <p>{line}</p>}</For>
        </Show>
      </section>
    </div>
  );
};

export default HostPanelRenderer;