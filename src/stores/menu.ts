import { createStore } from 'solid-js/store';
import { invoke } from '@tauri-apps/api/core';
import { canDispatchDeclarativeMenuAction } from './menu-actions';
import type { UiExtensionView } from '../lib/extension-ui';
import { evaluateMenuWhen } from '../lib/menu-when';

export type MenuTarget = string;

export type MenuRisk = 'low' | 'medium' | 'high';

export type MenuHostViewTarget = Pick<UiExtensionView,
  'viewId' | 'name' | 'icon' | 'zone' | 'placement' | 'renderer' | 'entry' | 'resourcePath' |
  'config' | 'allowClose' | 'defaultVisible'
>;

export type MenuBuiltinAction =
  | { type: 'OpenView'; view: MenuHostViewTarget }
  | { type: 'ToggleView'; view: MenuHostViewTarget }
  | { type: 'OpenDialog'; view: MenuHostViewTarget; size?: string | null; position?: string | null; modal?: boolean | null }
  | { type: 'RunScript'; scriptId: string; args?: unknown }
  | { type: 'SendMessage'; target: string; payload?: unknown };

export interface MenuActionItem {
  id: string;
  label: string;
  target: MenuTarget;
  command: string;
  risk?: MenuRisk;
  group?: string;
  when?: string;
  icon?: string;
  shortcut?: string;
  extensionId?: string;
  action?: MenuBuiltinAction;
}

interface MenuState {
  items: MenuActionItem[];
  openMenuId: string | null;
}

export const [menuState, setMenuState] = createStore<MenuState>({
  items: [],
  openMenuId: null,
});

export function getMenuItems(target: MenuTarget): MenuActionItem[] {
  return menuState.items
    .filter((item) => item.target === target)
    .filter((item) => evaluateMenuWhen(item.when));
}

export async function loadMenus(): Promise<void> {
  const items = await invoke<MenuActionItem[]>('ui_list_menus');
  setMenuState('items', items.filter(canDispatchDeclarativeMenuAction));
}

export function isMenuOpen(id: string): boolean {
  return menuState.openMenuId === id;
}

export function openMenu(id: string): void {
  setMenuState('openMenuId', id);
}

export function closeMenu(): void {
  setMenuState('openMenuId', null);
}

export function toggleMenu(id: string): void {
  setMenuState('openMenuId', (current) => (current === id ? null : id));
}

function getActiveMenuAnchor(id: string): HTMLElement | null {
  const anchors = document.querySelectorAll<HTMLElement>('[data-menu-anchor]');
  for (const anchor of anchors) {
    if (anchor.dataset.menuAnchor === id) return anchor;
  }
  return null;
}

export function installMenuDismissHandlers(): void {
  document.addEventListener(
    'pointerdown',
    (event) => {
      const openId = menuState.openMenuId;
      if (!openId) return;

      const target = event.target;
      if (!(target instanceof Node)) {
        closeMenu();
        return;
      }

      const activeAnchor = getActiveMenuAnchor(openId);
      if (activeAnchor?.contains(target)) return;

      closeMenu();
    },
    true,
  );

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && menuState.openMenuId) {
      closeMenu();
    }
  });
}

export function registerMenuItems(items: MenuActionItem[]): void {
  setMenuState('items', (current) => {
    const next = current.filter((item) => !items.some((incoming) => incoming.id === item.id));
    return [...next, ...items];
  });
}

export function unregisterMenuItems(extensionId: string): void {
  setMenuState('items', (current) => current.filter((item) => item.extensionId !== extensionId));
}
