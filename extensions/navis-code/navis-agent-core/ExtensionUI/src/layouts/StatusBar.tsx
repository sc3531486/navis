/**
 * ============================================================
 * Navis Go 状态栏 - layouts/StatusBar.tsx
 * ============================================================
 *
 * 设计稿 statusbar：
 *   高度 22px，浅灰背景，只放应用级状态
 *   左侧：Navis Go 运行状态
 *   右侧：保留给应用级状态信息
 *
 * ============================================================
 */

import { Component, For, Show } from 'solid-js';
import { agentRuntimeStatus } from '@agent-core/stores/agent-runtime';
import { hostState } from '@/stores/host';
import { executeExtensionPoint, extensionPointsByKind } from '@/stores/extension-points';

// ── 状态映射 ────────────────────────────────────────────

/** Agent 状态对应的显示文本 */
const AGENT_STATUS_MAP: Record<string, string> = {
  idle: '就绪',
  thinking: '思考中',
  tool_calling: '调用工具',
  waiting_permission: '等待授权',
  streaming: '输出中',
  recovering: '恢复中',
  error: '错误',
};

// ── 状态栏组件 ──────────────────────────────────────────

/**
 * 底部状态栏 — 应用级状态，保持低视觉权重。
 */
const StatusBar: Component = () => {
  const agentStatus = () => agentRuntimeStatus();
  const statusLabel = () => AGENT_STATUS_MAP[agentStatus()] ?? '未知';
  const isOffline = () => hostState.isOffline;

  return (
    <footer
      class="navis-statusbar flex items-center justify-between h-[22px] text-[11px]
             border-t border-[#dadada] bg-[#f7f7f7] text-[#555555]
             select-none flex-shrink-0"
      role="status"
      aria-label="状态栏"
    >
      {/* ── 左侧：Navis Go 运行状态 ── */}
      <div class="navis-statusbar-left flex items-center">
        <div class="navis-statusbar-agent flex items-center" title={`Navis Go 状态: ${statusLabel()}`}>
          <span
            class={`inline-block w-2 h-2 rounded-full ${
              agentStatus() === 'idle' ? 'navis-status-dot-ready' : 'bg-[#777777]'
            }`}
          />
          <span>Navis Go {statusLabel()}</span>
        </div>
        <Show when={isOffline()}>
          <span class="text-[#8a6d00]" title="当前处于离线状态">离线</span>
        </Show>
      </div>

      <div class="navis-statusbar-right flex items-center">
        <For each={extensionPointsByKind('statusbar')}>
          {(point) => (
            <button
              type="button"
              class="navis-statusbar-extension rounded px-1 text-[#555555] hover:bg-[#ececec] disabled:opacity-50"
              title={point.label ?? point.id}
              disabled={!point.command}
              onClick={() => executeExtensionPoint(point)}
            >
              {point.label ?? point.id}
            </button>
          )}
        </For>
      </div>
    </footer>
  );
};

export default StatusBar;


