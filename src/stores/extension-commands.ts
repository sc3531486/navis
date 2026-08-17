import { invoke } from '@tauri-apps/api/core';
import { commandPaletteAPI, type Command } from '../components/CommandPalette/store';
import { canDispatchDeclarativeMenuAction, executeDeclarativeMenuAction } from './menu-actions';
import type { MenuBuiltinAction } from './menu';

interface UiExtensionCommand {
  id: string;
  label: string;
  description?: string | null;
  category: string;
  icon?: string | null;
  extensionId: string;
  extensionName: string;
  action: MenuBuiltinAction;
}

let registeredExtensionCommandIds: string[] = [];
let loadToken = 0;

function unregisterExtensionCommands(): void {
  for (const id of registeredExtensionCommandIds) {
    commandPaletteAPI.unregister(id);
  }
  registeredExtensionCommandIds = [];
}

function canDispatchCommandAction(command: UiExtensionCommand): boolean {
  return canDispatchDeclarativeMenuAction({
    id: command.id,
    label: command.label,
    target: 'Tools',
    command: command.id,
    extensionId: command.extensionId,
    action: command.action,
  });
}

function toCommand(command: UiExtensionCommand): Command {
  return {
    id: command.id,
    label: command.label,
    description: command.description ?? undefined,
    category: command.category,
    icon: command.icon ?? undefined,
    source: 'extension',
    tags: [command.extensionName, command.extensionId, command.category],
    handler: () => {
      executeDeclarativeMenuAction({
        id: command.id,
        label: command.label,
        target: 'Tools',
        command: command.id,
        extensionId: command.extensionId,
        action: command.action,
      });
    },
  };
}

export async function loadExtensionCommands(): Promise<void> {
  const token = ++loadToken;
  const commands = await invoke<UiExtensionCommand[]>('ui_list_extension_commands');
  if (token !== loadToken) return;

  unregisterExtensionCommands();
  const dispatchableCommands = commands.filter((command) => canDispatchCommandAction(command));
  registeredExtensionCommandIds = dispatchableCommands.map((command) => command.id);
  commandPaletteAPI.registerBatch(dispatchableCommands.map(toCommand));
}
