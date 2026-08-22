import { Component, createSignal, Show, For, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';

export const Composer: Component<{ ctx: NavisContext }> = (props) => {
  const [text, setText] = createSignal('');
  const [permissionMode, setPermissionMode] = createSignal<'Bypass permissions' | 'Ask for confirmation' | 'Read-only'>('Bypass permissions');
  const [reasoningIntensity, setReasoningIntensity] = createSignal<'Off' | 'Low' | 'Medium' | 'High'>('High');
  const [activeDropdown, setActiveDropdown] = createSignal<'perm' | 'plus' | 'model' | 'reason' | 'context' | null>(null);

  const handleSend = () => {
    const content = text().trim();
    if (!content) return;

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
        {/* 吉祥物 Mascot */}
        <div
          onClick={() => toast.info('Navis Agent 在线待命 🦀')}
          style="position: absolute; right: 20px; top: -14px; z-index: 2; cursor: pointer; filter: drop-shadow(0 2px 4px rgba(0,0,0,0.1));"
          title="Navis Mascot"
        >
          <span style="font-size: 22px; display: block; transform: rotate(10deg);">🦀</span>
        </div>

        {/* 顶部上下文药丸胶囊 */}
        <div style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px 4px; border-bottom: 1px solid #f4f2ee;">
          <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
            <div
              onClick={(e) => {
                e.stopPropagation();
                setActiveDropdown(activeDropdown() === 'context' ? null : 'context');
              }}
              style="display: flex; align-items: center; gap: 4px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750; cursor: pointer;"
            >
              <span>💻</span>
              <span style="font-weight: 500;">Local</span>
            </div>
            <div
              style="display: flex; align-items: center; gap: 4px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750;"
            >
              <span>📁</span>
              <span style="font-weight: 500;">Navis Go</span>
            </div>
            <div
              style="display: flex; align-items: center; gap: 4px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750;"
            >
              <span style="font-size: 10px;"></span>
              <span style="font-weight: 500;">main</span>
            </div>
            <div
              style="display: flex; align-items: center; gap: 4px; background: #f7f6f2; border: 1px solid #eae7e1; padding: 2px 8px; border-radius: 6px; font-size: 11.5px; color: #5a5750;"
            >
              <span style="font-size: 11px;">☐</span>
              <span>worktree</span>
            </div>
            <button
              onClick={handleCopyContext}
              style="background: transparent; border: none; font-size: 12px; color: #8e8b83; cursor: pointer; padding: 2px 4px; border-radius: 4px;"
              title="复制上下文路径"
            >
              📋
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
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Describe a task or ask a question (Enter 发送, Shift+Enter 换行)"
            style="flex: 1; border: none; outline: none; background: transparent; font-size: 13.5px; line-height: 1.5; color: #2d2b28; resize: none; min-height: 48px; max-height: 160px; font-family: inherit;"
          />
          <button
            onClick={handleSend}
            disabled={!text().trim()}
            style={`width: 28px; height: 28px; border-radius: 6px; border: none; display: flex; align-items: center; justify-content: center; font-size: 13px; cursor: pointer; transition: all 0.1s ease; margin-bottom: 2px; ${
              text().trim() ? 'background: #2d2b28; color: #ffffff;' : 'background: #f0eee8; color: #b5b2aa; cursor: not-allowed;'
            }`}
            title="发送消息 (Enter)"
          >
            ↵
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
              <span style="font-size: 10px; opacity: 0.7;">▾</span>
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
              style="background: transparent; border: none; font-size: 14px; color: #76736c; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
              title="添加上下文文件或资源"
            >
              +
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
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28;"
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  📎 添加工作区文件...
                </div>
                <div
                  onClick={() => {
                    setActiveDropdown(null);
                    props.ctx.events.emit('settings:open', { tab: 'prompt' });
                  }}
                  style="padding: 6px 8px; border-radius: 4px; cursor: pointer; font-size: 12px; color: #2d2b28;"
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                >
                  📝 插入自定义 Prompt...
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
              <span style="font-size: 10px; opacity: 0.7;">▾</span>
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
              <span style="font-size: 10px; opacity: 0.7;">▾</span>
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
