/**
 * ============================================================
 * Navis AppState - 顶层全局 Store - stores/app.ts
 * ============================================================
 *
 * 管理跨模块共享的全局状态。所有模块从此处读取共享状态，
 * 避免直接跨 store 引用。
 *
 * 四层架构：
 *   第 1 层：AppState（顶层全局 Store）— 本文件
 *   第 2 层：模块 Store（agent/session/project）— 独立业务状态
 *   第 3 层：IPC 事件同步层 — useEvent/useStream 钩子自动同步
 *   第 4 层：持久化层 — Config 模块持久化偏好设置
 *
 * 跨 Store 同步规则：
 * ✅ 正确：模块 Store 通过 AppState 间接读取共享状态
 *   const { activeSessionId } = appState;
 *   const agent = useAgentStore(activeSessionId);
 *
 * ❌ 错误：模块 Store 直接引用其他模块的 Store（禁止！）
 *   const agent = useAgentStore(sessionStore.activeId);
 *
 * ============================================================
 */

import { createStore } from 'solid-js/store';
import { createEffect } from 'solid-js';
import type { UiExtensionView } from '../lib/extension-ui';
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
 * 顶层全局应用状态接口。
 *
 * 这是跨模块共享状态的唯一真实来源。
 * 模块 Store（agent、session、project）应通过 AppState 引用
 * activeSessionId、activeProjectId 等共享字段，而非各自维护副本。
 */
export interface AppState {
  /** 窗口状态 */
  windowState: WindowState;
  /** 当前激活的会话 ID（唯一真实来源） */
  activeSessionId: string | null;
  /** 当前激活的项目 ID */
  activeProjectId: string | null;
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
  /** 当前中心区域视图模式（chat / editor / terminal / 扩展自定义） */
  activeView: string;
  /** 当前开始页意图：普通会话或协作任务 */
  pendingStartKind: 'session' | 'task' | null;
  /** 开始页阶段暂存的 Provider 选择 */
  pendingStartProviderId: string | null;
  /** 开始页阶段暂存的模型选择 */
  pendingStartModelId: string | null;
  /** 开始页阶段暂存的权限策略 */
  pendingStartPermissionPolicy: string | null;
  /** 开始页阶段暂存的推理强度 */
  pendingStartReasoningEffort: 'low' | 'medium' | 'high' | 'extra-high' | 'max' | null;
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
const defaultAppState: AppState = {
  windowState: defaultWindowState,
  activeSessionId: null,
  activeProjectId: null,
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
  activeView: 'chat',
  pendingStartKind: null,
  pendingStartProviderId: null,
  pendingStartModelId: null,
  pendingStartPermissionPolicy: null,
  pendingStartReasoningEffort: null,
};

// ── Store 实例 ────────────────────────────────────────────

/**
 * 顶层全局应用状态 Store。
 * 使用 Solid.js createStore 实现嵌套响应式更新。
 *
 * 所有跨模块共享状态存放于此。模块 Store 应从此处
 * 读取 activeSessionId / activeProjectId，而非各自维护副本。
 *
 * @example
 * ```tsx
 * import { appState, setAppState } from '@/stores/app';
 *
 * // 读取
 * console.log(appState.activeSessionId);
 *
 * // 写入
 * setAppState('activeSessionId', 'session-id');
 * setAppState('sidebarVisible', (v) => !v);
 * ```
 */
export const [appState, setAppState] = createStore<AppState>({ ...defaultAppState });

// ── 持久化 ───────────────────────────────────────────────

/** localStorage 键前缀 */
const STORAGE_KEY = 'navis-app-state';

/**
 * 将持久化状态保存到 localStorage。
 * 仅保存用户偏好字段，不保存瞬态状态（如 globalLoading）。
 */
function persistState(): void {
  if (typeof localStorage === 'undefined') return;

  const persistable: Partial<AppState> = {
    sidebarVisible: appState.sidebarVisible,
    sidebarWidth: appState.sidebarWidth,
    rightWorkspaceWidth: appState.rightWorkspaceWidth,
    activeView: appState.activeView,
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
 * restoreAppState();
 * ```
 */
export function restoreAppState(): void {
  if (typeof localStorage === 'undefined') return;

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;

    const saved = JSON.parse(raw) as Partial<AppState>;

    // 逐字段恢复，确保类型安全
    if (typeof saved.sidebarVisible === 'boolean') {
      setAppState('sidebarVisible', saved.sidebarVisible);
    }
    if (typeof saved.sidebarWidth === 'number') {
      setAppState('sidebarWidth', Math.min(Math.max(saved.sidebarWidth, 240), 420));
    }
    if (typeof saved.rightWorkspaceWidth === 'number') {
      setAppState(
        'rightWorkspaceWidth',
        Math.min(Math.max(saved.rightWorkspaceWidth, RIGHT_WORKSPACE_MIN_WIDTH), RIGHT_WORKSPACE_MAX_WIDTH),
      );
    }
    setAppState('rightWorkspaceVisible', false);
    setAppState('rightWorkspaceColumns', []);
    setAppState('activeRightWorkspacePanelId', null);
    if (typeof saved.activeView === 'string') {
      setAppState('activeView', saved.activeView);
    }
  } catch {
    // JSON 解析失败，忽略损坏的数据
  }
}

export function installAppStatePersistence(): void {
  createEffect(() => {
    // 读取所有持久化字段，建立响应式依赖追踪
    void appState.sidebarVisible;
    void appState.sidebarWidth;
    void appState.rightWorkspaceWidth;
    void appState.activeView;

    // 延迟持久化，避免频繁写入
    queueMicrotask(persistState);
  });
}

// ── Action 方法 ────────────────────────────────────────────

/**
 * 设置当前激活的会话。
 * 这是 activeSessionId 的唯一真实来源。
 * 模块 Store 应从 AppState 读取此值。
 *
 * @param id - 要激活的会话 ID，null 表示无激活会话
 */
export function setActiveSession(id: string | null): void {
  setAppState('activeSessionId', id);
}

/**
 * 设置当前激活的项目。
 *
 * @param id - 要激活的项目 ID，null 表示无激活项目
 */
export function setActiveProject(id: string | null): void {
  setAppState('activeProjectId', id);
}

/**
 * 设置离线状态。
 *
 * @param isOffline - 应用是否离线
 */
export function setOffline(isOffline: boolean): void {
  setAppState('isOffline', isOffline);
}

/**
 * 设置全局错误（用于错误边界捕获）。
 *
 * @param error - 捕获的错误，null 表示清除错误
 */
export function setError(error: Error | null): void {
  setAppState('globalError', error);
}

/**
 * 设置全局加载状态。
 *
 * @param loading - 是否正在加载
 */
export function setLoading(loading: boolean): void {
  setAppState('globalLoading', loading);
}

/**
 * 切换侧边栏可见性。
 */
export function toggleSidebar(): void {
  setAppState('sidebarVisible', (v) => !v);
}

export function setSidebarWidth(width: number): void {
  setAppState('sidebarWidth', Math.min(Math.max(width, 240), 420));
}

/**
 * 切换右侧动态面板区可见性。
 */
export function toggleRightWorkspace(): void {
  setAppState('rightWorkspaceVisible', (v) => !v);
}

export function setRightWorkspaceWidth(width: number): void {
  const nextWidth = Math.min(Math.max(width, RIGHT_WORKSPACE_MIN_WIDTH), RIGHT_WORKSPACE_MAX_WIDTH);
  setAppState('rightWorkspaceWidth', nextWidth);
  setAppState('rightWorkspaceColumns', (columns) =>
    columns.map((column) => ({
      ...column,
      width: nextWidth,
    })),
  );
}

/**
 * 初始化 AppState，从 Config 恢复持久化状态。
 * 应在应用启动阶段调用一次。
 *
 * 当前委托给 restoreAppState() 从 localStorage 读取。
 * 当 Config 模块可用后，应改为从 Config 持久化层读取。
 */
export function init(): void {
  restoreAppState();
}

/**
 * 设置当前激活的视图。
 *
 * @param view - 视图标识（'chat' | 'editor' | 'terminal' 或扩展注册的 viewId）
 */
export function setActiveView(view: string): void {
  setAppState('activeView', view);
}

export function setPendingStartKind(kind: AppState['pendingStartKind']): void {
  setAppState('pendingStartKind', kind);
}

export function setPendingStartModelSelection(providerId: string | null, modelId: string | null): void {
  setAppState('pendingStartProviderId', providerId);
  setAppState('pendingStartModelId', modelId);
}

export function setPendingStartPermissionPolicy(permissionPolicy: string | null): void {
  setAppState('pendingStartPermissionPolicy', permissionPolicy);
}

export function setPendingStartReasoningEffort(
  reasoningEffort: AppState['pendingStartReasoningEffort'],
): void {
  setAppState('pendingStartReasoningEffort', reasoningEffort);
}

export function resetPendingStartSessionDefaults(): void {
  setAppState({
    pendingStartProviderId: null,
    pendingStartModelId: null,
    pendingStartPermissionPolicy: null,
    pendingStartReasoningEffort: null,
  });
}

export function hostViewInstanceId(extensionId: string, viewId: string): string {
  return `${extensionId}:${viewId}`;
}

function hostViewZone(view: Pick<UiExtensionView, 'zone' | 'placement'>): string {
  return view.zone || view.placement;
}

export function hostViewsForZone(zone: string): HostViewInstance[] {
  return appState.hostViewInstances.filter((view) => hostViewZone(view) === zone);
}

export function activeHostViewForZone(zone: string): HostViewInstance | undefined {
  const activeId = appState.activeHostViewByZone[zone] ?? appState.activeHostViewByPlacement[zone];
  return appState.hostViewInstances.find((view) => view.id === activeId && hostViewZone(view) === zone)
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
  return appState.hostViewInstances.some((view) => view.id === id);
}

export function isRightWorkspacePanelOpen(extensionId: string, viewId: string): boolean {
  const id = hostViewInstanceId(extensionId, viewId);
  return appState.rightWorkspaceColumns.some((column) =>
    column.panels.some((panel) => panel.id === id),
  );
}

export function openHostView(instance: HostViewInstance): void {
  setAppState('hostViewInstances', (instances) => {
    const exists = instances.some((view) => view.id === instance.id);
    return exists
      ? instances.map((view) => (view.id === instance.id ? { ...view, ...instance } : view))
      : [...instances, instance];
  });
  const zone = hostViewZone(instance);
  setAppState('activeHostViewByZone', zone, instance.id);
  setAppState('activeHostViewByPlacement', zone, instance.id);
}

export function closeHostView(instanceId: string): void {
  const closing = appState.hostViewInstances.find((view) => view.id === instanceId);
  if (!closing || !isHostViewClosable(closing)) return;

  setAppState('hostViewInstances', (instances) => instances.filter((view) => view.id !== instanceId));

  const next = appState.hostViewInstances.find(
    (view) => view.id !== instanceId && hostViewZone(view) === hostViewZone(closing),
  );
  const zone = hostViewZone(closing);
  setAppState('activeHostViewByZone', zone, next?.id);
  setAppState('activeHostViewByPlacement', zone, next?.id);
}

export function focusHostView(instanceId: string): void {
  const instance = appState.hostViewInstances.find((view) => view.id === instanceId);
  if (!instance) return;
  const zone = hostViewZone(instance);
  setAppState('activeHostViewByZone', zone, instance.id);
  setAppState('activeHostViewByPlacement', zone, instance.id);
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
    appState.hostViewInstances
      .filter((view) => view.extensionId === extensionId)
      .map((view) => view.id),
  );
  const remainingHostViews = appState.hostViewInstances.filter(
    (view) => view.extensionId !== extensionId,
  );
  const remainingColumns = appState.rightWorkspaceColumns
    .map((column) => ({
      ...column,
      panels: column.panels.filter((panel) => panel.extensionView?.extensionId !== extensionId),
    }))
    .filter((column) => column.panels.length > 0);

  setAppState('hostViewInstances', remainingHostViews);
  setAppState('activeHostViewByZone', (activeByPlacement) => {
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
  setAppState('activeHostViewByPlacement', (activeByPlacement) => {
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
  setAppState('rightWorkspaceColumns', remainingColumns);

  const activePanelId = appState.activeRightWorkspacePanelId;
  const activePanelStillExists = activePanelId !== null && remainingColumns.some((column) =>
    column.panels.some((panel) => panel.id === activePanelId),
  );
  if (activePanelId !== null && !activePanelStillExists) {
    setAppState('activeRightWorkspacePanelId', remainingColumns[0]?.panels[0]?.id ?? null);
  }
  if (remainingColumns.length === 0) {
    setAppState('rightWorkspaceVisible', false);
    setAppState('activeRightWorkspacePanelId', null);
  }
}

export function openRightWorkspacePanel(panel: RightWorkspacePanel): void {
  setAppState('rightWorkspaceVisible', true);
  setAppState('activeRightWorkspacePanelId', panel.id);
  setAppState('rightWorkspaceColumns', (columns) => {
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
      return [{ id: 'col-1', width: appState.rightWorkspaceWidth, panels: [panel] }];
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
        width: appState.rightWorkspaceWidth,
        panels: [panel],
      },
    ];
  });
}

export function closeRightWorkspacePanel(panelId: string): void {
  const closing = appState.rightWorkspaceColumns
    .flatMap((column) => column.panels)
    .find((panel) => panel.id === panelId);
  if (!closing || !isRightWorkspacePanelClosable(closing)) return;

  setAppState('rightWorkspaceColumns', (columns) => {
    const next = columns
      .map((column) => ({
        ...column,
        panels: column.panels.filter((panel) => panel.id !== panelId),
      }))
      .filter((column) => column.panels.length > 0);

    if (next.length === 0) {
      queueMicrotask(() => {
        setAppState('rightWorkspaceVisible', false);
        setAppState('activeRightWorkspacePanelId', null);
      });
    } else if (appState.activeRightWorkspacePanelId === panelId) {
      const nextPanelId = next[0]?.panels[0]?.id ?? null;
      queueMicrotask(() => setAppState('activeRightWorkspacePanelId', nextPanelId));
    }

    return next;
  });
}

export function focusRightWorkspacePanel(panelId: string): void {
  const exists = appState.rightWorkspaceColumns.some((column) =>
    column.panels.some((panel) => panel.id === panelId),
  );
  if (exists) {
    setAppState('activeRightWorkspacePanelId', panelId);
  }
}
