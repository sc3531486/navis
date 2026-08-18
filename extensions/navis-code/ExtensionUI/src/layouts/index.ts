/**
 * ============================================================
 * Navis Layouts 统一导出 - layouts/index.ts
 * ============================================================
 *
 * 集中导出所有布局组件，方便外部一次性引入。
 *
 * @example
 * ```tsx
 * import { MainLayout, Sidebar, StatusBar, Toolbar } from '@/layouts';
 * ```
 * ============================================================
 */

export { default as MainLayout } from './MainLayout';
export { default as Sidebar } from '@session/layouts/Sidebar';
export { default as StatusBar } from '@agent-core/layouts/StatusBar';
export { default as Toolbar } from './Toolbar';
