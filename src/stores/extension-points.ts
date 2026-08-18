import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import { registerExtensionLocale, type ExtensionLocaleBundle, type ExtensionLocaleMessages } from '@/i18n';
import type {
  UiExtensionLocale,
  UiExtensionPointRegistration,
  UiExtensionScript,
  UiZone,
} from '@/lib/extension-ui';
import type { MenuActionItem, MenuBuiltinAction } from './menu';
import { executeDeclarativeMenuAction } from './menu-actions';
import { evaluateMenuWhen, getMenuWhenContext } from '@/lib/menu-when';
import { installExtensionWorkerLifecycle, runActivationScripts } from './extension-workers';

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

export interface ExtensionProjectionState {
  loaded: boolean;
  loading: boolean;
  error: string | null;
  zones: UiZone[];
  scripts: UiExtensionScript[];
  locales: UiExtensionLocale[];
  points: UiExtensionPointRegistration[];
  commands: UiExtensionCommand[];
}

export const extensionProjectionState: ExtensionProjectionState = {
  loaded: false,
  loading: false,
  error: null,
  zones: [],
  scripts: [],
  locales: [],
  points: [],
  commands: [],
};

const [projectionState, setProjectionState] = createStore<ExtensionProjectionState>(extensionProjectionState);

export { projectionState as extensionPointsState };

let loadToken = 0;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function flatStringMap(value: unknown): ExtensionLocaleMessages {
  const result: ExtensionLocaleMessages = {};
  if (!isRecord(value)) return result;
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === 'string') result[key] = entry;
  }
  return result;
}

async function loadLocaleJson(locale: UiExtensionLocale): Promise<ExtensionLocaleMessages | null> {
  if (!locale.resourcePath) return null;
  const response = await fetch(convertFileSrc(locale.resourcePath));
  if (!response.ok) throw new Error(`Failed to load locale ${locale.lang}: ${response.status}`);
  return flatStringMap(await response.json());
}

async function registerLocales(locales: UiExtensionLocale[]): Promise<void> {
  const bundles = new Map<string, ExtensionLocaleBundle>();
  await Promise.all(locales.map(async (locale) => {
    const messages = await loadLocaleJson(locale).catch((error) => {
      console.warn('[ExtensionI18n] failed to load locale', locale, error);
      return null;
    });
    if (!messages) return;
    const bundle = bundles.get(locale.extensionId) ?? {};
    bundle[locale.lang] = messages;
    bundles.set(locale.extensionId, bundle);
  }));

  for (const [extensionId, bundle] of bundles) {
    registerExtensionLocale(extensionId, bundle);
  }
}

export async function loadExtensionPoints(): Promise<void> {
  const token = ++loadToken;
  setProjectionState({ loading: true, error: null });
  try {
    const [zones, scripts, locales, points, commands] = await Promise.all([
      invoke<UiZone[]>('ui_list_zones'),
      invoke<UiExtensionScript[]>('ui_list_extension_scripts'),
      invoke<UiExtensionLocale[]>('ui_list_extension_locales'),
      invoke<UiExtensionPointRegistration[]>('ui_list_extension_points'),
      invoke<UiExtensionCommand[]>('ui_list_extension_commands'),
    ]);
    if (token !== loadToken) return;
    await registerLocales(locales);
    if (token !== loadToken) return;
    setProjectionState({ zones, scripts, locales, points, commands, loaded: true, loading: false, error: null });
    // 阶段 5：安装 worker 生命周期订阅 + 自动触发 runOn 含 "activation" 的脚本。
    installExtensionWorkerLifecycle();
    void runActivationScripts(scripts).catch((error) => {
      console.warn('[ExtensionWorker] activation scripts failed', error);
    });
  } catch (error) {
    if (token !== loadToken) return;
    setProjectionState({ loading: false, error: error instanceof Error ? error.message : String(error) });
  }
}

export function extensionPointsByKind(kind: string): UiExtensionPointRegistration[] {
  const context = getMenuWhenContext();
  return projectionState.points
    .filter((point) => point.kind === kind && evaluateMenuWhen(point.when, context))
    .slice()
    .sort((a, b) => (a.group ?? '').localeCompare(b.group ?? '') || a.id.localeCompare(b.id));
}

export function executeExtensionPoint(point: UiExtensionPointRegistration): boolean {
  if (!point.command) return false;
  const command = projectionState.commands.find(
    (candidate) => candidate.extensionId === point.extensionId && candidate.id === point.command,
  );
  if (!command) return false;
  const item: MenuActionItem = {
    id: point.id,
    label: point.label ?? command.label,
    target: point.target ?? point.kind,
    command: command.id,
    extensionId: command.extensionId,
    action: command.action,
  };
  return executeDeclarativeMenuAction(item);
}

/**
 * inline 扩展点宿主目标。
 *
 * 目标由扩展协议声明，宿主不内置任何产品领域枚举。
 */
export type InlineHostTarget = string;

/** 从扩展点 data 透传字段取字符串（DTO 无顶层字段时使用 data）。 */
export function extensionPointDataString(point: UiExtensionPointRegistration, key: string): string | null {
  if (!isRecord(point.data)) return null;
  const value = point.data[key];
  return typeof value === 'string' && value.length > 0 ? value : null;
}

/** 扩展点图标，具体展示位置由产品扩展决定。 */
export function extensionPointIcon(point: UiExtensionPointRegistration): string | null {
  return extensionPointDataString(point, 'icon');
}

/** inline 扩展点位置，具体位置语义由使用方扩展决定。 */
export function inlinePointPosition(point: UiExtensionPointRegistration): string | null {
  return extensionPointDataString(point, 'position');
}

/** 按扩展声明的宿主目标过滤 inline 扩展点。 */
export function inlineExtensionPointsFor(target: InlineHostTarget): UiExtensionPointRegistration[] {
  return extensionPointsByKind('inline').filter((point) => point.target === target);
}
