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
  const [sessionTitle, setSessionTitle] = createSignal('Repeated Chinese Greetings');
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

<h4 style="font-size: 14px; font-weight: 700; color: #0f172a; margin: 14px 0 6px 0;">1. 后端引擎服务 (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/opencode</code>):</h4>

<ul style="margin: 0 0 12px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li>启动命令为 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">opencode serve</code> (如监听在 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">http://127.0.0.1:8046</code> 或动态端口)；</li>
  <li>暴露 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/v1/models</code>、<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/session</code>、<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/session/{id}/prompt</code> 等标准 REST 接口与 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">/event</code> SSE 流式事件通道；</li>
  <li>负责管理 Agent 上下文记忆、System Prompt、工具调用与沙箱权限校验。</li>
</ul>

<h4 style="font-size: 14px; font-weight: 700; color: #0f172a; margin: 14px 0 6px 0;">2. 前端客户端 (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/app</code> / <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/desktop</code>):</h4>

<ul style="margin: 0 0 14px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li>通过 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">ServerSDK</code> (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">packages/app/src/context/server-sdk.tsx</code>) 与 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">ServerSync</code> 建立 HTTP + SSE 长连接；</li>
  <li>用户在输入框按 Enter 后，触发 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">sdk.session.prompt</code>，并通过 SSE 监听 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">message.part.delta</code> (逐字流式打字效果) 和 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">tool.execute</code> (工具调用状态)。</li>
</ul>

<h3 style="font-size: 15px; font-weight: 700; color: #0f172a; margin: 18px 0 8px 0; border-top: 1px solid #f1f5f9; padding-top: 14px;">二、您之前“发送请求没反应 / 出现告警”的原因</h3>

<ol style="margin: 0 0 12px 0; padding-left: 20px; line-height: 1.7; font-size: 13px; color: #334155;">
  <li><b>写死端口的告警横幅</b>: 先前时间线组件顶部硬编码了 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">Can't reach 127.0.0.1:15721</code>，未动态关联您在设置中心配置的真实网关地址 (<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">http://127.0.0.1:8046/v1</code> 等)；</li>
  <li><b>缺乏上游 HTTP 调度服务</b>: 先前的前端时间线仅做了静态的 <code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px;">setTimeout</code> 演示，没有挂载真实发起 HTTP 请求、解析流式响应及回退机制的 Agent 调度服务。</li>
</ol>`,
      timestamp: Date.now() - 30000,
      tokensUsage: { prompt: 2450, completion: 560, total: 3010, cost: '$0.0058' },
    },
  ]);

  let scrollContainer: HTMLDivElement | undefined;
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

      setTimeout(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      }, 50);

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

      setTimeout(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      }, 50);
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
    <div
      style="flex: 1; width: 100%; height: 100%; display: flex; flex-direction: column; background: #ffffff; overflow: hidden; min-height: 0;"
    >
      {/* 顶部面包屑与标题栏 (对标 Antigravity / Claude Code) */}
      <div
        style="height: 42px; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between; padding: 0 20px; user-select: none; background: #ffffff; flex-shrink: 0;"
      >
        <div style="display: flex; align-items: center; gap: 8px; font-size: 13px; color: #475569;">
          <span style="font-weight: 500; color: #1e293b;">Navis Go</span>
          <span style="color: #94a3b8;">/</span>
          <span style="color: #64748b;">{sessionTitle()}</span>
        </div>

        <div style="display: flex; align-items: center; gap: 8px;">
          <button
            onClick={() => toast.info('Navis IDE 集成套件已就绪')}
            style="display: flex; align-items: center; gap: 5px; padding: 4px 10px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; font-size: 12px; font-weight: 500; color: #334155; cursor: pointer;"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f1f5f9')}
            onMouseLeave={(e) => (e.currentTarget.style.background = '#f8fafc')}
          >
            <span>🚀</span>
            <span>安装 IDE</span>
          </button>
          <button
            onClick={() => props.ctx.events.emit('settings:open', { tab: 'models' })}
            style="background: transparent; border: none; color: #64748b; padding: 4px 6px; border-radius: 4px; cursor: pointer; font-size: 14px;"
            title="更多选项"
          >
            ⋮
          </button>
        </div>
      </div>

      {/* 消息滚动流区域 */}
      <div
        ref={scrollContainer}
        style="flex: 1; overflow-y: auto; padding: 24px 32px 140px; display: flex; flex-direction: column; align-items: center; min-height: 0; overscroll-behavior: contain;"
      >
        <div style="width: 100%; max-width: 780px; display: flex; flex-direction: column; gap: 24px;">
          <For each={messages()}>
            {(msg) => (
              <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
                {/* 用户消息 */}
                <Show when={msg.role === 'user'}>
                  <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 6px;">
                    {/* 缩略图预览卡片 */}
                    <Show when={msg.thumbnail}>
                      <div style="display: flex; align-items: center; gap: 6px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 6px 10px; font-size: 12px; color: #475569;">
                        <span style="font-size: 14px;">🖼️</span>
                        <span>{msg.thumbnail}</span>
                      </div>
                    </Show>
                    <div style="font-size: 14px; font-weight: 500; color: #1e293b; line-height: 1.5;">
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
                                <div style="display: flex; align-items: center; gap: 6px; font-family: monospace; font-size: 12px; color: #1e293b;">
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

                    {/* Token 用量 */}
                    <Show when={msg.tokensUsage}>
                      <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #94a3b8; padding-top: 4px; border-top: 1px dashed #f1f5f9;">
                        <span>
                          Tokens: {msg.tokensUsage?.prompt} in / {msg.tokensUsage?.completion} out (Total: {msg.tokensUsage?.total})
                        </span>
                        <span>估算费用: {msg.tokensUsage?.cost}</span>
                      </div>
                    </Show>
                  </div>
                </Show>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};

export default Timeline;
