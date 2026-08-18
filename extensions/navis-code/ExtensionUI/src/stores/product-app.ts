import { createStore } from 'solid-js/store';
import { registerHostViewContextProvider } from '@/stores/host';
import { registerMenuWhenContextProvider } from '@/lib/menu-when';

/** Navis Code 产品级共享状态。 */
export interface NavisCodeProductState {
  /** 当前激活的会话标识。 */
  activeSessionId: string | null;
  /** 当前激活的项目标识。 */
  activeProjectId: string | null;
  /** 开始工作区的业务意图。 */
  pendingStartKind: 'session' | 'task' | null;
  /** 暂存的模型提供方标识。 */
  pendingStartProviderId: string | null;
  /** 暂存的模型标识。 */
  pendingStartModelId: string | null;
  /** 暂存的执行权限策略。 */
  pendingStartPermissionPolicy: string | null;
  /** 暂存的推理强度。 */
  pendingStartReasoningEffort: 'low' | 'medium' | 'high' | 'extra-high' | 'max' | null;
}

const defaultProductAppState: NavisCodeProductState = {
  activeSessionId: null,
  activeProjectId: null,
  pendingStartKind: null,
  pendingStartProviderId: null,
  pendingStartModelId: null,
  pendingStartPermissionPolicy: null,
  pendingStartReasoningEffort: null,
};

/** Navis Code 产品运行时的唯一共享状态源。 */
export const [navisCodeProductState, setNavisCodeProductState] =
  createStore<NavisCodeProductState>({ ...defaultProductAppState });

/** 设置当前会话。 */
export function setActiveSession(id: string | null): void {
  setNavisCodeProductState('activeSessionId', id);
}

/** 设置当前项目。 */
export function setActiveProject(id: string | null): void {
  setNavisCodeProductState('activeProjectId', id);
}

/** 设置开始工作区的业务意图。 */
export function setPendingStartKind(kind: NavisCodeProductState['pendingStartKind']): void {
  setNavisCodeProductState('pendingStartKind', kind);
}

/** 设置开始工作区的模型选择。 */
export function setPendingStartModelSelection(providerId: string | null, modelId: string | null): void {
  setNavisCodeProductState('pendingStartProviderId', providerId);
  setNavisCodeProductState('pendingStartModelId', modelId);
}

/** 设置开始工作区的权限策略。 */
export function setPendingStartPermissionPolicy(permissionPolicy: string | null): void {
  setNavisCodeProductState('pendingStartPermissionPolicy', permissionPolicy);
}

/** 设置开始工作区的推理强度。 */
export function setPendingStartReasoningEffort(
  reasoningEffort: NavisCodeProductState['pendingStartReasoningEffort'],
): void {
  setNavisCodeProductState('pendingStartReasoningEffort', reasoningEffort);
}

/** 清除一次会话启动后不应继续保留的产品暂存配置。 */
export function resetPendingStartSessionDefaults(): void {
  setNavisCodeProductState({
    pendingStartProviderId: null,
    pendingStartModelId: null,
    pendingStartPermissionPolicy: null,
    pendingStartReasoningEffort: null,
  });
}

/** 产品决定其领域上下文，宿主只透传该上下文给扩展沙箱。 */
registerHostViewContextProvider(() => ({
  session: { sessionId: navisCodeProductState.activeSessionId },
  activeProject: { projectId: navisCodeProductState.activeProjectId },
}));


/** 为 Navis Code 菜单表达式提供产品领域上下文，宿主只负责求值。 */
registerMenuWhenContextProvider(() => ({
  activeSession: Boolean(navisCodeProductState.activeSessionId),
  activeProject: Boolean(navisCodeProductState.activeProjectId),
  activeSessionId: navisCodeProductState.activeSessionId,
  activeProjectId: navisCodeProductState.activeProjectId,
}));
