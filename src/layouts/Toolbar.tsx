/**
 * ============================================================
 * Navis 自定义标题栏 - layouts/Toolbar.tsx
 * ============================================================
 *
 * 替换系统原生标题栏，实现 Navis 桌面端沉浸式体验。
 *
 * 布局结构（从左到右）：
 *   [菜单] [侧栏] [搜索] [后退] [前进] [── 拖拽区域（面包屑）──] [─] [□] [×]
 *
 * 窗口控制按钮位于 data-tauri-drag-region 之外，
 * 确保点击按钮不会触发窗口拖拽。
 *
 * ============================================================
 */

import { Component, For, Show, createSignal, onMount, onCleanup } from 'solid-js';
import { appState, toggleSidebar } from '../stores/app';
import { commandPaletteAPI } from '../components/CommandPalette';
import { globalSearchAPI } from '../components/GlobalSearchPalette';
import { useHotkeyCommand } from '../lib/hotkey';
import { openChatView, openEditorView } from '../stores/view-navigation';
import { FloatingMenu } from '../components/Menu';
import { closeMenu, getMenuItems, isMenuOpen, loadMenus, toggleMenu, type MenuActionItem } from '../stores/menu';
import { executeToolsMenuItem } from '../stores/tools-menu';
import { executeExtensionPoint, extensionPointIcon, extensionPointsByKind } from '../stores/extension-points';
import CloseIcon from '../components/Icon/CloseIcon';
import {
  IconHamburger,
  IconSidebar,
  IconSearch,
  IconArrowLeft,
  IconArrowRight,
  IconMinimize,
  IconMaximize,
  IconRestore,
} from '../components/Icon';

// ── Tauri 窗口 API ─────────────────────────────────────────

let tauriWindow: any = null;

/** 获取当前窗口实例 */
async function getAppWindow() {
  if (tauriWindow) return tauriWindow;
  try {
    const mod = await import('@tauri-apps/api/window');
    tauriWindow = mod.getCurrentWindow();
    return tauriWindow;
  } catch {
    return null;
  }
}

/** 最小化窗口 */
async function minimizeWindow() {
  const win = await getAppWindow();
  if (win) await win.minimize();
}

/** 最大化/还原窗口 */
async function toggleMaximize() {
  const win = await getAppWindow();
  if (!win) return;
  const maximized = await win.isMaximized();
  if (maximized) {
    await win.unmaximize();
  } else {
    await win.maximize();
  }
}

/** 关闭窗口 */
async function closeWindow() {
  const win = await getAppWindow();
  if (win) await win.close();
}

function navigateBack() {
  window.history.back();
}

function navigateForward() {
  window.history.forward();
}

/** 拖动窗口。data-tauri-drag-region 之外再加一层 API 兜底。 */
function startWindowDrag(event: MouseEvent) {
  if (event.button !== 0) return;

  const target = event.target as HTMLElement | null;
  if (target?.closest('button')) return;

  event.preventDefault();

  if (tauriWindow?.startDragging) {
    void tauriWindow.startDragging();
    return;
  }

  void getAppWindow().then((win) => win?.startDragging?.());
}

// ── SVG 图标组件已迁移至 src/components/Icon/index.ts ──────

// ── 样式常量 ──────────────────────────────────────────────

/** 顶栏操作按钮：24x24 圆角，hover 背景变亮，鼠标保持系统默认样式 */
const TOP_BTN =
  'relative flex h-[24px] w-[24px] items-center justify-center rounded-md ' +
  'text-[#777777] transition-colors hover:bg-[#ececec] hover:text-[#242424]';

/** 窗口控制按钮：紧凑标题栏尺寸，hover 背景变亮，鼠标保持系统默认样式 */
const WIN_CTRL_BTN =
  'flex h-[28px] w-[40px] items-center justify-center ' +
  'text-[#777777] transition-colors hover:bg-[#ececec] hover:text-[#242424]';

/** 关闭按钮特有样式：hover 时变红 */
const WIN_CLOSE_BTN =
  WIN_CTRL_BTN + ' hover:bg-[#d94f4f] hover:text-white';

// ── 工具栏组件 ──────────────────────────────────────────

/**
 * 自定义标题栏 — 替换系统原生标题栏。
 *
 * 左侧：菜单、侧栏、搜索、导航
 * 中间：拖拽区域 + 面包屑导航
 * 右侧：窗口控制按钮
 */
const Toolbar: Component = () => {
  /** 侧边栏是否可见 */
  const sidebarVisible = () => appState.sidebarVisible;

  /** 窗口最大化状态（用于切换图标） */
  const [maximized, setMaximized] = createSignal(false);
  const [toolsMenuLoading, setToolsMenuLoading] = createSignal(false);

  useHotkeyCommand('commandPalette.open', () => commandPaletteAPI.open());
  useHotkeyCommand('sidebar.toggle', toggleSidebar);

  const toolsMenuItems = () => getMenuItems('Tools');

  async function handleTopbarToolsClick(): Promise<void> {
    if (toolsMenuItems().length === 0) {
      if (toolsMenuLoading()) return;
      setToolsMenuLoading(true);
      try {
        await loadMenus();
      } finally {
        setToolsMenuLoading(false);
      }
    }

    if (toolsMenuItems().length > 0) {
      toggleMenu('topbar-tools');
      return;
    }

    closeMenu();
  }

  async function handleToolsMenuSelect(item: MenuActionItem): Promise<void> {
    closeMenu();
    await executeToolsMenuItem(item);
  }

  /** 监听窗口最大化事件，同步图标状态 */
  onMount(async () => {
    const commandIds = [
      'app.commandPalette.open',
      'app.sidebar.toggle',
      'navigation.back',
      'navigation.forward',
      'navigation.openChat',
      'navigation.openEditor',
      'window.minimize',
      'window.toggleMaximize',
      'window.close',
    ];
    commandPaletteAPI.registerBatch([
      {
        id: 'app.commandPalette.open',
        label: 'Open command palette',
        description: '搜索命令、文件、Skills 或符号',
        category: 'Application',
        keybinding: 'Ctrl+Shift+P',
        source: 'builtin',
        handler: () => commandPaletteAPI.open(),
      },
      {
        id: 'app.sidebar.toggle',
        label: 'Toggle sidebar',
        description: sidebarVisible() ? '收起左侧栏' : '展开左侧栏',
        category: 'Layout',
        source: 'builtin',
        handler: toggleSidebar,
      },
      {
        id: 'navigation.back',
        label: 'Back',
        description: '返回上一条导航历史',
        category: 'Navigation',
        source: 'builtin',
        handler: navigateBack,
      },
      {
        id: 'navigation.forward',
        label: 'Forward',
        description: '前进到下一条导航历史',
        category: 'Navigation',
        source: 'builtin',
        handler: navigateForward,
      },
      {
        id: 'navigation.openChat',
        label: 'Open chat Worktree',
        description: '切换到当前会话的聊天工作台',
        category: 'Navigation',
        source: 'builtin',
        handler: () => openChatView(),
      },
      {
        id: 'navigation.openEditor',
        label: 'Open editor',
        description: '切换到当前会话的 Worktree 编辑器',
        category: 'Navigation',
        source: 'builtin',
        handler: () => openEditorView(),
      },
      {
        id: 'window.minimize',
        label: 'Minimize window',
        category: 'Window',
        source: 'builtin',
        handler: minimizeWindow,
      },
      {
        id: 'window.toggleMaximize',
        label: 'Maximize or restore window',
        category: 'Window',
        source: 'builtin',
        handler: toggleMaximize,
      },
      {
        id: 'window.close',
        label: 'Close window',
        category: 'Window',
        source: 'builtin',
        handler: closeWindow,
      },
    ]);
    onCleanup(() => {
      for (const id of commandIds) {
        commandPaletteAPI.unregister(id);
      }
    });

    try {
      const win = await getAppWindow();
      const initialMaximized = await win.isMaximized();
      setMaximized(initialMaximized);

      // 监听最大化/还原事件
      const unlisten = await win.onResized(async () => {
        const m = await win.isMaximized();
        setMaximized(m);
      });

      onCleanup(() => {
        unlisten();
      });
    } catch {
      // 非 Tauri 环境（如浏览器预览），忽略
    }
  });

  return (
    <header
      class="flex h-[28px] flex-shrink-0 select-none items-center border-b border-[#dadada] bg-white text-[#242424] shadow-[0_1px_2px_rgba(0,0,0,0.03)]"
      role="toolbar"
      aria-label="标题栏"
      data-tauri-drag-region
      onMouseDown={startWindowDrag}
      onDblClick={toggleMaximize}
    >
      {/* ── 左侧：基础内联工具 ── */}
      <div class="relative flex h-full items-center" data-menu-anchor="topbar-tools">
        <button
          class={`${TOP_BTN} navis-topbar-start`}
          title="Tools"
          aria-label="Tools"
          aria-expanded={isMenuOpen('topbar-tools')}
          disabled={toolsMenuLoading()}
          onClick={() => void handleTopbarToolsClick()}
        >
          <IconHamburger />
        </button>
        <Show when={toolsMenuItems().length > 0 && isMenuOpen('topbar-tools')}>
          <FloatingMenu
            items={toolsMenuItems()}
            triggerLabel="Tools"
            placement="below"
            align="left"
            width={210}
            onSelect={(item) => void handleToolsMenuSelect(item)}
          />
        </Show>
      </div>
      <button
        class={`${TOP_BTN} navis-topbar-sidebar border-x border-[#dadada]`}
        onClick={toggleSidebar}
        title={sidebarVisible() ? '收起侧边栏' : '展开侧边栏'}
        aria-label="切换侧边栏"
      >
        <IconSidebar />
      </button>
      <button
        class={TOP_BTN}
        title="全局搜索"
        aria-label="全局搜索"
        onClick={() => globalSearchAPI.open()}
      >
        <IconSearch />
      </button>
      <button class={TOP_BTN} title="后退" aria-label="后退" onClick={navigateBack}>
        <IconArrowLeft />
      </button>
      <button
        class={`${TOP_BTN} text-[#bdbdbd] hover:text-[#777777]`}
        title="前进"
        aria-label="前进"
        onClick={navigateForward}
      >
        <IconArrowRight />
      </button>

      {/* ── 中间：窗口拖拽区域。顶部栏不承载会话标题。 ── */}
      <div
        class="flex h-full min-w-0 flex-1 items-center px-2"
        data-tauri-drag-region
      />

      {/* ── 右侧：交互按钮 + 窗口控制 ── */}
      <div class="flex items-center h-full flex-shrink-0">
        <For each={extensionPointsByKind('toolbar')}>
          {(point) => {
            const icon = extensionPointIcon(point);
            return (
              <button
                class={`${TOP_BTN} navis-topbar-extension`}
                title={point.label ?? point.id}
                aria-label={point.label ?? point.id}
                disabled={!point.command}
                onClick={() => executeExtensionPoint(point)}
              >
                <span class="inline-block max-w-[18px] overflow-hidden text-[10px] font-semibold leading-none whitespace-nowrap">
                  {icon ?? (point.label ?? point.id).slice(0, 2)}
                </span>
              </button>
            );
          }}
        </For>
        <div class="navis-topbar-divider h-[14px] w-px bg-[#dadada]" />

        {/* ── 窗口控制按钮（不在拖拽区域内） ── */}
        {/* 最小化 */}
        <button
          class={WIN_CTRL_BTN}
          onClick={minimizeWindow}
          title="最小化"
          aria-label="最小化窗口"
        >
          <IconMinimize />
        </button>

        {/* 最大化/还原 */}
        <button
          class={WIN_CTRL_BTN}
          onClick={toggleMaximize}
          title={maximized() ? '还原窗口' : '最大化窗口'}
          aria-label={maximized() ? '还原窗口' : '最大化窗口'}
        >
          {maximized() ? <IconRestore /> : <IconMaximize />}
        </button>

        {/* 关闭（hover 变红） */}
        <button
          class={WIN_CLOSE_BTN}
          onClick={closeWindow}
          title="关闭"
          aria-label="关闭窗口"
        >
          <CloseIcon />
        </button>
      </div>
    </header>
  );
};

export default Toolbar;


