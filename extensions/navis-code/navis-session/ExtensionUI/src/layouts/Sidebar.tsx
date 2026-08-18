/**
 * Navis 左侧栏。
 *
 * 依据 design/ui设计/navis-ui-left-sidebar.drawio：
 * 模式区域 / 模式菜单区域 / Worktree 区域 / Gateway 固定按钮。
 */

import { Component, For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { agentState, setWorkMode } from '@agent-core/stores/agent';
import { extensionState, getWorkModeDisplayName, loadExtensions, type RegisteredWorkMode } from '@/stores/extension';
import { SidebarWorktreeTitle } from '@session/components/Sidebar/SidebarWorktreeTitle';
import { closeMenu, getMenuItems, isMenuOpen, openMenu, toggleMenu, type MenuActionItem } from '@/stores/menu';
import { executeSessionMenuItem, getSessionMenuItems, getSessionSubmenuItems } from '@session/stores/session-menu';
import { executeWorktreeMenuItem } from '@session/stores/worktree-menu';
import { setPendingStartKind } from '@navis-code/stores/product-app';
import { openSettingsDialog } from '@settings-ext/components/Settings/openSettingsDialog';
import { languageState, loadLanguage } from '@editor-ext/stores/language';
import {
  executeGatewayMenuItem,
  gatewayMenuSelectedCommands,
  gatewayMenuSubmenuItems,
} from '@project-ext/stores/gateway-menu';
import { selectSessionWithEditorGuard } from '@editor-ext/stores/editor-context-transition';
import {
  activateSession,
  activeSession,
  loadSessionTree,
  activeSessionId,
  sessionTreeState,
  toggleWorktree,
} from '@session/stores/session-tree';
import { useEvent } from '@/lib/stream';
import { SidebarGatewayMenu } from '@session/components/Sidebar/SidebarGatewayMenu';
import { SidebarModeMenu } from '@session/components/Sidebar/SidebarModeMenu';
import { SidebarModeTabs } from '@session/components/Sidebar/SidebarModeTabs';
import { SidebarSessionRow } from '@session/components/Sidebar/SidebarSessionRow';
import {
  MODE_MENU,
  SESSION_TREE_REFRESH_EVENTS,
  workModeFromSessionMode,
  type BuiltinMode,
  type ModeTab,
  type SidebarMenuItem,
} from '@session/components/Sidebar/sidebar-model';
import gatewayIconUrl from '@agent-core/assets/navis-product-icon.svg';
import gatewayChevronUrl from '@agent-core/assets/gateway-v.svg';
import projectFolderCloseUrl from '@project-ext/assets/project-folder-close.svg';
import projectFolderOpenUrl from '@project-ext/assets/project-folder-open.svg';
import projectEditUrl from '@project-ext/assets/project-edit.svg';
import menuIconUrl from '@session/assets/menu.svg';
import runningIconUrl from '@agent-core/assets/running.svg';
import completedTaskIconUrl from '@agent-core/assets/task-completed-dot.svg';

const Sidebar: Component = () => {
  const [activeTab, setActiveTab] = createSignal<ModeTab>('code');
  const currentMode = () => agentState.workMode;
  const worktrees = () => sessionTreeState.worktrees;
  const selectedSessionId = activeSessionId;
  const customModes = () => extensionState.workModes;
  const builtinMenu = createMemo(() =>
    activeTab() === 'custom' ? [] : MODE_MENU[activeTab() as BuiltinMode],
  );
  const customModeKey = (mode: RegisteredWorkMode) => `custom:${mode.runtimeId}`;
  const activeCustomMode = createMemo(() => {
    const selected = currentMode();
    if (selected.type !== 'custom') return undefined;
    return customModes().find((mode) => mode.runtimeId === selected.runtimeId);
  });
  const projectScopeLabel = createMemo(() => {
    if (activeTab() !== 'custom') return '';
    const mode = activeCustomMode();
    return mode ? getWorkModeDisplayName(mode) : '选择模式扩展';
  });
  const activeModeKey = createMemo(() => {
    const tab = activeTab();
    if (tab === 'cowork' || tab === 'code') return tab;
    const selected = currentMode();
    return selected.type === 'custom' ? `custom:${selected.runtimeId}` : null;
  });
  const visibleWorktrees = createMemo(() => {
    const mode = activeModeKey();
    if (!mode) return [];

    return worktrees()
      .map((worktree, index) => ({
        worktree: {
          ...worktree,
          sessions: worktree.sessions.filter((session) => session.mode === mode),
        },
        originalIndex: index,
      }))
      .filter(({ worktree }) => worktree.sessions.length > 0);
  });

  onMount(() => {
    void loadSessionTree();
    void loadExtensions();
    void loadLanguage();
  });

  let refreshInFlight = false;
  let refreshPending = false;
  let refreshTimer: number | undefined;
  const refreshSessionTreeFromEvents = () => {
    if (refreshInFlight) {
      refreshPending = true;
      return;
    }

    refreshInFlight = true;
    refreshPending = false;
    void loadSessionTree().finally(() => {
      refreshInFlight = false;
      if (refreshPending) scheduleSessionTreeRefresh();
    });
  };

  const scheduleSessionTreeRefresh = () => {
    if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
    refreshTimer = window.setTimeout(() => {
      refreshTimer = undefined;
      refreshSessionTreeFromEvents();
    }, 120);
  };

  for (const eventName of SESSION_TREE_REFRESH_EVENTS) {
    useEvent(eventName, scheduleSessionTreeRefresh);
  }

  onCleanup(() => {
    if (refreshTimer !== undefined) window.clearTimeout(refreshTimer);
  });

  createEffect(() => {
    const workMode = workModeFromSessionMode(activeSession()?.mode);
    if (!workMode) return;

    setActiveTab(workMode.type === 'custom' ? 'custom' : workMode.type);
    setWorkMode(workMode);
  });

  const selectedCustomMode = (mode: RegisteredWorkMode) => {
    const selected = currentMode();
    return selected.type === 'custom' && selected.runtimeId === mode.runtimeId;
  };

  function selectBuiltinMode(mode: BuiltinMode): void {
    setActiveTab(mode);
    setWorkMode({ type: mode });
    void activateFirstSessionForMode(mode);
  }

  function selectCustomMode(mode: RegisteredWorkMode): void {
    setActiveTab('custom');
    setWorkMode({
      type: 'custom',
      extensionId: mode.extensionId,
      modeId: mode.modeId,
      runtimeId: mode.runtimeId,
    });
    void activateFirstSessionForMode(customModeKey(mode));
  }

  async function activateFirstSessionForMode(mode: string): Promise<void> {
    const sessions = worktrees().flatMap((worktree) =>
      worktree.sessions.filter((session) => session.mode === mode),
    );
    if (selectedSessionId() && sessions.some((session) => session.id === selectedSessionId())) return;
    await selectSessionWithEditorGuard(sessions[0]?.id ?? null);
  }

  const worktreeMenuId = (index: number) => `worktree:${index}`;
  const sessionMenuId = (sessionId: string) => `session:${sessionId}`;

  async function handleWorktreeMenuSelect(item: MenuActionItem, index: number): Promise<void> {
    const subject = worktrees()[index]?.name ?? 'Worktree';
    closeMenu();
    await executeWorktreeMenuItem(item, {
      worktreeIndex: index,
      worktreeName: subject,
      mode: activeModeKey(),
    });
  }

  async function renameVisibleWorktree(index: number): Promise<void> {
    const renameItem = getMenuItems('WorktreeContext').find((item) => item.command === 'worktree.rename');
    if (!renameItem) return;
    await handleWorktreeMenuSelect(renameItem, index);
  }

  async function handleSessionMenuSelect(item: MenuActionItem, sessionId: string): Promise<void> {
    const subject = sessionById(sessionId)?.name ?? 'Session';
    closeMenu();
    await executeSessionMenuItem(item, {
      sessionId,
      sessionName: subject,
      currentWorktreeName: currentSessionWorktreeName(sessionId),
    });
  }

  function sessionById(sessionId: string) {
    return worktrees()
      .flatMap((worktree) => worktree.sessions)
      .find((session) => session.id === sessionId);
  }

  function currentSessionWorktreeName(sessionId: string): string | undefined {
    return worktrees().find((worktree) =>
      worktree.sessions.some((session) => session.id === sessionId),
    )?.name;
  }

  async function handleBuiltinMenuSelect(item: SidebarMenuItem): Promise<void> {
    if (item.id === 'new-task') {
      setPendingStartKind('task');
      await selectSessionWithEditorGuard(null);
    } else if (item.id === 'new-session') {
      setPendingStartKind('session');
      await selectSessionWithEditorGuard(null);
    } else if (item.id === 'customize') {
      await openSettingsDialog(
        'extensions',
        '',
        { extensionsFilter: 'modes' },
      );
    }
  }

  async function handleGatewayMenuSelect(item: MenuActionItem): Promise<void> {
    closeMenu();
    await executeGatewayMenuItem(item);
  }

  return (
    <nav class="navis-sidebar-root flex h-full flex-col text-[#242424]" aria-label="左侧栏">
      <div class="navis-sidebar-shell">
        <SidebarModeTabs
          activeTab={activeTab()}
          onSelectBuiltinMode={selectBuiltinMode}
          onSelectCustomTab={() => setActiveTab('custom')}
        />

        <SidebarModeMenu
          activeTab={activeTab()}
          builtinMenu={builtinMenu()}
          customModes={customModes()}
          selectedCustomMode={selectedCustomMode}
          onBuiltinMenuSelect={(item) => void handleBuiltinMenuSelect(item)}
          onSelectCustomMode={selectCustomMode}
          onOpenModeExtensions={() => void openSettingsDialog(
            'extensions',
            'Custom modes are provided by mode extensions.',
            { extensionsFilter: 'modes' },
          )}
        />

        <section class="navis-sidebar-worktrees flex min-h-0 flex-1 flex-col">
          <div class="navis-sidebar-worktree-card relative min-h-0 flex-1 overflow-y-auto">
            <div class="navis-project-header">
            <Show when={projectScopeLabel()}>
              <span class="navis-project-scope">{projectScopeLabel()}</span>
            </Show>
          </div>
          <For each={visibleWorktrees()}>
            {({ worktree, originalIndex }) => {
              const menuId = worktreeMenuId(originalIndex);
              const isWorktreeActive = () =>
                worktree.sessions.some((session) => session.id === selectedSessionId());
              const hasRunningTask = () => worktree.sessions.some((session) => session.hasRunningTask);

              return (
              <div class="relative">
                <SidebarWorktreeTitle
                  worktree={worktree}
                  menuId={menuId}
                  isContextActive={isMenuOpen(menuId)}
                  isActive={isWorktreeActive()}
                  hasRunningTask={hasRunningTask()}
                  menuItems={getMenuItems('WorktreeContext')}
                  projectFolderCloseUrl={projectFolderCloseUrl}
                  projectFolderOpenUrl={projectFolderOpenUrl}
                  disclosureIconUrl={gatewayChevronUrl}
                  menuIconUrl={menuIconUrl}
                  editIconUrl={projectEditUrl}
                  onToggleCollapsed={() => toggleWorktree(originalIndex)}
                  onOpenContextMenu={() => openMenu(menuId)}
                  onToggleMenu={() => toggleMenu(menuId)}
                  onRename={() => void renameVisibleWorktree(originalIndex)}
                  onMenuSelect={(item) => handleWorktreeMenuSelect(item, originalIndex)}
                />
                <Show when={!worktree.collapsed}>
                  <For each={worktree.sessions}>
                    {(session) => (
                      <SidebarSessionRow
                        session={session}
                        menuId={sessionMenuId(session.id)}
                        selected={selectedSessionId() === session.id}
                        contextActive={isMenuOpen(sessionMenuId(session.id))}
                        menuItems={getSessionMenuItems(getMenuItems('SessionContext'), session)}
                        availableWorktreeNames={visibleWorktrees().map(({ worktree }) => worktree.name)}
                        currentWorktreeName={currentSessionWorktreeName(session.id)}
                        menuIconUrl={menuIconUrl}
                        runningIconUrl={runningIconUrl}
                        completedTaskIconUrl={completedTaskIconUrl}
                        getSubmenuItems={(item) =>
                          getSessionSubmenuItems(item, {
                            sessionId: session.id,
                            target: 'SessionContext',
                            availableWorktreeNames: visibleWorktrees().map(({ worktree }) => worktree.name),
                            currentWorktreeName: currentSessionWorktreeName(session.id),
                          })}
                        onSelectSession={() => void selectSessionWithEditorGuard(session.id)}
                        onOpenMenu={() => openMenu(sessionMenuId(session.id))}
                        onToggleMenu={() => toggleMenu(sessionMenuId(session.id))}
                        onMenuSelect={(item) => handleSessionMenuSelect(item, session.id)}
                      />
                    )}
                  </For>
                </Show>
              </div>
              );
            }}
          </For>
          </div>
        </section>

        <SidebarGatewayMenu
          isOpen={isMenuOpen('gateway')}
          gatewayIconUrl={gatewayIconUrl}
          disclosureIconUrl={gatewayChevronUrl}
          items={getMenuItems('Gateway')}
          selectedCommands={gatewayMenuSelectedCommands()}
          getSubmenuItems={gatewayMenuSubmenuItems}
          onToggle={() => toggleMenu('gateway')}
          onSelect={handleGatewayMenuSelect}
        />
      </div>
    </nav>
  );
};

export default Sidebar;



