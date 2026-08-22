import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';

interface SessionItem {
  id: string;
  title: string;
  status: 'needs_input' | 'running' | 'completed' | 'idle';
  timestamp: string;
}

interface ProjectGroup {
  id: string;
  name: string;
  sessions: SessionItem[];
}

export const SessionList: Component<{ ctx: NavisContext }> = (props) => {
  const [activeMode, setActiveMode] = createSignal<'cowork' | 'code'>('cowork');
  const [activeSessionId, setActiveSessionId] = createSignal<string>('s-1');
  const [userMenuOpen, setUserMenuOpen] = createSignal(false);
  const [projectMenuId, setProjectMenuId] = createSignal<string | null>(null);

  const [projectGroups, setProjectGroups] = createSignal<ProjectGroup[]>([
    {
      id: 'p-docs',
      name: '设计文档',
      sessions: [
        { id: 's-1', title: '流水设计审查', status: 'needs_input', timestamp: '昨天' },
        { id: 's-2', title: '流水设计文档编写', status: 'idle', timestamp: '3天前' },
      ],
    },
    {
      id: 'p-msg',
      name: 'message-center',
      sessions: [
        { id: 's-3', title: 'message-center架构文档', status: 'idle', timestamp: '4天前' },
        { id: 's-4', title: '压测', status: 'idle', timestamp: '上周' },
      ],
    },
    {
      id: 'p-novel',
      name: '小说',
      sessions: [
        { id: 's-5', title: '小说审查', status: 'idle', timestamp: '上周' },
        { id: 's-6', title: '项目初始化', status: 'completed', timestamp: '2周前' },
      ],
    },
    {
      id: 'p-gm',
      name: 'GM',
      sessions: [
        { id: 's-7', title: 'Project initialization', status: 'completed', timestamp: '2周前' },
        { id: 's-8', title: '基础版本交易梳理 -TE', status: 'idle', timestamp: '1个月前' },
        { id: 's-9', title: '基础版本交易梳理 -ECTIP', status: 'idle', timestamp: '1个月前' },
      ],
    },
    {
      id: 'p-workbee',
      name: 'workbee',
      sessions: [
        { id: 's-10', title: '分析00-21', status: 'idle', timestamp: '1个月前' },
        { id: 's-11', title: 'WorkBee产品设计审查', status: 'idle', timestamp: '1个月前' },
        { id: 's-12', title: '分析', status: 'idle', timestamp: '1个月前' },
      ],
    },
  ]);

  const handleModeChange = (mode: 'cowork' | 'code') => {
    setActiveMode(mode);
    props.ctx.events.emit('navis:mode:change', { mode });
    toast.info(`已切换至 ${mode === 'cowork' ? 'Cowork 协同模式' : 'Code 编码模式'}`);
  };

  const handleSelectSession = (session: SessionItem, groupName: string) => {
    setActiveSessionId(session.id);
    props.ctx.events.emit('session:selected', {
      session,
      groupName,
    });
    toast.info(`载入会话: ${session.title}`);
  };

  const handleNewSession = () => {
    const newId = `s-${Date.now()}`;
    const newSession: SessionItem = {
      id: newId,
      title: `新会话 #${projectGroups()[0].sessions.length + 1}`,
      status: 'idle',
      timestamp: '刚刚',
    };

    setProjectGroups((prev) => {
      const copy = [...prev];
      copy[0].sessions.unshift(newSession);
      return copy;
    });

    setActiveSessionId(newId);
    props.ctx.events.emit('session:created', newSession);
    toast.success('已新建会话，可以开始提问！');
  };

  const handleOpenCustomize = () => {
    props.ctx.events.emit('settings:open', { tab: 'prompt' });
  };

  // 全局点击关闭浮动菜单
  onMount(() => {
    const handleClickOutside = () => {
      setUserMenuOpen(false);
      setProjectMenuId(null);
    };
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  return (
    <div style="display: flex; flex-direction: column; height: 100%; width: 100%; user-select: none; position: relative;">
      {/* 顶部模式切换胶囊 */}
      <div style="padding: 12px 14px 8px;">
        <div style="display: flex; background: #eae8e1; padding: 2px; border-radius: 8px; gap: 2px;">
          <button
            onClick={() => handleModeChange('cowork')}
            style={`flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 5px 0; font-size: 12.5px; font-weight: 500; border-radius: 6px; border: none; cursor: pointer; transition: all 0.15s ease; ${
              activeMode() === 'cowork'
                ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.08); font-weight: 600;'
                : 'background: transparent; color: #76736c;'
            }`}
          >
            <span style="font-size: 13px; color: #c2410c;">✨</span>
            <span>Cowork</span>
          </button>
          <button
            onClick={() => handleModeChange('code')}
            style={`flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px; padding: 5px 0; font-size: 12.5px; font-weight: 500; border-radius: 6px; border: none; cursor: pointer; transition: all 0.15s ease; ${
              activeMode() === 'code'
                ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.08); font-weight: 600;'
                : 'background: transparent; color: #76736c;'
            }`}
          >
            <span style="font-size: 11px; opacity: 0.8;">&lt;/&gt;</span>
            <span>Code</span>
          </button>
        </div>
      </div>

      {/* 核心操作按钮 */}
      <div style="padding: 0 14px 10px; display: flex; flex-direction: column; gap: 6px;">
        <button
          onClick={handleNewSession}
          style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 12px; background: #eceae4; border: 1px solid #e2dfd7; border-radius: 8px; color: #2d2b28; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: all 0.1s ease;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#e3e1da')}
          onMouseLeave={(e) => (e.currentTarget.style.background = '#eceae4')}
        >
          <span style="font-size: 15px; font-weight: 600; line-height: 1;">+</span>
          <span>New</span>
        </button>

        <button
          onClick={handleOpenCustomize}
          style="display: flex; align-items: center; gap: 8px; width: 100%; padding: 7px 10px; background: transparent; border: none; border-radius: 7px; color: #2d2b28; font-size: 13px; font-weight: 500; cursor: pointer; text-align: left; transition: background 0.1s;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eceae4')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
        >
          <span style="font-size: 14px; opacity: 0.8;">🎛️</span>
          <span>Customize</span>
        </button>
      </div>

      {/* 项目与会话列表 (可滚动区域) */}
      <div style="flex: 1; overflow-y: auto; padding: 4px 10px 12px; display: flex; flex-direction: column; gap: 14px; min-height: 0;">
        <For each={projectGroups()}>
          {(group) => (
            <div style="display: flex; flex-direction: column; gap: 2px;">
              {/* 项目组标题 */}
              <div style="display: flex; align-items: center; justify-content: space-between; padding: 4px 6px; font-size: 11px; font-weight: 600; color: #918e87; letter-spacing: 0.2px; position: relative;">
                <span>{group.name}</span>
                <span
                  onClick={(e) => {
                    e.stopPropagation();
                    setProjectMenuId(projectMenuId() === group.id ? null : group.id);
                  }}
                  style="font-size: 11px; opacity: 0.7; cursor: pointer; padding: 2px 4px; border-radius: 4px;"
                >
                  ⋮
                </span>

                {/* 项目快捷菜单 */}
                <Show when={projectMenuId() === group.id}>
                  <div
                    onClick={(e) => e.stopPropagation()}
                    style="position: absolute; right: 0; top: 22px; width: 140px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.12); padding: 4px; z-index: 100; display: flex; flex-direction: column; gap: 2px;"
                  >
                    <button
                      onClick={() => {
                        setProjectMenuId(null);
                        handleNewSession();
                      }}
                      style="padding: 6px 8px; text-align: left; background: transparent; border: none; border-radius: 4px; font-size: 12px; color: #2d2b28; cursor: pointer;"
                    >
                      + 新建会话
                    </button>
                    <button
                      onClick={() => {
                        setProjectMenuId(null);
                        toast.info(`重命名项目: ${group.name}`);
                      }}
                      style="padding: 6px 8px; text-align: left; background: transparent; border: none; border-radius: 4px; font-size: 12px; color: #2d2b28; cursor: pointer;"
                    >
                      ✏️ 重命名
                    </button>
                  </div>
                </Show>
              </div>

              {/* 组内会话条目 */}
              <For each={group.sessions}>
                {(item) => (
                  <div
                    onClick={() => handleSelectSession(item, group.name)}
                    style={`display: flex; align-items: center; gap: 8px; padding: 5px 8px; border-radius: 6px; cursor: pointer; font-size: 12.5px; transition: all 0.1s ease; ${
                      activeSessionId() === item.id
                        ? 'background: #eceae4; color: #1e1d1b; font-weight: 500;'
                        : 'color: #4b4843;'
                    }`}
                    onMouseEnter={(e) => {
                      if (activeSessionId() !== item.id) e.currentTarget.style.background = '#f0eee8';
                    }}
                    onMouseLeave={(e) => {
                      if (activeSessionId() !== item.id) e.currentTarget.style.background = 'transparent';
                    }}
                  >
                    {/* 状态小圆点 */}
                    <span
                      style={`width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; ${
                        item.status === 'needs_input'
                          ? 'background: #d97706;'
                          : item.status === 'running'
                          ? 'background: #2563eb;'
                          : item.status === 'completed'
                          ? 'background: #16a34a;'
                          : 'border: 1.5px solid #a8a49c; background: transparent;'
                      }`}
                    />
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1;">
                      {item.title}
                    </span>
                  </div>
                )}
              </For>
            </div>
          )}
        </For>
      </div>

      {/* 底部用户信息栏 (常驻贴底) */}
      <div
        style="height: 44px; flex-shrink: 0; border-top: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; padding: 0 14px; font-size: 12px; color: #4b4843; position: relative;"
      >
        <div
          onClick={(e) => {
            e.stopPropagation();
            setUserMenuOpen(!userMenuOpen());
          }}
          style="display: flex; align-items: center; gap: 6px; cursor: pointer; padding: 4px 6px; border-radius: 4px; transition: background 0.1s;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eceae4')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
        >
          <span style="color: #c2410c; font-size: 14px; font-weight: bold;">✳</span>
          <span style="font-weight: 500; color: #1e1d1b;">super</span>
          <span style="color: #918e87; font-size: 11px;">· Gateway ∨</span>
        </div>

        {/* 用户与网关弹出菜单 */}
        <Show when={userMenuOpen()}>
          <div
            onClick={(e) => e.stopPropagation()}
            style="position: absolute; left: 10px; bottom: 48px; width: 220px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 10px; box-shadow: 0 8px 24px rgba(0,0,0,0.14); padding: 6px; z-index: 200; display: flex; flex-direction: column; gap: 2px;"
          >
            <div style="padding: 6px 8px; border-bottom: 1px solid #eae7e1; font-size: 11.5px; color: #76736c;">
              当前网关: <b style="color: #1e1d1b;">127.0.0.1:15721</b>
            </div>
            <button
              onClick={() => {
                setUserMenuOpen(false);
                props.ctx.events.emit('settings:open', { tab: 'gateway' });
              }}
              style="padding: 7px 8px; text-align: left; background: transparent; border: none; border-radius: 5px; font-size: 12.5px; color: #2d2b28; cursor: pointer;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              🌐 网关配置与状态
            </button>
            <button
              onClick={() => {
                setUserMenuOpen(false);
                props.ctx.events.emit('settings:open', { tab: 'keys' });
              }}
              style="padding: 7px 8px; text-align: left; background: transparent; border: none; border-radius: 5px; font-size: 12.5px; color: #2d2b28; cursor: pointer;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              🔑 切换 API 密钥
            </button>
            <button
              onClick={() => {
                setUserMenuOpen(false);
                toast.info('已清除本地登录会话');
              }}
              style="padding: 7px 8px; text-align: left; background: transparent; border: none; border-radius: 5px; font-size: 12.5px; color: #c2410c; cursor: pointer;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#fef2f2')}
              onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
            >
              🚪 注销登录
            </button>
          </div>
        </Show>

        <button
          onClick={() => props.ctx.events.emit('settings:open', { tab: 'general' })}
          style="background: transparent; border: none; font-size: 13px; color: #918e87; cursor: pointer; padding: 4px; border-radius: 4px; display: flex; align-items: center; justify-content: center;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#eceae4')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          title="打开全局设置"
        >
          ⚙
        </button>
      </div>
    </div>
  );
};

export default SessionList;
