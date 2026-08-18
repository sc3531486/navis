import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { openSettingsDialog } from '@settings-ext/components/Settings/openSettingsDialog';
import { commandPaletteAPI } from '@navis-code/components/CommandPalette/store';
import { executeDeclarativeMenuAction } from '@/stores/menu-actions';
import type { MenuActionItem } from '@/stores/menu';
import {
  setPendingStartModelSelection,
  setPendingStartPermissionPolicy,
  setPendingStartReasoningEffort,
} from '@navis-code/stores/product-app';
import { gatewayState, loadGatewayCatalog, type GatewayModel, type GatewayModelSelection } from '@project-ext/stores/gateway';
import { setSessionModelSelection, setSessionPermissionPolicy, setSessionReasoningEffort, type ReasoningEffort } from '@session/stores/session-tree';

export const PERMISSION_OPTIONS = [
  { value: 'suggest', label: 'Ask for approval' },
  { value: 'auto-edit', label: 'Review risk only' },
  { value: 'full-auto', label: 'Full access' },
] as const;

export const REASONING_EFFORT_OPTIONS: Array<{ value: ReasoningEffort; label: string; shortcut: string }> = [
  { value: 'low', label: 'Low', shortcut: 'L' },
  { value: 'medium', label: 'Medium', shortcut: 'M' },
  { value: 'high', label: 'High', shortcut: 'H' },
  { value: 'extra-high', label: 'Extra high', shortcut: 'E' },
  { value: 'max', label: 'Max', shortcut: 'X' },
];

export const BUILTIN_INPUT_PLUS_COMMANDS = new Set([
  'composer.addFiles',
  'composer.addFolder',
  'composer.insertSlashCommand',
  'composer.addConnectors',
  'composer.addExtensions',
  'composer.togglePlanMode',
  'composer.toggleMultiAgent',
  'composer.toggleGoalTracking',
]);

export type PermissionPolicy = (typeof PERMISSION_OPTIONS)[number]['value'];

const permissionValues = new Set<string>(PERMISSION_OPTIONS.map((option) => option.value));

interface ExecuteComposerInputPlusOptions {
  onInsertReferences: (kind: 'file' | 'folder', paths: string[]) => void;
  onTogglePlanMode: () => void;
  onToggleMultiAgent: () => void;
  onToggleGoalTracking: () => void;
  onInfo?: (message: string) => void;
}

function selectedPaths(result: string | string[] | null): string[] {
  if (!result) return [];
  return Array.isArray(result) ? result : [result];
}

async function selectFilesForComposer(): Promise<string[]> {
  const result = await openDialog({
    multiple: true,
    directory: false,
    title: 'Add photos and files',
  });

  return selectedPaths(result);
}

async function selectFoldersForComposer(): Promise<string[]> {
  const result = await openDialog({
    multiple: true,
    directory: true,
    title: 'Add folder',
  });

  return selectedPaths(result);
}

function countLabel(count: number, singular: string, plural: string): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

export function permissionLabel(policy: string): string {
  return PERMISSION_OPTIONS.find((option) => option.value === policy)?.label ?? PERMISSION_OPTIONS[0].label;
}

export function normalizePermissionPolicy(policy: string | null | undefined): PermissionPolicy {
  return permissionValues.has(policy ?? '') ? (policy as PermissionPolicy) : 'suggest';
}

export function buildPermissionMenuItems(): MenuActionItem[] {
  return PERMISSION_OPTIONS.map((option) => ({
      id: `composer-permission:${option.value}`,
      label: option.label,
      target: 'InputPlus' as const,
      command: `composer.permission:${option.value}`,
      group: 'permissions',
    }));
}

export function reasoningEffortLabel(value: ReasoningEffort): string {
  return REASONING_EFFORT_OPTIONS.find((option) => option.value === value)?.label ?? 'High';
}

export function modelLabel(models: GatewayModel[], selection: GatewayModelSelection | null): string {
  if (!selection) return 'Select model';
  const model = models.find((item) => item.providerId === selection.providerId && item.id === selection.modelId);
  if (!model) return `${selection.providerId} / ${selection.modelId}`;
  return `${model.providerId} / ${model.name || model.id}`;
}

export function modelNameLabel(models: GatewayModel[], selection: GatewayModelSelection | null): string {
  if (!selection) return 'Select model';
  return models.find((model) => model.providerId === selection.providerId && model.id === selection.modelId)?.name || selection.modelId;
}

export function modelEffortLabel(
  models: GatewayModel[],
  selection: GatewayModelSelection | null,
  currentEffort: ReasoningEffort,
): string {
  return `${modelLabel(models, selection)} · ${reasoningEffortLabel(currentEffort)}`;
}

export async function executeComposerInputPlusItem(
  item: MenuActionItem,
  options: ExecuteComposerInputPlusOptions,
): Promise<boolean> {
  if (executeDeclarativeMenuAction(item)) {
    options.onInfo?.(`Opened ${item.label}`);
    return true;
  }

  switch (item.command) {
    case 'composer.addFiles': {
      const paths = await selectFilesForComposer();
      if (paths.length === 0) return false;
      options.onInsertReferences('file', paths);
      options.onInfo?.(`Inserted ${countLabel(paths.length, 'file reference', 'file references')}`);
      return true;
    }
    case 'composer.addFolder': {
      const paths = await selectFoldersForComposer();
      if (paths.length === 0) return false;
      options.onInsertReferences('folder', paths);
      options.onInfo?.(`Inserted ${countLabel(paths.length, 'folder reference', 'folder references')}`);
      return true;
    }
    case 'composer.insertSlashCommand':
      commandPaletteAPI.open('slash');
      options.onInfo?.('Opened slash commands');
      return true;
    case 'composer.addConnectors':
      await openSettingsDialog(
        'extensions',
        '',
        { extensionsFilter: 'connectors' },
      );
      options.onInfo?.('Opened connector extensions');
      return true;
    case 'composer.addExtensions':
      await openSettingsDialog('extensions');
      options.onInfo?.('Opened Settings > Extensions');
      return true;
    case 'composer.togglePlanMode':
      options.onTogglePlanMode();
      return true;
    case 'composer.toggleMultiAgent':
      options.onToggleMultiAgent();
      return true;
    case 'composer.toggleGoalTracking':
      options.onToggleGoalTracking();
      return true;
    default:
      return false;
  }
}

export async function executeComposerPermissionMenuItem(
  item: MenuActionItem,
  sessionId: string | null,
): Promise<boolean> {
  const prefix = 'composer.permission:';
  if (!item.command.startsWith(prefix)) return false;

  const permissionPolicy = normalizePermissionPolicy(item.command.slice(prefix.length));
  if (!sessionId) {
    setPendingStartPermissionPolicy(permissionPolicy);
    return true;
  }

  await setSessionPermissionPolicy(sessionId, permissionPolicy);
  return true;
}

export async function resolveComposerModelMenuTrigger(
  sessionId: string | null,
  isOpen: boolean,
): Promise<'noop' | 'close' | 'open' | 'gateway-settings'> {
  if (isOpen) return 'close';
  void sessionId;

  if (!gatewayState.loaded) {
    await loadGatewayCatalog();
  }

  if (gatewayState.models.length === 0) {
    return 'gateway-settings';
  }

  return 'open';
}

export async function executeComposerModelSelection(
  sessionId: string | null,
  currentSelection: GatewayModelSelection | null,
  nextSelection: GatewayModelSelection,
): Promise<boolean> {
  if (
    !nextSelection.providerId.trim() ||
    !nextSelection.modelId.trim() ||
    (nextSelection.providerId === currentSelection?.providerId && nextSelection.modelId === currentSelection?.modelId)
  ) {
    return false;
  }
  if (!sessionId) {
    setPendingStartModelSelection(nextSelection.providerId, nextSelection.modelId);
    return true;
  }

  await setSessionModelSelection(sessionId, nextSelection.providerId, nextSelection.modelId);
  return true;
}

export async function executeComposerReasoningEffortSelection(
  sessionId: string | null,
  currentEffort: ReasoningEffort,
  nextEffort: ReasoningEffort,
): Promise<boolean> {
  if (nextEffort === currentEffort) return false;
  if (!sessionId) {
    setPendingStartReasoningEffort(nextEffort);
    return true;
  }

  await setSessionReasoningEffort(sessionId, nextEffort);
  return true;
}
