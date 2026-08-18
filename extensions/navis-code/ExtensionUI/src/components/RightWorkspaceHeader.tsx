/**
 * RightWorkspaceHeader — 右侧面板顶部工具栏
 *
 * 包含下拉菜单按钮和面板选择器。
 * 提取自 router/index.tsx 中的内联代码。
 */
import { Component, Show } from 'solid-js';
import { closeMenu, getMenuItems, isMenuOpen, toggleMenu, type MenuActionItem } from '@/stores/menu';
import { executeRightWorkspaceMenuItem, getOpenRightWorkspaceCommands } from '@project-ext/stores/right-workspace-menu';
import { FloatingMenu } from './Menu';
import { PanelIconCompact, ChevronDownCompact } from './Icon';

export const RightWorkspaceHeader: Component = () => {
  const handleSelect = (item: MenuActionItem): void => {
    executeRightWorkspaceMenuItem(item);
    closeMenu();
  };

  return (
    <div class="relative" data-menu-anchor="right-panel">
      <button
        type="button"
        class="navis-chat-panel-button flex h-6 items-center rounded-md"
        aria-label="Right workspace menu"
        title="Right workspace menu"
        aria-expanded={isMenuOpen('right-panel')}
        onClick={() => toggleMenu('right-panel')}
      >
        <PanelIconCompact />
        <ChevronDownCompact />
      </button>
      <Show when={isMenuOpen('right-panel')}>
        <FloatingMenu
          items={getMenuItems('RightPanel')}
          triggerLabel="Right workspace menu"
          align="right"
          width={180}
          showSourceLabel={false}
          selectedCommands={getOpenRightWorkspaceCommands(getMenuItems('RightPanel'))}
          onSelect={handleSelect}
        />
      </Show>
    </div>
  );
};
