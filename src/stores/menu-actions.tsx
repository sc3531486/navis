import { invoke } from '@tauri-apps/api/core';
import { openSettingsDialog } from '../components/Settings/openSettingsDialog';
import { openExtensionDialog, extensionDialogForView, closeExtensionDialog, parseExtensionDialogOptions } from '../components/ExtensionDialog/store';
import { spawnExtensionScriptWorker } from './extension-workers';
import {
  getHostViewRendererDescriptor,
  getHostViewSurfaceDescriptor,
  type HostViewSurfaceKind,
} from '../components/HostView/registry';
import {
  appState,
  closeHostView,
  closeRightWorkspacePanel,
  focusHostView,
  hostViewInstanceId,
  isHostViewOpen,
  openHostView,
  openRightWorkspacePanel,
} from './app';
import { extensionState } from './extension';
import { viewZone, type UiExtensionScript, type UiExtensionView } from '../lib/extension-ui';
import type { MenuActionItem, MenuBuiltinAction } from './menu';

interface HostViewSurfaceOperations {
  open: (item: MenuActionItem) => boolean;
  isOpen: (item: MenuActionItem) => boolean;
  close: (item: MenuActionItem) => boolean;
}

type ActionHandler = (item: MenuActionItem) => boolean;

const actionRegistry = new Map<MenuBuiltinAction['type'], ActionHandler>();

function hostViewAction(item: MenuActionItem): Extract<MenuBuiltinAction, { view: unknown }> | null {
  const action = item.action;
  if (!action) return null;
  return 'view' in action ? action : null;
}

function hostViewItemId(item: MenuActionItem): string | null {
  const action = hostViewAction(item);
  if (!item.extensionId || !action) return null;
  return hostViewInstanceId(item.extensionId, action.view.viewId);
}

function extensionViewFor(item: MenuActionItem): UiExtensionView | null {
  const action = hostViewAction(item);
  if (!item.extensionId || !action) return null;
  const extension = extensionState.extensions.find((candidate) => candidate.id === item.extensionId);
  if (!extension) return null;

  return {
    extensionId: item.extensionId,
    extensionName: extension.name,
    extensionDescription: extension.description,
    ...action.view,
    zone: action.view.zone || action.view.placement,
    placement: action.view.zone || action.view.placement,
    contributionCounts: extension.contributionCounts,
  };
}

function openRightWorkspaceHostView(item: MenuActionItem): boolean {
  const view = extensionViewFor(item);
  if (!view) return false;

  openRightWorkspacePanel({
    id: hostViewInstanceId(view.extensionId, view.viewId),
    title: view.name,
    viewId: view.viewId,
    config: view.config,
    extensionView: view,
  });
  return true;
}

function rightWorkspaceHostViewIsOpen(item: MenuActionItem): boolean {
  const instanceId = hostViewItemId(item);
  if (!instanceId) return false;
  return appState.rightWorkspaceColumns.some((column) =>
    column.panels.some((panel) => panel.id === instanceId),
  );
}

function closeRightWorkspaceHostView(item: MenuActionItem): boolean {
  const instanceId = hostViewItemId(item);
  if (!instanceId) return false;
  closeRightWorkspacePanel(instanceId);
  return true;
}

function openInlineHostSurface(item: MenuActionItem): boolean {
  const view = extensionViewFor(item);
  if (!view) return false;

  const id = hostViewInstanceId(view.extensionId, view.viewId);
  openHostView({ id, ...view });
  focusHostView(id);
  return true;
}

function closeInlineHostSurface(item: MenuActionItem): boolean {
  const instanceId = hostViewItemId(item);
  if (!instanceId) return false;
  closeHostView(instanceId);
  return true;
}

function inlineHostSurfaceIsOpen(item: MenuActionItem): boolean {
  const action = hostViewAction(item);
  return isHostViewOpen(item.extensionId ?? '', action?.view.viewId ?? '');
}

function openSettingsHostSurface(item: MenuActionItem): boolean {
  if (!openInlineHostSurface(item)) return false;
  void openSettingsDialog('extensions');
  return true;
}

const inlineHostSurface: HostViewSurfaceOperations = {
  open: openInlineHostSurface,
  isOpen: inlineHostSurfaceIsOpen,
  close: closeInlineHostSurface,
};

const HOST_VIEW_SURFACE_OPERATIONS: Readonly<Record<HostViewSurfaceKind, HostViewSurfaceOperations>> = {
  rightWorkspace: {
    open: openRightWorkspaceHostView,
    isOpen: rightWorkspaceHostViewIsOpen,
    close: closeRightWorkspaceHostView,
  },
  inline: inlineHostSurface,
  settings: {
    open: openSettingsHostSurface,
    isOpen: inlineHostSurfaceIsOpen,
    close: closeInlineHostSurface,
  },
  dialog: {
    open: openDialogHostSurface,
    isOpen: (item) => {
      const view = extensionViewFor(item);
      return view ? Boolean(extensionDialogForView(view.extensionId, view.viewId)) : false;
    },
    close: (item) => {
      const view = extensionViewFor(item);
      const entry = view ? extensionDialogForView(view.extensionId, view.viewId) : undefined;
      if (!entry) return false;
      closeExtensionDialog(entry.id);
      return true;
    },
  },
};

export function canDispatchHostViewZone(zone: string): boolean {
  return getHostViewSurfaceDescriptor(zone) !== undefined;
}

/** Deprecated compatibility wrapper. */
export function canDispatchHostViewPlacement(placement: string): boolean {
  return canDispatchHostViewZone(placement);
}

function hostViewSurfaceFor(item: MenuActionItem): HostViewSurfaceOperations | undefined {
  const action = hostViewAction(item);
  if (!item.extensionId || !action) return undefined;
  const descriptor = getHostViewSurfaceDescriptor(action.view.zone || action.view.placement);
  return descriptor ? HOST_VIEW_SURFACE_OPERATIONS[descriptor.kind] : undefined;
}

function canDispatchHostViewAction(action: Extract<MenuBuiltinAction, { view: unknown }>): boolean {
  return getHostViewRendererDescriptor(action.view.renderer) !== undefined
    && canDispatchHostViewZone(action.view.zone || action.view.placement);
}

function openDialogHostSurface(item: MenuActionItem): boolean {
  const view = extensionViewFor(item);
  if (!view || item.action?.type !== 'OpenDialog') return false;
  openExtensionDialog(view, parseExtensionDialogOptions({
    size: item.action.size,
    position: item.action.position,
    modal: item.action.modal,
  }));
  return true;
}

function runExtensionScript(item: MenuActionItem): boolean {
  if (!item.extensionId || item.action?.type !== 'RunScript') return false;
  const { extensionId } = item;
  const { scriptId, args } = item.action;
  void invoke<UiExtensionScript[]>('ui_list_extension_scripts')
    .then((scripts) => {
      const script = scripts.find((candidate) => candidate.extensionId === extensionId && candidate.scriptId === scriptId);
      if (!script?.resourcePath) throw new Error(`Extension script '${scriptId}' has no resource path`);
      const handle = spawnExtensionScriptWorker(script, { args });
      if (!handle) throw new Error(`Extension script '${scriptId}' has no resource path`);
    })
    .catch((error) => console.warn('[ExtensionScript] failed to run script', error));
  return true;
}

function sendExtensionMessage(item: MenuActionItem): boolean {
  if (!item.extensionId || item.action?.type !== 'SendMessage') return false;
  void invoke('ui_extension_route_call', {
    request: {
      callerExtensionId: item.extensionId,
      target: item.action.target,
      action: 'message.send',
      payload: item.action.payload ?? {},
    },
  }).catch((error) => console.warn('[ExtensionRoute] message rejected', error));
  return true;
}

export function registerDeclarativeAction(type: MenuBuiltinAction['type'], handler: ActionHandler): () => void {
  const previous = actionRegistry.get(type);
  actionRegistry.set(type, handler);
  return () => {
    if (previous) actionRegistry.set(type, previous);
    else actionRegistry.delete(type);
  };
}

registerDeclarativeAction('OpenView', (item) => hostViewSurfaceFor(item)?.open(item) ?? false);
registerDeclarativeAction('ToggleView', (item) => {
  const surface = hostViewSurfaceFor(item);
  return surface ? (surface.isOpen(item) ? surface.close(item) : surface.open(item)) : false;
});
registerDeclarativeAction('OpenDialog', openDialogHostSurface);
registerDeclarativeAction('RunScript', runExtensionScript);
registerDeclarativeAction('SendMessage', sendExtensionMessage);

export function canDispatchDeclarativeMenuAction(item: MenuActionItem): boolean {
  if (!item.extensionId || !item.action) return true;
  if (!actionRegistry.has(item.action.type)) return false;
  return 'view' in item.action ? canDispatchHostViewAction(item.action) : true;
}

export function executeDeclarativeMenuAction(item: MenuActionItem): boolean {
  if (!item.extensionId || !item.action) return false;
  return actionRegistry.get(item.action.type)?.(item) ?? false;
}
