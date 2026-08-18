import { invoke } from '@tauri-apps/api/core';
import { commandPaletteAPI, type Command } from '@navis-code/components/CommandPalette/store';
import { insertComposerSlashTrigger } from './composer-input';

interface UiSlashCommand {
  trigger: string;
  name: string;
  description: string;
  triggerType: string;
  source: string;
  sourceLabel: string;
  extensionId?: string | null;
}

let registeredSlashCommandIds: string[] = [];
let loadToken = 0;

function unregisterSlashCommands(): void {
  for (const id of registeredSlashCommandIds) {
    commandPaletteAPI.unregister(id);
  }
  registeredSlashCommandIds = [];
}

function slashCommandId(command: UiSlashCommand): string {
  return `slash:${command.source}:${command.triggerType}:${command.trigger}`;
}

function slashCommandCategory(command: UiSlashCommand): string {
  switch (command.triggerType) {
    case 'command':
      return 'Slash command';
    case 'enhanced':
      return 'Enhanced skill';
    default:
      return 'Skill';
  }
}

function toCommand(command: UiSlashCommand): Command {
  return {
    id: slashCommandId(command),
    label: command.trigger,
    description: command.description,
    category: slashCommandCategory(command),
    source: command.triggerType === 'command' ? 'command' : 'skill',
    tags: [
      command.name,
      command.trigger,
      command.triggerType,
      command.source,
      command.sourceLabel,
      ...(command.extensionId ? [command.extensionId] : []),
    ],
    handler: () => {
      insertComposerSlashTrigger(command.trigger);
    },
  };
}

export async function loadSlashCommands(): Promise<void> {
  const token = ++loadToken;
  const commands = await invoke<UiSlashCommand[]>('ui_list_slash_commands');
  if (token !== loadToken) return;

  unregisterSlashCommands();
  registeredSlashCommandIds = commands.map(slashCommandId);
  commandPaletteAPI.registerBatch(commands.map(toCommand));
}
