import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
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
}

export const Timeline: Component<{ ctx: NavisContext }> = (props) => {
  const [alertVisible, setAlertVisible] = createSignal(true);
  const [isChecking, setIsChecking] = createSignal(false);
  const [messages, setMessages] = createSignal<MessageTurn[]>([]);
  let scrollContainer: HTMLDivElement | undefined;

  const handleCheckAgain = async () => {
    setIsChecking(true);
    toast.info('正在检测本地网关 127.0.0.1:15721...');
    const res = await gatewayStore.testConnection('gateway-local');
    setIsChecking(false);
    if (res.success) {
      setAlertVisible(false);
      toast.success(`网关连接成功！延迟: ${res.pingMs}ms，Agent 引擎已就绪`);
    } else {
      toast.error('网关连接失败，请检查端口是否开启');
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
    // 监听发送消息
    const unsubTurn = props.ctx.events.on('agent:turn:start', (payload: any) => {
      const userMsg: MessageTurn = {
        id: `u-${Date.now()}`,
        role: 'user',
        content: payload.content,
        timestamp: Date.now(),
      };

      setMessages((prev) => [...prev, userMsg]);

      const raw = payload.content.trim();

      // ── 处理 Slash 快捷命令 ──────────────────────────────────
      if (raw.startsWith('/')) {
        setTimeout(() => {
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

- 🟢 **AI Model Gateway**: Connected (http://127.0.0.1:15721, Latency: 18ms)
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

          const aiMsg: MessageTurn = {
            id: `a-${Date.now()}`,
            role: 'assistant',
            thinking,
            tokensUsage: { prompt: 120, completion: 280, total: 400, cost: '$0.0006' },
            content: replyContent,
            timestamp: Date.now(),
          };
          setMessages((prev) => [...prev, aiMsg]);
          setTimeout(() => {
            if (scrollContainer) {
              scrollContainer.scrollTop = scrollContainer.scrollHeight;
            }
          }, 50);
        }, 400);
        return;
      }

      // ── 常规 Agent 对话与工具调用模拟 ────────────────────────
      const isNeedConfirm = payload.permission === 'Ask for confirmation';

      setTimeout(() => {
        const toolCalls: ToolCallItem[] = [
          {
            id: `tc-${Date.now()}-1`,
            toolName: 'view_file',
            argsSummary: 'D:\\myworkspace\\Navis Go\\AGENTS.md',
            outputSummary: 'AGENTS.md loaded successfully (74 lines)',
            status: 'completed',
          },
          {
            id: `tc-${Date.now()}-2`,
            toolName: 'run_command',
            argsSummary: 'cargo check',
            outputSummary: isNeedConfirm ? undefined : 'Finished `dev` profile in 0.48s',
            status: isNeedConfirm ? 'pending' : 'completed',
            needsApproval: isNeedConfirm,
          },
        ];

        const aiMsg: MessageTurn = {
          id: `a-${Date.now()}`,
          role: 'assistant',
          thinking: `使用模型 ${payload.model || 'gemini-3.7-flash'} 分析任务需求与上下文，规划执行计划...`,
          toolCalls,
          tokensUsage: { prompt: 2150, completion: 620, total: 2770, cost: '$0.0055' },
          content: `收到您的任务指令：“**${payload.content}**”。\n\nAgent 正在以 **${payload.permission || 'Bypass permissions'}** 权限模式协同处理中。`,
          timestamp: Date.now(),
        };
        setMessages((prev) => [...prev, aiMsg]);
        setTimeout(() => {
          if (scrollContainer) {
            scrollContainer.scrollTop = scrollContainer.scrollHeight;
          }
        }, 50);
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
      id="timeline-scroll-container"
      style="flex: 1; width: 100%; height: 100%; overflow-y: auto; padding: 28px 32px 140px; display: flex; flex-direction: column; align-items: center; min-height: 0;"
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

        {/* 2. 警告横幅卡片 (可关闭 / 可重新检测) */}
        <Show when={alertVisible()}>
          <div
            style="background: #fdfaf3; border: 1px solid #f2e9d2; border-radius: 12px; padding: 14px 18px; display: flex; flex-direction: column; gap: 10px; position: relative;"
          >
            <button
              onClick={() => {
                setAlertVisible(false);
                toast.info('已忽略告警提示');
              }}
              style="position: absolute; right: 14px; top: 12px; background: transparent; border: none; font-size: 14px; color: #b5ad9b; cursor: pointer; padding: 2px 6px; border-radius: 4px; display: flex; align-items: center;"
              title="关闭通知"
            >
              <IconClose size={14} />
            </button>

            <div style="display: flex; align-items: center; gap: 8px; font-size: 13.5px; font-weight: 600; color: #8a6d25;">
              <span>⚠️</span>
              <span>Can't reach 127.0.0.1:15721</span>
            </div>

            <div style="font-size: 12px; color: #a1833c; cursor: pointer;" onClick={handleOpenSetup}>
              Details
            </div>

            <div style="display: flex; align-items: center; gap: 10px; margin-top: 2px;">
              <button
                onClick={handleOpenSetup}
                style="padding: 5px 14px; background: #ffffff; border: 1px solid #dfd5be; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #443719; cursor: pointer; transition: all 0.1s;"
              >
                Open Setup
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
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = '#d6d3ca';
                e.currentTarget.style.boxShadow = '0 2px 8px rgba(0,0,0,0.04)';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = '#eae7e1';
                e.currentTarget.style.boxShadow = 'none';
              }}
            >
              <div style="display: flex; align-items: center; gap: 8px;">
                <span style="color: #c2410c; font-size: 14px;">●</span>
                <span style="font-size: 12px; color: #c2410c; font-weight: 600;">Needs input</span>
                <span style="font-size: 13px; color: #2d2b28; font-weight: 500;">General coding session</span>
              </div>
              <div style="display: flex; align-items: center; gap: 6px; color: #8e8b83; font-size: 12px;">
                <span>yesterday</span>
                <IconChevronRight size={13} />
              </div>
            </div>
          </div>
        </Show>

        {/* 4. 消息流与对话气泡 */}
        <For each={messages()}>
          {(msg) => (
            <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
              {/* 用户提问气泡 */}
              <Show when={msg.role === 'user'}>
                <div style="display: flex; justify-content: flex-end;">
                  <div
                    style="background: #f0eee8; color: #1e1d1b; padding: 10px 16px; border-radius: 12px 12px 2px 12px; font-size: 13.5px; line-height: 1.5; max-width: 85%; word-break: break-word;"
                  >
                    {msg.content}
                  </div>
                </div>
              </Show>

              {/* 智能体回复气泡 */}
              <Show when={msg.role === 'assistant'}>
                <div style="display: flex; flex-direction: column; gap: 10px; width: 100%;">
                  {/* 思考过程折叠块 */}
                  <Show when={msg.thinking}>
                    <details style="background: #faf9f6; border: 1px solid #eae7e1; border-radius: 8px; padding: 8px 12px; font-size: 12px; color: #76736c;">
                      <summary style="cursor: pointer; font-weight: 500; user-select: none; display: flex; align-items: center; gap: 6px;">
                        <IconLightbulb size={14} color="#eab308" />
                        <span>Thinking 思考过程</span>
                      </summary>
                      <div style="margin-top: 8px; line-height: 1.5; color: #5a5750; white-space: pre-wrap; font-family: inherit;">
                        {msg.thinking}
                      </div>
                    </details>
                  </Show>

                  {/* 工具调用卡片列表 */}
                  <Show when={msg.toolCalls && msg.toolCalls.length > 0}>
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                      <For each={msg.toolCalls}>
                        {(tool) => (
                          <div
                            style={`border-radius: 8px; border: 1px solid; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px; font-size: 12px; ${
                              tool.needsApproval
                                ? 'background: #fffbeb; border-color: #fde68a;'
                                : tool.status === 'completed'
                                ? 'background: #f8fafc; border-color: #e2e8f0;'
                                : 'background: #fef2f2; border-color: #fecaca;'
                            }`}
                          >
                            <div style="display: flex; align-items: center; justify-content: space-between;">
                              <div style="display: flex; align-items: center; gap: 6px;">
                                <IconWrench size={13} color="#64748b" />
                                <b style="color: #1e293b; font-family: monospace;">{tool.toolName}</b>
                                <span style="color: #64748b; font-family: monospace;">{tool.argsSummary}</span>
                              </div>
                              <span
                                style={`font-size: 10.5px; padding: 1px 6px; border-radius: 4px; font-weight: 500; ${
                                  tool.needsApproval
                                    ? 'background: #fef3c7; color: #92400e;'
                                    : tool.status === 'completed'
                                    ? 'background: #dcfce7; color: #166534;'
                                    : 'background: #fee2e2; color: #991b1b;'
                                }`}
                              >
                                {tool.needsApproval ? '等待确认' : tool.status === 'completed' ? '执行成功' : '已拒绝'}
                              </span>
                            </div>

                            <Show when={tool.outputSummary}>
                              <div style="font-size: 11px; color: #475569; background: #ffffff; padding: 4px 8px; border-radius: 4px; border: 1px solid #e2e8f0; font-family: monospace;">
                                {tool.outputSummary}
                              </div>
                            </Show>

                            {/* 人工审批按钮 */}
                            <Show when={tool.needsApproval}>
                              <div style="display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 4px;">
                                <button
                                  onClick={() => handleRejectTool(msg.id, tool.id)}
                                  style="padding: 4px 10px; background: #ffffff; border: 1px solid #cbd5e1; border-radius: 4px; font-size: 11.5px; color: #475569; cursor: pointer;"
                                >
                                  ✕ 拒绝
                                </button>
                                <button
                                  onClick={() => handleApproveTool(msg.id, tool.id)}
                                  style="padding: 4px 12px; background: #16a34a; border: none; border-radius: 4px; font-size: 11.5px; color: #ffffff; font-weight: 500; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                                >
                                  <IconCheck size={12} />
                                  <span>允许执行 (Approve)</span>
                                </button>
                              </div>
                            </Show>
                          </div>
                        )}
                      </For>
                    </div>
                  </Show>

                  {/* 回复正文 */}
                  <div
                    style="background: #ffffff; border: 1px solid #eae7e1; border-radius: 12px; padding: 16px; font-size: 13.5px; line-height: 1.6; color: #2d2b28; display: flex; flex-direction: column; gap: 10px;"
                  >
                    <div style="white-space: pre-wrap;">{msg.content}</div>

                    {/* Token 用量与统计 */}
                    <Show when={msg.tokensUsage}>
                      {(usage) => (
                        <div
                          style="display: flex; align-items: center; justify-content: space-between; border-top: 1px solid #f4f2ee; padding-top: 8px; margin-top: 4px; font-size: 11px; color: #8e8b83;"
                        >
                          <span>
                            Tokens: {usage().prompt} in / {usage().completion} out (Total: {usage().total})
                          </span>
                          <span>估算费用: {usage().cost}</span>
                        </div>
                      )}
                    </Show>
                  </div>
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
