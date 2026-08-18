/**
 * Navis 通用宿主状态。
 *
 * 仅维护桌面白板、扩展视图投影和通用窗口偏好；任何产品领域状态必须由产品自身维护。
 */

import { createStore } from 'solid-js/store';
import { createEffect } from 'solid-js';
import type { UiExtensionView } from '@/lib/extension-ui';
import type { BridgeContextSnapshot } from './bridge';
import { closeExtensionDialogsForExtension } from '../components/ExtensionDialog/store';

// ── 类型定义 ──────────────────────────────────────────────

/**
 * 窗口状态描述。
 * 用于记录窗口位置和尺寸，支持持久化与恢复。
 */
export interface WindowState {
  /** 窗口 X 坐标（像素） */
  x: number;
  /** 窗口 Y 坐标（像素） */
  y: number;
  /** 窗口宽度（像素） */
  width: number;
  /** 窗口高度（像素） */
  height: number;
  /** 是否最大化 */
  maximized: boolean;
}

export interface RightWorkspacePanel {
  id: string;
  title: string;
  viewId: string;
  config?: unknown;
  /** 历史面板上下文兼容字段；新扩展应优先使用 config。 */
  sessionId?: string;
  extensionView?: UiExtensionView;
}

export type HostViewZone =
  | 'rightWorkspace'
  | 'chatAside'
  | 'bottomDrawer'
  | 'settingsSection'
  | string;

export interface HostViewInstance extends UiExtensionView {
  id: string;
}

/** 产品可注册扩展沙箱上下文提供器，宿主不解释其中的领域字段。 */
export type HostViewContextProvider = (view: UiExtensionView) => BridgeContextSnapshot;

let hostViewContextProvider: HostViewContextProvider | undefined;

/** 注册当前产品的扩展沙箱上下文提供器。 */
export function registerHostViewContextProvider(provider: HostViewContextProvider): void {
  hostViewContextProvider = provider;
}

/** 获取扩展沙箱上下文；未加载产品时返回空上下文。 */
export function getHostViewContext(view: UiExtensionView): BridgeContextSnapshot {
  return hostViewContextProvider?.(view) ?? {};
}

export function isHostViewClosable(view: Pick<UiExtensionView, 'allowClose'> | undefined): boolean {
  return view?.allowClose ?? true;
}

export function isRightWorkspacePanelClosable(panel: RightWorkspacePanel): boolean {
  return isHostViewClosable(panel.extensionView);
}

export interface RightWorkspaceColumn {
  id: string;
  width: number;
  panels: RightWorkspacePanel[];
}

/**
 * 通用宿主状态接口。
 *
 * 这是通用宿主 UI 的唯一真实来源。
 * 产品业务状态由各产品自己的 Store 维护，不能写入宿主状态。
 */
export interface HostState {
  /** 窗口状态 */
  windowState: WindowState;
  /** 全局加载指示器 */
  globalLoading: boolean;
  /** 全局错误（用作错误边界兜底） */
  globalError: Error | null;
  /** 是否离线 */
  isOffline: boolean;
  /** 是否有可用更新 */
  updateAvailable: boolean;
  /** 侧边栏是否可见 */
  sidebarVisible: boolean;
  /** 左侧栏宽度 */
  sidebarWidth: number;
  /** 右侧动态分列面板区是否可见 */
  rightWorkspaceVisible: boolean;
  /** 右侧动态分列面板区整体宽度 */
  rightWorkspaceWidth: number;
  /** 右侧动态分列面板区当前列布局 */
  rightWorkspaceColumns: RightWorkspaceColumn[];
  /** 当前聚焦的右侧面板区面板 ID */
  activeRightWorkspacePanelId: string | null;
  /** UI Host view 运行态实例；这是 UI surface 状态，不是 Kernel/Extension Registry。 */
  hostViewInstances: HostViewInstance[];
  /** 每个 UI Host view surface 当前聚焦的实例 ID。 */
  activeHostViewByZone: Record<string, string | undefined>;
  /** Deprecated compatibility alias; do not use for new code. */
  activeHostViewByPlacement: Record<string, string | undefined>;
  /** 当前宿主投影视图标识，由产品路由或扩展投影更新。 */
  activeView: string;
}

// ── 默认值 ────────────────────────────────────────────────

/** 窗口状态默认值 */
const defaultWindowState: WindowState = {
  x: 100,
  y: 100,
  width: 1280,
  height: 800,
  maximized: false,
};

const RIGHT_WORKSPACE_MIN_WIDTH = 280;
const RIGHT_WORKSPACE_MAX_WIDTH = 560;

/** 全局应用状态默认值 */
const defaultHostState: HostState = {
  windowState: defaultWindowState,
  globalLoading: false,
  globalError: null,
  isOffline: false,
  updateAvailable: false,
  sidebarVisible: true,
  sidebarWidth: 280,
  rightWorkspaceVisible: false,
  rightWorkspaceWidth: 360,
  rightWorkspaceColumns: [],
  activeRightWorkspacePanelId: null,
  hostViewInstances: [],
  activeHostViewByZone: {},
  activeHostViewByPlacement: {},
  activeView: 'main',
};

// ── Store 实例 ────────────────────────────────────────────

/**
 * 通用宿主状态 Store。
 * 使用 Solid.js createStore 实现嵌套响应式更新。
 *
 * 所有通用宿主 UI 状态存放于此；产品业务状态不属于该 Store。
 *
 */
export const [hostState, setHostState] = createStore<HostState>({ ...defaultHostState });

// ── 持久化 ───────────────────────────────────────────────

/** localStorage 键前缀 */
const STORAGE_KEY = 'navis-host-state';

/**
 * 将持久化状态保存到 localStorage。
 * 仅保存用户偏好字段，不保存瞬态状态（如 globalLoading）。
 */
function persistState(): void {
  if (typeof localStorage === 'undefined') return;

  const persistable: Partial<HostState> = {
    sidebarVisible: hostState.sidebarVisible,
    sidebarWidth: hostState.sidebarWidth,
    rightWorkspaceWidth: hostState.rightWorkspaceWidth,
    activeView: hostState.activeView,
  };

  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(persistable));
  } catch {
    // localStorage 可能已满或不可用，静默失败
  }
}

/**
 * 从 localStorage 恢复持久化状态。
 * 仅恢复有效字段，其余使用默认值。
 * 应在应用初始化阶段调用一次。
 *
 * @example
 * ```ts
 * // 在 App.tsx 顶层调用
 * restoreHostState();
 * ```
 */
export function restoreHostState(): void {
  if (typeof localStorage === 'undefined') return;

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;

    const saved = JSON.parse(raw) as Partial<HostState>;

    // 逐字段恢复，确保类型安全
    if (typeof saved.sidebarVisible === 'boolean') {
      setHostState('sidebarVisible', saved.sidebarVisible);
    }
    if (typeof saved.sidebarWidth === 'number') {
      setHostState('sidebarWidth', Math.min(Math.max(saved.sidebarWidth, 240), 420));
    }
    if (typeof saved.rightWorkspaceWidth === 'number') {
      setHostState(
        'rightWorkspaceWidth',
        Math.min(Math.max(saved.rightWorkspaceWidth, RIGHT_WORKSPACE_MIN_WIDTH), RIGHT_WORKSPACE_MAX_WIDTH),
      );
    }
    setHostState('rightWorkspaceVisible', false);
    setHostState('rightWorkspaceColumns', []);
    setHostState('activeRightWorkspacePanelId', null);
    if (typeof saved.activeView === 'string') {
      setHostState('activeView', saved.activeView);
    }
  } catch {
    // JSON 解析失败，忽略损坏的数据
  }
}

export function installHostStatePersistence(): void {
  createEffect(() => {
    // 读取所有持久化字段，建立响应式依赖追踪
    void hostState.sidebarVisible;
    void hostState.sidebarWidth;
    void hostState.rightWorkspaceWidth;
    void hostState.activeView;

    // 延迟持久化，避免频繁写入
    queueMicrotask(persistState);
  });
}

// ── 宿主操作 ── ────────────────────────────────────────────


/**
 * 设置离线状态。
 *
 * @param isOffline - 应用是否离线
 */
export function setOffline(isOffline: boolean): void {
  setHostState('isOffline', isOffline);
}

/**
 * 设置全局错误（用于错误边界捕获）。
 *
 * @param error - 捕获的错误，null 表示清除错误
 */
export function setError(error: Error | null): void {
  setHostState('globalError', error);
}

/**
 * 设置全局加载状态。
 *
 * @param loading - 是否正在加载
 */
export function setLoading(loading: boolean): void {
  setHostState('globalLoading', loading);
}

/**
 * 切换侧边栏可见性。
 */
export function toggleSidebar(): void {
  setHostState('sidebarVisible', (v) => !v);
}

export function setSidebarWidth(width: number): void {
  setHostState('sidebarWidth', Math.min(Math.max(width, 240), 420));
}

/**
 * 切换右侧动态面板区可见性。
 */
export function toggleRightWorkspace(): void {
  setHostState('rightWorkspaceVisible', (v) => !v);
}

export function setRightWorkspaceWidth(width: number): void {
  const nextWidth = Math.min(Math.max(width, RIGHT_WORKSPACE_MIN_WIDTH), RIGHT_WORKSPACE_MAX_WIDTH);
  setHostState('rightWorkspaceWidth', nextWidth);
  setHostState('rightWorkspaceColumns', (columns) =>
    columns.map((column) => ({
      ...column,
      width: nextWidth,
    })),
  );
}

/**
 * 初始化宿主状态并恢复通用持久化偏好。
 * 应在应用启动阶段调用一次。
 *
 * 当前委托给 restoreHostState() 从 localStorage 读取。
 */
export function initHostState(): void {
  restoreHostState();
}

/**
 * 设置当前宿主投影视图标识。
 *
 * @param view - 产品路由或扩展投影提供的视图标识
 */
export function setActiveView(view: string): void {
  setHostState('activeView', view);
}


export function hostViewInstanceId(extensionId: string, viewId: string): string {
  return `${extensionId}:${viewId}`;
}

function hostViewZone(view: Pick<UiExtensionView, 'zone' | 'placement'>): string {
  return view.zone || view.placement;
}

export function hostViewsForZone(zone: string): HostViewInstance[] {
  return hostState.hostViewInstances.filter((view) => hostViewZone(view) === zone);
}

export function activeHostViewForZone(zone: string): HostViewInstance | undefined {
  const activeId = hostState.activeHostViewByZone[zone] ?? hostState.activeHostViewByPlacement[zone];
  return hostState.hostViewInstances.find((view) => view.id === activeId && hostViewZone(view) === zone)
    ?? hostViewsForZone(zone)[0];
}

/** Deprecated compatibility wrapper. */
export function hostViewsForPlacement(placement: string): HostViewInstance[] {
  return hostViewsForZone(placement);
}

/** Deprecated compatibility wrapper. */
export function activeHostViewForPlacement(placement: string): HostViewInstance | undefined {
  return activeHostViewForZone(placement);
}

export function isHostViewOpen(extensionId: string, viewId: string): boolean {
  const id = hostViewInstanceId(extensionId, viewId);
  return hostState.hostViewInstances.some((view) => view.id === id);
}

export function isRightWorkspacePanelOpen(extensionId: string, viewId: string): boolean {
  const id = hostViewInstanceId(extensionId, viewId);
  return hostState.rightWorkspaceColumns.some((column) =>
    column.panels.some((panel) => panel.id === id),
  );
}

export function openHostView(instance: HostViewInstance): void {
  setHostState('hostViewInstances', (instances) => {
    const exists = instances.some((view) => view.id === instance.id);
    return exists
      ? instances.map((view) => (view.id === instance.id ? { ...view, ...instance } : view))
      : [...instances, instance];
  });
  const zone = hostViewZone(instance);
  setHostState('activeHostViewByZone', zone, instance.id);
  setHostState('activeHostViewByPlacement', zone, instance.id);
}

export function closeHostView(instanceId: string): void {
  const closing = hostState.hostViewInstances.find((view) => view.id === instanceId);
  if (!closing || !isHostViewClosable(closing)) return;

  setHostState('hostViewInstances', (instances) => instances.filter((view) => view.id !== instanceId));

  const next = hostState.hostViewInstances.find(
    (view) => view.id !== instanceId && hostViewZone(view) === hostViewZone(closing),
  );
  const zone = hostViewZone(closing);
  setHostState('activeHostViewByZone', zone, next?.id);
  setHostState('activeHostViewByPlacement', zone, next?.id);
}

export function focusHostView(instanceId: string): void {
  const instance = hostState.hostViewInstances.find((view) => view.id === instanceId);
  if (!instance) return;
  const zone = hostViewZone(instance);
  setHostState('activeHostViewByZone', zone, instance.id);
  setHostState('activeHostViewByPlacement', zone, instance.id);
}

/**
 * Remove all runtime UI projections contributed by an extension.
 * Extension lifecycle stores call this after a successful disable/uninstall;
 * individual surfaces must not own extension cleanup rules.
 */
export function removeHostViewsForExtension(extensionId: string): void {
  // 禁用/卸载时同步关闭该扩展的全部弹框（设计 §7 状态转换 / §13 阶段 3）
  closeExtensionDialogsForExtension(extensionId);

  const removedHostViewIds = new Set(
    hostState.hostViewInstances
      .filter((view) => view.extensionId === extensionId)
      .map((view) => view.id),
  );
  const remainingHostViews = hostState.hostViewInstances.filter(
    (view) => view.extensionId !== extensionId,
  );
  const remainingColumns = hostState.rightWorkspaceColumns
    .map((column) => ({
      ...column,
      panels: column.panels.filter((panel) => panel.extensionView?.extensionId !== extensionId),
    }))
    .filter((column) => column.panels.length > 0);

  setHostState('hostViewInstances', remainingHostViews);
  setHostState('activeHostViewByZone', (activeByPlacement) => {
    const next = { ...activeByPlacement };
    for (const [placement, activeId] of Object.entries(activeByPlacement)) {
      if (!activeId || !removedHostViewIds.has(activeId)) continue;
      const fallback = remainingHostViews.find((view) => hostViewZone(view) === placement);
      if (fallback) {
        next[placement] = fallback.id;
      } else {
        delete next[placement];
      }
    }
    return next;
  });
  setHostState('activeHostViewByPlacement', (activeByPlacement) => {
    const next = { ...activeByPlacement };
    for (const [placement, activeId] of Object.entries(activeByPlacement)) {
      if (!activeId || !removedHostViewIds.has(activeId)) continue;
      const fallback = remainingHostViews.find((view) => hostViewZone(view) === placement);
      if (fallback) {
        next[placement] = fallback.id;
      } else {
        delete next[placement];
      }
    }
    return next;
  });
  setHostState('rightWorkspaceColumns', remainingColumns);

  const activePanelId = hostState.activeRightWorkspacePanelId;
  const activePanelStillExists = activePanelId !== null && remainingColumns.some((column) =>
    column.panels.some((panel) => panel.id === activePanelId),
  );
  if (activePanelId !== null && !activePanelStillExists) {
    setHostState('activeRightWorkspacePanelId', remainingColumns[0]?.panels[0]?.id ?? null);
  }
  if (remainingColumns.length === 0) {
    setHostState('rightWorkspaceVisible', false);
    setHostState('activeRightWorkspacePanelId', null);
  }
}

export function openRightWorkspacePanel(panel: RightWorkspacePanel): void {
  setHostState('rightWorkspaceVisible', true);
  setHostState('activeRightWorkspacePanelId', panel.id);
  setHostState('rightWorkspaceColumns', (columns) => {
    const existing = columns.some((column) =>
      column.panels.some((item) => item.id === panel.id),
    );
    if (existing) {
      return columns.map((column) => ({
        ...column,
        panels: column.panels.map((item) =>
          item.id === panel.id
            ? { ...item, ...panel, config: panel.config, sessionId: panel.sessionId }
            : item,
        ),
      }));
    }

    if (columns.length === 0) {
      return [{ id: 'col-1', width: hostState.rightWorkspaceWidth, panels: [panel] }];
    }

    const lastColumn = columns[columns.length - 1];
    if (lastColumn.panels.length === 1) {
      return columns.map((column, index) =>
        index === columns.length - 1
          ? { ...column, panels: [...column.panels, panel] }
          : column,
      );
    }

    return [
      ...columns,
      {
        id: `col-${columns.length + 1}`,
        width: hostState.rightWorkspaceWidth,
        panels: [panel],
      },
    ];
  });
}

export function closeRightWorkspacePanel(panelId: string): void {
  const closing = hostState.rightWorkspaceColumns
    .flatMap((column) => column.panels)
    .find((panel) => panel.id === panelId);
  if (!closing || !isRightWorkspacePanelClosable(closing)) return;

  setHostState('rightWorkspaceColumns', (columns) => {
    const next = columns
      .map((column) => ({
        ...column,
        panels: column.panels.filter((panel) => panel.id !== panelId),
      }))
      .filter((column) => column.panels.length > 0);

    if (next.length === 0) {
      queueMicrotask(() => {
        setHostState('rightWorkspaceVisible', false);
        setHostState('activeRightWorkspacePanelId', null);
      });
    } else if (hostState.activeRightWorkspacePanelId === panelId) {
      const nextPanelId = next[0]?.panels[0]?.id ?? null;
      queueMicrotask(() => setHostState('activeRightWorkspacePanelId', nextPanelId));
    }

    return next;
  });
}

export function focusRightWorkspacePanel(panelId: string): void {
  const exists = hostState.rightWorkspaceColumns.some((column) =>
    column.panels.some((panel) => panel.id === panelId),
  );
  if (exists) {
    setHostState('activeRightWorkspacePanelId', panelId);
  }
}
