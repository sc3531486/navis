import { invoke } from '@tauri-apps/api/core'
import { createStore } from 'solid-js/store'

export type EditorWordWrap = 'off' | 'on'

export interface ExternalEditorConfig {
  id: string
  name: string
  path: string
  isDefault: boolean
}

export interface EditorSettings {
  fontSize: number
  tabSize: number
  wordWrap: EditorWordWrap
  minimap: boolean
  formatOnSave: boolean
  externalEditors: ExternalEditorConfig[]
  defaultExternalEditorId: string | null
  toolPermissions: ToolPermissionRule[]
}

export type ToolPermissionKey =
  | 'read'
  | 'edit'
  | 'glob'
  | 'grep'
  | 'list'
  | 'bash'
  | 'todo'
  | 'task'
  | 'skill'
  | 'lsp'
  | 'webfetch'
  | 'websearch'
  | 'external_directory'
  | 'browser'

export type ToolPermissionAction = 'allow' | 'ask' | 'deny'

export interface ToolPermissionRule {
  permission: ToolPermissionKey
  pattern: string
  suggest: ToolPermissionAction
  autoEdit: ToolPermissionAction
  fullAuto: ToolPermissionAction
}

interface SettingsState {
  editor: EditorSettings
  savedEditor: EditorSettings
  loaded: boolean
  loading: boolean
  saving: boolean
  isDirty: boolean
  error: string | null
}

export const TOOL_PERMISSION_KEYS: ToolPermissionKey[] = [
  'read',
  'edit',
  'glob',
  'grep',
  'list',
  'bash',
  'todo',
  'task',
  'skill',
  'lsp',
  'webfetch',
  'websearch',
  'external_directory',
  'browser',
]

const toolPermissionKeySet = new Set<string>(TOOL_PERMISSION_KEYS)
const toolPermissionActions = new Set<string>(['allow', 'ask', 'deny'])

function defaultToolPermissionRules(): ToolPermissionRule[] {
  return TOOL_PERMISSION_KEYS.map((permission) => {
    if (permission === 'read' || permission === 'glob' || permission === 'grep' || permission === 'list' || permission === 'todo') {
      return { permission, pattern: '*', suggest: 'allow', autoEdit: 'allow', fullAuto: 'allow' }
    }
    if (permission === 'edit') {
      return { permission, pattern: '*', suggest: 'ask', autoEdit: 'allow', fullAuto: 'allow' }
    }
    if (permission === 'external_directory' || permission === 'browser') {
      return { permission, pattern: '*', suggest: 'ask', autoEdit: 'ask', fullAuto: 'ask' }
    }
    return { permission, pattern: '*', suggest: 'ask', autoEdit: 'ask', fullAuto: 'allow' }
  })
}

function normalizeToolPermissionKey(value: string | null | undefined): ToolPermissionKey | null {
  const normalized = value?.trim().toLowerCase().replace(/-/g, '_') ?? ''
  return toolPermissionKeySet.has(normalized) ? normalized as ToolPermissionKey : null
}

function normalizeToolPermissionAction(value: string | null | undefined, fallback: ToolPermissionAction): ToolPermissionAction {
  const normalized = value?.trim().toLowerCase() ?? ''
  return toolPermissionActions.has(normalized) ? normalized as ToolPermissionAction : fallback
}

function normalizeToolPermissionRules(value: ToolPermissionRule[] | null | undefined): ToolPermissionRule[] {
  const defaults = defaultToolPermissionRules()
  const defaultByPermission = new Map(defaults.map((rule) => [rule.permission, rule]))
  const byPermission = new Map<ToolPermissionKey, ToolPermissionRule>()

  for (const source of value ?? []) {
    const permission = normalizeToolPermissionKey(source.permission)
    if (!permission || byPermission.has(permission)) continue
    const fallback = defaultByPermission.get(permission)!
    byPermission.set(permission, {
      permission,
      pattern: source.pattern?.trim() || '*',
      suggest: normalizeToolPermissionAction(source.suggest, fallback.suggest),
      autoEdit: normalizeToolPermissionAction(source.autoEdit, fallback.autoEdit),
      fullAuto: normalizeToolPermissionAction(source.fullAuto, fallback.fullAuto),
    })
  }

  return defaults.map((fallback) => byPermission.get(fallback.permission) ?? fallback)
}

const defaultEditorSettings: EditorSettings = {
  fontSize: 14,
  tabSize: 2,
  wordWrap: 'on',
  minimap: true,
  formatOnSave: true,
  externalEditors: [],
  defaultExternalEditorId: null,
  toolPermissions: defaultToolPermissionRules(),
}

const defaultSettingsState: SettingsState = {
  editor: { ...defaultEditorSettings },
  savedEditor: { ...defaultEditorSettings },
  loaded: false,
  loading: false,
  saving: false,
  isDirty: false,
  error: null,
}

function normalizeExternalEditors(
  editors: ExternalEditorConfig[] | null | undefined,
  defaultExternalEditorId: string | null | undefined,
): { externalEditors: ExternalEditorConfig[]; defaultExternalEditorId: string | null } {
  const seen = new Set<string>()
  const externalEditors = (editors ?? [])
    .map((editor) => ({
      id: editor.id.trim(),
      name: editor.name.trim(),
      path: editor.path.trim(),
      isDefault: Boolean(editor.isDefault),
    }))
    .filter((editor) => {
      if (!editor.id || !editor.name || !editor.path || seen.has(editor.id)) return false
      seen.add(editor.id)
      return true
    })

  const requestedDefault =
    defaultExternalEditorId?.trim() ||
    externalEditors.find((editor) => editor.isDefault)?.id ||
    null
  const resolvedDefault = requestedDefault && externalEditors.some((editor) => editor.id === requestedDefault)
    ? requestedDefault
    : null

  return {
    externalEditors: externalEditors.map((editor) => ({
      ...editor,
      isDefault: resolvedDefault === editor.id,
    })),
    defaultExternalEditorId: resolvedDefault,
  }
}

function normalizeEditorSettings(value: Partial<EditorSettings> | null | undefined): EditorSettings {
  const fontSize = Number(value?.fontSize)
  const tabSize = Number(value?.tabSize)
  const external = normalizeExternalEditors(value?.externalEditors, value?.defaultExternalEditorId)

  return {
    fontSize: Number.isFinite(fontSize) ? Math.min(32, Math.max(8, Math.round(fontSize))) : defaultEditorSettings.fontSize,
    tabSize: Number.isFinite(tabSize) ? Math.min(8, Math.max(1, Math.round(tabSize))) : defaultEditorSettings.tabSize,
    wordWrap: value?.wordWrap === 'off' ? 'off' : 'on',
    minimap: typeof value?.minimap === 'boolean' ? value.minimap : defaultEditorSettings.minimap,
    formatOnSave: typeof value?.formatOnSave === 'boolean' ? value.formatOnSave : defaultEditorSettings.formatOnSave,
    externalEditors: external.externalEditors,
    defaultExternalEditorId: external.defaultExternalEditorId,
    toolPermissions: normalizeToolPermissionRules(value?.toolPermissions),
  }
}

function sameEditorSettings(left: EditorSettings, right: EditorSettings): boolean {
  return left.fontSize === right.fontSize
    && left.tabSize === right.tabSize
    && left.wordWrap === right.wordWrap
    && left.minimap === right.minimap
    && left.formatOnSave === right.formatOnSave
    && left.defaultExternalEditorId === right.defaultExternalEditorId
    && JSON.stringify(left.externalEditors) === JSON.stringify(right.externalEditors)
    && JSON.stringify(left.toolPermissions) === JSON.stringify(right.toolPermissions)
}

export const [settingsState, setSettingsState] = createStore<SettingsState>({
  ...defaultSettingsState,
})

export async function loadEditorSettings(force = false): Promise<EditorSettings> {
  if (settingsState.loading) {
    return settingsState.editor
  }

  if (settingsState.loaded && !force) {
    return settingsState.editor
  }

  setSettingsState({
    loading: true,
    error: null,
  })

  try {
    const payload = await invoke<EditorSettings>('ui_get_editor_settings')
    const normalized = normalizeEditorSettings(payload)
    setSettingsState({
      editor: normalized,
      savedEditor: normalized,
      loaded: true,
      loading: false,
      saving: false,
      isDirty: false,
      error: null,
    })
    return normalized
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    setSettingsState({
      loaded: true,
      loading: false,
      error: message,
    })
    throw error
  }
}

export function updateEditorSettings(partial: Partial<EditorSettings>): void {
  const nextEditor = normalizeEditorSettings({
    ...settingsState.editor,
    ...partial,
  })

  setSettingsState({
    editor: nextEditor,
    isDirty: !sameEditorSettings(nextEditor, settingsState.savedEditor),
    error: null,
  })
}

export function resetEditorSettingsDraft(): void {
  const nextEditor = normalizeEditorSettings(settingsState.savedEditor)
  setSettingsState({
    editor: nextEditor,
    isDirty: false,
    error: null,
  })
}

export async function saveEditorSettings(): Promise<EditorSettings> {
  const payload = normalizeEditorSettings(settingsState.editor)
  setSettingsState({
    saving: true,
    error: null,
  })

  try {
    const saved = normalizeEditorSettings(
      await invoke<EditorSettings>('ui_save_editor_settings', { payload }),
    )
    setSettingsState({
      editor: saved,
      savedEditor: saved,
      loaded: true,
      loading: false,
      saving: false,
      isDirty: false,
      error: null,
    })
    return saved
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    setSettingsState({
      saving: false,
      error: message,
    })
    throw error
  }
}

export async function openSessionExternalEditor(sessionId: string, editorId: string): Promise<void> {
  await invoke('ui_open_session_external_editor', {
    payload: {
      sessionId,
      editorId,
    },
  })
}

export function resetSettingsState(): void {
  setSettingsState({ ...defaultSettingsState })
}
