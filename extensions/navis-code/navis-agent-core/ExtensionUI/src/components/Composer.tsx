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
  const [reasoningIntensity, setReasoningIntensity] = createSignal<'High' | 'Medium' | 'Low' | 'Off'>('High');
  const [permissionMode, setPermissionMode] = createSignal<'Bypass permissions' | 'Ask for confirmation' | 'Read-only'>('Bypass permissions');
  const [showModelPicker, setShowModelPicker] = createSignal(false);
  const [selectedSlashIndex, setSelectedSlashIndex] = createSignal(0);
  const [hasAttachment, setHasAttachment] = createSignal(true);

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
    setHasAttachment(false);
    toast.info(`已发送指令至 ${currentModel?.name || 'Agent'}`);
  };

  onMount(() => {
    const handleClickOutside = () => setShowModelPicker(false);
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  const activeModelName = () => {
    const m = gatewayStore.activeModel();
    if (m?.name) return m.name.replace(/^gemini-/, 'Gemini ').replace(/-flash$/, ' Flash');
    return 'Gemini 3.7 Flash';
  };

  return (
    <div
      style="width: 100%; max-width: 780px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 12px; box-shadow: 0 1px 6px rgba(0, 0, 0, 0.04); display: flex; flex-direction: column; position: relative;"
    >
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

      {/* 顶部附件微型缩略图 (完全对标 Antigravity 样式) */}
      <Show when={hasAttachment()}>
        <div style="padding: 8px 12px 2px; display: flex; align-items: center;">
          <div
            style="position: relative; width: 34px; height: 34px; background: #f8fafc; border: 1px solid #cbd5e1; border-radius: 6px; display: flex; flex-direction: column; justify-content: center; padding: 3px; gap: 2px; cursor: pointer;"
            title="查看附件 (media.png)"
          >
            <div style="height: 2px; background: #ea580c; border-radius: 1px; width: 60%;"></div>
            <div style="height: 2px; background: #94a3b8; border-radius: 1px; width: 85%;"></div>
            <div style="height: 2px; background: #94a3b8; border-radius: 1px; width: 75%;"></div>
            <div style="height: 2px; background: #0284c7; border-radius: 1px; width: 50%;"></div>
            <button
              onClick={() => setHasAttachment(false)}
              style="position: absolute; -top: 4px; -right: 4px; width: 12px; height: 12px; border-radius: 50%; background: #64748b; color: #ffffff; border: none; font-size: 9px; display: none; align-items: center; justify-content: center; cursor: pointer;"
            >
              ×
            </button>
          </div>
        </div>
      </Show>

      {/* 核心输入区域 */}
      <div style="display: flex; align-items: flex-end; padding: 4px 12px 2px;">
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
          placeholder="提问任何问题，使用 @ 提及，/ 执行操作"
          style="flex: 1; border: none; outline: none; background: transparent; font-size: 13px; line-height: 1.5; color: #1e293b; resize: none; min-height: 32px; max-height: 140px; font-family: inherit; padding: 0;"
        />
      </div>

      {/* 底部参数控制工具栏 */}
      <div style="display: flex; align-items: center; justify-content: space-between; padding: 4px 10px 8px; position: relative;">
        {/* 左侧：+ 按钮 + 模型与思考强度胶囊 */}
        <div style="display: flex; align-items: center; gap: 8px; position: relative;">
          {/* + 按钮 */}
          <button
            onClick={() => {
              setHasAttachment(true);
              toast.success('已附加图片/文件到上下文');
            }}
            style="background: transparent; border: none; color: #64748b; font-size: 14px; cursor: pointer; padding: 2px 4px; display: flex; align-items: center;"
            title="添加附件或上下文"
          >
            +
          </button>

          {/* 模型与思考强度选择胶囊 */}
          <div
            onClick={(e) => {
              e.stopPropagation();
              setShowModelPicker(!showModelPicker());
            }}
            style="display: flex; align-items: center; gap: 5px; font-size: 12px; color: #475569; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f8fafc')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            <span>{activeModelName()}</span>
            <span style="color: #64748b;">{reasoningIntensity()}</span>
            <span style="color: #94a3b8; font-size: 10px;">^</span>
          </div>

          {/* 模型与参数弹出层 */}
          <Show when={showModelPicker()}>
            <div
              onClick={(e) => e.stopPropagation()}
              style="position: absolute; left: 0; bottom: 100%; margin-bottom: 8px; width: 260px; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 10px; box-shadow: 0 10px 25px rgba(0,0,0,0.1); padding: 8px; z-index: 120; display: flex; flex-direction: column; gap: 8px;"
            >
              <div style="font-size: 11px; font-weight: 600; color: #64748b; padding: 2px 4px;">
                选择模型与思考等级
              </div>

              {/* 模型列表 */}
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

              {/* 思考等级分段切换 */}
              <div style="border-top: 1px solid #f1f5f9; padding-top: 6px; display: flex; flex-direction: column; gap: 4px;">
                <div style="font-size: 10.5px; color: #64748b; font-weight: 500;">思考等级 (Reasoning)</div>
                <div style="display: flex; background: #f1f5f9; border-radius: 6px; padding: 2px;">
                  <For each={['High', 'Medium', 'Low', 'Off'] as const}>
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
        </div>

        {/* 右侧：语音按钮 + 蓝色圆形发送按钮 */}
        <div style="display: flex; align-items: center; gap: 8px;">
          <button
            onClick={() => toast.info('语音输入就绪 (Mic Active)')}
            style="background: transparent; border: none; color: #64748b; cursor: pointer; padding: 2px; font-size: 14px; display: flex; align-items: center;"
            title="语音输入"
          >
            🎤
          </button>

          {/* 蓝色圆形发送按钮 (➔) */}
          <button
            onClick={handleSend}
            disabled={!text().trim()}
            style={`width: 26px; height: 26px; border-radius: 50%; border: none; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.15s ease; ${
              text().trim()
                ? 'background: #0284c7; color: #ffffff;'
                : 'background: #0284c7; color: #ffffff; opacity: 0.9;'
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
