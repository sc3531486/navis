import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import type { StatusPresentation } from '@/lib/status';
import { viewZone, type UiExtensionContributionCounts, type UiExtensionView } from '@/lib/extension-ui';
import { loadMenus } from './menu';
import { loadExtensionCommands } from './extension-commands';
import { loadExtensionKeybindings } from './extension-keybindings';
import {
  hostViewInstanceId,
  isHostViewOpen,
  isRightWorkspacePanelOpen,
  openHostView,
  openRightWorkspacePanel,
  removeHostViewsForExtension,
} from './host';

export interface WorkModeModelPreferences {
  temperature?: number;
  maxTokens?: number;
  extendedThinking?: boolean;
  languageQualityEmphasis?: number;
}

export interface WorkModeRegistration {
  id: string;
  name?: string;
  description?: string;
  icon?: string;
  role?: string;
  availableTools?: string[];
  skills?: string[];
  commands?: string[];
  contextPolicy?: string;
  behaviorRules?: string[];
  entryView?: string;
  defaultViews?: string[];
  defaultModel?: string;
  modelPreferences?: WorkModeModelPreferences;
  capabilities?: string[];
}

export interface RegisteredWorkMode {
  extensionId: string;
  extensionName: string;
  modeId: string;
  runtimeId: string;
  mode: WorkModeRegistration;
}

export interface ExtensionPermissions {
  filesystem: string[];
  terminal: string[];
  network: string[];
  ipc: string[];
  events: string[];
  resources: {
    maxMemoryMb: number;
    maxCpuPercent: number;
    timeoutMs: number;
  };
}

export type ExtensionContributionCounts = UiExtensionContributionCounts;

export type ExtensionStatus =
  | 'installed'
  | 'loading'
  | 'enabled'
  | 'disabling'
  | 'disabled'
  | 'unloading'
  | 'error';

export interface ExtensionRuntimeState {
  id: string;
  status: ExtensionStatus;
  statusPresentation: StatusPresentation;
  name: string;
  version: string;
  description: string;
  author: string;
  installPath: string;
  installedAt: string;
  enabledAt?: string | null;
  error?: string | null;
  permissions: ExtensionPermissions;
  contributionCounts: ExtensionContributionCounts;
  /** 扩展贡献的能力清单（后端 UiExtensionState.provides，camelCase 透传）。 */
  provides: string[];
}

interface ExtensionState {
  extensions: ExtensionRuntimeState[];
  views: UiExtensionView[];
  workModes: RegisteredWorkMode[];
  loaded: boolean;
  loading: boolean;
  error: string | null;
}

export const [extensionState, setExtensionState] = createStore<ExtensionState>({
  extensions: [],
  views: [],
  workModes: [],
  loaded: false,
  loading: false,
  error: null,
});

export function collectDefaultVisibleExtensionViews(views: readonly UiExtensionView[]): UiExtensionView[] {
  return views.filter((view) => view.defaultVisible);
}

function projectDefaultVisibleExtensionViews(): void {
  for (const extension of extensionState.extensions) {
    if (extension.status !== 'enabled') removeHostViewsForExtension(extension.id);
  }

  for (const view of collectDefaultVisibleExtensionViews(extensionState.views)) {
    const id = hostViewInstanceId(view.extensionId, view.viewId);
    if (viewZone(view) === 'rightWorkspace') {
      if (!isRightWorkspacePanelOpen(view.extensionId, view.viewId)) {
        openRightWorkspacePanel({
          id,
          title: view.name,
          viewId: view.viewId,
          config: view.config,
          extensionView: view,
        });
      }
    } else if (!isHostViewOpen(view.extensionId, view.viewId)) {
      openHostView({ id, ...view });
    }
  }
}

function applyExtensionData(
  extensions: ExtensionRuntimeState[],
  views: UiExtensionView[],
  workModes: RegisteredWorkMode[],
): void {
  setExtensionState({
    extensions: extensions.map((extension) => ({ ...extension, provides: extension.provides ?? [] })),
    views,
    workModes,
    loaded: true,
    loading: false,
    error: null,
  });
}
async function loadWorkModes(): Promise<RegisteredWorkMode[]> {
  return invoke<RegisteredWorkMode[]>('ui_list_custom_work_modes');
}

async function loadExtensionViews(): Promise<UiExtensionView[]> {
  return invoke<UiExtensionView[]>('ui_list_extension_views');
}

export async function loadExtensions(): Promise<void> {
  setExtensionState('loading', true);
  setExtensionState('error', null);

  try {
    const [extensions, views, workModes] = await Promise.all([
      invoke<ExtensionRuntimeState[]>('ui_list_extensions'),
      loadExtensionViews(),
      loadWorkModes(),
    ]);
    applyExtensionData(extensions, views, workModes);
    await Promise.all([loadMenus(), loadExtensionCommands(), loadExtensionKeybindings()]);
    projectDefaultVisibleExtensionViews();
  } catch (error) {
    setExtensionState('loading', false);
    setExtensionState('loaded', true);
    setExtensionState('error', error instanceof Error ? error.message : String(error));
  }
}

export async function setExtensionEnabled(extensionId: string, enabled: boolean): Promise<void> {
  setExtensionState('loading', true);
  setExtensionState('error', null);

  try {
    const extensions = await invoke<ExtensionRuntimeState[]>('ui_set_extension_enabled', {
      payload: { extensionId, enabled },
    });
    const [views, workModes] = await Promise.all([loadExtensionViews(), loadWorkModes()]);
    applyExtensionData(extensions, views, workModes);
    if (!enabled) {
      removeHostViewsForExtension(extensionId);
    }
    await Promise.all([
      loadMenus(),
      loadExtensionCommands(),
      loadExtensionKeybindings(),
    ]);
    projectDefaultVisibleExtensionViews();
  } catch (error) {
    setExtensionState('loading', false);
    setExtensionState('error', error instanceof Error ? error.message : String(error));
  }
}

export async function installExtension(sourcePath: string): Promise<void> {
  setExtensionState('loading', true);
  setExtensionState('error', null);

  try {
    const extensions = await invoke<ExtensionRuntimeState[]>('ui_install_extension', {
      payload: { sourcePath },
    });
    const [views, workModes] = await Promise.all([loadExtensionViews(), loadWorkModes()]);
    applyExtensionData(extensions, views, workModes);
    await Promise.all([
      loadMenus(),
      loadExtensionCommands(),
      loadExtensionKeybindings(),
    ]);
    projectDefaultVisibleExtensionViews();
  } catch (error) {
    setExtensionState('loading', false);
    setExtensionState('error', error instanceof Error ? error.message : String(error));
  }
}

export async function uninstallExtension(extensionId: string): Promise<void> {
  setExtensionState('loading', true);
  setExtensionState('error', null);

  try {
    const extensions = await invoke<ExtensionRuntimeState[]>('ui_uninstall_extension', {
      payload: { extensionId },
    });
    const [views, workModes] = await Promise.all([loadExtensionViews(), loadWorkModes()]);
    applyExtensionData(extensions, views, workModes);
    removeHostViewsForExtension(extensionId);
    await Promise.all([
      loadMenus(),
      loadExtensionCommands(),
      loadExtensionKeybindings(),
    ]);
    projectDefaultVisibleExtensionViews();
  } catch (error) {
    setExtensionState('loading', false);
    setExtensionState('error', error instanceof Error ? error.message : String(error));
  }
}

export function setWorkModes(workModes: RegisteredWorkMode[]): void {
  setExtensionState('workModes', workModes);
}

export function getWorkModeDisplayName(workMode: RegisteredWorkMode): string {
  return workMode.mode.name ?? workMode.extensionName;
}
