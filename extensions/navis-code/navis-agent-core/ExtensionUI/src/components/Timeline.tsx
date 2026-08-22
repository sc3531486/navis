import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';

interface MessageTurn {
  id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  thinking?: string;
  timestamp: number;
}

export const Timeline: Component<{ ctx: NavisContext }> = (props) => {
  const [alertVisible, setAlertVisible] = createSignal(true);
  const [isChecking, setIsChecking] = createSignal(false);
  const [messages, setMessages] = createSignal<MessageTurn[]>([]);
  let scrollContainer: HTMLDivElement | undefined;

  const handleCheckAgain = () => {
    setIsChecking(true);
    toast.info('正在检测本地网关 127.0.0.1:15721...');
    setTimeout(() => {
      setIsChecking(false);
      setAlertVisible(false);
      toast.success('网关连接成功！Agent 引擎已就绪');
    }, 1200);
  };

  const handleOpenSetup = () => {
    props.ctx.events.emit('settings:open', { tab: 'gateway' });
  };

  const handleRestoreSession = () => {
    setMessages([
      {
        id: 'msg-1',
        role: 'user',
        content: '请帮我梳理一下 Navis Go 的架构与流水设计。',
        timestamp: Date.now() - 3600000,
      },
      {
        id: 'msg-2',
        role: 'assistant',
        thinking: '用户需要梳理架构与流水设计，我将基于微内核与万物皆扩展原则进行解答。',
        content: `### Navis 核心架构总览

Navis 是基于 **Tauri 2** 的通用桌面应用白板与扩展运行时底座：
1. **纯净微内核**：宿主仅负责窗口生命周期、扩展加载、IoC 容器（DI）与 DynamicSlot 响应式插槽树。
2. **万物皆扩展**：AI 网关、Agent 核心流、代码编辑器与会话全部作为独立业务扩展装配。

\`\`\`rust
// 宿主微内核 RPC 路由分发
pub async fn navis_dispatch_rpc(route: &str, payload: Value) -> Result<Value, String> {
    core_router().dispatch(route, payload).await
}
\`\`\``,
        timestamp: Date.now() - 3500000,
      },
    ]);
    toast.info('已恢复会话历史消息');
  };

  onMount(() => {
    // 监听发送消息
    const unsubTurn = props.ctx.events.on('agent:turn:start', (payload: any) => {
      const userMsg: MessageTurn = {
        id: `u-${Date.now()}`,
        role: 'user',
        content: payload.content,
        timestamp: Date.now(),
      };

      setMessages((prev) => [...prev, userMsg]);

      // 模拟 AI 流式回复
      setTimeout(() => {
        const aiMsg: MessageTurn = {
          id: `a-${Date.now()}`,
          role: 'assistant',
          thinking: `使用模型 ${payload.model || 'gemini-3.7-flash'} 分析任务需求与上下文并执行推理...`,
          content: `收到您的任务指令：“**${payload.content}**”。\n\nAgent 正在以 **${payload.permission || 'Bypass permissions'}** 权限模式协同处理中。`,
          timestamp: Date.now(),
        };
        setMessages((prev) => [...prev, aiMsg]);
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      }, 600);
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
      ref={scrollContainer}
      style="flex: 1; overflow-y: auto; padding: 28px 32px 140px; display: flex; flex-direction: column; align-items: center; min-height: 0;"
    >
      <div style="width: 100%; max-width: 760px; display: flex; flex-direction: column; gap: 20px;">
        {/* 空状态欢迎屏 */}
        <Show when={messages().length === 0}>
          {/* 大标题 */}
          <div style="display: flex; align-items: center; gap: 10px; margin-top: 8px;">
            <span style="font-size: 26px; color: #c2410c; font-weight: bold; line-height: 1;">✳</span>
            <h1 style="font-size: 24px; font-weight: 600; color: #1e1d1b; margin: 0; letter-spacing: -0.3px;">
              Welcome back, super
            </h1>
          </div>

          {/* 告警提示横幅 */}
          <Show when={alertVisible()}>
            <div
              style="background: #fffbeb; border: 1px solid #fef3c7; border-radius: 12px; padding: 14px 18px; display: flex; flex-direction: column; gap: 10px; position: relative; box-shadow: 0 1px 4px rgba(217, 119, 6, 0.06);"
            >
              <div style="display: flex; align-items: center; justify-content: space-between;">
                <div style="display: flex; align-items: center; gap: 8px; font-size: 13.5px; font-weight: 600; color: #92400e;">
                  <span>⚠️</span>
                  <span>Can't reach 127.0.0.1:15721</span>
                </div>
                <button
                  onClick={() => setAlertVisible(false)}
                  style="background: transparent; border: none; font-size: 14px; color: #b45309; cursor: pointer; padding: 2px 4px; border-radius: 4px;"
                  title="关闭提示"
                >
                  ✕
                </button>
              </div>

              <div>
                <a
                  href="javascript:void(0)"
                  onClick={() => toast.info('网关排查：请确认 LiteLLM 或本地网关进程已在 15721 端口监听。')}
                  style="font-size: 12.5px; color: #b45309; text-decoration: underline;"
                >
                  Details
                </a>
              </div>

              <div style="display: flex; align-items: center; gap: 10px; margin-top: 2px;">
                <button
                  onClick={handleOpenSetup}
                  style="padding: 6px 14px; background: #ffffff; border: 1px solid #fde68a; border-radius: 7px; font-size: 12.5px; font-weight: 500; color: #92400e; cursor: pointer; box-shadow: 0 1px 2px rgba(0,0,0,0.04);"
                >
                  Open Setup
                </button>
                <button
                  onClick={handleCheckAgain}
                  disabled={isChecking()}
                  style="padding: 6px 14px; background: #ffffff; border: 1px solid #fde68a; border-radius: 7px; font-size: 12.5px; font-weight: 500; color: #92400e; cursor: pointer; box-shadow: 0 1px 2px rgba(0,0,0,0.04); display: flex; align-items: center; gap: 6px;"
                >
                  <Show when={isChecking()}>
                    <span style="display: inline-block; animation: navis-spin 1s linear infinite;">⏳</span>
                  </Show>
                  <span>{isChecking() ? 'Checking...' : 'Check again'}</span>
                </button>
              </div>
            </div>
          </Show>

          {/* Sessions 专属卡片 */}
          <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 4px;">
            <span style="font-size: 12px; font-weight: 600; color: #76736c;">Sessions</span>
            <div
              onClick={handleRestoreSession}
              style="display: flex; align-items: center; justify-content: space-between; background: #f7f6f2; border: 1px solid #eae7e1; border-radius: 10px; padding: 12px 16px; cursor: pointer; transition: all 0.1s ease;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
              onMouseLeave={(e) => (e.currentTarget.style.background = '#f7f6f2')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span style="width: 7px; height: 7px; border-radius: 50%; background: #d97706; flex-shrink: 0;" />
                <span style="font-size: 12.5px; font-weight: 500; color: #d97706;">Needs input</span>
                <span style="font-size: 13px; font-weight: 600; color: #1e1d1b;">General coding session</span>
              </div>
              <div style="display: flex; align-items: center; gap: 10px; font-size: 12px; color: #8e8b83;">
                <span>sc3531486/navis</span>
                <span>yesterday</span>
                <span>&gt;</span>
              </div>
            </div>
          </div>
        </Show>

        {/* 多轮消息流 */}
        <Show when={messages().length > 0}>
          <div style="display: flex; flex-direction: column; gap: 18px; width: 100%;">
            <For each={messages()}>
              {(msg) => (
                <div
                  style={`display: flex; flex-direction: column; gap: 8px; ${
                    msg.role === 'user' ? 'align-items: flex-end;' : 'align-items: flex-start;'
                  }`}
                >
                  <Show when={msg.role === 'user'}>
                    <div style="background: #eceae4; color: #1e1d1b; padding: 10px 16px; border-radius: 12px 12px 2px 12px; max-width: 85%; font-size: 13.5px; line-height: 1.5;">
                      {msg.content}
                    </div>
                  </Show>

                  <Show when={msg.role === 'assistant'}>
                    <div style="width: 100%; display: flex; flex-direction: column; gap: 8px;">
                      {/* 思考折叠块 */}
                      <Show when={msg.thinking}>
                        <details style="background: #faf9f6; border: 1px solid #eae7e1; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #76736c;">
                          <summary style="cursor: pointer; font-weight: 500;">
                            💡 Thinking 思考过程
                          </summary>
                          <div style="margin-top: 6px; color: #8e8b83; line-height: 1.4;">
                            {msg.thinking}
                          </div>
                        </details>
                      </Show>

                      {/* 消息正文 */}
                      <div style="background: #ffffff; border: 1px solid #eae7e1; border-radius: 12px; padding: 16px; font-size: 13.5px; line-height: 1.6; color: #2d2b28; width: 100%;">
                        <div style="white-space: pre-wrap;">{msg.content}</div>
                      </div>
                    </div>
                  </Show>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default Timeline;
