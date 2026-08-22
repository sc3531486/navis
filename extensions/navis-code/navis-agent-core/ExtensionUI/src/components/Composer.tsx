import { Component, createSignal, Show, For, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconSend,
  IconPlus,
  IconCopy,
  IconFolder,
  IconGitBranch,
  IconChevronDown,
  IconSparkles,
  IconPrompt,
  IconShield,
  IconCpu,
  IconActivity,
  IconDollarSign,
  IconPlug,
  IconTrash,
  IconCheck,
} from '@/components/icons';

interface SlashCommand {
  id: string;
  name: string;
  desc: string;
  iconComponent: any;
}

const slashCommands: SlashCommand[] = [
  { id: '/help', name: '/help', desc: '查看可用指令列表与使用指南', iconComponent: IconPrompt },
  { id: '/init', name: '/init', desc: '分析工作区并初始化项目记忆与规范', iconComponent: IconSparkles },
  { id: '/compact', name: '/compact', desc: '压缩会话上下文窗口并释放 Token 预算', iconComponent: IconCpu },
  { id: '/cost', name: '/cost', desc: '查看当前会话的 Token 用量与费用统计', iconComponent: IconDollarSign },
  { id: '/test', name: '/test', desc: '运行项目自动化测试套件 (cargo / npm test)', iconComponent: IconActivity },
  { id: '/doctor', name: '/doctor', desc: '运行网关、Node、Rust 与沙箱健康诊断', iconComponent: IconShield },
  { id: '/mcp', name: '/mcp', desc: '查看已连接的 MCP 服务与扩展工具', iconComponent: IconPlug },
  { id: '/clear', name: '/clear', desc: '清空当前会话的时间线消息', iconComponent: IconTrash },
];

export const Composer: Component<{ ctx: NavisContext }> = (props) => {
  const [text, setText] = createSignal('');
  const [permissionMode, setPermissionMode] = createSignal<'Bypass permissions' | 'Ask for confirmation' | 'Read-only'>('Bypass permissions');
  const [reasoningIntensity, setReasoningIntensity] = createSignal<'Off' | 'Low' | 'Medium' | 'High'>('High');
  const [activeDropdown, setActiveDropdown] = createSignal<'perm' | 'plus' | 'model' | 'reason' | 'context' | null>(null);
  const [selectedSlashIndex, setSelectedSlashIndex] = createSignal(0);

  // 匹配 Slash 命令
  const showSlashMenu = () => {
    const val = text();
    return val.startsWith('/') && !val.includes(' ');
  };

  const filteredSlashCommands = () => {
    const val = text().toLowerCase();
    return slashCommands.filter((c) => c.name.toLowerCase().startsWith(val));
  };

  const handleSelectSlash = (cmd: SlashCommand) => {
    setText(cmd.name + ' ');
    if (cmd.id === '/clear') {
      props.ctx.events.emit('session:created');
      setText('');
      toast.success('已清空当前会话消息');
      return;
    }
  };

  const handleSend = () => {
    const content = text().trim();
    if (!content) return;

    if (content === '/clear') {
      props.ctx.events.emit('session:created');
      setText('');
      toast.success('已清空当前会话消息');
      return;
    }

    const currentModel = gatewayStore.activeModel();

    props.ctx.events.emit('agent:turn:start', {
      content,
      model: currentModel?.name || gatewayStore.activeModelId(),
      modelId: gatewayStore.activeModelId(),
      provider: gatewayStore.activeProvider()?.name,
      permission: permissionMode(),
      reasoning: reasoningIntensity(),
      timestamp: Date.now(),
    });

    setText('');
    toast.info(`已发送指令至 ${currentModel?.name || 'Agent'}`);
  };

  const handleCopyContext = () => {
    if (navigator.clipboard) {
      navigator.clipboard.writeText('D:\\myworkspace\\Navis Go [branch: main]');
    }
    toast.success('已复制上下文信息至剪贴板');
  };

  onMount(() => {
    const handleClickOutside = () => setActiveDropdown(null);
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  return (
    <div style="position: absolute; bottom: 16px; left: 0; right: 0; display: flex; justify-content: center; padding: 0 24px; z-index: 50; pointer-events: none;">
      <div
        style="width: 100%; max-width: 760px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 14px; box-shadow: 0 6px 24px rgba(0, 0, 0, 0.07); display: flex; flex-direction: column; overflow: visible; position: relative; pointer-events: auto;"
      >
        {/* Slash 命令浮动弹窗 */}
        <Show when={showSlashMenu() && filteredSlashCommands().length > 0}>
          <div
            style="position: absolute; left: 0; bottom: 100%; margin-bottom: 8px; width: 100%; max-width: 420px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 10px; box-shadow: 0 10px 30px rgba(0,0,0,0.14); padding: 6px; z-index: 120; display: flex; flex-direction: column; gap: 2px;"
          >
            <div style="padding: 4px 8px; font-size: 11px; font-weight: 600; color: #8e8b83; border-bottom: 1px solid #f4f2ee;">
              SLASH 指令 (快捷命令)
            </div>
            <For each={filteredSlashCommands()}>
              {(cmd, idx) => {
                const IconComp = cmd.iconComponent;
                return (
                  <div
                    onClick={() => handleSelectSlash(cmd)}
                    onMouseEnter={() => setSelectedSlashIndex(idx())}
                    style={`padding: 7px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; transition: background 0.1s; ${
                      selectedSlashIndex() === idx() ? 'background: #f7f6f2;' : 'background: transparent;'
                    }`}
                  >
                    <div style="display: flex; align-items: center; gap: 8px;">
                      <span style="color: #ea580c; display: flex; align-items: center;">
                        <IconComp size={14} />
                      </span>
                      <b style="color: #1e1d1b; font-family: monospace;">{cmd.name}</b>
                    </div>
                    <span style="font-size: 11.5px; color: #76736c;">{cmd.desc}</span>
                  </div>
                );
              }}
            </For>
          </div>
        </Show>

        {/* 顶部上下文药丸胶囊 */}
        <div style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px 4px; border-bottom: 1px solid #f4f2ee;">
          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
            <div
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'context' ? null : 'context');
              }}
              style="display: flex; align-items: center; gap: 5px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750; cursor: pointer;"
            >
              <IconCpu size={12} color="#71717a" />
              <span style="font-weight: 500;">Local</span>
            </div>
            <div
              style="display: flex; align-items: center; gap: 5px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750;"
            >
              <IconFolder size={12} color="#71717a" />
              <span style="font-weight: 500;">Navis Go</span>
            </div>
            <div
              style="display: flex; align-items: center; gap: 5px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750;"
            >
              <IconGitBranch size={12} color="#71717a" />
              <span style="font-weight: 500;">main</span>
            </div>
            <button
              onClick={handleCopyContext}
              style="background: transparent; border: none; font-size: 12px; color: #8e8b83; cursor: pointer; padding: 2px 4px; border-radius: 4px; display: flex; align-items: center;"
              title="复制上下文路径"
            >
              <IconCopy size={12} />
            </button>
          </div>
        </div>

        {/* 核心输入区域 */}
        <div style="display: flex; align-items: flex-end; padding: 8px 12px 4px;">
          <textarea
            rows={2}
            value={text()}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (showSlashMenu() && filteredSlashCommands().length > 0) {
                if (e.key === 'ArrowDown') {
                  e.preventDefault();
                  setSelectedSlashIndex((i) => Math.min(i + 1, filteredSlashCommands().length - 1));
                  return;
                } else if (e.key === 'ArrowUp') {
                  e.preventDefault();
                  setSelectedSlashIndex((i) => Math.max(i - 1, 0));
                  return;
                } else if (e.key === 'Tab' || (e.key === 'Enter' && !e.shiftKey)) {
                  e.preventDefault();
                  const list = filteredSlashCommands();
                  if (list[selectedSlashIndex()]) {
                    handleSelectSlash(list[selectedSlashIndex()]);
                    return;
                  }
                }
              }

              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="输入任务指令、提问，或键入 / 唤起快捷指令 (Enter 发送)"
            style="flex: 1; border: none; outline: none; background: transparent; font-size: 13.5px; line-height: 1.5; color: #2d2b28; resize: none; min-height: 48px; max-height: 160px; font-family: inherit;"
          />
          <button
            onClick={handleSend}
            disabled={!text().trim()}
            style={`width: 28px; height: 28px; border-radius: 6px; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.1s ease; margin-bottom: 2px; ${
              text().trim() ? 'background: #18181b; color: #ffffff;' : 'background: #f0eee8; color: #b5b2aa; cursor: not-allowed;'
            }`}
            title="发送消息 (Enter)"
          >
            <IconSend size={13} />
          </button>
        </div>

        {/* 底部参数控制栏 */}
        <div style="display: flex; align-items: center; justify-content: space-between; padding: 6px 12px 8px; border-top: 1px solid #f4f2ee; font-size: 12px; color: #76736c; position: relative;">
          {/* 左侧：权限模式与附件 */}
          <div style="display: flex; align-items: center; gap: 8px; position: relative;">
            <div
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'perm' ? null : 'perm');
              }}
              style="display: flex; align-items: center; gap: 4px; cursor: pointer; padding: 3px 6px; border-radius: 4px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <span>{permissionMode()}</span>
              <IconChevronDown size={11} color="#8e8b83" />
            </div>

            {/* 权限选择下拉菜单 */}
            <Show when={activeDropdown() === 'perm'}>
              <div
                onClick={(e) => e.stopPropagation()}
                style="position: absolute; left: 0; bottom: 32px; width: 260px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 4px; z-index: 100; display: flex; flex-direction: column; gap: 2px;"
              >
                <div
                  onClick={() => {
                    setPermissionMode('Bypass permissions');
                    setActiveDropdown(null);
                    toast.info('已切换为自动执行模式');
                  }}
                  style={`padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; ${
                    permissionMode() === 'Bypass permissions' ? 'background: #f7f6f2;' : ''
                  }`}
                >
                  <b style="color: #2d2b28;">Bypass permissions</b>
                  <div style="font-size: 11px; color: #8e8b83;">自动放行文件与终端命令执行</div>
                </div>
                <div
                  onClick={() => {
                    setPermissionMode('Ask for confirmation');
                    setActiveDropdown(null);
                    toast.info('已切换为确认审批模式');
                  }}
                  style={`padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; ${
                    permissionMode() === 'Ask for confirmation' ? 'background: #f7f6f2;' : ''
                  }`}
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <b style="color: #2d2b28;">Ask for confirmation</b>
                  <div style="font-size: 11px; color: #8e8b83;">危险操作触发弹窗人工审批</div>
                </div>
                <div
                  onClick={() => {
                    setPermissionMode('Read-only');
                    setActiveDropdown(null);
                    toast.info('已切换为严格只读模式');
                  }}
                  style={`padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; ${
                    permissionMode() === 'Read-only' ? 'background: #f7f6f2;' : ''
                  }`}
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <b style="color: #2d2b28;">Read-only</b>
                  <div style="font-size: 11px; color: #8e8b83;">禁止写入任何文件与执行命令</div>
                </div>
              </div>
            </Show>

            {/* 附件添加按钮 */}
            <button
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'plus' ? null : 'plus');
              }}
              style="background: transparent; border: none; color: #76736c; cursor: pointer; padding: 2px 6px; border-radius: 4px; display: flex; align-items: center;"
              title="添加上下文文件或资源"
            >
              <IconPlus size={13} />
            </button>

            {/* 附件选择下拉菜单 */}
            <Show when={activeDropdown() === 'plus'}>
              <div
                onClick={(e) => e.stopPropagation()}
                style="position: absolute; left: 140px; bottom: 32px; width: 180px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 4px; z-index: 100; display: flex; flex-direction: column; gap: 2px;"
              >
                <div
                  onClick={() => {
                    setActiveDropdown(null);
                    props.ctx.commands.execute('project:open-folder');
                    toast.info('已添加工作区文件');
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28; display: flex; align-items: center; gap: 6px;"
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <IconFolder size={13} color="#71717a" />
                  <span>添加工作区文件...</span>
                </div>
                <div
                  onClick={() => {
                    setActiveDropdown(null);
                    props.ctx.events.emit('settings:open', { tab: 'prompt' });
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28; display: flex; align-items: center; gap: 6px;"
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  <IconPrompt size={13} color="#71717a" />
                  <span>插入自定义 Prompt...</span>
                </div>
              </div>
            </Show>
          </div>

          {/* 右侧：模型选择、思考强度与状态指示灯 */}
          <div style="display: flex; align-items: center; gap: 10px; position: relative;">
            {/* 动态模型选择器 */}
            <div
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'model' ? null : 'model');
              }}
              style="display: flex; align-items: center; gap: 4px; cursor: pointer; padding: 3px 6px; border-radius: 4px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <span style="font-weight: 500; color: #4b4843;">{gatewayStore.activeModel()?.name || gatewayStore.activeModelId()}</span>
              <IconChevronDown size={11} color="#8e8b83" />
            </div>

            {/* 动态模型下拉菜单 */}
            <Show when={activeDropdown() === 'model'}>
              <div
                onClick={(e) => e.stopPropagation()}
                style="position: absolute; right: 80px; bottom: 32px; width: 260px; max-height: 280px; overflow-y: auto; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 4px; z-index: 100; display: flex; flex-direction: column; gap: 2px;"
              >
                <div style="padding: 4px 8px; font-size: 11px; font-weight: 600; color: #8e8b83; border-bottom: 1px solid #f0eee8;">
                  当前 Provider: {gatewayStore.activeProvider().name}
                </div>
                <For each={gatewayStore.activeProvider().models}>
                  {(m) => (
                    <div
                      onClick={() => {
                        gatewayStore.setActiveModel(m.id);
                        setActiveDropdown(null);
                        toast.info(`已切换模型: ${m.name}`);
                      }}
                      style={`padding: 6px 8px; border-radius: 4px; cursor: pointer; display: flex; flex-direction: column; gap: 2px; ${
                        gatewayStore.activeModelId() === m.id ? 'background: #f7f6f2;' : ''
                      }`}
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                      onMouseLeave={(e) => {
                        if (gatewayStore.activeModelId() !== m.id) e.currentTarget.style.background = 'transparent';
                      }}
                    >
                      <div style="display: flex; align-items: center; justify-content: space-between;">
                        <b style="font-size: 12px; color: #1e1d1b;">{m.name}</b>
                        <Show when={m.capabilities.reasoning}>
                          <span style="font-size: 9px; color: #c2410c; background: #fff7ed; padding: 1px 4px; border-radius: 3px;">
                            Reasoning
                          </span>
                        </Show>
                      </div>
                      <span style="font-size: 10.5px; color: #8e8b83; font-family: monospace;">{m.id}</span>
                    </div>
                  )}
                </For>
                <div
                  onClick={() => {
                    setActiveDropdown(null);
                    props.ctx.events.emit('settings:open', { tab: 'models' });
                  }}
                  style="padding: 6px 8px; border-top: 1px solid #f0eee8; cursor: pointer; font-size: 11.5px; color: #c2410c; text-align: center; font-weight: 500;"
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#fff7ed')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  ⚙️ 打开模型配置中心...
                </div>
              </div>
            </Show>

            {/* 思考强度选择器 */}
            <div
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'reason' ? null : 'reason');
              }}
              style="display: flex; align-items: center; gap: 4px; cursor: pointer; padding: 3px 6px; border-radius: 4px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <span>{reasoningIntensity()}</span>
              <IconChevronDown size={11} color="#8e8b83" />
            </div>

            {/* 思考强度下拉 */}
            <Show when={activeDropdown() === 'reason'}>
              <div
                onClick={(e) => e.stopPropagation()}
                style="position: absolute; right: 20px; bottom: 32px; width: 140px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 4px; z-index: 100; display: flex; flex-direction: column; gap: 2px;"
              >
                <div
                  onClick={() => {
                    setReasoningIntensity('Off');
                    setActiveDropdown(null);
                    toast.info('关闭深度思考');
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28;"
                >
                  Off (关闭)
                </div>
                <div
                  onClick={() => {
                    setReasoningIntensity('Low');
                    setActiveDropdown(null);
                    toast.info('轻量思考模式');
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28;"
                >
                  Low (轻量)
                </div>
                <div
                  onClick={() => {
                    setReasoningIntensity('Medium');
                    setActiveDropdown(null);
                    toast.info('中等思考模式');
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28;"
                >
                  Medium (中等)
                </div>
                <div
                  onClick={() => {
                    setReasoningIntensity('High');
                    setActiveDropdown(null);
                    toast.info('深度思考模式 (推荐)');
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28; background: #f7f6f2;"
                >
                  High (深度)
                </div>
              </div>
            </Show>

            {/* 就绪指示灯 */}
            <span
              onClick={() => toast.success(`当前 Provider [${gatewayStore.activeProvider().name}] 就绪`)}
              style="width: 7px; height: 7px; border-radius: 50%; background: #16a34a; box-shadow: 0 0 6px rgba(22,163,74,0.4); cursor: pointer;"
              title="Agent 就绪"
            />
          </div>
        </div>
      </div>
    </div>
  );
};

export default Composer;
