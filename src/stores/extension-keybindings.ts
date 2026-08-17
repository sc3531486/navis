import { invoke } from '@tauri-apps/api/core';
import { getHotkeyManager, HotkeyScope } from '../lib/hotkey';
import type { HotkeyBinding } from '../lib/hotkey';
import { executeDeclarativeMenuAction } from './menu-actions';
import type { MenuBuiltinAction } from './menu';

interface UiExtensionKeybinding {
  id: string;
  keybinding: string;
  command: string;
  description: string;
  category: string;
  extensionId: string;
  extensionName: string;
  action: MenuBuiltinAction;
}

interface RegisteredExtensionKeybinding {
  bindingId: string;
  command: string;
}

let registeredExtensionKeybindings: RegisteredExtensionKeybinding[] = [];
let loadToken = 0;
let loadQueue: Promise<void> = Promise.resolve();

function bindingId(keybinding: UiExtensionKeybinding): string {
  return `extension:${keybinding.extensionId}:${keybinding.id}`;
}

function toHotkeyBinding(keybinding: UiExtensionKeybinding): HotkeyBinding {
  return {
    id: bindingId(keybinding),
    keybinding: keybinding.keybinding,
    scope: HotkeyScope.App,
    command: keybinding.command,
    description: keybinding.description,
    category: keybinding.category,
    is_custom: true,
  };
}

async function unregisterRegistrations(registrations: RegisteredExtensionKeybinding[]): Promise<void> {
  const hotkeyManager = getHotkeyManager();

  for (const registration of registrations) {
    hotkeyManager.offCommand(registration.command);

    try {
      await hotkeyManager.unregister(registration.bindingId);
    } catch (error) {
      console.warn(
        `[ExtensionHotkey] Failed to unregister binding ${registration.bindingId}:`,
        error,
      );
    }
  }
}

async function unregisterExtensionKeybindings(): Promise<void> {
  await unregisterRegistrations(registeredExtensionKeybindings);
  registeredExtensionKeybindings = [];
}

function registerCommandCallback(
  keybinding: UiExtensionKeybinding,
  binding: HotkeyBinding,
): void {
  const hotkeyManager = getHotkeyManager();
  hotkeyManager.onCommand(keybinding.command, () => {
    executeDeclarativeMenuAction({
      id: binding.id,
      label: keybinding.description,
      target: 'Tools',
      command: keybinding.command,
      extensionId: keybinding.extensionId,
      action: keybinding.action,
    });
  });
}

export function loadExtensionKeybindings(): Promise<void> {
  const token = ++loadToken;
  const task = loadQueue.then(async () => {
    const hotkeyManager = getHotkeyManager();
    hotkeyManager.init();

    const keybindings = await invoke<UiExtensionKeybinding[]>('ui_list_extension_keybindings');
    if (token !== loadToken) return;

    await unregisterExtensionKeybindings();

    const registeredCommands = new Set<string>();
    const nextRegistrations: RegisteredExtensionKeybinding[] = [];

    for (const keybinding of keybindings) {
      const binding = toHotkeyBinding(keybinding);

      try {
        await hotkeyManager.register(binding);
      } catch (error) {
        console.warn(
          `[ExtensionHotkey] Skipped conflicting or invalid binding ${keybinding.id} ` +
            `for command ${keybinding.command}:`,
          error,
        );
        continue;
      }

      if (!registeredCommands.has(keybinding.command)) {
        registerCommandCallback(keybinding, binding);
        registeredCommands.add(keybinding.command);
      }

      nextRegistrations.push({
        bindingId: binding.id,
        command: keybinding.command,
      });
    }

    if (token !== loadToken) {
      await unregisterRegistrations(nextRegistrations);
      return;
    }

    registeredExtensionKeybindings = nextRegistrations;
  });

  loadQueue = task.catch(() => undefined);
  return task;
}
