import { ParentProps, onMount } from 'solid-js';
import { getHotkeyManager } from './lib/hotkey';
import { initHostState, installHostStatePersistence } from './stores/host';
import { installMenuDismissHandlers, loadMenus } from './stores/menu';
import { loadExtensions } from './stores/extension';
import { installThemeLifecycle, restoreTheme } from './theme';

let initialized = false;

/**
 * 初始化通用框架运行时。
 *
 * 产品入口只负责选择产品界面，扩展发现、菜单、主题和快捷键由框架统一启动。
 */
export function initializeFramework(): void {
  if (initialized) return;
  initialized = true;

  getHotkeyManager().init();
  void loadMenus();
  void loadExtensions();
  installMenuDismissHandlers();
  initHostState();
  restoreTheme();
}

/**
 * 为产品入口提供通用的生命周期挂载。
 */
export function FrameworkLifecycle(props: ParentProps) {
  initializeFramework();
  onMount(() => {
    installHostStatePersistence();
    installThemeLifecycle();
  });
  return props.children;
}
