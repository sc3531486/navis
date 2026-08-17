import { Component, For, Show } from 'solid-js';
import { FloatingMenu } from '../Menu';
import type { MenuActionItem } from '../../stores/menu';
import { closeMenu, getMenuItems, isMenuOpen, toggleMenu } from '../../stores/menu';
import { executeDeclarativeMenuAction } from '../../stores/menu-actions';
import { executeToolsMenuItem } from '../../stores/tools-menu';

/**
 * 顶部菜单栏 target 命名空间（设计 §3.4，34 文档 L395）：
 * 扩展贡献项按 `Menubar.File/Edit/View/Help/Tools` 声明。
 * 后端 ui_list_menus 目前仍输出裸 target（menu_targets 常量，如 `Tools`），
 * 因此每个按钮同时查询 Menubar.* 与裸 target，保证新旧约定下扩展菜单项都不丢失。
 */
interface MenuBarTarget {
  /** 设计命名空间：Menubar.<Name> */
  namespace: string;
  /** 按钮显示名 */
  label: string;
  /** 兼容后端裸 target（menu_targets 常量，如 `Tools`） */
  legacy: string;
}

const MENU_TARGETS: readonly MenuBarTarget[] = [
  { namespace: 'Menubar.File', label: 'File', legacy: 'File' },
  { namespace: 'Menubar.Edit', label: 'Edit', legacy: 'Edit' },
  { namespace: 'Menubar.View', label: 'View', legacy: 'View' },
  { namespace: 'Menubar.Help', label: 'Help', legacy: 'Help' },
  { namespace: 'Menubar.Tools', label: 'Tools', legacy: 'Tools' },
];

/** 合并 Menubar.* 命名空间与裸 target 的菜单项，按 id 去重。 */
function menuItemsFor(entry: MenuBarTarget): MenuActionItem[] {
  const seen = new Set<string>();
  return [...getMenuItems(entry.namespace), ...getMenuItems(entry.legacy)].filter((item) => {
    if (seen.has(item.id)) return false;
    seen.add(item.id);
    return true;
  });
}

const MenuBar: Component = () => {
  const selectItem = async (item: MenuActionItem): Promise<void> => {
    closeMenu();
    if (item.extensionId) {
      executeDeclarativeMenuAction(item);
      return;
    }
    await executeToolsMenuItem(item);
  };

  return (
    <nav class="flex h-7 shrink-0 items-center border-b border-[#e8e8e8] bg-[#fafafa] px-2 text-[11px] text-[#444]" aria-label="Application menu">
<For each={MENU_TARGETS}>
        {(target) => {
          const menuId = `menu-bar-${target.label.toLowerCase()}`;
          const items = () => menuItemsFor(target);
          return (
            <div class="relative h-full" data-menu-anchor={menuId}>
              <button
                type="button"
                class="h-full rounded px-2 hover:bg-[#ededed] disabled:opacity-50"
                aria-haspopup="menu"
                aria-expanded={isMenuOpen(menuId)}
                disabled={items().length === 0}
                onClick={() => toggleMenu(menuId)}
              >
                {target.label}
              </button>
              <Show when={items().length > 0 && isMenuOpen(menuId)}>
                <FloatingMenu
                  items={items()}
                  triggerLabel={target.label}
                  width={220}
                  showSourceLabel={false}
                  onSelect={(item) => void selectItem(item)}
                />
              </Show>
            </div>
          );
        }}
      </For>
    </nav>
  );
};

export default MenuBar;
