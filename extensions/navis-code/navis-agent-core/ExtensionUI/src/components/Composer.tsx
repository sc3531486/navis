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

// ══════════════════════════════════════════════════════════════════════
// 1:1 像素级复刻参考图 1 的高清矢量与渐变应用图标
// ══════════════════════════════════════════════════════════════════════
const IconPaperclipSVG = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#18181b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink: 0;">
    <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48" />
  </svg>
);

const IconTargetSVG = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#18181b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink: 0;">
    <circle cx="12" cy="12" r="10" />
    <circle cx="12" cy="12" r="6" />
    <circle cx="12" cy="12" r="2" />
    <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
  </svg>
);

const IconBulbSVG = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#18181b" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink: 0;">
    <path d="M9 18h6" />
    <path d="M10 22h4" />
    <path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14" />
    <path d="M12 2v2M4.93 4.93l1.41 1.41M19.07 4.93l-1.41 1.41" />
  </svg>
);

const IconBrowserApp = () => (
  <div style="width: 17px; height: 17px; border-radius: 4px; background: #3b82f6; display: flex; flex-direction: column; overflow: hidden; flex-shrink: 0; box-shadow: 0 1px 2px rgba(59, 130, 246, 0.25);">
    <div style="height: 5px; background: #2563eb; display: flex; align-items: center; gap: 2px; padding-left: 2px;">
      <div style="width: 2px; height: 2px; border-radius: 50%; background: #ffffff; opacity: 0.9;" />
      <div style="width: 2px; height: 2px; border-radius: 50%; background: #ffffff; opacity: 0.9;" />
    </div>
    <div style="flex: 1; background: #f8fafc; margin: 1px; border-radius: 1px;" />
  </div>
);

const IconComputerApp = () => (
  <div style="width: 17px; height: 17px; border-radius: 4.5px; background: linear-gradient(135deg, #7dd3fc 0%, #c084fc 50%, #f472b6 100%); display: flex; align-items: center; justify-content: center; flex-shrink: 0; box-shadow: 0 1px 3px rgba(192, 132, 252, 0.3);">
    <svg width="10" height="10" viewBox="0 0 24 24" fill="#ffffff">
      <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
    </svg>
  </div>
);

const IconVisualizeApp = () => (
  <div style="width: 17px; height: 17px; border-radius: 4.5px; background: linear-gradient(135deg, #38bdf8 0%, #2563eb 100%); display: flex; align-items: center; justify-content: center; flex-shrink: 0; box-shadow: 0 1px 3px rgba(37, 99, 235, 0.3);">
    <svg width="9" height="9" viewBox="0 0 24 24" fill="#ffffff">
      <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z" />
    </svg>
  </div>
);

const IconHandSVG = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#475569" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink: 0;">
    <path d="M18 11V6a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v0M14 10V4a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v6M10 10.5V6a2 2 0 0 0-2-2v0a2 2 0 0 0-2 2v8M6 14a6 6 0 0 0 12 0v-3" />
  </svg>
);

const IconTrashSVG = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
    <line x1="10" y1="11" x2="10" y2="17"></line>
    <line x1="14" y1="11" x2="14" y2="17"></line>
  </svg>
);

const IconPlayCircleSVG = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <polygon points="10 8 16 12 10 16 10 8" fill="currentColor"></polygon>
  </svg>
);

const IconPauseCircleSVG = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="10"></circle>
    <line x1="10" y1="15" x2="10" y2="9"></line>
    <line x1="14" y1="15" x2="14" y2="9"></line>
  </svg>
);

const IconExpandCornersSVG = () => (
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"></path>
  </svg>
);

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

  const [activeSpecialMode, setActiveSpecialMode] = createSignal<'normal' | 'goal' | 'plan'>('normal');
  const [isHoveringModeChip, setIsHoveringModeChip] = createSignal(false);

  interface ActiveGoal {
    id: string;
    title: string;
    status: 'running' | 'paused' | 'stagnant';
    createdAt: number;
  }

  const [activeGoal, setActiveGoal] = createSignal<ActiveGoal | null>(null);
  const [hoverGoalBtn, setHoverGoalBtn] = createSignal<'trash' | 'play' | 'expand' | null>(null);
  const [nowTs, setNowTs] = createSignal(Date.now());

  onMount(() => {
    const timer = setInterval(() => setNowTs(Date.now()), 1000);
    onCleanup(() => clearInterval(timer));
  });

  const getGoalElapsed = () => {
    const g = activeGoal();
    if (!g) return '2s';
    const diffSec = Math.max(1, Math.floor((nowTs() - g.createdAt) / 1000));
    if (diffSec < 60) return `${diffSec}s`;
    const diffMin = Math.floor(diffSec / 60);
    return `${diffMin}m`;
  };

  const getGoalStatusText = () => {
    const g = activeGoal();
    if (!g) return '已暂停的目标';
    if (g.status === 'paused') return '已暂停的目标';
    if (g.status === 'stagnant') return '目标已停滞';
    return '进行中的目标';
  };

  const placeholderText = () => {
    if (activeSpecialMode() === 'goal') {
      return '描述你的目标，定义可衡量的成果，以获得最佳效果';
    }
    if (activeSpecialMode() === 'plan') {
      return '描述你的任务以生成套餐...';
    }
    return '随心输入、提问任何问题，使用 @ 提及，/ 执行操作';
  };

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

    // 检查是否配置了有效模型与 Provider
    const activeProvider = gatewayStore.activeProvider();
    const activeModel = gatewayStore.activeModel();
    const mode = activeSpecialMode();

    props.ctx.events.emit('agent:turn:start', {
      content,
      model: activeModel?.name || gatewayStore.activeModelId() || 'gemini-3.7-flash',
      modelId: gatewayStore.activeModelId(),
      provider: activeProvider?.name,
      permissionMode: permissionMode(),
      reasoning: reasoningIntensity(),
      mode: mode,
      timestamp: Date.now(),
    });

    if (mode === 'goal') {
      setActiveGoal({
        id: `g-${Date.now()}`,
        title: content,
        status: 'paused',
        createdAt: Date.now(),
      });
      setActiveSpecialMode('normal');
    }

    // 模拟增加本次交互的 Token 消耗
    setUsedTokens((prev) => prev + Math.floor(content.length * 1.5 + 400));
    setText('');
    toast.info(`已发送指令至 ${activeModel?.name || 'gemini-3.7-flash'}`);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (showSlashMenu() && filteredSlashCommands().length > 0) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedSlashIndex((prev) => (prev + 1) % filteredSlashCommands().length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedSlashIndex((prev) => (prev - 1 + filteredSlashCommands().length) % filteredSlashCommands().length);
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        e.preventDefault();
        const selected = filteredSlashCommands()[selectedSlashIndex()];
        if (selected) handleSelectSlash(selected);
        return;
      }
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  onMount(() => {
    const handleGlobalClick = () => {
      setShowModelPicker(false);
      setShowAddMenu(false);
      setShowPermMenu(false);
    };
    window.addEventListener('click', handleGlobalClick);

    const unsubGoalUpdate = props.ctx.events.on('goal:title:updated', (payload: { title: string }) => {
      const g = activeGoal();
      if (g) {
        setActiveGoal({ ...g, title: payload.title });
      }
    });

    onCleanup(() => {
      window.removeEventListener('click', handleGlobalClick);
      unsubGoalUpdate();
    });
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
    return '3.7 Flash';
  };

  return (
    <div
      style="width: 100%; max-width: 780px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 14px; box-shadow: 0 1px 6px rgba(0, 0, 0, 0.04); display: flex; flex-direction: column; position: relative;"
    >
      {/* 1. 点击 + 弹出的完整多功能卡片 (1:1 像素级对标参考图 1) */}
      <Show when={showAddMenu()}>
        <div
          onClick={(e) => e.stopPropagation()}
          style="position: absolute; left: 0; bottom: 100%; margin-bottom: 8px; width: 340px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.12); padding: 10px 12px; z-index: 130; display: flex; flex-direction: column; gap: 10px;"
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
              style="display: flex; align-items: center; gap: 10px; padding: 7px 10px; background: #f1f5f9; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b; font-weight: 500;"
            >
              <IconPaperclipSVG />
              <span>文件和文件夹</span>
            </div>

            {/* 目标 (1:1 对标参考图 2) */}
            <div
              id="menu-item-goal"
              onClick={() => {
                setActiveSpecialMode('goal');
                setShowAddMenu(false);
                toast.info('已开启目标设定模式 (Goal Mode)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <IconTargetSVG />
                <span>目标</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">设置要持续追求的目标</span>
            </div>

            {/* 计划模式 (1:1 对标参考图 3) */}
            <div
              id="menu-item-plan"
              onClick={() => {
                setActiveSpecialMode('plan');
                setShowAddMenu(false);
                toast.info('已开启计划模式 (Planning Mode)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <IconBulbSVG />
                <span>计划模式</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">开启计划模式</span>
            </div>
          </div>

          {/* 分组二：插件 */}
          <div style="display: flex; flex-direction: column; gap: 2px;">
            <div style="font-size: 11px; font-weight: 600; color: #94a3b8; padding: 2px 6px;">插件</div>

            {/* 浏览器 */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('已连接内置浏览器 (Control the in-app browser)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <IconBrowserApp />
                <span>浏览器</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">Control the in-app browser</span>
            </div>

            {/* 电脑 */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('Windows 宿主自动化已接入 (Control Windows apps)');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <IconComputerApp />
                <span>电脑</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">Control Windows apps</span>
            </div>

            {/* Visualize */}
            <div
              onClick={() => {
                setShowAddMenu(false);
                toast.info('Visualize 图表渲染引擎已就绪');
              }}
              style="display: flex; align-items: center; justify-content: space-between; padding: 7px 10px; border-radius: 8px; cursor: pointer; font-size: 13px; color: #18181b;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <div style="display: flex; align-items: center; gap: 10px;">
                <IconVisualizeApp />
                <span>Visualize</span>
              </div>
              <span style="font-size: 11.5px; color: #94a3b8;">Turn ideas and data</span>
            </div>
          </div>

          {/* 分组三：文件和聊天 */}
          <div style="display: flex; flex-direction: column; gap: 4px;">
            <div style="font-size: 11px; font-weight: 600; color: #94a3b8; padding: 2px 6px;">文件和聊天</div>
            <div style="padding: 7px 10px; font-size: 12.5px; color: #94a3b8; background: #f8fafc; border-radius: 6px; border: 1px dashed #e2e8f0; cursor: text;">
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
                  <span style="font-size: 12.5px; font-weight: 600; color: #2e2b26;">{cmd.name}</span>
                </div>
                <span style="font-size: 11px; color: #8e8b83;">{cmd.desc}</span>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* 活跃目标横幅 (1:1 像素级复刻参考图 1-5) */}
      <Show when={activeGoal()}>
        <div
          id="active-goal-banner"
          style="border-bottom: 1px solid #f1f5f9; background: #fafafa; border-radius: 13px 13px 0 0; padding: 8px 14px; display: flex; align-items: center; justify-content: space-between; gap: 10px;"
        >
          {/* 左侧：目标图标 + 状态文本 + 目标描述 + 耗时 */}
          <div style="display: flex; align-items: center; gap: 7px; overflow: hidden; flex: 1;">
            <div style="color: #94a3b8; display: flex; align-items: center; flex-shrink: 0;">
              <IconTargetSVG />
            </div>
            <span style="font-size: 13px; font-weight: 600; color: #18181b; white-space: nowrap; flex-shrink: 0;">
              {getGoalStatusText()}
            </span>
            <span
              style="font-size: 13px; color: #64748b; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
              title={activeGoal()?.title}
            >
              {activeGoal()?.title}
            </span>
            <span style="font-size: 12px; color: #94a3b8; white-space: nowrap; flex-shrink: 0;">
              • {getGoalElapsed()}
            </span>
          </div>

          {/* 右侧：操作按钮组 (清除目标 / 恢复或暂停目标 / 编辑目标) */}
          <div style="display: flex; align-items: center; gap: 6px; position: relative;">
            {/* 1. 清除目标按钮 */}
            <div style="position: relative;">
              <button
                id="goal-btn-trash"
                onClick={() => {
                  setActiveGoal(null);
                  toast.info('已清除目标');
                }}
                onMouseEnter={() => setHoverGoalBtn('trash')}
                onMouseLeave={() => setHoverGoalBtn(null)}
                style={`width: 26px; height: 26px; border: none; border-radius: 50%; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s ease; ${
                  hoverGoalBtn() === 'trash'
                    ? 'background: #f1f5f9; color: #18181b;'
                    : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconTrashSVG />
              </button>

              {/* 清除目标 Tooltip (图二) */}
              <Show when={hoverGoalBtn() === 'trash'}>
                <div
                  style="position: absolute; bottom: 100%; left: 50%; transform: translateX(-50%); margin-bottom: 6px; background: #18181b; color: #ffffff; padding: 4px 8px; border-radius: 6px; font-size: 11.5px; font-weight: 500; white-space: nowrap; z-index: 150; box-shadow: 0 4px 12px rgba(0,0,0,0.15); pointer-events: none;"
                >
                  清除目标
                </div>
              </Show>
            </div>

            {/* 2. 播放/暂停 状态切换按钮 */}
            <div style="position: relative;">
              <button
                id="goal-btn-play-pause"
                onClick={() => {
                  const g = activeGoal();
                  if (!g) return;
                  const newStatus = g.status === 'running' ? 'paused' : 'running';
                  setActiveGoal({ ...g, status: newStatus });
                  toast.info(newStatus === 'running' ? '已恢复目标运行' : '已暂停目标');
                }}
                onMouseEnter={() => setHoverGoalBtn('play')}
                onMouseLeave={() => setHoverGoalBtn(null)}
                style={`width: 26px; height: 26px; border: none; border-radius: 50%; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s ease; ${
                  hoverGoalBtn() === 'play'
                    ? 'background: #f1f5f9; color: #18181b;'
                    : 'background: transparent; color: #71717a;'
                }`}
              >
                <Show
                  when={activeGoal()?.status === 'running'}
                  fallback={<IconPlayCircleSVG />}
                >
                  <IconPauseCircleSVG />
                </Show>
              </button>

              {/* 恢复目标 / 暂停目标 Tooltip (图三/图四) */}
              <Show when={hoverGoalBtn() === 'play'}>
                <div
                  style="position: absolute; bottom: 100%; left: 50%; transform: translateX(-50%); margin-bottom: 6px; background: #18181b; color: #ffffff; padding: 4px 8px; border-radius: 6px; font-size: 11.5px; font-weight: 500; white-space: nowrap; z-index: 150; box-shadow: 0 4px 12px rgba(0,0,0,0.15); pointer-events: none;"
                >
                  {activeGoal()?.status === 'running' ? '暂停目标' : '恢复目标'}
                </div>
              </Show>
            </div>

            {/* 3. 编辑目标按钮 (唤起右侧目标编辑区域) */}
            <div style="position: relative;">
              <button
                id="goal-btn-expand"
                onClick={() => {
                  const g = activeGoal();
                  const targetTitle = g?.title || text() || '我们的目标是做一个万物皆扩展的底座';
                  props.ctx.events.emit('goal:editor:open', { title: targetTitle });
                  toast.info('已在右侧展开目标编辑区域');
                }}
                onMouseEnter={() => setHoverGoalBtn('expand')}
                onMouseLeave={() => setHoverGoalBtn(null)}
                style={`width: 26px; height: 26px; border: none; border-radius: 50%; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s ease; ${
                  hoverGoalBtn() === 'expand'
                    ? 'background: #f1f5f9; color: #18181b;'
                    : 'background: transparent; color: #71717a;'
                }`}
              >
                <IconExpandCornersSVG />
              </button>

              {/* 编辑目标 Tooltip (图五) */}
              <Show when={hoverGoalBtn() === 'expand'}>
                <div
                  style="position: absolute; bottom: 100%; left: 50%; transform: translateX(-50%); margin-bottom: 6px; background: #18181b; color: #ffffff; padding: 4px 8px; border-radius: 6px; font-size: 11.5px; font-weight: 500; white-space: nowrap; z-index: 150; box-shadow: 0 4px 12px rgba(0,0,0,0.15); pointer-events: none;"
                >
                  编辑目标
                </div>
              </Show>
            </div>
          </div>
        </div>
      </Show>

      {/* 主输入文本区域 (动态切换占位符) */}
      <div style="padding: 12px 14px 4px;">
        <textarea
          rows={3}
          value={text()}
          onInput={(e) => setText(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholderText()}
          style="width: 100%; border: none; outline: none; resize: none; font-size: 13.5px; line-height: 1.6; color: #18181b; background: transparent; font-family: inherit;"
        />
      </div>

      {/* 底部操作工具条 */}
      <div
        style="display: flex; align-items: center; justify-content: space-between; padding: 6px 10px 10px; border-top: 1px solid #f8fafc;"
      >
        {/* 左侧：+ 按钮 与 请求批准胶囊 与 特殊模式微标 (目标/计划) */}
        <div style="display: flex; align-items: center; gap: 8px;">
          {/* + 按钮 */}
          <div style="position: relative;">
            <button
              onClick={(e) => {
                e.stopPropagation();
                setShowAddMenu(!showAddMenu());
              }}
              onMouseEnter={() => setShowPlusTooltip(true)}
              onMouseLeave={() => setShowPlusTooltip(false)}
              style="width: 26px; height: 26px; border: 1px solid #e2e8f0; border-radius: 6px; background: #ffffff; color: #64748b; font-size: 16px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s ease;"
              title="添加文件等内容 @"
            >
              +
            </button>

            {/* Hover 提示词浮窗 */}
            <Show when={showPlusTooltip() && !showAddMenu()}>
              <div
                style="position: absolute; left: 0; bottom: 100%; margin-bottom: 6px; background: #0f172a; color: #ffffff; padding: 4px 8px; border-radius: 5px; font-size: 11px; white-space: nowrap; pointer-events: none; z-index: 140; box-shadow: 0 4px 12px rgba(0,0,0,0.15);"
              >
                添加文件等内容 @
              </div>
            </Show>
          </div>

          {/* 权限模式：请求批准 */}
          <div style="position: relative;">
            <div
              onClick={(e) => {
                e.stopPropagation();
                setShowPermMenu(!showPermMenu());
              }}
              style="display: flex; align-items: center; gap: 5px; font-size: 12.5px; color: #475569; cursor: pointer; padding: 3px 6px; border-radius: 6px;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              <IconHandSVG />
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

          {/* 目标模式微标 (1:1 像素级对齐参考图 3：悬停变 ⓧ 胶囊) */}
          <Show when={activeSpecialMode() === 'goal'}>
            <div style="display: flex; align-items: center; gap: 8px;">
              <div style="width: 1px; height: 13px; background: #e2e8f0;" />
              <div
                id="active-mode-chip-goal"
                onClick={() => {
                  setActiveSpecialMode('normal');
                  setIsHoveringModeChip(false);
                  toast.info('已退出目标模式');
                }}
                onMouseEnter={() => setIsHoveringModeChip(true)}
                onMouseLeave={() => setIsHoveringModeChip(false)}
                style={`display: flex; align-items: center; gap: 6px; font-size: 13px; cursor: pointer; padding: 3px 10px; border-radius: 9999px; transition: all 0.15s ease; user-select: none; ${
                  isHoveringModeChip()
                    ? 'background: #f1f5f9; color: #475569;'
                    : 'background: transparent; color: #334155;'
                }`}
                title="点击退出目标模式"
              >
                <Show
                  when={isHoveringModeChip()}
                  fallback={<IconTargetSVG />}
                >
                  <div style="width: 14px; height: 14px; border-radius: 50%; background: #64748b; color: #ffffff; display: flex; align-items: center; justify-content: center; font-size: 9px; font-weight: 700; line-height: 1;">
                    ✕
                  </div>
                </Show>
                <span>目标</span>
              </div>
            </div>
          </Show>

          {/* 计划模式微标 (1:1 像素级对齐参考图 3：悬停变 ⓧ 胶囊) */}
          <Show when={activeSpecialMode() === 'plan'}>
            <div style="display: flex; align-items: center; gap: 8px;">
              <div style="width: 1px; height: 13px; background: #e2e8f0;" />
              <div
                id="active-mode-chip-plan"
                onClick={() => {
                  setActiveSpecialMode('normal');
                  setIsHoveringModeChip(false);
                  toast.info('已退出计划模式');
                }}
                onMouseEnter={() => setIsHoveringModeChip(true)}
                onMouseLeave={() => setIsHoveringModeChip(false)}
                style={`display: flex; align-items: center; gap: 6px; font-size: 13px; cursor: pointer; padding: 3px 10px; border-radius: 9999px; transition: all 0.15s ease; user-select: none; ${
                  isHoveringModeChip()
                    ? 'background: #f1f5f9; color: #475569;'
                    : 'background: transparent; color: #334155;'
                }`}
                title="点击退出计划模式"
              >
                <Show
                  when={isHoveringModeChip()}
                  fallback={<IconBulbSVG />}
                >
                  <div style="width: 14px; height: 14px; border-radius: 50%; background: #64748b; color: #ffffff; display: flex; align-items: center; justify-content: center; font-size: 9px; font-weight: 700; line-height: 1;">
                    ✕
                  </div>
                </Show>
                <span>计划</span>
              </div>
            </div>
          </Show>
        </div>

        {/* 右侧：模型选择 (3.7 Flash 高) + 上下文使用率饼图 + 蓝色发送按钮 */}
        <div style="display: flex; align-items: center; gap: 10px; position: relative;">
          {/* 模型与思考强度胶囊 */}
          <div
            onClick={(e) => {
              e.stopPropagation();
              setShowModelPicker(!showModelPicker());
            }}
            style="display: flex; align-items: center; gap: 5px; font-size: 12px; color: #475569; cursor: pointer; padding: 3px 6px; border-radius: 6px;"
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
                  <span style="color: #38bdf8; font-weight: 600;">{usagePercentage()}% ({usedTokens().toLocaleString()} Tokens)</span>
                </div>
                <div style="display: flex; justify-content: space-between;">
                  <span style="color: #94a3b8;">剩余可用:</span>
                  <span style="color: #4ade80; font-weight: 600;">{remainingPercentage()}% ({remainingTokens().toLocaleString()} Tokens)</span>
                </div>
                <div style="display: flex; justify-content: space-between; border-top: 1px dashed #334155; padding-top: 4px; font-size: 10.5px; color: #64748b;">
                  <span>窗口总上限:</span>
                  <span>{contextCapacity().toLocaleString()} Tokens</span>
                </div>
              </div>
            </Show>
          </div>

          {/* 发送按钮 */}
          <button
            id="composer-send-btn"
            onClick={handleSend}
            disabled={!text().trim()}
            style={`width: 28px; height: 28px; border-radius: 50%; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.15s ease; ${
              text().trim()
                ? 'background: #0284c7; color: #ffffff; box-shadow: 0 2px 6px rgba(2,132,199,0.3);'
                : 'background: #f1f5f9; color: #94a3b8; cursor: not-allowed;'
            }`}
            title="发送消息 (Enter)"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
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
