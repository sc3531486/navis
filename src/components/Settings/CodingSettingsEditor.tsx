import { Component, For, Show, createEffect, createSignal, onMount } from 'solid-js';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  loadEditorSettings,
  resetEditorSettingsDraft,
  saveEditorSettings,
  settingsState,
  updateEditorSettings,
  type ExternalEditorConfig,
  type EditorWordWrap,
} from '../../stores/settings';

const DEFAULT_EXTERNAL_EDITOR_NAME = 'Coding Editor';

const wrapOptions: Array<{ value: EditorWordWrap; label: string; description: string }> = [
  {
    value: 'on',
    label: 'On',
    description: 'Wrap long lines inside the Coding Editor viewport.',
  },
  {
    value: 'off',
    label: 'Off',
    description: 'Keep long lines on a single horizontal row.',
  },
];

const tabSizeOptions = [2, 4];
const fontSizeOptions = [12, 13, 14, 16, 18];

function newExternalEditorId(): string {
  return `external-editor-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function selectedPath(result: string | string[] | null): string | null {
  if (!result) return null;
  return Array.isArray(result) ? result[0] ?? null : result;
}

function fileNameFromPath(path: string): string {
  return path.split(/[\\/]+/).filter(Boolean).at(-1)?.replace(/\.(exe|cmd|bat|app)$/i, '') ?? DEFAULT_EXTERNAL_EDITOR_NAME;
}

const CodingSettingsEditor: Component = () => {
  const [status, setStatus] = createSignal('');
  const [statusTone, setStatusTone] = createSignal<'neutral' | 'success' | 'error'>('neutral');

  onMount(() => {
    if (!settingsState.loaded) void loadEditorSettings();
  });

  createEffect(() => {
    if (settingsState.isDirty && statusTone() !== 'error') {
      setStatus('');
      setStatusTone('neutral');
    }
  });

  async function save(): Promise<void> {
    setStatus('Saving Coding Editor settings');
    setStatusTone('neutral');

    try {
      await saveEditorSettings();
      setStatus('Coding Editor settings saved');
      setStatusTone('success');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
      setStatusTone('error');
    }
  }

  function setFontSize(value: number): void {
    updateEditorSettings({ fontSize: value });
  }

  function setTabSize(value: number): void {
    updateEditorSettings({ tabSize: value });
  }

  function updateExternalEditors(
    updater: (editors: ExternalEditorConfig[], defaultId: string | null) => {
      editors: ExternalEditorConfig[];
      defaultId: string | null;
    },
  ): void {
    const result = updater(
      settingsState.editor.externalEditors.map((editor) => ({ ...editor })),
      settingsState.editor.defaultExternalEditorId,
    );
    updateEditorSettings({
      externalEditors: result.editors,
      defaultExternalEditorId: result.defaultId,
    });
  }

  function addExternalEditor(): void {
    const id = newExternalEditorId();
    updateExternalEditors((editors, defaultId) => ({
      editors: [
        ...editors,
        {
          id,
          name: DEFAULT_EXTERNAL_EDITOR_NAME,
          path: '',
          isDefault: editors.length === 0,
        },
      ],
      defaultId: defaultId ?? id,
    }));
  }

  function patchExternalEditor(editorId: string, patch: Partial<ExternalEditorConfig>): void {
    updateExternalEditors((editors, defaultId) => ({
      editors: editors.map((editor) =>
        editor.id === editorId
          ? { ...editor, ...patch }
          : editor,
      ),
      defaultId,
    }));
  }

  function removeExternalEditor(editorId: string): void {
    updateExternalEditors((editors, defaultId) => {
      const nextEditors = editors.filter((editor) => editor.id !== editorId);
      const nextDefaultId = defaultId === editorId ? nextEditors[0]?.id ?? null : defaultId;
      return {
        editors: nextEditors.map((editor) => ({ ...editor, isDefault: editor.id === nextDefaultId })),
        defaultId: nextDefaultId,
      };
    });
  }

  function setDefaultExternalEditor(editorId: string): void {
    updateExternalEditors((editors) => ({
      editors: editors.map((editor) => ({ ...editor, isDefault: editor.id === editorId })),
      defaultId: editorId,
    }));
  }

  async function chooseExternalEditorPath(editorId: string): Promise<void> {
    const result = await openDialog({
      multiple: false,
      directory: false,
      title: 'Choose programming tool executable',
    });
    const path = selectedPath(result);
    if (!path) return;
    const editor = settingsState.editor.externalEditors.find((item) => item.id === editorId);
    patchExternalEditor(editorId, {
      path,
      name: editor?.name && !['Editor', DEFAULT_EXTERNAL_EDITOR_NAME].includes(editor.name) ? editor.name : fileNameFromPath(path),
    });
  }

  return (
    <section class="navis-editor-settings-editor">
      <div>
        <div class="navis-settings-section-title">Coding Editor</div>
        <p class="navis-editor-settings-subtitle">
          Configure Navis Go Coding Editor behavior, the right-side File panel, and external programming tools from one place.
        </p>
      </div>

      <Show
        when={!settingsState.loading || settingsState.loaded}
        fallback={<div class="navis-settings-state-warning">Loading Coding Editor settings</div>}
      >
        <div class="navis-editor-settings-grid">
          <article class="navis-editor-settings-card">
            <div class="navis-editor-settings-card-head">
              <div>
                <div class="navis-editor-settings-card-title">Typography</div>
                <div class="navis-editor-settings-card-copy">CodeMirror uses these values immediately in the Coding Editor Worktree.</div>
              </div>
              <span>{settingsState.editor.fontSize}px</span>
            </div>
            <div class="navis-editor-settings-control-stack">
              <label>
                Font size
                <input
                  type="number"
                  min="8"
                  max="32"
                  value={settingsState.editor.fontSize}
                  onInput={(event) => setFontSize(Number(event.currentTarget.value) || 8)}
                />
              </label>
              <div class="navis-editor-settings-chip-row">
                {fontSizeOptions.map((size) => (
                  <button
                    type="button"
                    class={settingsState.editor.fontSize === size ? 'is-active' : ''}
                    onClick={() => setFontSize(size)}
                  >
                    {size}px
                  </button>
                ))}
              </div>
            </div>
          </article>

          <article class="navis-editor-settings-card">
            <div class="navis-editor-settings-card-head">
              <div>
                <div class="navis-editor-settings-card-title">Indentation</div>
                <div class="navis-editor-settings-card-copy">Tab width also updates the Coding Editor indent unit.</div>
              </div>
              <span>{settingsState.editor.tabSize} spaces</span>
            </div>
            <div class="navis-editor-settings-control-stack">
              <label>
                Tab size
                <input
                  type="number"
                  min="1"
                  max="8"
                  value={settingsState.editor.tabSize}
                  onInput={(event) => setTabSize(Number(event.currentTarget.value) || 1)}
                />
              </label>
              <div class="navis-editor-settings-chip-row">
                {tabSizeOptions.map((size) => (
                  <button
                    type="button"
                    class={settingsState.editor.tabSize === size ? 'is-active' : ''}
                    onClick={() => setTabSize(size)}
                  >
                    {size} spaces
                  </button>
                ))}
              </div>
            </div>
          </article>

          <article class="navis-editor-settings-card">
            <div class="navis-editor-settings-card-head">
              <div>
                <div class="navis-editor-settings-card-title">Line layout</div>
                <div class="navis-editor-settings-card-copy">Use a single supported wrap mode until column-based wrapping is implemented end to end.</div>
              </div>
              <span>{settingsState.editor.wordWrap === 'on' ? 'Wrapping' : 'No wrap'}</span>
            </div>
            <div class="navis-editor-settings-segmented" role="tablist" aria-label="Coding Editor word wrap">
              {wrapOptions.map((option) => (
                <button
                  type="button"
                  role="tab"
                  aria-selected={settingsState.editor.wordWrap === option.value}
                  class={settingsState.editor.wordWrap === option.value ? 'is-active' : ''}
                  onClick={() => updateEditorSettings({ wordWrap: option.value })}
                >
                  <strong>{option.label}</strong>
                  <span>{option.description}</span>
                </button>
              ))}
            </div>
          </article>

          <article class="navis-editor-settings-card">
            <div class="navis-editor-settings-card-head">
              <div>
                <div class="navis-editor-settings-card-title">Overview</div>
                <div class="navis-editor-settings-card-copy">Minimap is now a real Coding Editor surface instead of a placeholder toggle.</div>
              </div>
              <span>{settingsState.editor.minimap ? 'Visible' : 'Hidden'}</span>
            </div>
            <button
              type="button"
              class={`navis-editor-settings-toggle ${settingsState.editor.minimap ? 'is-active' : ''}`}
              aria-pressed={settingsState.editor.minimap}
              onClick={() => updateEditorSettings({ minimap: !settingsState.editor.minimap })}
            >
              <span>Show minimap</span>
              <small>{settingsState.editor.minimap ? 'Displayed on the right side of the Coding Editor.' : 'Coding Editor uses the full width.'}</small>
            </button>
          </article>

          <article class="navis-editor-settings-card navis-editor-external-tools-card">
            <div class="navis-editor-settings-card-head">
              <div>
                <div class="navis-editor-settings-card-title">External programming tools</div>
                <div class="navis-editor-settings-card-copy">
                  Open in uses these tools to open the current session Worktree.
                </div>
              </div>
              <button type="button" onClick={addExternalEditor}>
                Add tool
              </button>
            </div>
            <Show
              when={settingsState.editor.externalEditors.length > 0}
              fallback={<div class="navis-editor-external-empty">No external programming tools configured.</div>}
            >
              <div class="navis-editor-external-list">
                <For each={settingsState.editor.externalEditors}>
                  {(editor) => (
                    <div class="navis-editor-external-row">
                      <label>
                        Name
                        <input
                          value={editor.name}
                          placeholder="Zed"
                          onInput={(event) => patchExternalEditor(editor.id, { name: event.currentTarget.value })}
                        />
                      </label>
                      <label class="navis-editor-external-path">
                        Absolute path
                        <input
                          value={editor.path}
                          placeholder="C:\\Program Files\\Zed\\zed.exe"
                          onInput={(event) => patchExternalEditor(editor.id, { path: event.currentTarget.value })}
                        />
                      </label>
                      <button type="button" onClick={() => void chooseExternalEditorPath(editor.id)}>
                        Browse
                      </button>
                      <button
                        type="button"
                        class={editor.isDefault ? 'is-active' : ''}
                        onClick={() => setDefaultExternalEditor(editor.id)}
                      >
                        {editor.isDefault ? 'Default' : 'Set default'}
                      </button>
                      <button type="button" class="is-danger" onClick={() => removeExternalEditor(editor.id)}>
                        Remove
                      </button>
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </article>
        </div>

        <div class="navis-editor-settings-footer">
          <div class="navis-editor-settings-status">
            <Show when={settingsState.error}>
              {(message) => <span class="is-error">{message()}</span>}
            </Show>
            <Show when={!settingsState.error && status()}>
              <span class={`is-${statusTone()}`}>{status()}</span>
            </Show>
            <Show when={!settingsState.error && !status() && settingsState.isDirty}>
              <span class="is-neutral">Unsaved Coding Editor changes</span>
            </Show>
          </div>
          <div class="navis-editor-settings-actions">
            <button
              type="button"
              onClick={() => resetEditorSettingsDraft()}
              disabled={!settingsState.isDirty || settingsState.saving}
            >
              Reset
            </button>
            <button
              type="button"
              onClick={() => void save()}
              disabled={!settingsState.isDirty || settingsState.saving}
            >
              {settingsState.saving ? 'Saving' : 'Save Coding Editor'}
            </button>
          </div>
        </div>
      </Show>
    </section>
  );
};

export default CodingSettingsEditor;
