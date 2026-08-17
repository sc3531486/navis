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
 * inline 扩展点宿主目标，与后端 `format!("{:?}", InlineTarget)` 的 Debug 序列化对齐
 * （PascalCase：Chat / Editor / Terminal，见 src-tauri/src/extension/models.rs InlineTarget）。
 */
export type InlineHostTarget = 'Chat' | 'Editor' | 'Terminal';

export const INLINE_HOST_TARGETS: readonly InlineHostTarget[] = ['Chat', 'Editor', 'Terminal'];

export function isInlineHostTarget(value: string | null | undefined): value is InlineHostTarget {
  return typeof value === 'string' && (INLINE_HOST_TARGETS as readonly string[]).includes(value);
}

/** 从扩展点 data 透传字段取字符串（DTO 无顶层 icon/position，icon 仅存于 data）。 */
export function extensionPointDataString(point: UiExtensionPointRegistration, key: string): string | null {
  if (!isRecord(point.data)) return null;
  const value = point.data[key];
  return typeof value === 'string' && value.length > 0 ? value : null;
}

/** 工具栏/状态栏扩展项 icon（manifest ToolbarItemRegistration.icon 经 data 透传）。 */
export function extensionPointIcon(point: UiExtensionPointRegistration): string | null {
  return extensionPointDataString(point, 'icon');
}

/** inline 扩展点在宿主目标内的位置（BeforeInput / AfterMessages / Sidebar / Top / Bottom）。 */
export function inlinePointPosition(point: UiExtensionPointRegistration): string | null {
  return extensionPointDataString(point, 'position');
}

/**
 * 按宿主目标过滤 inline 扩展点。fail-closed：未知 target 一律不匹配。
 * target 语义与后端 InlineTarget Debug 序列化一致（Chat/Editor/Terminal）。
 */
export function inlineExtensionPointsFor(target: InlineHostTarget): UiExtensionPointRegistration[] {
  return extensionPointsByKind('inline').filter((point) => point.target === target);
}

/** Composer 工具栏承接的 inline 扩展点：Chat 目标 + BeforeInput 位置。 */
export function composerInlineExtensionPoints(): UiExtensionPointRegistration[] {
  return inlineExtensionPointsFor('Chat').filter((point) => {
    const position = inlinePointPosition(point);
    return position === null || position === 'BeforeInput';
  });
}

