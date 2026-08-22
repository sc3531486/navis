import { Component, createSignal, Show, For, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconPlus,
  IconSparkles,
  IconPrompt,
  IconShield,
  IconCpu,
  IconActivity,
  IconDollarSign,
  IconPlug,
  IconTrash,
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
  const [reasoningIntensity, setReasoningIntensity] = createSignal<'高' | '中' | '低' | '关'>('高');
  const [permissionMode, setPermissionMode] = createSignal<'请求批准' | '直接执行' | '只读模式'>('请求批准');
  const [showModelPicker, setShowModelPicker] = createSignal(false);
  const [showAddMenu, setShowAddMenu] = createSignal(false);
  const [showPermMenu, setShowPermMenu] = createSignal(false);
  const [showContextTooltip, setShowContextTooltip] = createSignal(false);
  const [showPlusTooltip, setShowPlusTooltip] = createSignal(false);
  const [selectedSlashIndex, setSelectedSlashIndex] = createSignal(0);
  const [usedTokens, setUsedTokens] = createSignal(14820); // 响应式 Token 用量

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

    // 递增已用 Token
    setUsedTokens((prev) => prev + Math.round(content.length * 2.5) + 320);

    setText('');
    setShowAddMenu(false);
    toast.info(`已发送指令至 ${currentModel?.name || 'Agent'}`);
  };

  onMount(() => {
    const handleClickOutside = () => {
      setShowModelPicker(false);
      setShowAddMenu(false);
      setShowPermMenu(false);
    };
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  const activeModel = () => gatewayStore.activeModel();
  const contextCapacity = () => activeModel()?.contextWindow || 1000000;
  const usageRatio = () => Math.min(1, Math.max(0.005, usedTokens() / contextCapacity()));
  const usagePercentage = () => ((usedTokens() / contextCapacity()) * 100).toFixed(1);
  const remainingPercentage = () => (100 - Number(usagePercentage())).toFixed(1);
  const remainingTokens = () => Math.max(0, contextCapacity() - usedTokens());

  const activeModelDisplayName = () => {
    const m = activeModel();
    if (m?.name) return m.name.replace(/^gemini-/, '').replace(/-flash$/, ' Flash');
    return '自定义';
  };

  return (
    <div
      style="width: 100%; max-width: 780px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 14px; box-shadow: 0 1px 6px rgba(0, 0, 0, 0.04); display: flex; flex-direction: column; position: relative;"
    >
      {/* 1. 点击 + 弹出的完整多功能卡片 (1:1 像素级复刻参考图) */}
      <Show when={showAddMenu()}>
        <div
          onClick={(e) => e.stopPropagation()}
          style="position: absolute; left: 0; right: 0; bottom: 100%; margin-bottom: 8px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.12); padding: 10px 12px; z-index: 130; display: flex; flex-direction: column; gap: 12px;"
        >
          {/* 分组一：添加 */}
          <div style="display: flex; flex-direction: column; gap: 2px;">
            <div style="font-size: 11px; font-weight: 600; color: #94a3b8; padding: 2px 6px;">添加</div>

            {/* 文件和文件夹 (高亮选中项) */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                props.ctx.commands.execute('command:palette');
                toast.success('已唤起文件选择器');
              }}
              style="display: flex; align-items: center; gap: 10px; padding: 7px 10px; background: #f1f5f9; border-radius: 8px; cursor: pointer; font-size: 12.5px; color: #1e293b; font-weight: 500;"
            >
              <span>📎</span>
              <span>文件和文件夹</span>
            </div>

            {/* 在项目中使用 Work */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('已选择项目上下文: Navis Go');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 12.5px; color: #334155;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span>📁</span>
                <span>在项目中使用 Work</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">为新聊天选择项目</span>
            </div>

            {/* 目标 */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('目标设定面板已就绪');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 12.5px; color: #334155;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span>🎯</span>
                <span>目标</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">设置要持续追求的目标</span>
            </div>

            {/* 计划模式 */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.success('已开启计划模式 (Planning Mode)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 12.5px; color: #334155;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span>💡</span>
                <span>计划模式</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">开启计划模式</span>
            </div>
          </div>

          {/* 分组二：插件 */}
          <div style="display: flex; flex-direction: column; gap: 2px;">
            <div style="font-size: 11px; font-weight: 600; color: #94a3b8; padding: 2px 6px;">插件</div>
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('Windows 宿主自动化已接入 (Control Windows apps)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 12.5px; color: #334155;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <span>🖼️</span>
                <span>电脑</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">Control Windows apps</span>
            </div>
          </div>

          {/* 分组三：文件和聊天 */}
          <div style="display: flex; flex-direction: column; gap: 4px;">
            <div style="font-size: 11px; font-weight: 600; color: #94a3b8; padding: 2px 6px;">文件和聊天</div>
            <div style="padding: 6px 10px; font-size: 12px; color: #94a3b8; background: #f8fafc; border-radius: 6px; border: 1px dashed #e2e8f0;">
              输入内容以搜索文件或聊天
            </div>
          </div>
        </div>
      </Show>

      {/* Slash 命令浮动弹窗 */}
      <Show when={showSlashMenu() && filteredSlashCommands().length > 0}>
        <div
          style="position: absolute; left: 0; bottom: 100%; margin-bottom: 8px; width: 100%; max-width: 420px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 10px; box-shadow: 0 10px 30px rgba(0,0,0,0.12); padding: 6px; z-index: 120; display: flex; flex-direction: column; gap: 2px;"
        >
          <div style="padding: 4px 8px; font-size: 11px; font-weight: 600; color: #8e8b83; border-bottom: 1px solid #f4f2ee;">
            SLASH 指令 (快捷命令)
          </div>
          <For each={filteredSlashCommands()}>
            {(cmd, idx) => (
              <div
                onClick={() => handleSelectSlash(cmd)}
                style={`display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; border-radius: 6px; cursor: pointer; ${
                  selectedSlashIndex() === idx() ? 'background: #f4f2ee;' : 'background: transparent;'
                }`}
                onMouseEnter={() => setSelectedSlashIndex(idx())}
              >
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span style="color: #ea580c; display: flex; align-items: center;">
                    <cmd.iconComponent size={14} />
                  </span>
                  <span style="font-weight: 600; font-size: 12px; color: #1e1d1b;">{cmd.name}</span>
                </div>
                <span style="font-size: 11px; color: #8e8b83;">{cmd.desc}</span>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* 核心输入区域 */}
      <div style="display: flex; align-items: flex-end; padding: 8px 14px 4px;">
        <textarea
          rows={1}
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
          placeholder="随心输入、提问任何问题，使用 @ 提及，/ 执行操作"
          style="flex: 1; border: none; outline: none; background: transparent; font-size: 13px; line-height: 1.5; color: #1e293b; resize: none; min-height: 32px; max-height: 140px; font-family: inherit; padding: 0;"
        />
      </div>

      {/* 底部参数控制工具栏 */}
      <div style="display: flex; align-items: center; justify-content: space-between; padding: 4px 10px 8px; position: relative;">
        {/* 左侧：+ 按钮 (带黑色提示悬浮框) + 权限模式胶囊 */}
        <div style="display: flex; align-items: center; gap: 8px; position: relative;">
          {/* + 按钮 */}
          <div style="position: relative; display: flex; align-items: center;">
            <button
              onClick={(e) => {
                e.stopPropagation();
                setShowAddMenu(!showAddMenu());
              }}
              onMouseEnter={() => setShowPlusTooltip(true)}
              onMouseLeave={() => setShowPlusTooltip(false)}
              style="width: 24px; height: 24px; border-radius: 6px; border: 1px solid #e2e8f0; background: #ffffff; color: #475569; font-size: 15px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-weight: 300;"
            >
              +
            </button>

            {/* + 按钮 Hover 黑色提示浮窗 */}
            <Show when={showPlusTooltip() && !showAddMenu()}>
              <div
                style="position: absolute; left: 0; bottom: 100%; margin-bottom: 6px; background: #0f172a; color: #ffffff; font-size: 11px; padding: 4px 8px; border-radius: 6px; white-space: nowrap; pointer-events: none; z-index: 150; box-shadow: 0 4px 12px rgba(0,0,0,0.15);"
              >
                添加文件等内容 @
              </div>
            </Show>
          </div>

          {/* 权限模式选择胶囊 (e.g. ✋ 请求批准) */}
          <div style="position: relative;">
            <div
              onClick={(e) => {
                e.stopPropagation();
                setShowPermMenu(!showPermMenu());
              }}
              style="display: flex; align-items: center; gap: 4px; font-size: 12px; color: #475569; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <span>✋</span>
              <span>{permissionMode()}</span>
            </div>

            {/* 权限模式下拉菜单 */}
            <Show when={showPermMenu()}>
              <div
                onClick={(e) => e.stopPropagation()}
                style="position: absolute; left: 0; bottom: 100%; margin-bottom: 6px; width: 140px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.08); padding: 4px; z-index: 120; display: flex; flex-direction: column; gap: 2px;"
              >
                <For each={['请求批准', '直接执行', '只读模式'] as const}>
                  {(mode) => (
                    <div
                      onClick={() => {
                        setPermissionMode(mode);
                        setShowPermMenu(false);
                      }}
                      style={`padding: 5px 8px; border-radius: 4px; font-size: 12px; cursor: pointer; ${
                        permissionMode() === mode ? 'background: #f1f5f9; color: #0284c7; font-weight: 600;' : 'color: #334155;'
                      }`}
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
                      onMouseLeave={(e) =>
                        (e.currentTarget.style.background = permissionMode() === mode ? '#f1f5f9' : 'transparent')
                      }
                    >
                      {mode}
                    </div>
                  )}
                </For>
              </div>
            </Show>
          </div>
        </div>

        {/* 右侧：模型选择 (自定义 高) + 上下文使用率饼图 + 蓝色发送按钮 */}
        <div style="display: flex; align-items: center; gap: 10px; position: relative;">
          {/* 模型与思考强度胶囊 (自定义 高) */}
          <div
            onClick={(e) => {
              e.stopPropagation();
              setShowModelPicker(!showModelPicker());
            }}
            style="display: flex; align-items: center; gap: 5px; font-size: 12px; color: #475569; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            <span>{activeModelDisplayName()}</span>
            <span style="color: #64748b;">{reasoningIntensity()}</span>
          </div>

          {/* 模型弹出层 */}
          <Show when={showModelPicker()}>
            <div
              onClick={(e) => e.stopPropagation()}
              style="position: absolute; right: 80px; bottom: 100%; margin-bottom: 8px; width: 240px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 10px; box-shadow: 0 10px 25px rgba(0,0,0,0.1); padding: 8px; z-index: 120; display: flex; flex-direction: column; gap: 8px;"
            >
              <div style="font-size: 11px; font-weight: 600; color: #64748b; padding: 2px 4px;">
                选择模型与思考等级
              </div>

              <div style="display: flex; flex-direction: column; gap: 2px; max-height: 160px; overflow-y: auto;">
                <For each={gatewayStore.activeProvider()?.models || []}>
                  {(m) => (
                    <div
                      onClick={() => {
                        gatewayStore.setActiveModel(m.id);
                        setShowModelPicker(false);
                        toast.success(`已切换模型为: ${m.name}`);
                      }}
                      style={`display: flex; align-items: center; justify-content: space-between; padding: 5px 8px; border-radius: 6px; cursor: pointer; font-size: 12px; ${
                        gatewayStore.activeModelId() === m.id
                          ? 'background: #f1f5f9; color: #0284c7; font-weight: 600;'
                          : 'color: #334155;'
                      }`}
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
                      onMouseLeave={(e) =>
                        (e.currentTarget.style.background = gatewayStore.activeModelId() === m.id ? '#f1f5f9' : 'transparent')
                      }
                    >
                      <span>{m.name}</span>
                      <span style="font-size: 10.5px; color: #94a3b8;">{m.contextWindow / 1000}k</span>
                    </div>
                  )}
                </For>
              </div>

              <div style="border-top: 1px solid #f1f5f9; padding-top: 6px; display: flex; flex-direction: column; gap: 4px;">
                <div style="font-size: 10.5px; color: #64748b; font-weight: 500;">思考等级 (Reasoning)</div>
                <div style="display: flex; background: #f1f5f9; border-radius: 6px; padding: 2px;">
                  <For each={['高', '中', '低', '关'] as const}>
                    {(level) => (
                      <button
                        onClick={() => setReasoningIntensity(level)}
                        style={`flex: 1; border: none; border-radius: 4px; padding: 3px 0; font-size: 11px; cursor: pointer; ${
                          reasoningIntensity() === level
                            ? 'background: #ffffff; color: #0f172a; font-weight: 600; box-shadow: 0 1px 2px rgba(0,0,0,0.05);'
                            : 'background: transparent; color: #64748b;'
                        }`}
                      >
                        {level}
                      </button>
                    )}
                  </For>
                </div>
              </div>
            </div>
          </Show>

          {/* 4. 上下文使用率饼图 (SVG 环形饼图 + 鼠标 Hover 浮窗) */}
          <div
            id="context-pie-btn"
            style="position: relative; display: flex; align-items: center; cursor: pointer; padding: 2px;"
            onMouseEnter={() => setShowContextTooltip(true)}
            onMouseLeave={() => setShowContextTooltip(false)}
          >
            <svg width="18" height="18" viewBox="0 0 36 36" style="transform: rotate(-90deg);">
              {/* 背景底圈 (剩余容量) */}
              <circle
                cx="18"
                cy="18"
                r="14"
                fill="none"
                stroke="#e2e8f0"
                stroke-width="4.5"
              />
              {/* 前景进度弧 (已使用比例) */}
              <circle
                cx="18"
                cy="18"
                r="14"
                fill="none"
                stroke="#0284c7"
                stroke-width="4.5"
                stroke-dasharray={`${usageRatio() * 88} 88`}
                stroke-linecap="round"
              />
            </svg>

            {/* 上下文使用率详细浮窗 */}
            <Show when={showContextTooltip()}>
              <div
                style="position: absolute; right: 0; bottom: 100%; margin-bottom: 8px; width: 220px; background: #0f172a; color: #ffffff; border-radius: 8px; padding: 10px 12px; font-size: 11.5px; z-index: 150; box-shadow: 0 10px 25px rgba(0,0,0,0.25); display: flex; flex-direction: column; gap: 6px; pointer-events: none;"
              >
                <div style="font-weight: 600; border-bottom: 1px solid #334155; padding-bottom: 4px; color: #38bdf8;">
                  上下文窗口使用率
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: #94a3b8;">已使用:</span>
                  <span style="font-weight: 500;">{usagePercentage()}% ({usedTokens().toLocaleString()} Tokens)</span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: #94a3b8;">剩余可用:</span>
                  <span style="color: #4ade80;">{remainingPercentage()}% ({remainingTokens().toLocaleString()} Tokens)</span>
                </div>
                <div style="display: flex; justify-content: space-between; border-top: 1px dashed #334155; padding-top: 4px;">
                  <span style="color: #94a3b8;">窗口总上限:</span>
                  <span style="color: #cbd5e1;">{contextCapacity().toLocaleString()} Tokens</span>
                </div>
              </div>
            </Show>
          </div>

          {/* 蓝色圆形发送按钮 (➔) */}
          <button
            onClick={handleSend}
            disabled={!text().trim()}
            style={`width: 26px; height: 26px; border-radius: 50%; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.15s ease; ${
              text().trim()
                ? 'background: #0284c7; color: #ffffff;'
                : 'background: #64748b; color: #ffffff; opacity: 0.8;'
            }`}
            title="发送消息 (Enter)"
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="12" y1="19" x2="12" y2="5"></line>
              <polyline points="5 12 12 5 19 12"></polyline>
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
};

export default Composer;
