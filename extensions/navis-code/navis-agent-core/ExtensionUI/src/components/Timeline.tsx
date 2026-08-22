import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import { AgentService, type AgentPromptPayload } from '../services/AgentService';
import {
  IconClose,
  IconCheck,
  IconZap,
  IconLightbulb,
  IconWrench,
  IconChevronRight,
} from '@/components/icons';

export interface ToolCallItem {
  id: string;
  toolName: string;
  argsSummary: string;
  outputSummary?: string;
  status: 'pending' | 'approved' | 'rejected' | 'completed';
  needsApproval?: boolean;
}

export interface MessageTurn {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thumbnail?: string;
  thinking?: string;
  toolCalls?: ToolCallItem[];
  tokensUsage?: { prompt: number; completion: number; total: number; cost: string };
  timestamp: number;
  isLoading?: boolean;
}

export const Timeline: Component<{ ctx: NavisContext }> = (props) => {
  const [messages, setMessages] = createSignal<MessageTurn[]>([
    {
      id: 'user-init-1',
      role: 'user',
      content: '你可以看看D:\\myworkspace\\opencode-dev这个是怎么交互的。我现在发送请求没反应。',
      thumbnail: 'media_preview.png',
      timestamp: Date.now() - 60000,
    },
    {
      id: 'assistant-init-1',
      role: 'assistant',
      content: `我对 \`D:\\myworkspace\\opencode-dev\` 的全套交互链路进行了深入源码剖析：

<div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 12px 16px; margin: 10px 0; font-family: monospace; font-size: 12px; color: #334155; display: flex; align-items: center; justify-content: center; gap: 8px;">
  <span>4. Tool Execution (view_file / run_command)</span>
  <span style="color: #64748b;">───────►</span>
  <span style="background: #ffffff; border: 1px solid #cbd5e1; padding: 4px 8px; border-radius: 4px;">Local Workspace / Shell</span>
</div>

<h4 style="font-size: 13.5px; font-weight: 700; color: #0f172a; margin: 14px 0 6px 0;">1. 后端引擎服务 (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/opencode</code>):</h4>

<ul style="margin: 0 0 12px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li>启动命令为 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">opencode serve</code> (如监听在 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">http://127.0.0.1:8046</code> 或动态端口)；</li>
  <li>暴露 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/v1/models</code>、<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/session</code>、<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/session/{id}/prompt</code> 等标准 REST 接口与 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/event</code> SSE 流式事件通道；</li>
  <li>负责管理 Agent 上下文记忆、System Prompt、工具调用与沙箱权限校验。</li>
</ul>

<h4 style="font-size: 13.5px; font-weight: 700; color: #0f172a; margin: 14px 0 6px 0;">2. 前端客户端 (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/app</code> / <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/desktop</code>):</h4>

<ul style="margin: 0 0 14px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li>通过 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">ServerSDK</code> (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/app/src/context/server-sdk.tsx</code>) 与 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">ServerSync</code> 建立 HTTP + SSE 长连接；</li>
  <li>用户在输入框按 Enter 后，触发 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">sdk.session.prompt</code>，并通过 SSE 监听 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">message.part.delta</code> (逐字流式打字效果) 和 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">tool.execute</code> (工具调用状态)。</li>
</ul>

<h3 style="font-size: 14px; font-weight: 700; color: #0f172a; margin: 18px 0 8px 0; border-top: 1px solid #f1f5f9; padding-top: 14px;">三、Git 提交与远端状态</h3>

<ul style="margin: 0 0 14px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li><b>Commit ID</b>: <code style="color: #dc2626; background: #fef2f2; padding: 1px 5px; border-radius: 4px;">56cf050</code></li>
  <li><b>Commit Message</b>: <code style="color: #dc2626; background: #fef2f2; padding: 1px 5px; border-radius: 4px;">feat(ui): replicate exact Antigravity IDE workspace layout with right context drawer and floating composer</code></li>
  <li><b>远程分支</b>: <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">origin/main</code> (已完成推送)</li>
</ul>`,
      timestamp: Date.now() - 30000,
      tokensUsage: { prompt: 2450, completion: 560, total: 3010, cost: '$0.0058' },
    },
  ]);

  const agentService = new AgentService(props.ctx);

  const handleApproveTool = (msgId: string, toolId: string) => {
    setMessages((prev) =>
      prev.map((msg) => {
        if (msg.id === msgId && msg.toolCalls) {
          const toolCalls = msg.toolCalls.map((tc) =>
            tc.id === toolId ? { ...tc, status: 'completed' as const, needsApproval: false } : tc,
          );
          return { ...msg, toolCalls };
        }
        return msg;
      }),
    );
    toast.success('已批准工具调用并成功执行！');
  };

  const handleRejectTool = (msgId: string, toolId: string) => {
    setMessages((prev) =>
      prev.map((msg) => {
        if (msg.id === msgId && msg.toolCalls) {
          const toolCalls = msg.toolCalls.map((tc) =>
            tc.id === toolId ? { ...tc, status: 'rejected' as const, needsApproval: false } : tc,
          );
          return { ...msg, toolCalls };
        }
        return msg;
      }),
    );
    toast.warning('已拒绝该操作');
  };

  onMount(() => {
    // 监听发送消息事件
    const unsubTurn = props.ctx.events.on('agent:turn:start', async (payload: AgentPromptPayload) => {
      const userMsg: MessageTurn = {
        id: `u-${Date.now()}`,
        role: 'user',
        content: payload.content,
        timestamp: Date.now(),
      };

      const pendingAiMsgId = `a-${Date.now()}`;
      const pendingAiMsg: MessageTurn = {
        id: pendingAiMsgId,
        role: 'assistant',
        thinking: `正在连接 ${gatewayStore.activeProvider()?.name || 'Agent'} 并分析上下文...`,
        content: 'Agent 正在思考并执行任务中...',
        timestamp: Date.now(),
        isLoading: true,
      };

      setMessages((prev) => [...prev, userMsg, pendingAiMsg]);

      try {
        const result = await agentService.executeTurn(payload);
        setMessages((prev) =>
          prev.map((m) =>
            m.id === pendingAiMsgId
              ? {
                  ...m,
                  thinking: result.thinking,
                  content: result.content,
                  toolCalls: result.toolCalls,
                  tokensUsage: result.tokensUsage,
                  isLoading: false,
                }
              : m,
          ),
        );
      } catch (err: any) {
        setMessages((prev) =>
          prev.map((m) =>
            m.id === pendingAiMsgId
              ? {
                  ...m,
                  thinking: '执行发生异常',
                  content: `抱歉，执行失败：${err?.message || '请求超时'}。`,
                  isLoading: false,
                }
              : m,
          ),
        );
      }
    });

    const unsubSession = props.ctx.events.on('session:created', () => {
      setMessages([]);
    });

    onCleanup(() => {
      unsubTurn();
      unsubSession();
    });
  });

  return (
    <div style="width: 100%; max-width: 780px; display: flex; flex-direction: column; gap: 24px;">
      <For each={messages()}>
        {(msg) => (
          <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
            {/* 用户消息 */}
            <Show when={msg.role === 'user'}>
              <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 6px;">
                {/* 缩略图预览卡片 */}
                <Show when={msg.thumbnail}>
                  <div style="display: flex; align-items: center; gap: 6px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; padding: 4px 8px; font-size: 12px; color: #475569;">
                    <span style="font-size: 12px;">🖼️</span>
                    <span>{msg.thumbnail}</span>
                  </div>
                </Show>
                <div style="font-size: 13.5px; font-weight: 500; color: #1e293b; line-height: 1.5;">
                  {msg.content}
                </div>
              </div>
            </Show>

            {/* 助手消息 */}
            <Show when={msg.role === 'assistant'}>
              <div style="display: flex; flex-direction: column; gap: 12px; width: 100%;">
                {/* 思考过程 */}
                <Show when={msg.thinking}>
                  <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #64748b; display: flex; align-items: center; gap: 8px;">
                    <span style="color: #ea580c; display: flex; align-items: center;">
                      <IconLightbulb size={14} />
                    </span>
                    <span style="font-weight: 600; color: #334155;">Thinking 思考过程</span>
                    <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-style: italic;">
                      {msg.thinking}
                    </span>
                  </div>
                </Show>

                {/* 工具调用卡片 */}
                <Show when={msg.toolCalls && msg.toolCalls.length > 0}>
                  <div style="display: flex; flex-direction: column; gap: 8px;">
                    <For each={msg.toolCalls}>
                      {(tool) => (
                        <div style="background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; box-shadow: 0 1px 3px rgba(0,0,0,0.02);">
                          <div style="display: flex; align-items: center; justify-content: space-between;">
                            <div style="display: flex; align-items: center; gap: 6px; font-family: monospace; font-size: 12px; color: #1e1d1b;">
                              <IconWrench size={13} color="#ea580c" />
                              <b>{tool.toolName}</b>
                              <span style="color: #64748b;">{tool.argsSummary}</span>
                            </div>

                            <div style="display: flex; align-items: center; gap: 6px;">
                              <Show when={tool.status === 'completed'}>
                                <span style="font-size: 11px; background: #dcfce7; color: #15803d; padding: 2px 6px; border-radius: 4px; font-weight: 500;">
                                  执行成功
                                </span>
                              </Show>
                              <Show when={tool.status === 'rejected'}>
                                <span style="font-size: 11px; background: #fee2e2; color: #b91c1c; padding: 2px 6px; border-radius: 4px; font-weight: 500;">
                                  已拒绝
                                </span>
                              </Show>
                              <Show when={tool.needsApproval && tool.status === 'pending'}>
                                <div style="display: flex; gap: 6px;">
                                  <button
                                    onClick={() => handleApproveTool(msg.id, tool.id)}
                                    style="background: #16a34a; color: #ffffff; border: none; padding: 3px 8px; border-radius: 4px; font-size: 11px; cursor: pointer; display: flex; align-items: center; gap: 3px;"
                                  >
                                    <IconCheck size={11} />
                                    <span>批准执行</span>
                                  </button>
                                  <button
                                    onClick={() => handleRejectTool(msg.id, tool.id)}
                                    style="background: #f8fafc; color: #64748b; border: 1px solid #e2e8f0; padding: 3px 8px; border-radius: 4px; font-size: 11px; cursor: pointer;"
                                  >
                                    拒绝
                                  </button>
                                </div>
                              </Show>
                            </div>
                          </div>

                          <Show when={tool.outputSummary}>
                            <div style="background: #f8fafc; border: 1px solid #f1f5f9; border-radius: 4px; padding: 6px 8px; font-family: monospace; font-size: 11.5px; color: #475569; white-space: pre-wrap;">
                              {tool.outputSummary}
                            </div>
                          </Show>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>

                {/* 格式化 Markdown 输出 */}
                <div
                  style="font-size: 13.5px; line-height: 1.6; color: #1e293b; word-break: break-word;"
                  innerHTML={msg.content}
                />

                {/* 交付件与改动卡片 (对标 Antigravity Artifacts / Walkthrough 卡片) */}
                <div style="display: flex; flex-direction: column; gap: 6px; margin-top: 4px;">
                  {/* Artifact 1: Verify Antigravity Layout */}
                  <div
                    onClick={() => toast.info('查看交付件: Verify Antigravity Layout')}
                    style="display: flex; align-items: center; gap: 8px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 12px; font-size: 12.5px; color: #334155; cursor: pointer; transition: all 0.15s ease;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = '#ffffff')}
                  >
                    <span style="color: #64748b;">📄</span>
                    <span style="font-weight: 500;">Verify Antigravity Layout</span>
                  </div>

                  {/* Artifact 2: Walkthrough */}
                  <div
                    onClick={() => toast.info('查看交付件: Walkthrough 自测报告')}
                    style="display: flex; flex-direction: column; gap: 4px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; padding: 10px 12px; cursor: pointer; transition: all 0.15s ease;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = '#ffffff')}
                  >
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 12.5px; font-weight: 600; color: #1e293b;">
                      <span>📖</span>
                      <span>Walkthrough</span>
                    </div>
                    <div style="font-size: 12px; color: #64748b;">
                      Navis Code 完整复刻 Antigravity IDE 现代化三栏工作区与执行流自测报告
                    </div>
                  </div>

                  {/* Artifact 3: Files Changed Diff bar */}
                  <div
                    style="display: flex; align-items: center; justify-content: space-between; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 12px; font-size: 12px;"
                  >
                    <div style="display: flex; align-items: center; gap: 6px; color: #334155;">
                      <span style="font-weight: 500;">4 files changed</span>
                      <span style="color: #16a34a; font-weight: 600;">+590</span>
                      <span style="color: #dc2626; font-weight: 600;">-668</span>
                      <span style="color: #94a3b8;">&gt;</span>
                    </div>
                    <button
                      onClick={() => toast.info('已开启差异代码评审')}
                      style="display: flex; align-items: center; gap: 4px; padding: 3px 8px; background: #f8fafc; border: 1px solid #cbd5e1; border-radius: 5px; font-size: 11.5px; color: #475569; cursor: pointer;"
                    >
                      <span>📝</span>
                      <span>评审</span>
                    </button>
                  </div>
                </div>

                {/* 底部时间戳与操作微标 */}
                <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #94a3b8; padding-top: 6px; border-top: 1px solid #f8fafc;">
                  <span>22:33</span>
                  <div style="display: flex; align-items: center; gap: 8px; cursor: pointer;">
                    <span title="复制" onClick={() => toast.success('已复制回答')}>📋</span>
                    <span title="有帮助" onClick={() => toast.success('感谢您的反馈')}>👍</span>
                    <span title="需改进" onClick={() => toast.info('已记录改进建议')}>👎</span>
                  </div>
                </div>
              </div>
            </Show>
          </div>
        )}
      </For>
    </div>
  );
};

export default Timeline;
