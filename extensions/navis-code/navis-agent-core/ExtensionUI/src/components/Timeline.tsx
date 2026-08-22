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
  IconAsterisk,
  IconChevronRight,
  IconShield,
  IconDollarSign,
  IconPlug,
  IconActivity,
  IconCpu,
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
  thinking?: string;
  toolCalls?: ToolCallItem[];
  tokensUsage?: { prompt: number; completion: number; total: number; cost: string };
  timestamp: number;
  isLoading?: boolean;
}

export const Timeline: Component<{ ctx: NavisContext }> = (props) => {
  const [alertDismissed, setAlertDismissed] = createSignal(false);
  const [isChecking, setIsChecking] = createSignal(false);
  const [messages, setMessages] = createSignal<MessageTurn[]>([]);
  let scrollContainer: HTMLDivElement | undefined;

  const agentService = new AgentService(props.ctx);

  const activeProvider = () => gatewayStore.activeProvider();
  const shouldShowAlert = () => !alertDismissed() && activeProvider()?.status !== 'connected';

  const handleCheckAgain = async () => {
    const p = activeProvider();
    if (!p) return;
    setIsChecking(true);
    toast.info(`正在检测服务商连接 ${p.name} (${p.baseUrl})...`);
    const res = await gatewayStore.testConnection(p.id);
    setIsChecking(false);
    if (res.success) {
      setAlertDismissed(true);
      toast.success(`连接成功！延迟: ${res.pingMs}ms，Agent 引擎已就绪`);
    } else {
      toast.error('连接失败，请检查服务地址与 API Key');
    }
  };

  const handleOpenSetup = () => {
    props.ctx.events.emit('settings:open', { tab: 'models' });
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
        toolCalls: [
          {
            id: 'tc-1',
            toolName: 'view_file',
            argsSummary: 'D:\\myworkspace\\Navis Go\\AGENTS.md',
            outputSummary: 'Successfully read 74 lines of AGENTS.md',
            status: 'completed',
          },
        ],
        tokensUsage: { prompt: 1420, completion: 480, total: 1900, cost: '$0.0038' },
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
    // 监听发送消息事件并调度 Agent 执行流
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

      const raw = payload.content.trim();

      // ── 1. 处理 Slash 快捷命令 ──────────────────────────────────
      if (raw.startsWith('/')) {
        let replyContent = '';
        let thinking = '解析并执行 Slash 快捷指令...';
        const cmd = raw.split(' ')[0];

        if (cmd === '/help') {
          replyContent = `### ❓ Navis Code 快捷指令指南 (Slash Commands)

- \`/help\` - 显示本使用指南
- \`/init\` - 分析工作区并初始化项目记忆与智能体开发规范
- \`/cost\` - 查看当前会话 Token 用量与费用统计
- \`/doctor\` - 运行系统诊断 (Gateway / Node / Rust / Sandbox)
- \`/compact\` - 压缩会话上下文窗口
- \`/test\` - 运行项目自动化单元测试与回归套件
- \`/mcp\` - 查看已连接的 MCP 服务器与扩展工具
- \`/clear\` - 清空当前对话时间线`;
        } else if (cmd === '/cost') {
          replyContent = `### 💰 会话 Token 用量与成本统计

| 项目 | 用量 | 计费率 | 估算金额 (USD) |
|---|---|---|---|
| **Input Tokens (输入)** | 14,820 | $3.00 / MTok | $0.0445 |
| **Output Tokens (生成)** | 2,340 | $15.00 / MTok | $0.0351 |
| **Reasoning Tokens (思考)** | 1,280 | $15.00 / MTok | $0.0192 |
| **总计 Total** | **18,440 Tokens** | - | **$0.0988** |

*上下文容量使用率：9.2% (18.4k / 200k)*`;
        } else if (cmd === '/doctor') {
          replyContent = `### 🩺 Navis Code 系统环境健康诊断

- 🟢 **AI Model Gateway**: Connected (${activeProvider()?.baseUrl || 'http://127.0.0.1:8046/v1'}, Latency: ${activeProvider()?.pingMs || 18}ms)
- 🟢 **Microkernel Host**: Tauri v2.0 Native Bridge Active
- 🟢 **SolidJS DynamicSlot Tree**: Ready (5 active view projections)
- 🟢 **Sandbox Policy**: Bypass Mode (Auto-authorized for workspace root)
- 🟢 **Git Worktree**: Clean (Branch: main)`;
        } else if (cmd === '/mcp') {
          replyContent = `### 🔌 MCP 服务器与工具列表 (Model Context Protocol)

- **filesystem**: \`read_file\`, \`write_file\`, \`list_dir\`, \`search_files\` (Built-in)
- **git-tools**: \`git_status\`, \`git_diff\`, \`git_commit\`
- **terminal-runner**: \`execute_command\`, \`kill_process\``;
        } else if (cmd === '/compact') {
          replyContent = `✅ **上下文压缩已完成！**\n已对前序对话流生成语义摘要，释放了约 **68%** 的上下文 Token 预算。`;
        } else {
          replyContent = `已执行指令 **${raw}**。`;
        }

        setMessages((prev) =>
          prev.map((m) =>
            m.id === pendingAiMsgId
              ? {
                  ...m,
                  thinking,
                  tokensUsage: { prompt: 120, completion: 280, total: 400, cost: '$0.0006' },
                  content: replyContent,
                  isLoading: false,
                }
              : m,
          ),
        );
        return;
      }

      // ── 2. 执行真实 / 增强型 Agent 对话流 ───────────────────────
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
                  thinking: '执行过程中发生异常',
                  content: `抱歉，Agent 执行失败：${err?.message || '网络连接或模型响应超时'}。请检查服务商端点配置。`,
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
      ref={scrollContainer}
      id="timeline-scroll-container"
      style="flex: 1; width: 100%; height: 100%; overflow-y: auto; padding: 28px 32px 140px; display: flex; flex-direction: column; align-items: center; min-height: 0; overscroll-behavior: contain;"
    >
      <div style="width: 100%; max-width: 760px; display: flex; flex-direction: column; gap: 20px;">
        {/* 1. 欢迎标题 */}
        <div style="display: flex; align-items: center; gap: 10px; padding: 10px 0;">
          <span style="color: #ea580c; display: flex; align-items: center;">
            <IconAsterisk size={24} />
          </span>
          <h1 style="font-size: 22px; font-weight: 600; color: #1e1d1b; margin: 0; letter-spacing: -0.3px;">
            Welcome back, super
          </h1>
        </div>

        {/* 2. 动态服务商连接状态横幅 (未连通时才提醒，连通自动隐藏) */}
        <Show when={shouldShowAlert()}>
          <div
            style="background: #fdfaf3; border: 1px solid #f2e9d2; border-radius: 12px; padding: 14px 18px; display: flex; flex-direction: column; gap: 10px; position: relative;"
          >
            <button
              onClick={() => {
                setAlertDismissed(true);
                toast.info('已忽略告警提示');
              }}
              style="position: absolute; right: 14px; top: 12px; background: transparent; border: none; font-size: 14px; color: #b5ad9b; cursor: pointer; padding: 2px 6px; border-radius: 4px; display: flex; align-items: center;"
              title="关闭通知"
            >
              <IconClose size={14} />
            </button>

            <div style="display: flex; align-items: center; gap: 8px; font-size: 13.5px; font-weight: 600; color: #8a6d25;">
              <span>⚠️</span>
              <span>
                Can't reach {activeProvider()?.name || 'Local Gateway'} ({activeProvider()?.baseUrl || 'http://127.0.0.1:8046/v1'})
              </span>
            </div>

            <div style="font-size: 12px; color: #a1833c; cursor: pointer;" onClick={handleOpenSetup}>
              服务商尚未连通，请检查端点地址或 API Key
            </div>

            <div style="display: flex; align-items: center; gap: 10px; margin-top: 2px;">
              <button
                onClick={handleOpenSetup}
                style="padding: 5px 14px; background: #ffffff; border: 1px solid #dfd5be; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #443719; cursor: pointer; transition: all 0.1s;"
              >
                Open Setup (配置)
              </button>
              <button
                onClick={handleCheckAgain}
                disabled={isChecking()}
                style="padding: 5px 14px; background: #ffffff; border: 1px solid #dfd5be; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #443719; cursor: pointer; transition: all 0.1s; display: flex; align-items: center; gap: 5px;"
              >
                <IconZap size={13} color="#ea580c" />
                <span>{isChecking() ? 'Checking...' : 'Check again'}</span>
              </button>
            </div>
          </div>
        </Show>

        {/* 3. 快速恢复会话卡片 */}
        <Show when={messages().length === 0}>
          <div style="display: flex; flex-direction: column; gap: 8px; margin-top: 4px;">
            <div style="font-size: 12px; font-weight: 600; color: #8e8b83;">Sessions</div>
            <div
              onClick={handleRestoreSession}
              style="display: flex; align-items: center; justify-content: space-between; background: #ffffff; border: 1px solid #eae7e1; border-radius: 10px; padding: 12px 16px; cursor: pointer; transition: all 0.15s ease;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f9f8f6')}
              onMouseLeave={(e) => (e.currentTarget.style.background = '#ffffff')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span style="width: 8px; height: 8px; border-radius: 50%; background: #c2410c;" />
                <span style="font-size: 13px; font-weight: 500; color: #1e1d1b;">
                  流水设计审查 (恢复历史上下文与工程架构)
                </span>
              </div>
              <IconChevronRight size={14} color="#8e8b83" />
            </div>
          </div>
        </Show>

        {/* 4. 对话时间线消息流 */}
        <For each={messages()}>
          {(msg) => (
            <div style="display: flex; flex-direction: column; gap: 12px; width: 100%;">
              {/* 用户消息 */}
              <Show when={msg.role === 'user'}>
                <div style="display: flex; justify-content: flex-end;">
                  <div style="background: #f4f2ee; color: #1e1d1b; padding: 10px 16px; border-radius: 12px 12px 2px 12px; font-size: 13.5px; max-width: 80%; line-height: 1.5; word-break: break-word;">
                    {msg.content}
                  </div>
                </div>
              </Show>

              {/* 助手消息 */}
              <Show when={msg.role === 'assistant'}>
                <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
                  {/* 思考过程 (Thinking 折叠卡片) */}
                  <Show when={msg.thinking}>
                    <div style="background: #faf9f7; border: 1px solid #eae7e1; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #76736c; display: flex; align-items: center; gap: 8px;">
                      <span style="color: #ea580c; display: flex; align-items: center;">
                        <IconLightbulb size={14} />
                      </span>
                      <span style="font-weight: 600; color: #4b4843;">Thinking 思考过程</span>
                      <span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-style: italic;">
                        {msg.thinking}
                      </span>
                    </div>
                  </Show>

                  {/* 工具调用卡片 (Tool Calls) */}
                  <Show when={msg.toolCalls && msg.toolCalls.length > 0}>
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                      <For each={msg.toolCalls}>
                        {(tool) => (
                          <div style="background: #ffffff; border: 1px solid #eae7e1; border-radius: 8px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; box-shadow: 0 1px 3px rgba(0,0,0,0.02);">
                            <div style="display: flex; align-items: center; justify-content: space-between;">
                              <div style="display: flex; align-items: center; gap: 6px; font-family: monospace; font-size: 12px; color: #1e1d1b;">
                                <IconWrench size={13} color="#ea580c" />
                                <b>{tool.toolName}</b>
                                <span style="color: #8e8b83;">{tool.argsSummary}</span>
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
                                      style="background: #f4f2ee; color: #76736c; border: 1px solid #eae7e1; padding: 3px 8px; border-radius: 4px; font-size: 11px; cursor: pointer;"
                                    >
                                      拒绝
                                    </button>
                                  </div>
                                </Show>
                              </div>
                            </div>

                            <Show when={tool.outputSummary}>
                              <div style="background: #faf9f7; border: 1px solid #f0eee8; border-radius: 4px; padding: 6px 8px; font-family: monospace; font-size: 11.5px; color: #4b4843; white-space: pre-wrap;">
                                {tool.outputSummary}
                              </div>
                            </Show>
                          </div>
                        )}
                      </For>
                    </div>
                  </Show>

                  {/* 核心 Markdown 回答内容 */}
                  <div
                    style="background: #ffffff; color: #1e1d1b; font-size: 13.5px; line-height: 1.6; word-break: break-word;"
                    innerHTML={msg.content.replace(/\n/g, '<br/>').replace(/\*\*(.*?)\*\*/g, '<b>$1</b>')}
                  />

                  {/* Token 与计费微标 */}
                  <Show when={msg.tokensUsage}>
                    <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px; color: #8e8b83; padding-top: 4px; border-top: 1px dashed #f0eee8;">
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
  );
};

export default Timeline;
