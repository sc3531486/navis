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

const IconCopyClean = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
  </svg>
);

const IconEditClean = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 20h9"></path>
    <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path>
  </svg>
);

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
  const [editingMsgId, setEditingMsgId] = createSignal<string | null>(null);
  const [editText, setEditText] = createSignal<string>('');

  const agentService = new AgentService(props.ctx);

  const scrollToBottom = () => {
    setTimeout(() => {
      const container = document.getElementById('timeline-scroll-container');
      if (container) {
        container.scrollTop = container.scrollHeight;
      }
    }, 20);
  };

  const formatTimestamp = (ts: number) => {
    const d = new Date(ts);
    const month = d.getMonth() + 1;
    const date = d.getDate();
    const hours = d.getHours();
    const minutes = d.getMinutes().toString().padStart(2, '0');
    return `${month}月${date}日 ${hours}:${minutes}`;
  };

  const handleCopyText = (content: string, label = '内容') => {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(content);
      toast.success(`已复制${label}`);
    }
  };

  const handleStartEdit = (msg: MessageTurn) => {
    setEditingMsgId(msg.id);
    setEditText(msg.content);
  };

  const handleCancelEdit = () => {
    setEditingMsgId(null);
    setEditText('');
  };

  const handleConfirmEditAndResend = async (msgId: string) => {
    const newContent = editText().trim();
    if (!newContent) return;

    const currentMsgs = messages();
    const targetIdx = currentMsgs.findIndex((m) => m.id === msgId);
    if (targetIdx === -1) return;

    // 保留该消息之前的所有历史记录
    const prevHistory = currentMsgs.slice(0, targetIdx).map((m) => ({
      role: m.role,
      content: m.content,
    }));

    // 更新当前 User 消息
    const updatedUserMsg: MessageTurn = {
      ...currentMsgs[targetIdx],
      content: newContent,
      timestamp: Date.now(),
    };

    // 新建待填充的 Assistant 消息
    const pendingAiMsgId = `a-${Date.now()}`;
    const pendingAiMsg: MessageTurn = {
      id: pendingAiMsgId,
      role: 'assistant',
      thinking: '',
      content: '',
      timestamp: Date.now(),
      isLoading: true,
    };

    // 截断此轮之后的全部消息，并追加新的回复轮次
    setMessages([...currentMsgs.slice(0, targetIdx), updatedUserMsg, pendingAiMsg]);
    setEditingMsgId(null);
    setEditText('');
    scrollToBottom();
    toast.info('正在根据修改后的提示词重新请求模型...');

    const payload: AgentPromptPayload = {
      content: newContent,
      model: gatewayStore.activeModel()?.name || gatewayStore.activeModelId(),
      modelId: gatewayStore.activeModelId(),
      provider: gatewayStore.activeProvider()?.name,
      timestamp: Date.now(),
    };

    await agentService.streamTurn(payload, prevHistory, {
      onThinkingDelta: (delta) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === pendingAiMsgId ? { ...m, thinking: (m.thinking || '') + delta } : m)),
        );
        scrollToBottom();
      },
      onContentDelta: (delta) => {
        setMessages((prev) =>
          prev.map((m) => (m.id === pendingAiMsgId ? { ...m, content: m.content + delta } : m)),
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
          prev.map((m) => (m.id === pendingAiMsgId ? { ...m, isLoading: false, error: err.message } : m)),
        );
        scrollToBottom();
      },
    });
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
            prev.map((m) => {
              if (m.id !== pendingAiMsgId) return m;
              const existing = m.toolCalls || [];
              const idx = existing.findIndex((t) => t.id === toolCall.id);
              const updated =
                idx >= 0
                  ? existing.map((t, i) => (i === idx ? toolCall : t))
                  : [...existing, toolCall];
              return { ...m, toolCalls: updated };
            }),
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

    const unsubGoalStep = props.ctx.events.on('timeline:goal:step', (payload: any) => {
      const stepMsgId = `goal-step-${payload.round}-${Date.now()}`;
      setMessages((prev) => [
        ...prev,
        {
          id: stepMsgId,
          role: 'assistant',
          content: `🎯 **${payload.phase}**\n\n- **核心行动**: ${payload.action}\n- **阶段状态**: ✅ 自动验证完成，继续推进下阶段`,
          thinking: payload.thought,
          timestamp: payload.timestamp,
        },
      ]);
      scrollToBottom();
    });

    const unsubSession = props.ctx.events.on('session:created', () => {
      setMessages([]);
    });

    onCleanup(() => {
      unsubTurn();
      unsubGoalStep();
      unsubSession();
    });
  });

  return (
    <div style="width: 100%; max-width: 820px; display: flex; flex-direction: column; gap: 28px;">
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
          <div style="display: flex; flex-direction: column; width: 100%;">
            {/* ══════════════════════════════════════════════════════════════════
                1. 用户消息：右侧气泡框 + 时间 + 复制 + 编辑重新发送
               ══════════════════════════════════════════════════════════════════ */}
            <Show when={msg.role === 'user'}>
              <div style="display: flex; flex-direction: column; align-items: flex-end; width: 100%; gap: 6px;">
                <Show when={msg.thumbnail}>
                  <div style="display: flex; align-items: center; gap: 6px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; padding: 4px 8px; font-size: 12px; color: #475569;">
                    <span style="font-size: 12px;">🖼️</span>
                    <span>{msg.thumbnail}</span>
                  </div>
                </Show>

                {/* 气泡框模式 vs 在线编辑模式 */}
                <Show
                  when={editingMsgId() === msg.id}
                  fallback={
                    /* 用户提示词气泡框 (1:1 像素级复刻参考图) */
                    <div
                      style="max-width: 82%; background: #f4f4f5; border: 1px solid #e4e4e7; border-radius: 14px; padding: 12px 16px; font-size: 13.5px; line-height: 1.65; color: #18181b; word-break: break-word; white-space: pre-wrap; box-shadow: 0 1px 2px rgba(0,0,0,0.02);"
                    >
                      {msg.content}
                    </div>
                  }
                >
                  {/* 编辑模式输入框与操作条 */}
                  <div
                    style="width: 85%; background: #ffffff; border: 1px solid #0284c7; border-radius: 12px; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px; box-shadow: 0 4px 14px rgba(2, 132, 199, 0.12);"
                  >
                    <textarea
                      rows={4}
                      value={editText()}
                      onInput={(e) => setEditText(e.currentTarget.value)}
                      style="width: 100%; border: none; outline: none; background: transparent; font-size: 13.5px; line-height: 1.6; color: #18181b; resize: vertical; font-family: inherit;"
                    />
                    <div style="display: flex; align-items: center; justify-content: flex-end; gap: 8px; border-top: 1px solid #f1f5f9; padding-top: 8px;">
                      <button
                        onClick={handleCancelEdit}
                        style="padding: 4px 10px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; font-size: 12px; color: #64748b; cursor: pointer;"
                      >
                        取消
                      </button>
                      <button
                        onClick={() => handleConfirmEditAndResend(msg.id)}
                        style="padding: 4px 12px; background: #0284c7; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; color: #ffffff; cursor: pointer;"
                      >
                        确认并重新发送
                      </button>
                    </div>
                  </div>
                </Show>

                {/* 气泡框下方：时间戳 + 复制 + 编辑图标 (1:1 像素级复刻参考图二) */}
                <div style="display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: #a1a1aa; padding-right: 4px; user-select: none;">
                  <span style="font-size: 11.5px; color: #9ca3af;">{formatTimestamp(msg.timestamp)}</span>
                  <button
                    onClick={() => handleCopyText(msg.content, '提示词')}
                    style="background: transparent; border: none; color: #9ca3af; cursor: pointer; padding: 2px; border-radius: 4px; display: flex; align-items: center; transition: color 0.15s ease;"
                    title="复制提示词"
                    onMouseEnter={(e) => (e.currentTarget.style.color = '#3b82f6')}
                    onMouseLeave={(e) => (e.currentTarget.style.color = '#9ca3af')}
                  >
                    <IconCopyClean />
                  </button>
                  <button
                    onClick={() => handleStartEdit(msg)}
                    style="background: transparent; border: none; color: #9ca3af; cursor: pointer; padding: 2px; border-radius: 4px; display: flex; align-items: center; transition: color 0.15s ease;"
                    title="编辑提示词并重新发送"
                    onMouseEnter={(e) => (e.currentTarget.style.color = '#0284c7')}
                    onMouseLeave={(e) => (e.currentTarget.style.color = '#9ca3af')}
                  >
                    <IconEditClean />
                  </button>
                </div>
              </div>
            </Show>

            {/* ══════════════════════════════════════════════════════════════════
                2. 助手回复：左侧流式输出 + 思考过程 + 异常诊断 + Token 费用
               ══════════════════════════════════════════════════════════════════ */}
            <Show when={msg.role === 'assistant'}>
              <div style="display: flex; flex-direction: column; align-items: flex-start; gap: 12px; width: 100%; margin-top: 4px;">
                {/* 思考过程流 (Thinking 动态卡片) */}
                <Show when={msg.thinking}>
                  <div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #64748b; display: flex; align-items: center; gap: 8px; width: 100%;">
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
                  <div style="background: #fef2f2; border: 1px solid #fecaca; border-radius: 10px; padding: 14px 16px; display: flex; flex-direction: column; gap: 8px; width: 100%;">
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 600; color: #b91c1c;">
                      <span>⚠️</span>
                      <span>上游模型请求失败</span>
                    </div>
                    <div style="font-size: 12.5px; color: #7f1d1d; line-height: 1.5; word-break: break-word;">
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

                {/* 真实工具调用卡片 (Tool Calls & Execution Result) */}
                <Show when={msg.toolCalls && msg.toolCalls.length > 0}>
                  <div style="display: flex; flex-direction: column; gap: 8px; width: 100%; margin: 4px 0 8px;">
                    <For each={msg.toolCalls}>
                      {(tool) => (
                        <div
                          style="background: #ffffff; border: 1px solid #e2e8f0; border-radius: 10px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,0.03);"
                        >
                          {/* 工具卡片头部 */}
                          <div
                            style="padding: 8px 12px; background: #f8fafc; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between;"
                          >
                            <div style="display: flex; align-items: center; gap: 8px;">
                              <span style="color: #2563eb; font-size: 13px;">⚙️</span>
                              <span style="font-weight: 600; font-size: 12.5px; color: #0f172a; font-family: monospace;">
                                {tool.toolName}
                              </span>
                              <span
                                style={`font-size: 11px; padding: 1px 6px; border-radius: 4px; font-weight: 500; ${
                                  tool.status === 'completed'
                                    ? 'background: #dcfce7; color: #15803d;'
                                    : tool.status === 'approved'
                                    ? 'background: #e0f2fe; color: #0369a1;'
                                    : tool.status === 'rejected'
                                    ? 'background: #fee2e2; color: #b91c1c;'
                                    : 'background: #fef3c7; color: #b45309;'
                                }`}
                              >
                                {tool.status === 'completed'
                                  ? '✓ 已执行'
                                  : tool.status === 'rejected'
                                  ? '✕ 已拒绝'
                                  : tool.needsApproval
                                  ? '⏳ 等待批准'
                                  : '⚙ 执行中...'}
                              </span>
                            </div>
                          </div>

                          {/* 工具参数与输出结果预览 */}
                          <div style="padding: 8px 12px; display: flex; flex-direction: column; gap: 6px; font-size: 12px;">
                            <div style="color: #64748b; font-family: monospace; font-size: 11px; white-space: pre-wrap; word-break: break-all; background: #fafafa; padding: 6px 10px; border-radius: 6px; border: 1px solid #f1f5f9; max-height: 140px; overflow-y: auto;">
                              {tool.argsSummary}
                            </div>
                            <Show when={tool.outputSummary}>
                              <div style="color: #166534; background: #f0fdf4; padding: 6px 10px; border-radius: 6px; border: 1px solid #dcfce7; font-size: 11.5px; font-family: monospace;">
                                {tool.outputSummary}
                              </div>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>

                {/* 真实流式输出内容 */}
                <Show when={msg.content}>
                  <div
                    style="font-size: 13.5px; line-height: 1.65; color: #1e293b; word-break: break-word; width: 100%;"
                    innerHTML={formatMarkdown(msg.content)}
                  />
                </Show>

                {/* Token 用量与统计 */}
                <Show when={msg.tokensUsage && !msg.isLoading}>
                  <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #94a3b8; padding-top: 6px; border-top: 1px dashed #f1f5f9; width: 100%;">
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
