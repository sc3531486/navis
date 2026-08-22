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
  error?: string;
}

export const Timeline: Component<{ ctx: NavisContext }> = (props) => {
  const [messages, setMessages] = createSignal<MessageTurn[]>([]);
  let scrollContainer: HTMLDivElement | undefined;
  const agentService = new AgentService(props.ctx);

  const scrollToBottom = () => {
    setTimeout(() => {
      const container = document.getElementById('timeline-scroll-container');
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    }, 20);
  };

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

  const formatMarkdown = (content: string) => {
    if (!content) return '';
    return content
      .replace(/\n/g, '<br/>')
      .replace(/\*\*(.*?)\*\*/g, '<b>$1</b>')
      .replace(/`([^`]+)`/g, '<code style="color: #ea580c; background: #fff7ed; padding: 1px 5px; border-radius: 4px; font-family: monospace; font-size: 12px;">$1</code>');
  };

  onMount(() => {
    // 监听发送消息事件并启动真实 SSE 流式交互
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
        thinking: '',
        content: '',
        timestamp: Date.now(),
        isLoading: true,
      };

      // 提取历史对话上下文
      const historyContext = messages().map((m) => ({
        role: m.role,
        content: m.content,
      }));

      setMessages((prev) => [...prev, userMsg, pendingAiMsg]);
      scrollToBottom();

      // 发起真实上游流式调用
      await agentService.streamTurn(payload, historyContext, {
        onThinkingDelta: (delta) => {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === pendingAiMsgId
                ? { ...m, thinking: (m.thinking || '') + delta }
                : m,
            ),
          );
          scrollToBottom();
        },
        onContentDelta: (delta) => {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === pendingAiMsgId
                ? { ...m, content: m.content + delta }
                : m,
            ),
          );
          scrollToBottom();
        },
        onToolCall: (toolCall) => {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === pendingAiMsgId
                ? { ...m, toolCalls: [...(m.toolCalls || []), toolCall] }
                : m,
            ),
          );
          scrollToBottom();
        },
        onComplete: (result) => {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === pendingAiMsgId
                ? {
                    ...m,
                    content: result.content || m.content || '(模型未返回文本内容)',
                    thinking: result.thinking || m.thinking,
                    tokensUsage: result.tokensUsage,
                    isLoading: false,
                  }
                : m,
            ),
          );
          scrollToBottom();
        },
        onError: (err) => {
          setMessages((prev) =>
            prev.map((m) =>
              m.id === pendingAiMsgId
                ? {
                    ...m,
                    isLoading: false,
                    error: err.message,
                  }
                : m,
            ),
          );
          scrollToBottom();
        },
      });
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
      {/* 初始空白欢迎占位 */}
      <Show when={messages().length === 0}>
        <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 60px 0 20px; color: #64748b; gap: 12px;">
          <div style="width: 48px; height: 48px; border-radius: 12px; background: #fff7ed; display: flex; align-items: center; justify-content: center; font-size: 24px; color: #ea580c;">
            ✦
          </div>
          <div style="font-size: 16px; font-weight: 600; color: #0f172a;">
            欢迎使用 Navis Code 真实模型工作区
          </div>
          <div style="font-size: 13px; color: #64748b; text-align: center; max-width: 420px; line-height: 1.6;">
            当前连接服务商：<b style="color: #0f172a;">{gatewayStore.activeProvider()?.name || 'Local Gateway'}</b> ({gatewayStore.activeModel()?.name || 'gemini-3.7-flash'})。
            <br />
            在下方输入问题或任务指令，即可体验实时流式思考与大模型代码生成。
          </div>
        </div>
      </Show>

      {/* 消息对话流 */}
      <For each={messages()}>
        {(msg) => (
          <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
            {/* 用户消息 */}
            <Show when={msg.role === 'user'}>
              <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 6px;">
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
                {/* 思考过程流 (Thinking 动态卡片) */}
                <Show when={msg.thinking}>
                  <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #64748b; display: flex; align-items: center; gap: 8px;">
                    <span style="color: #ea580c; display: flex; align-items: center;">
                      <IconLightbulb size={14} />
                    </span>
                    <span style="font-weight: 600; color: #334155;">
                      {msg.isLoading ? 'Thinking 思考中...' : 'Thinking 思考过程'}
                    </span>
                    <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-style: italic;">
                      {msg.thinking}
                    </span>
                  </div>
                </Show>

                {/* 加载中未返回文字时的打字占位 */}
                <Show when={msg.isLoading && !msg.content && !msg.thinking && !msg.error}>
                  <div style="display: flex; align-items: center; gap: 8px; font-size: 13px; color: #64748b; padding: 4px 0;">
                    <span style="display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #0284c7; animation: pulse 1.2s infinite ease-in-out;"></span>
                    <span>正在连接上游模型并生成回复...</span>
                  </div>
                </Show>

                {/* 真实异常报错卡片 (带一键打开设置与诊断) */}
                <Show when={msg.error}>
                  <div style="background: #fef2f2; border: 1px solid #fecaca; border-radius: 10px; padding: 14px 16px; display: flex; flex-direction: column; gap: 8px;">
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 600; color: #b91c1c;">
                      <span>⚠️</span>
                      <span>上游模型请求失败</span>
                    </div>
                    <div style="font-size: 12.5px; color: #7f1d1d; line-height: 1.5;">
                      {msg.error}
                    </div>
                    <div style="display: flex; align-items: center; gap: 10px; margin-top: 4px;">
                      <button
                        onClick={() => props.ctx.events.emit('settings:open', { tab: 'models' })}
                        style="padding: 4px 12px; background: #ffffff; border: 1px solid #fca5a5; border-radius: 6px; font-size: 12px; font-weight: 500; color: #b91c1c; cursor: pointer;"
                      >
                        ⚙️ 打开模型设置
                      </button>
                    </div>
                  </div>
                </Show>

                {/* 真实流式输出内容 */}
                <Show when={msg.content}>
                  <div
                    style="font-size: 13.5px; line-height: 1.65; color: #1e293b; word-break: break-word;"
                    innerHTML={formatMarkdown(msg.content)}
                  />
                </Show>

                {/* Token 用量与统计 */}
                <Show when={msg.tokensUsage && !msg.isLoading}>
                  <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #94a3b8; padding-top: 6px; border-top: 1px dashed #f1f5f9;">
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
  );
};

export default Timeline;
