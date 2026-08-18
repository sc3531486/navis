import { openSettingsDialog } from '@settings-ext/components/Settings/openSettingsDialog';
import type { SupportedLocale } from '@/i18n';
import { executeDeclarativeMenuAction } from '@/stores/menu-actions';
import type { MenuActionItem } from '@/stores/menu';
import { languageState, loadLanguage, setAppLanguage } from '@editor-ext/stores/language';

function languageCommand(locale: string): string {
  return `gateway.language:${locale}`;
}

export function gatewayMenuSelectedCommands(): string[] {
  return [languageCommand(languageState.language)];
}

export function gatewayMenuSubmenuItems(item: MenuActionItem): MenuActionItem[] {
  if (item.command !== 'gateway.language') return [];

  return [
    ...languageState.builtinLanguages.map((locale) => ({
      id: `gateway.language.${locale.value}`,
      label: locale.label,
      target: 'Gateway' as const,
      command: languageCommand(locale.value),
      group: 'builtin-language',
    })),
  ];
}

export async function executeGatewayMenuItem(item: MenuActionItem): Promise<boolean> {
  if (item.command === 'gateway.settings') {
    await openSettingsDialog('gateway');
    return true;
  }

  if (item.command.startsWith('gateway.language:')) {
    if (!languageState.loaded) {
      await loadLanguage();
    }

    const nextLocale = item.command.replace('gateway.language:', '') as SupportedLocale;
    if (nextLocale !== languageState.language) {
      await setAppLanguage(nextLocale);
    }
    return true;
  }

  return executeDeclarativeMenuAction(item);
}
