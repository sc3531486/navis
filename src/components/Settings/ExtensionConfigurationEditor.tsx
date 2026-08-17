import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Component, For, Show, createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import type { UiExtensionPointRegistration } from '../../lib/extension-ui';

type JsonObject = Record<string, unknown>;
type SchemaNode = JsonObject & { properties?: JsonObject; required?: string[]; enum?: unknown[]; type?: string; title?: string; description?: string; default?: unknown };
interface ExtensionConfigurationResponse {
  extensionId: string;
  schema: unknown;
  value: unknown;
}

function asSchema(value: unknown): SchemaNode {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as SchemaNode : {};
}

function propertiesOf(schema: SchemaNode): Array<[string, SchemaNode]> {
  return schema.properties && typeof schema.properties === 'object'
    ? Object.entries(schema.properties).filter(([, value]) => value && typeof value === 'object').map(([key, value]) => [key, asSchema(value)])
    : [];
}

function jsonText(value: unknown): string {
  try { return JSON.stringify(value, null, 2) ?? ''; } catch { return ''; }
}

const ExtensionConfigurationEditor: Component<{ point: UiExtensionPointRegistration }> = (props) => {
  const [response, setResponse] = createSignal<ExtensionConfigurationResponse | null>(null);
  const [draft, setDraft] = createSignal<JsonObject>({});
  const [loading, setLoading] = createSignal(true);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [saved, setSaved] = createSignal(false);
  let unlisten: UnlistenFn | undefined;

  const schema = createMemo(() => asSchema(response()?.schema ?? props.point.data));
  const properties = createMemo(() => propertiesOf(schema()));
  const isFlatSchema = () => properties().length > 0;

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await invoke<ExtensionConfigurationResponse>('ui_get_extension_config', { extensionId: props.point.extensionId });
      setResponse(next);
      setDraft(next.value && typeof next.value === 'object' && !Array.isArray(next.value) ? next.value as JsonObject : {});
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    void load();
    void listen<ExtensionConfigurationResponse>('extension.config.updated', (event) => {
      if (event.payload.extensionId === props.point.extensionId) {
        setResponse(event.payload);
        setDraft(event.payload.value && typeof event.payload.value === 'object' && !Array.isArray(event.payload.value) ? event.payload.value as JsonObject : {});
      }
    }).then((stop) => { unlisten = stop; });
  });
  onCleanup(() => unlisten?.());

  const setField = (key: string, value: unknown) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setSaved(false);
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      const next = await invoke<ExtensionConfigurationResponse>('ui_set_extension_config', {
        update: { extensionId: props.point.extensionId, value: draft() },
      });
      setResponse(next);
      setDraft(next.value && typeof next.value === 'object' && !Array.isArray(next.value) ? next.value as JsonObject : {});
      setSaved(true);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="navis-settings-section">
      <div class="navis-settings-section-title">{props.point.label ?? props.point.extensionId}</div>
      <Show when={loading()}><div class="navis-settings-item-meta">Loading configuration…</div></Show>
      <Show when={error()}>{(message) => <div class="navis-settings-state-warning">{message()}</div>}</Show>
      <Show when={!loading() && !error()}>
        <Show when={isFlatSchema()} fallback={
          <textarea
            class="min-h-28 w-full rounded border border-[#d7d7d7] bg-white p-2 font-mono text-[11px]"
            aria-label={`${props.point.extensionId} configuration JSON`}
            value={jsonText(response()?.value)}
            onInput={(event) => {
              try {
                const value = JSON.parse(event.currentTarget.value);
                if (value && typeof value === 'object' && !Array.isArray(value)) setDraft(value as JsonObject);
                setError(null);
              } catch { setError('Configuration must be valid JSON'); }
            }}
          />
        }>
          <div class="space-y-2">
            <For each={properties()}>
              {([key, field]) => {
                const label = field.title ?? key;
                const value = () => draft()[key] ?? field.default ?? '';
                return (
                  <label class="block text-[11px] text-[#444]">
                    <span class="mb-1 block font-medium">{label}{schema().required?.includes(key) ? ' *' : ''}</span>
                    <Show when={field.description}><span class="mb-1 block text-[10px] text-[#777]">{field.description}</span></Show>
                    <Show when={field.enum?.length}>
                      <select class="w-full rounded border border-[#d7d7d7] bg-white px-2 py-1 text-[11px]" value={String(value())} onChange={(event) => setField(key, event.currentTarget.value)}>
                        <For each={field.enum ?? []}>{(option) => <option value={String(option)}>{String(option)}</option>}</For>
                      </select>
                    </Show>
                    <Show when={!field.enum?.length && field.type === 'boolean'}>
                      <input type="checkbox" checked={Boolean(value())} onChange={(event) => setField(key, event.currentTarget.checked)} />
                    </Show>
                    <Show when={!field.enum?.length && (field.type === 'number' || field.type === 'integer')}>
                      <input class="w-full rounded border border-[#d7d7d7] px-2 py-1 text-[11px]" type="number" value={String(value())} onInput={(event) => setField(key, field.type === 'integer' ? Number.parseInt(event.currentTarget.value, 10) : Number(event.currentTarget.value))} />
                    </Show>
                    <Show when={!field.enum?.length && field.type === 'string'}>
                      <input class="w-full rounded border border-[#d7d7d7] px-2 py-1 text-[11px]" type="text" value={String(value())} onInput={(event) => setField(key, event.currentTarget.value)} />
                    </Show>
                    <Show when={!field.enum?.length && !['boolean', 'number', 'integer', 'string'].includes(field.type ?? '')}>
                      <textarea class="min-h-20 w-full rounded border border-[#d7d7d7] bg-white p-2 font-mono text-[11px]" value={jsonText(value())} onInput={(event) => { try { setField(key, JSON.parse(event.currentTarget.value)); setError(null); } catch { setError(`${key} must be valid JSON`); } }} />
                    </Show>
                  </label>
                );
              }}
            </For>
          </div>
        </Show>
        <div class="mt-2 flex items-center gap-2">
          <button type="button" class="rounded border border-[#c8c8c8] bg-[#fafafa] px-2 py-1 text-[11px] hover:bg-[#f0f0f0] disabled:opacity-50" disabled={saving()} onClick={() => void save()}>{saving() ? 'Saving…' : 'Save'}</button>
          <Show when={saved()}><span class="text-[10px] text-green-700">Saved</span></Show>
        </div>
      </Show>
    </div>
  );
};

export default ExtensionConfigurationEditor;
