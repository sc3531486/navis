import { hostState, openRightWorkspacePanel, type RightWorkspacePanel } from '@/stores/host';
import type { MenuActionItem } from '@/stores/menu';
import { executeDeclarativeMenuAction } from '@/stores/menu-actions';

const BUILTIN_RIGHT_WORKSPACE_PANELS: Record<string, RightWorkspacePanel> = {
  'rightWorkspace.open.diff': {
    id: 'diff',
    title: 'Diff',
    viewId: 'diff',
  },
  'rightWorkspace.open.backgroundTasks': {
    id: 'background-tasks',
    title: 'Background tasks',
    viewId: 'background-tasks',
  },
  'rightWorkspace.open.plan': {
    id: 'plan',
    title: 'Plan',
    viewId: 'plan',
  },
  'rightWorkspace.open.design': {
    id: 'design',
    title: 'Design',
    viewId: 'design',
  },
};

const BUILTIN_RIGHT_WORKSPACE_COMMANDS = Object.entries(BUILTIN_RIGHT_WORKSPACE_PANELS).reduce<
  Record<string, string>
>((commands, [command, panel]) => {
  commands[panel.id] = command;
  return commands;
}, {});

function openRightWorkspaceIds(): Set<string> {
  return new Set(
    hostState.rightWorkspaceColumns.flatMap((column) => column.panels.map((panel) => panel.id)),
  );
}

export function getOpenRightWorkspaceCommands(items: MenuActionItem[]): string[] {
  const openIds = openRightWorkspaceIds();
  const selectedCommands = Object.entries(BUILTIN_RIGHT_WORKSPACE_COMMANDS)
    .filter(([panelId]) => openIds.has(panelId))
    .map(([, command]) => command);

  for (const item of items) {
    if (!item.extensionId || !item.action || !('view' in item.action)) continue;
    if ((item.action.view.zone || item.action.view.placement) !== 'rightWorkspace') continue;
    const panelId = `${item.extensionId}:${item.action.view.viewId}`;
    if (openIds.has(panelId)) {
      selectedCommands.push(item.command);
    }
  }

  return selectedCommands;
}

export function executeRightWorkspaceMenuItem(item: MenuActionItem): boolean {
  const panel = BUILTIN_RIGHT_WORKSPACE_PANELS[item.command];
  if (panel) {
    openRightWorkspacePanel(panel);
    return true;
  }

  return executeDeclarativeMenuAction(item);
}
