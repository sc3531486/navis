import { openSettingsDialog } from '../components/Settings/openSettingsDialog';
import { commandPaletteAPI } from '../components/CommandPalette/store';
import type { MenuActionItem } from './menu';
import { executeDeclarativeMenuAction } from './menu-actions';

export async function executeToolsMenuItem(item: MenuActionItem): Promise<boolean> {
  if (executeDeclarativeMenuAction(item)) return true;

  switch (item.command) {
    case 'tools.commandPalette':
      commandPaletteAPI.open('commands');
      return true;
    case 'tools.settings':
      await openSettingsDialog('gateway');
      return true;
    case 'tools.gateway':
      await openSettingsDialog('gateway');
      return true;
    case 'tools.codingEditor':
      await openSettingsDialog('coding');
      return true;
    case 'tools.extensions':
      await openSettingsDialog('extensions');
      return true;
    default:
      return false;
  }
}
