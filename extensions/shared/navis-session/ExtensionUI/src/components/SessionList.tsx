import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconPlus,
  IconCpu,
  IconSettings,
  IconPrompt,
  IconFolder,
  IconChevronRight,
  IconSparkles,
} from '@/components/icons';

interface SessionItem {
  id: string;
  title: string;
  group: string;
  updatedAt: string;
  active?: boolean;
}

export const SessionList: Component<{ ctx: NavisContext }> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'cowork' | 'code'>('cowork');
  const [showGatewayMenu, setShowGatewayMenu] = createSignal(false);
  const [sessions, setSessions] = createSignal<SessionItem[]>([
    { id: '1', title: '流水设计审查', group: '设计文档', updatedAt: 'yesterday', active: true },
    { id: '2', title: '流水设计文档编写', group: '设计文档', updatedAt: '2d ago' },
    { id: '3', title: 'message-center架构文档', group: 'message-center', updatedAt: '3d ago' },
    { id: '4', title: '压测', group: 'message-center', updatedAt: '4d ago' },
    { id: '5', title: '小说审查', group: '小说', updatedAt: '5d ago' },
    { id: '6', title: '项目初始化', group: '小说', updatedAt: '6d ago' },
    { id: '7', title: '基础版本交易梳理 -TE', group: 'GM', updatedAt: '7d ago' },
    { id: '8', title: '基础版本交易梳理 -ECTIP', group: 'GM', updatedAt: '8d ago' },
    { id: '9', title: '分析00-21', group: 'workbee', updatedAt: '9d ago' },
  ]);

  const handleTabChange = (mode: 'cowork' | 'code') => {
    setActiveTab(mode);
    props.ctx.events.emit('navis:mode:change', { mode });
    toast.info(`已切换至 ${mode === 'cowork' ? 'Cowork 协同模式' : 'Code 开发模式'}`);
  };

  const handleNewSession = () => {
    const newId = String(Date.now());
    const newTitle = `新会话 ${sessions().length + 1}`;
    const newSess: SessionItem = {
      id: newId,
      title: newTitle,
      group: '工作区',
      updatedAt: 'just now',
      active: true,
    };
    setSessions((prev) => [newSess, ...prev.map((s) => ({ ...s, active: false }))]);
    props.ctx.events.emit('session:created', { id: newId, title: newTitle });
    toast.success('已新建会话');
  };

  const handleSelectSession = (id: string) => {
    setSessions((prev) =>
      prev.map((s) => ({
        ...s,
        active: s.id === id,
      })),
    );
    const target = sessions().find((s) => s.id === id);
    if (target) {
      props.ctx.events.emit('session:switched', { id: target.id, title: target.title });
      toast.info(`切换至会话: ${target.title}`);
    }
  };

  onMount(() => {
    const handleClickOutside = () => setShowGatewayMenu(false);
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  return (
    <div style="display: flex; flex-direction: column; height: 100%; min-height: 0; background: #f8f8f7; color: #2d2b28; font-size: 13px;">
      {/* 顶部双模式切换胶囊 (Cowork vs Code) */}
      <div style="padding: 10px 12px 6px;">
        <div style="display: flex; background: #eae7e1; padding: 2px; border-radius: 8px;">
          <button
            onClick={() => handleTabChange('cowork')}
            style={`flex: 1; padding: 4px 0; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 4px; transition: all 0.1s ease; ${
              activeTab() === 'cowork' ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.06);' : 'background: transparent; color: #76736c;'
            }`}
          >
            <IconSparkles size={13} color="#ea580c" />
            <span>Cowork</span>
          </button>
          <button
            onClick={() => handleTabChange('code')}
            style={`flex: 1; padding: 4px 0; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 4px; transition: all 0.1s ease; ${
              activeTab() === 'code' ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.06);' : 'background: transparent; color: #76736c;'
            }`}
          >
            <span>&lt;/&gt; Code</span>
          </button>
        </div>
      </div>

      {/* 新建会话与快捷操作 */}
      <div style="padding: 4px 12px 8px; display: flex; flex-direction: column; gap: 4px;">
        <button
          onClick={handleNewSession}
          style="width: 100%; display: flex; align-items: center; gap: 6px; padding: 6px 10px; background: transparent; border: 1px solid #e7e4dc; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #2d2b28; cursor: pointer; transition: background 0.1s ease;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eeebe4')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
        >
          <IconPlus size={14} />
          <span>New Session</span>
        </button>

        <button
          onClick={() => props.ctx.events.emit('settings:open', { tab: 'prompt' })}
          style="width: 100%; display: flex; align-items: center; gap: 6px; padding: 6px 10px; background: transparent; border: none; border-radius: 6px; font-size: 12.5px; color: #5a5750; cursor: pointer; transition: background 0.1s ease;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eeebe4')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
        >
          <IconPrompt size={14} />
          <span>Customize Prompt</span>
        </button>
      </div>

      {/* 会话列表区域 */}
      <div style="flex: 1; overflow-y: auto; padding: 0 8px 12px; display: flex; flex-direction: column; gap: 2px;">
        <For each={sessions()}>
          {(sess, index) => {
            const isFirstInGroup = () => index() === 0 || sessions()[index() - 1]?.group !== sess.group;
            return (
              <>
                <Show when={isFirstInGroup()}>
                  <div style="padding: 10px 8px 3px; font-size: 11px; font-weight: 600; color: #8e8b83; text-transform: uppercase; letter-spacing: 0.3px; display: flex; align-items: center; gap: 4px;">
                    <IconFolder size={12} />
                    <span>{sess.group}</span>
                  </div>
                </Show>
                <div
                  onClick={() => handleSelectSession(sess.id)}
                  style={`display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; border-radius: 6px; cursor: pointer; transition: background 0.1s ease; ${
                    sess.active ? 'background: #eae7e1; color: #1e1d1b; font-weight: 500;' : 'background: transparent; color: #4b4843;'
                  }`}
                  onMouseEnter={(e) => {
                    if (!sess.active) e.currentTarget.style.background = '#f0eee8';
                  }}
                  onMouseLeave={(e) => {
                    if (!sess.active) e.currentTarget.style.background = 'transparent';
                  }}
                >
                  <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis;">
                    <span style={`width: 6px; height: 6px; border-radius: 50%; ${sess.active ? 'background: #c2410c;' : 'background: transparent;'}`} />
                    <span style="overflow: hidden; text-overflow: ellipsis; font-size: 12.5px;">{sess.title}</span>
                  </div>
                  <Show when={sess.active}>
                    <IconChevronRight size={12} color="#8e8b83" />
                  </Show>
                </div>
              </>
            );
          }}
        </For>
      </div>

      {/* 侧边栏底部模型状态与设置入口 */}
      <div style="border-top: 1px solid #eae7e1; padding: 8px 12px; display: flex; align-items: center; justify-content: space-between; position: relative;">
        <div
          onClick={(e) => {
            e.stopPropagation();
            setShowGatewayMenu(!showGatewayMenu());
          }}
          style="display: flex; align-items: center; gap: 6px; cursor: pointer; padding: 4px 6px; border-radius: 4px;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eae7e1')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          title="模型与网关状态"
        >
          <IconCpu size={14} color="#ea580c" />
          <span style="font-size: 11.5px; font-weight: 500; color: #4b4843; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
            {gatewayStore.activeModel()?.name || gatewayStore.activeModelId()}
          </span>
        </div>

        <button
          onClick={() => props.ctx.events.emit('settings:open', { tab: 'models' })}
          style="background: transparent; border: none; font-size: 13px; color: #76736c; cursor: pointer; padding: 4px 6px; border-radius: 4px; display: flex; align-items: center;"
          title="打开设置中心"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eae7e1')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
        >
          <IconSettings size={15} />
        </button>

        {/* 弹出菜单 */}
        <Show when={showGatewayMenu()}>
          <div
            onClick={(e) => e.stopPropagation()}
            style="position: absolute; bottom: 44px; left: 10px; width: 220px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 6px; z-index: 100; display: flex; flex-direction: column; gap: 4px;"
          >
            <div style="font-size: 11px; font-weight: 600; color: #8e8b83; padding: 2px 6px;">
              当前 Provider: {gatewayStore.activeProvider()?.name}
            </div>
            <For each={gatewayStore.activeProvider()?.models}>
              {(m) => (
                <div
                  onClick={() => {
                    gatewayStore.setActiveModel(m.id);
                    setShowGatewayMenu(false);
                    toast.info(`已切换默认模型为: ${m.name}`);
                  }}
                  style={`padding: 5px 8px; border-radius: 4px; font-size: 12px; cursor: pointer; display: flex; align-items: center; justify-content: space-between; ${
                    gatewayStore.activeModelId() === m.id ? 'background: #f7f6f2; font-weight: 600;' : ''
                  }`}
                  onMouseEnter={(e) => (e.currentTarget.style.background = '#f7f6f2')}
                  onMouseLeave={(e) => {
                    if (gatewayStore.activeModelId() !== m.id) e.currentTarget.style.background = 'transparent';
                  }}
                >
                  <span>{m.name}</span>
                  <Show when={gatewayStore.activeModelId() === m.id}>
                    <span style="font-size: 10px; color: #16a34a;">●</span>
                  </Show>
                </div>
              )}
            </For>
            <div
              onClick={() => {
                setShowGatewayMenu(false);
                props.ctx.events.emit('settings:open', { tab: 'models' });
              }}
              style="border-top: 1px solid #f4f2ee; padding: 5px 8px; font-size: 11.5px; color: #c2410c; cursor: pointer; margin-top: 2px;"
            >
              配置更多 Provider...
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default SessionList;
