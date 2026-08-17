/**
 * ============================================================
 * Navis 主布局 - layouts/MainLayout.tsx
 * ============================================================
 *
 * 应用主布局组件，遵循 design/22-ui-framework.md 三布局系统。
 *
 * 布局结构：
 *   ┌──────────────────────────────────────────┐
 *   │  Toolbar（顶部工具栏）                    │
 *   ├──────┬───────────────────────────────────┤
 *   │      │  Content Area（中心内容区）        │
 *   │  S   │  view:chat / view:editor /        │
 *   │  i   │  view:content:* 扩展自定义视图     │
 *   │  d   ├───────────────────────────────────┤
 *   │  e   │  Panel（底部/右侧面板，可选）      │
 *   │  b   │                                   │
 *   │  a   │                                   │
 *   │  r   │                                   │
 *   └──────────────────────────────────────────┘
 *
 * 使用 <Show> 控制面板可见性，根据 activeView 切换内容区。
 *
 * 来源：design/22-ui-framework.md 第三章 §三 布局系统
 * ============================================================
 */

import { Component, ParentProps, Show, For, createEffect, createSignal, onCleanup, onMount } from 'solid-js';
import { useLocation, useNavigate } from '@solidjs/router';
import {
  appState,
  closeRightWorkspacePanel,
  focusRightWorkspacePanel,
  isRightWorkspacePanelClosable,
  setRightWorkspaceWidth,
  setSidebarWidth,
} from '../stores/app';
import Toolbar from './Toolbar';
import StatusBar from './StatusBar';
import Sidebar from './Sidebar';
import { HostViewSurface } from '../components/HostView';
import { hostViewsForZone } from '../stores/app';
import { DialogManager } from '../components/Dialog';
import { ExtensionDialogManager } from '../components/ExtensionDialog';
import { MenuBar } from '../components/MenuBar';
import { CommandPalette } from '../components/CommandPalette';
import { GlobalSearchPalette } from '../components/GlobalSearchPalette';
import { WorkspacePanelShell } from '../components/WorkspacePanel';
import BuiltinRightWorkspaceContent from '../components/WorkspacePanel/BuiltinRightWorkspaceContent';
import { installEditorBeforeUnloadGuard } from '../components/Editor/stores/editor-lifecycle-guard';
import { registerAppNavigator, syncActiveViewFromPath } from '../stores/view-navigation';
import { loadComposerRunState } from '../stores/composer-run';
import { ToastContainer, useNotificationEventProjection } from '../components/Notification';
import { extensionPointsState, loadExtensionPoints } from '../stores/extension-points';
import { installExtensionDiscoveryInvalidation } from '../stores/discovery';

const BUILTIN_HOST_ZONES = new Set([
  'rightWorkspace',
  'chatAside',
  'bottomDrawer',
  'settingsSection',
  'dialog',
]);

/**
 * The host, not an extension, decides where a custom zone is projected.
 * We only expose two safe parent slots in the main layout; arbitrary DOM
 * selectors and layout instructions from a manifest are intentionally ignored.
 */
function extensionZonesForParent(parent: 'main' | 'rightWorkspace') {
  return extensionPointsState.zones.filter((zone) => {
    if (BUILTIN_HOST_ZONES.has(zone.id) || hostViewsForZone(zone.id).length === 0) return false;
    const anchorParent = zone.anchorParent?.trim();
    return parent === 'rightWorkspace'
      ? anchorParent === 'rightWorkspace'
      : anchorParent !== 'rightWorkspace';
  });
}

// ── 主布局组件 ──────────────────────────────────────────

/**
 * 应用主布局。
 * 管理侧边栏、内容区、面板、工具栏、状态栏的整体布局。
 */
const MainLayout: Component<ParentProps> = (props) => {
  const navigate = useNavigate();
  const location = useLocation();

  /** 侧边栏是否可见 */
  const sidebarVisible = () => appState.sidebarVisible;

  /** 右侧动态面板区是否可见 */
  const rightWorkspaceVisible = () =>
    (appState.rightWorkspaceVisible && appState.rightWorkspaceColumns.length > 0)
    || extensionZonesForParent('rightWorkspace').length > 0;

  const [sidebarResizing, setSidebarResizing] = createSignal(false);
  const [rightWorkspaceResizing, setRightWorkspaceResizing] = createSignal(false);
  const notificationEventProjection = useNotificationEventProjection();

  onMount(() => {
    const disposeNavigator = registerAppNavigator(navigate);
    const disposeEditorBeforeUnloadGuard = installEditorBeforeUnloadGuard();
    notificationEventProjection.start();
    void loadExtensionPoints();
    const disposeDiscoveryInvalidation = installExtensionDiscoveryInvalidation();
    onCleanup(() => {
      notificationEventProjection.stop();
      disposeNavigator();
      disposeEditorBeforeUnloadGuard();
      disposeDiscoveryInvalidation();
    });
  });

  createEffect(() => {
    syncActiveViewFromPath(location.pathname);
  });

  createEffect(() => {
    void loadComposerRunState(appState.activeSessionId);
  });

  const startSidebarResize = (event: PointerEvent) => {
    event.preventDefault();
    setSidebarResizing(true);
    const startX = event.clientX;
    const startWidth = appState.sidebarWidth;

    const handleMove = (moveEvent: PointerEvent) => {
      setSidebarWidth(startWidth + moveEvent.clientX - startX);
    };

    const handleUp = () => {
      setSidebarResizing(false);
      document.removeEventListener('pointermove', handleMove);
      document.removeEventListener('pointerup', handleUp);
    };

    document.addEventListener('pointermove', handleMove);
    document.addEventListener('pointerup', handleUp);
  };

  const rightWorkspaceTotalWidth = () => {
    const columns = appState.rightWorkspaceColumns.length;
    if (columns === 0) return appState.rightWorkspaceWidth;
    return appState.rightWorkspaceWidth * columns + Math.max(0, columns - 1) * 6;
  };

  const startRightWorkspaceResize = (event: PointerEvent) => {
    event.preventDefault();
    setRightWorkspaceResizing(true);
    const startX = event.clientX;
    const startWidth = appState.rightWorkspaceWidth;
    const columnCount = Math.max(appState.rightWorkspaceColumns.length, 1);

    const handleMove = (moveEvent: PointerEvent) => {
      setRightWorkspaceWidth(startWidth + (startX - moveEvent.clientX) / columnCount);
    };

    const handleUp = () => {
      setRightWorkspaceResizing(false);
      document.removeEventListener('pointermove', handleMove);
      document.removeEventListener('pointerup', handleUp);
    };

    document.addEventListener('pointermove', handleMove);
    document.addEventListener('pointerup', handleUp);
  };

  return (
    <div class="flex h-screen w-screen flex-col overflow-hidden bg-white text-[#242424]">
      {/* ── 顶部工具栏与应用菜单 ── */}
      <Toolbar />
      <MenuBar />

       {/* ── 主体区域：侧边栏 + 内容区 + 面板 ── */}
      <div class="flex flex-1 overflow-hidden bg-white">
        {/* ── 侧边栏 ── */}
        <Show when={sidebarVisible()}>
          <aside
            id="leftSidebar"
            class="flex-shrink-0 overflow-hidden bg-white"
            style={{ width: `${appState.sidebarWidth}px`, 'min-width': '240px' }}
          >
            <Sidebar />
          </aside>
          <div
            class={`navis-resize-handle navis-sidebar-resizer ${sidebarResizing() ? 'is-resizing' : ''}`}
            role="separator"
            aria-label="调整左侧栏宽度"
            aria-orientation="vertical"
            onPointerDown={startSidebarResize}
          />
        </Show>

        {/* ── 右侧：对话工作台 + 右侧动态分列面板区 ── */}
        <div class="flex flex-1 overflow-hidden bg-white">
          {/* ── 中心内容区 ── */}
          <main class="flex flex-1 flex-col overflow-hidden bg-white">
            {/* 内容视图区由路由提供。 */}
            <div class="flex-1 overflow-hidden">
              {props.children}
            </div>

            <HostViewSurface zone="bottomDrawer" title="Extensions" />
            <For each={extensionZonesForParent('main')}>
              {(zone) => <HostViewSurface zone={zone.id} title={zone.name} />}
            </For>
          </main>

          <HostViewSurface zone="chatAside" title="Extensions" />

          {/* ── 右侧动态分列工作区 ── */}
          <Show when={rightWorkspaceVisible()}>
            <div
              class={`navis-resize-handle navis-right-workspace-resizer ${rightWorkspaceResizing() ? 'is-resizing' : ''}`}
              role="separator"
              aria-label="调整右侧面板区宽度"
              aria-orientation="vertical"
              onPointerDown={startRightWorkspaceResize}
            />
            <div
              id="rightWorkspace"
              class="navis-right-workspace flex flex-shrink-0 overflow-hidden"
              style={{ width: `${rightWorkspaceTotalWidth()}px` }}
            >
              <For each={appState.rightWorkspaceColumns}>
                {(column) => (
                  <div
                    class="navis-right-column flex min-h-0 min-w-[280px] flex-col overflow-hidden"
                    style={{ width: `${column.width}px` }}
                  >
                    <For each={column.panels}>
                      {(panel) => (
                        <WorkspacePanelShell
                          title={panel.title}
                          active={appState.activeRightWorkspacePanelId === panel.id}
                          closable={isRightWorkspacePanelClosable(panel)}
                          onFocus={() => focusRightWorkspacePanel(panel.id)}
                          onClose={(event) => {
                            event.stopPropagation();
                            closeRightWorkspacePanel(panel.id);
                          }}
                        >
                          <BuiltinRightWorkspaceContent panel={panel} />
                        </WorkspacePanelShell>
                      )}
                    </For>
                  </div>
                )}
              </For>
              <For each={extensionZonesForParent('rightWorkspace')}>
                {(zone) => <HostViewSurface zone={zone.id} title={zone.name} />}
              </For>
            </div>
          </Show>
        </div>
      </div>

      <StatusBar />
      <DialogManager />
      <ExtensionDialogManager />
      <ToastContainer />
      <GlobalSearchPalette />
      <CommandPalette />
    </div>
  );
};

export default MainLayout;



