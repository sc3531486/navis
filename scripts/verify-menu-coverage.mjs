import assert from 'node:assert/strict';
import { mkdir, readFile, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const outdir = '.tmp/verify-menu-coverage';
const outfile = `${outdir}/menu-command-coverage.mjs`;

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

try {
await build({
  entryPoints: ['src/stores/menu-command-coverage.ts'],
  outfile,
  bundle: true,
  format: 'esm',
  platform: 'node',
  logLevel: 'silent',
});

const { BUILTIN_MENU_COMMAND_COVERAGE, isBuiltinMenuCommandCovered } = await import(pathToFileURL(outfile).href);
const uiSource = await readFile('src-tauri/src/ui/menus.rs', 'utf8');
const builtinStart = uiSource.indexOf('fn builtin_menus() -> Vec<MenuRegistration>');
assert.notEqual(builtinStart, -1, 'builtin_menus() was not found in src-tauri/src/ui/menus.rs');

const countsStart = uiSource.indexOf('pub fn ui_list_menus', builtinStart);
assert.notEqual(countsStart, -1, 'ui_list_menus() marker was not found after builtin_menus()');

const builtinSource = uiSource.slice(builtinStart, countsStart);

// 菜单 target 常量模块（src-tauri/src/extension/models.rs::menu_targets）。
// 替代已删除的 is_supported_menu_target() / MenuTarget 枚举白名单：target 已开放为字符串，
// 内置 target 由常量模块集中声明，扩展 target 走 `Menubar.*` 或 `{extId}:{targetId}` 命名空间。
const menuTargetConstToValue = {
  TOOLS: 'Tools',
  INPUT_PLUS: 'InputPlus',
  CHAT_TITLE: 'ChatTitle',
  RIGHT_PANEL: 'RightPanel',
  GATEWAY: 'Gateway',
  WORKTREE_CONTEXT: 'WorktreeContext',
  SESSION_CONTEXT: 'SessionContext',
};

const extensionModelsSource = await readFile('src-tauri/src/extension/models.rs', 'utf8');
const menuTargetsStart = extensionModelsSource.indexOf('pub mod menu_targets');
assert.notEqual(
  menuTargetsStart,
  -1,
  'menu_targets constants module was not found in src-tauri/src/extension/models.rs',
);
const menuTargetsEnd = extensionModelsSource.indexOf('\n}', menuTargetsStart);
assert.notEqual(menuTargetsEnd, -1, 'menu_targets module terminator was not found');
const menuTargetsSource = extensionModelsSource.slice(menuTargetsStart, menuTargetsEnd);

for (const [constName, value] of Object.entries(menuTargetConstToValue)) {
  assert.ok(
    menuTargetsSource.includes(`pub const ${constName}: &str = "${value}"`),
    `menu_targets::${constName} must be declared as "${value}"`,
  );
}

// File/Edit/View/Help 保留给顶部菜单栏 Menubar.* 命名空间（设计 §3.4），不作为裸内置 target 暴露。
const reservedExtensionMenuTargets = ['File', 'Edit', 'View', 'Help'];
for (const target of reservedExtensionMenuTargets) {
  assert.ok(
    !menuTargetsSource.includes(`pub const ${target.toUpperCase()}: &str = "${target}"`),
    `Reserved menu target ${target} must not be exposed as a bare builtin target (use Menubar.${target})`,
  );
}

const hostViewSource = await readFile('src-tauri/src/extension/host_view.rs', 'utf8');
const uiHostViewSource = await readFile('src-tauri/src/ui/host_view.rs', 'utf8');
assert.ok(
  hostViewSource.includes('pub(crate) fn validate_extension_view') &&
    hostViewSource.includes('pub(crate) fn effective_view_zone'),
  'Extension view validation contract is missing from extension/host_view.rs',
);
assert.ok(
  uiHostViewSource.includes('ui_extension_view_descriptor') &&
    uiHostViewSource.includes('validate_extension_view(view)') &&
    uiHostViewSource.includes('effective_view_zone(view)'),
  'Extension view UI projection must consume the shared HostView validation contract',
);
assert.ok(
  uiSource.includes('UiMenuBuiltinAction') &&
    uiSource.includes('extension_host_view_targets') &&
    uiSource.includes('ui_extension_view_descriptor'),
  'Extension view renderability must be restricted to explicit host placement + renderer view targets',
);
assert.ok(
  uiSource.includes('extension_host_view_targets(&extension)'),
  'Extension menu/command export must compute host view targets from the enabled extension state',
);
assert.ok(
  uiSource.includes('ui_menu_builtin_action(action, &view_targets)'),
  'Extension OpenView/ToggleView actions must be resolved to placement + renderer targets before reaching the frontend',
);

const toolbarSource = await readFile('src/layouts/Toolbar.tsx', 'utf8');
assert.ok(
  toolbarSource.includes("getMenuItems('Tools')"),
  'Toolbar must source Tools menu items from backend ui_list_menus state',
);
assert.ok(
  toolbarSource.includes('executeToolsMenuItem(item)'),
  'Toolbar Tools menu must execute through the shared Tools menu handler',
);
assert.ok(
  toolbarSource.includes('loadMenus()') &&
    !toolbarSource.includes('openCommandEntry') &&
    !toolbarSource.includes('toolsMenuItems().length === 0) {\n      openCommandEntry();'),
  'Toolbar Tools button must refresh the real backend Tools menu instead of falling back to Command Palette when the menu is empty',
);
const toolsMenuSource = await readFile('src/stores/tools-menu.ts', 'utf8');
const menuActionsSource = await readFile('src/stores/menu-actions.tsx', 'utf8');
const menuStoreSource = await readFile('src/stores/menu.ts', 'utf8');
const extensionCommandsSource = await readFile('src/stores/extension-commands.ts', 'utf8');
const hostViewRegistrySource = await readFile('src/components/HostView/registry.ts', 'utf8');
assert.ok(
  toolsMenuSource.includes('executeDeclarativeMenuAction(item)') &&
    menuActionsSource.includes('view.placement') &&
    menuActionsSource.includes('getHostViewSurfaceDescriptor') &&
    menuActionsSource.includes('getHostViewRendererDescriptor') &&
    hostViewRegistrySource.includes('getHostViewSurfaceDescriptor') &&
    hostViewRegistrySource.includes('getHostViewRendererDescriptor') &&
    menuActionsSource.includes('canDispatchHostViewPlacement'),
  'Tools menu handler must keep extension declarative actions on the registry-backed placement-aware host view dispatcher',
);
assert.ok(
  menuStoreSource.includes('canDispatchDeclarativeMenuAction') &&
    extensionCommandsSource.includes('canDispatchDeclarativeMenuAction'),
  'Menus and Command Palette must hide host view actions until their placement has a real frontend surface dispatcher',
);
assert.ok(
  hostViewSource.includes('HOST_VIEW_ZONE_CHAT_ASIDE') &&
    hostViewSource.includes('HOST_VIEW_ZONE_BOTTOM_DRAWER') &&
    hostViewSource.includes('HOST_VIEW_ZONE_SETTINGS_SECTION') &&
    hostViewRegistrySource.includes('chatAside') &&
    hostViewRegistrySource.includes('bottomDrawer') &&
    hostViewRegistrySource.includes('settingsSection'),
  'Host view placement support must include chatAside, bottomDrawer, and settingsSection data surfaces',
);
assert.ok(
  !uiSource.includes('host:markdown-panel') && !menuActionsSource.includes('host:markdown-panel'),
  'Host view renderer support must not include host:markdown-panel',
);

// 顶部菜单栏 Menubar.* 命名空间（设计 §3.4 / 34 文档 L395）：
// 扩展按 `Menubar.File/Edit/View/Help/Tools` 声明贡献项；MenuBar 同时查询
// Menubar.* 与后端当前裸 target（menu_targets 常量），保证扩展菜单项不丢失。
const menuBarSource = await readFile('src/components/MenuBar/MenuBar.tsx', 'utf8');
for (const name of ['File', 'Edit', 'View', 'Help', 'Tools']) {
  assert.ok(
    menuBarSource.includes(`'Menubar.${name}'`),
    `MenuBar must expose the Menubar.${name} namespace target (design §3.4)`,
  );
}
assert.ok(
  menuBarSource.includes('getMenuItems(entry.namespace)') &&
    menuBarSource.includes('getMenuItems(entry.legacy)'),
  'MenuBar must query both the Menubar.* namespace and the legacy bare target so extension menu items keep rendering',
);

const routerSource = await readFile('src/components/Chat/ConversationMessage.tsx', 'utf8');
const workspacePanelSource = await readFile('src/components/WorkspacePanel/BuiltinRightWorkspaceContent.tsx', 'utf8');
const commandPaletteSource = await readFile('src/components/CommandPalette/store.ts', 'utf8');
const composerMenuSource = await readFile('src/stores/composer-menu.ts', 'utf8');
const sessionMenuSource = await readFile('src/stores/session-menu.ts', 'utf8');
const settingsDialogSource = await readFile('src/components/Settings/openSettingsDialog.tsx', 'utf8');
const settingsContentSource = await readFile('src/components/Settings/SettingsDialogContent.tsx', 'utf8');
assert.ok(
  routerSource.includes('AgentTimelineView') && routerSource.includes('agentTimelineParts'),
  'Chat transcript view must render agent timeline parts through the shared timeline component',
);
assert.ok(
  workspacePanelSource.includes("viewId === 'session-transcript'") &&
    workspacePanelSource.includes('SessionTranscriptPanel'),
  'Right workspace must route the session transcript view to SessionTranscriptPanel',
);
assert.ok(
    workspacePanelSource.includes('HostViewRenderer') &&
    workspacePanelSource.includes('props.panel.extensionView') &&
    workspacePanelSource.includes("props.panel.viewId === 'design'"),
  'Right workspace extension panels must render through the shared HostViewRenderer instead of an empty fallback',
);
assert.ok(
  !/[\u4e00-\u9fff]/.test(workspacePanelSource),
  'Right workspace built-in panels must keep first-version visible UI copy in English',
);
assert.ok(
  commandPaletteSource.includes('openActiveSessionFilePanel') &&
    commandPaletteSource.includes('openRightWorkspacePanel({') &&
    commandPaletteSource.includes("title: 'File'") &&
    commandPaletteSource.includes("viewId: 'editor'"),
  'Command Palette file and symbol results must open the right workspace File panel',
);
assert.ok(
  !commandPaletteSource.includes('openEditorView'),
  'Command Palette file and symbol results must not route to the full-screen /editor view',
);
assert.ok(
  commandPaletteSource.includes("'slash'") &&
    !commandPaletteSource.includes("'skills'") &&
    composerMenuSource.includes("commandPaletteAPI.open('slash')"),
  'InputPlus Slash commands must open the explicit slash scope, not a legacy skills scope',
);
assert.ok(
  sessionMenuSource.includes('session.openIn.configureExternalEditors') &&
    sessionMenuSource.includes('session.openIn.externalEditor:') &&
    sessionMenuSource.includes('openSessionExternalEditor'),
  'Chat title Open in submenu must use configured external coding tools',
);
assert.ok(
  !sessionMenuSource.includes('session.openIn.editor') &&
    !sessionMenuSource.includes('openSessionInEditor') &&
    !sessionMenuSource.includes("label: 'File'"),
  'Chat title Open in submenu must not expose the right workspace File panel',
);
assert.ok(
  !toolsMenuSource.includes("openSettingsDialog('personal')") &&
    !settingsDialogSource.includes("initialTab: SettingsTab = 'personal'") &&
    settingsDialogSource.includes("initialTab: SettingsTab = 'gateway'"),
  'Settings menu must default to the real Gateway settings tab, not a placeholder Personal landing page',
);
assert.ok(
  !settingsContentSource.includes("'personal'") &&
    !settingsContentSource.includes("'integrations'") &&
    settingsContentSource.includes("export type SettingsTab = 'gateway' | 'coding' | 'extensions'"),
  'Settings tabs must expose only currently implemented settings sections',
);

const menuPattern = /menu(?:_with_meta|_with_icon)?\(\s*"[^"]+"\s*,\s*"[^"]+"\s*,\s*menu_targets::(\w+)\s*,\s*"([^"]+)"/g;
const missing = [];
const seen = [];

for (const match of builtinSource.matchAll(menuPattern)) {
  const [, targetConst, command] = match;
  const target = menuTargetConstToValue[targetConst];
  assert.ok(target, `Unknown menu_targets const ${targetConst}`);
  seen.push(`${target}:${command}`);
  if (!isBuiltinMenuCommandCovered(target, command)) {
    missing.push(`${target}:${command}`);
  }
}

assert.ok(seen.length > 0, 'No builtin menu registrations were parsed');
assert.deepEqual(missing, [], `Builtin menu commands without frontend coverage:\n${missing.join('\n')}`);

const handlerSourcesByTarget = {
  Tools: ['src/stores/tools-menu.ts'],
  InputPlus: ['src/stores/composer-menu.ts'],
  ChatTitle: ['src/stores/session-menu.ts'],
  RightPanel: ['src/stores/right-workspace-menu.ts'],
  Gateway: ['src/stores/gateway-menu.ts'],
  WorktreeContext: ['src/stores/worktree-menu.ts'],
  SessionContext: ['src/stores/session-menu.ts'],
};

const handlerSources = {};
for (const [target, files] of Object.entries(handlerSourcesByTarget)) {
  handlerSources[target] = (await Promise.all(files.map((file) => readFile(file, 'utf8')))).join('\n');
}

const staleCoverage = [];
for (const [target, coverage] of Object.entries(BUILTIN_MENU_COMMAND_COVERAGE)) {
  const source = handlerSources[target] ?? '';
  const exactCommands = [
    ...coverage.directCommands,
    ...coverage.submenuParentCommands,
    ...(coverage.generatedCommands ?? []),
  ];
  for (const command of exactCommands) {
    if (!source.includes(command)) {
      staleCoverage.push(`${target}:${command}`);
    }
  }
  for (const prefix of coverage.generatedCommandPrefixes ?? []) {
    if (!source.includes(prefix)) {
      staleCoverage.push(`${target}:${prefix}*`);
    }
  }
}

assert.deepEqual(
  staleCoverage,
  [],
  `Builtin menu commands are claimed covered but missing from their frontend handler source:\n${staleCoverage.join('\n')}`,
);

console.log(`Verified ${seen.length} builtin menu commands`);
} finally {
  await rm(outdir, { recursive: true, force: true });
}
