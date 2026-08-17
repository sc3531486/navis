/**
 * ============================================================
 * Navis 主题系统 - 切换逻辑
 * ============================================================
 *
 * 提供主题管理的核心功能：
 *   - 主题类型定义（light / dark / system）
 *   - 响应式信号：当前生效的主题
 *   - 切换主题的函数
 *   - 监听系统主题变化（prefers-color-scheme）
 *
 * 依赖：Solid.js 响应式原语
 * 被依赖：应用主题生命周期与主题切换入口
 *
 * 来源：design/22-ui-framework.md 第四章 主题系统
 * ============================================================
 */

import { createSignal, createEffect, onCleanup } from 'solid-js';

// ── 类型定义 ────────────────────────────────────────────

/** 主题偏好：用户可选择的值 */
export type ThemePreference = 'light' | 'dark' | 'system';

/** 实际生效的主题：排除 system，只有 light 或 dark */
export type ResolvedTheme = 'light' | 'dark';

// ── 响应式信号 ──────────────────────────────────────────

/**
 * 用户的主题偏好。
 * 'system' 表示跟随操作系统设置。
 */
const [themePreference, setThemePreference] = createSignal<ThemePreference>('system');

/**
 * 实际生效的主题（已解析 system 为具体值）。
 * 只读信号，外部通过 setThemePreference 改变。
 */
const [resolvedTheme, setResolvedTheme] = createSignal<ResolvedTheme>('light');

// ── 内部工具函数 ────────────────────────────────────────

/**
 * 获取操作系统当前的颜色方案偏好。
 * 使用 matchMedia API 检测 prefers-color-scheme: dark。
 *
 * @returns 'dark' 如果系统偏好暗色，否则 'light'
 */
function getSystemTheme(): ResolvedTheme {
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light';
}

/**
 * 将主题应用到 DOM。
 * 在 <html> 元素上设置 data-theme 属性，
 * CSS 选择器 [data-theme="dark"] / [data-theme="light"]
 * 会自动激活对应的变量覆盖。
 *
 * @param theme - 要应用的已解析主题
 */
function applyThemeToDOM(theme: ResolvedTheme): void {
  if (typeof document === 'undefined') return;
  document.documentElement.setAttribute('data-theme', theme);
}

/**
 * 解析主题偏好为实际主题。
 * 如果偏好是 'system'，则读取操作系统设置。
 *
 * @param preference - 用户偏好
 * @returns 实际生效的主题
 */
function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference === 'system') return getSystemTheme();
  return preference;
}

// ── 生命周期安装 ────────────────────────────────────────

/**
 * 安装主题响应式副作用和系统主题监听。
 * 必须在 Solid render root 内调用，确保监听和响应式计算能随应用生命周期释放。
 */
export function installThemeLifecycle(): void {
  createEffect(() => {
    const preference = themePreference();
    const resolved = resolveTheme(preference);
    setResolvedTheme(resolved);
    applyThemeToDOM(resolved);
  });

  if (typeof window === 'undefined') return;

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

  const handleSystemThemeChange = () => {
    if (themePreference() === 'system') {
      const resolved = getSystemTheme();
      setResolvedTheme(resolved);
      applyThemeToDOM(resolved);
    }
  };

  mediaQuery.addEventListener('change', handleSystemThemeChange);

  onCleanup(() => {
    mediaQuery.removeEventListener('change', handleSystemThemeChange);
  });
}

// ── 公开 API ────────────────────────────────────────────

/**
 * 设置主题偏好。
 *
 * @param preference - 'light' | 'dark' | 'system'
 *
 * @example
 * ```ts
 * setTheme('dark');    // 强制暗色
 * setTheme('system');  // 跟随系统
 * ```
 */
export function setTheme(preference: ThemePreference): void {
  setThemePreference(preference);

  // 持久化到 localStorage，下次启动时恢复
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('navis-theme', preference);
  }
}

/**
 * 在 light ↔ dark 之间快速切换。
 * 如果当前是 system 模式，切换为与当前生效主题相反的固定主题。
 */
export function toggleTheme(): void {
  const current = resolvedTheme();
  setTheme(current === 'dark' ? 'light' : 'dark');
}

/**
 * 从 localStorage 恢复用户上次的主题偏好。
 * 应在应用初始化时调用一次。
 *
 * @example
 * ```ts
 * // App.tsx 顶层
 * restoreTheme();
 * ```
 */
export function restoreTheme(): void {
  if (typeof localStorage === 'undefined') return;
  const saved = localStorage.getItem('navis-theme') as ThemePreference | null;
  if (saved && ['light', 'dark', 'system'].includes(saved)) {
    setTheme(saved);
  }
}

/**
 * 获取当前主题偏好（只读）。
 */
export function getThemePreference(): ThemePreference {
  return themePreference();
}

/**
 * 获取当前生效的主题（只读）。
 * 已将 system 解析为 light 或 dark。
 */
export function getResolvedTheme(): ResolvedTheme {
  return resolvedTheme();
}
