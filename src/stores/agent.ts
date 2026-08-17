/**
 * ============================================================
 * Navis Agent 状态 - stores/agent.ts
 * ============================================================
 *
 * 管理 AI Agent 的工作模式状态。
 *
 * 使用 Solid.js 的 createStore 实现响应式状态。
 *
 * 来源：design/22-ui-framework.md 第六章 全局状态
 * ============================================================
 */

import { createStore } from 'solid-js/store';

// ── 类型定义 ────────────────────────────────────────────

export type WorkMode =
  | { type: 'code' }
  | { type: 'cowork' }
  | { type: 'custom'; modeId: string; extensionId: string; runtimeId: string };

/**
 * Agent 状态接口。
 * 当前只保留跨视图共享的工作模式。
 * 运行状态改由真实聊天流状态推导，避免前端再维护一套伪任务生命周期。
 */
export interface AgentState {
  /** 当前会话工作模式 */
  workMode: WorkMode;
}

// ── 默认值 ──────────────────────────────────────────────

/** Agent 状态默认值 */
const defaultAgentState: AgentState = {
  workMode: { type: 'code' },
};

// ── Store 实例 ──────────────────────────────────────────

/**
 * Agent 状态 store。
 *
 * @example
 * ```tsx
 * import { agentState, setWorkMode } from '@/stores/agent';
 *
 * // 读取当前工作模式
 * console.log(agentState.workMode.type);
 *
 * // 更新工作模式
 * setWorkMode({ type: 'cowork' });
 * ```
 */
export const [agentState, setAgentState] = createStore<AgentState>({
  ...defaultAgentState,
});

// ── 便捷操作函数 ────────────────────────────────────────

export function setWorkMode(workMode: WorkMode): void {
  setAgentState('workMode', workMode);
}
