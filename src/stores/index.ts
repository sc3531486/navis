/**
 * Navis 框架层状态导出。
 *
 * 这里只导出窗口、扩展、菜单和宿主视图等通用状态；产品业务状态由产品扩展自行导出。
 */
export * from './host';
export * from './bridge';
export * from './discovery';
export * from './extension';
export * from './extension-commands';
export * from './extension-keybindings';
export * from './extension-points';
export * from './extension-workers';
export * from './menu';
export * from './menu-actions';
export * from './menu-command-coverage';
export * from './view-navigation';
