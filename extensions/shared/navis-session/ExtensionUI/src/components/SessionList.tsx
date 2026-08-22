import { Component, createSignal, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconPlus,
  IconSettings,
  IconPrompt,
  IconFolder,
  IconChevronRight,
  IconSparkles,
  IconTrash,
} from '@/components/icons';

interface SessionItem {
  id: string;
  title: string;
  group: string;
  updatedAt: string;
  active?: boolean;
}

const STORAGE_KEY = 'navis_sessions_list_v2';

const DEFAULT_SESSIONS: SessionItem[] = [
  { id: '1', title: '流水设计审查', group: '设计文档', updatedAt: 'yesterday', active: true },
  { id: '2', title: '流水设计文档编写', group: '设计文档', updatedAt: '2d ago' },
  { id: '3', title: 'message-center架构文档', group: 'MESSAGE-CENTER', updatedAt: '3d ago' },
  { id: '4', title: '压测', group: 'MESSAGE-CENTER', updatedAt: '4d ago' },
  { id: '5', title: '小说审查', group: '小说', updatedAt: '5d ago' },
  { id: '6', title: '项目初始化', group: '小说', updatedAt: '6d ago' },
  { id: '7', title: '基础版本交易梳理 -TE', group: 'GM', updatedAt: '7d ago' },
  { id: '8', title: '基础版本交易梳理 -ECTIP', group: 'GM', updatedAt: '8d ago' },
  { id: '9', title: '分析00-21', group: 'WORKBEE', updatedAt: '9d ago' },
];

export const SessionList: Component<{ ctx: NavisContext }> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'cowork' | 'code'>('cowork');
  const [showGatewayMenu, setShowGatewayMenu] = createSignal(false);
  const [hoveredSessionId, setHoveredSessionId] = createSignal<string | null>(null);
  const [hoveredGroup, setHoveredGroup] = createSignal<string | null>(null);

  // 初始化会话列表（支持持久化存储）
  const getInitialSessions = (): SessionItem[] => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
    } catch (_) {}
    return DEFAULT_SESSIONS;
  };

  const [sessions, setSessionsState] = createSignal<SessionItem[]>(getInitialSessions());

  const setSessions = (updater: (prev: SessionItem[]) => SessionItem[]) => {
    setSessionsState((prev) => {
      const next = updater(prev);
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      } catch (_) {}
      return next;
    });
  };

  // 右键菜单与重命名弹窗状态
  const [contextMenu, setContextMenu] = createSignal<{
    type: 'session' | 'project';
    id?: string;
    group?: string;
    x: number;
    y: number;
  } | null>(null);

  const [editingItem, setEditingItem] = createSignal<{
    type: 'session' | 'project';
    id?: string;
    currentName: string;
    group?: string;
  } | null>(null);

  const [renameInput, setRenameInput] = createSignal('');

  // 新建项目弹窗
  const [showNewProjectModal, setShowNewProjectModal] = createSignal(false);
  const [newProjectName, setNewProjectName] = createSignal('');

  const handleTabChange = (mode: 'cowork' | 'code') => {
    setActiveTab(mode);
    props.ctx.events.emit('navis:mode:change', { mode });
    toast.info(`已切换至 ${mode === 'cowork' ? 'Cowork 协同模式' : 'Code 开发模式'}`);
  };

  /** 新建会话 */
  const handleNewSession = (targetGroup = '工作区') => {
    const newId = String(Date.now());
    const newTitle = `新会话 ${sessions().length + 1}`;
    const newSess: SessionItem = {
      id: newId,
      title: newTitle,
      group: targetGroup,
      updatedAt: 'just now',
      active: true,
    };
    setSessions((prev) => [newSess, ...prev.map((s) => ({ ...s, active: false }))]);
    props.ctx.events.emit('session:created', { id: newId, title: newTitle, group: targetGroup });
    props.ctx.events.emit('session:switched', { id: newId, title: newTitle, group: targetGroup });
    toast.success(`已新建会话: ${newTitle}`);
  };

  /** 选中切换会话 */
  const handleSelectSession = (id: string) => {
    setSessions((prev) =>
      prev.map((s) => ({
        ...s,
        active: s.id === id,
      })),
    );
    const target = sessions().find((s) => s.id === id);
    if (target) {
      props.ctx.events.emit('session:switched', { id: target.id, title: target.title, group: target.group });
      toast.info(`切换至会话: ${target.title}`);
    }
  };

  /** 删除单个会话 */
  const handleDeleteSession = (id: string, e?: MouseEvent) => {
    if (e) e.stopPropagation();
    const target = sessions().find((s) => s.id === id);
    if (!target) return;

    setSessions((prev) => {
      const remaining = prev.filter((s) => s.id !== id);
      if (target.active && remaining.length > 0) {
        remaining[0].active = true;
        props.ctx.events.emit('session:switched', {
          id: remaining[0].id,
          title: remaining[0].title,
          group: remaining[0].group,
        });
      }
      return remaining;
    });

    props.ctx.events.emit('session:deleted', { id: target.id, title: target.title });
    toast.success(`已删除会话: ${target.title}`);
    setContextMenu(null);
  };

  /** 删除整个项目及其所属的所有会话 */
  const handleDeleteProject = (groupName: string, e?: MouseEvent) => {
    if (e) e.stopPropagation();
    const toDelete = sessions().filter((s) => s.group === groupName);
    if (toDelete.length === 0) return;

    const hasActive = toDelete.some((s) => s.active);

    setSessions((prev) => {
      const remaining = prev.filter((s) => s.group !== groupName);
      if (hasActive && remaining.length > 0) {
        remaining[0].active = true;
        props.ctx.events.emit('session:switched', {
          id: remaining[0].id,
          title: remaining[0].title,
          group: remaining[0].group,
        });
      }
      return remaining;
    });

    props.ctx.events.emit('project:deleted', { group: groupName, deletedCount: toDelete.length });
    toast.success(`已删除项目「${groupName}」及其所属 ${toDelete.length} 个会话`);
    setContextMenu(null);
  };

  /** 确认重命名 */
  const handleConfirmRename = () => {
    const edit = editingItem();
    if (!edit || !renameInput().trim()) {
      setEditingItem(null);
      return;
    }
    const newName = renameInput().trim();

    if (edit.type === 'session' && edit.id) {
      setSessions((prev) =>
        prev.map((s) => (s.id === edit.id ? { ...s, title: newName } : s)),
      );
      toast.success(`会话已重命名为: ${newName}`);
    } else if (edit.type === 'project' && edit.group) {
      const oldGroup = edit.group;
      setSessions((prev) =>
        prev.map((s) => (s.group === oldGroup ? { ...s, group: newName } : s)),
      );
      toast.success(`项目已重命名为: ${newName}`);
    }
    setEditingItem(null);
  };

  /** 确认创建新项目 */
  const handleConfirmCreateProject = () => {
    const name = newProjectName().trim();
    if (!name) {
      setShowNewProjectModal(false);
      return;
    }
    handleNewSession(name);
    setShowNewProjectModal(false);
    setNewProjectName('');
  };

  onMount(() => {
    const handleClickOutside = () => {
      setShowGatewayMenu(false);
      setContextMenu(null);
    };
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  return (
    <div style="display: flex; flex-direction: column; height: 100%; min-height: 0; background: #f8f8f7; color: #2d2b28; font-size: 13px; position: relative;">
      {/* 顶部双模式切换胶囊 (Cowork vs Code) */}
      <div style="padding: 10px 12px 6px;">
        <div style="display: flex; background: #eae7e1; padding: 2px; border-radius: 8px;">
          <button
            onClick={() => handleTabChange('cowork')}
            style={`flex: 1; padding: 4px 0; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 4px; transition: all 0.1s ease; ${
              activeTab() === 'cowork'
                ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.06);'
                : 'background: transparent; color: #76736c;'
            }`}
          >
            <IconSparkles size={13} color="#ea580c" />
            <span>Cowork</span>
          </button>
          <button
            onClick={() => handleTabChange('code')}
            style={`flex: 1; padding: 4px 0; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; display: flex; align-items: center; justify-content: center; gap: 4px; transition: all 0.1s ease; ${
              activeTab() === 'code'
                ? 'background: #ffffff; color: #1e1d1b; box-shadow: 0 1px 3px rgba(0,0,0,0.06);'
                : 'background: transparent; color: #76736c;'
            }`}
          >
            <span>&lt;/&gt; Code</span>
          </button>
        </div>
      </div>

      {/* 新建会话与快捷操作 */}
      <div style="padding: 4px 12px 8px; display: flex; flex-direction: column; gap: 4px;">
        <button
          onClick={() => handleNewSession()}
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

      {/* 会话与项目分组列表区域 */}
      <div style="flex: 1; overflow-y: auto; padding: 0 8px 12px; display: flex; flex-direction: column; gap: 2px; overscroll-behavior: contain;">
        <For each={sessions()}>
          {(sess, index) => {
            const isFirstInGroup = () => index() === 0 || sessions()[index() - 1]?.group !== sess.group;
            return (
              <>
                {/* 项目/分组表头 (带删除项目与新建会话按钮) */}
                <Show when={isFirstInGroup()}>
                  <div
                    onMouseEnter={() => setHoveredGroup(sess.group)}
                    onMouseLeave={() => setHoveredGroup(null)}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      setContextMenu({ type: 'project', group: sess.group, x: e.clientX, y: e.clientY });
                    }}
                    style="padding: 10px 8px 3px; font-size: 11px; font-weight: 600; color: #8e8b83; text-transform: uppercase; letter-spacing: 0.3px; display: flex; align-items: center; justify-content: space-between; border-radius: 4px; transition: background 0.1s ease;"
                  >
                    <div style="display: flex; align-items: center; gap: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                      <IconFolder size={12} />
                      <span style="overflow: hidden; text-overflow: ellipsis;">{sess.group}</span>
                    </div>

                    {/* 项目悬停快捷操作：新建会话 + 删除项目 */}
                    <div
                      style={`display: flex; align-items: center; gap: 4px; opacity: ${
                        hoveredGroup() === sess.group ? 1 : 0
                      }; transition: opacity 0.15s ease;`}
                    >
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleNewSession(sess.group);
                        }}
                        style="background: transparent; border: none; padding: 2px; border-radius: 4px; color: #71717a; cursor: pointer; display: flex; align-items: center;"
                        title={`在「${sess.group}」下新建会话`}
                        onMouseEnter={(e) => (e.currentTarget.style.background = '#e4e4e7')}
                        onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                      >
                        <IconPlus size={12} />
                      </button>

                      <button
                        id={`delete-project-${sess.group}`}
                        onClick={(e) => handleDeleteProject(sess.group, e)}
                        style="background: transparent; border: none; padding: 2px; border-radius: 4px; color: #71717a; cursor: pointer; display: flex; align-items: center; transition: color 0.15s ease;"
                        title={`删除项目「${sess.group}」及其所有会话`}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.color = '#ef4444';
                          e.currentTarget.style.background = '#fee2e2';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.color = '#71717a';
                          e.currentTarget.style.background = 'transparent';
                        }}
                      >
                        <IconTrash size={12} />
                      </button>
                    </div>
                  </div>
                </Show>

                {/* 单个会话项 (带删除会话按钮与右键菜单) */}
                <div
                  id={`session-item-${sess.id}`}
                  onClick={() => handleSelectSession(sess.id)}
                  onMouseEnter={(e) => {
                    setHoveredSessionId(sess.id);
                    if (!sess.active) e.currentTarget.style.background = '#f0eee8';
                  }}
                  onMouseLeave={(e) => {
                    setHoveredSessionId(null);
                    if (!sess.active) e.currentTarget.style.background = 'transparent';
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setContextMenu({ type: 'session', id: sess.id, group: sess.group, x: e.clientX, y: e.clientY });
                  }}
                  style={`display: flex; align-items: center; justify-content: space-between; padding: 6px 8px; border-radius: 6px; cursor: pointer; transition: background 0.1s ease; position: relative; ${
                    sess.active ? 'background: #eae7e1; color: #1e1d1b; font-weight: 500;' : 'background: transparent; color: #4b4843;'
                  }`}
                >
                  <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; flex: 1;">
                    <span style={`width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; ${sess.active ? 'background: #c2410c;' : 'background: transparent;'}`} />
                    <span style="overflow: hidden; text-overflow: ellipsis; font-size: 12.5px;">{sess.title}</span>
                  </div>

                  {/* 悬停出现删除会话图标 / 激活态箭头 */}
                  <div style="display: flex; align-items: center; gap: 2px;">
                    <Show
                      when={hoveredSessionId() === sess.id}
                      fallback={
                        <Show when={sess.active}>
                          <IconChevronRight size={12} color="#8e8b83" />
                        </Show>
                      }
                    >
                      <button
                        id={`delete-session-${sess.id}`}
                        onClick={(e) => handleDeleteSession(sess.id, e)}
                        style="background: transparent; border: none; padding: 2px 4px; border-radius: 4px; color: #71717a; cursor: pointer; display: flex; align-items: center; transition: all 0.15s ease;"
                        title={`删除会话: ${sess.title}`}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.color = '#ef4444';
                          e.currentTarget.style.background = '#fee2e2';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.color = '#71717a';
                          e.currentTarget.style.background = 'transparent';
                        }}
                      >
                        <IconTrash size={12} />
                      </button>
                    </Show>
                  </div>
                </div>
              </>
            );
          }}
        </For>
      </div>

      {/* 右键上下文菜单 */}
      <Show when={contextMenu()}>
        {(menu) => (
          <div
            onClick={(e) => e.stopPropagation()}
            style={`position: fixed; top: ${menu().y}px; left: ${menu().x}px; width: 170px; background: #ffffff; border: 1px solid #e4e4e7; border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.12); padding: 4px; z-index: 1000; display: flex; flex-direction: column; gap: 2px; font-size: 12.5px;`}
          >
            <Show when={menu().type === 'session'}>
              <div
                onClick={() => {
                  const target = sessions().find((s) => s.id === menu().id);
                  if (target) {
                    setEditingItem({ type: 'session', id: target.id, currentName: target.title });
                    setRenameInput(target.title);
                  }
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 5px; cursor: pointer; color: #3f3f46; display: flex; align-items: center; gap: 6px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <span>✏️</span>
                <span>重命名会话</span>
              </div>
              <div
                onClick={() => menu().id && handleDeleteSession(menu().id!)}
                style="padding: 6px 10px; border-radius: 5px; cursor: pointer; color: #ef4444; display: flex; align-items: center; gap: 6px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#fee2e2')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <span>🗑️</span>
                <span>删除会话</span>
              </div>
            </Show>

            <Show when={menu().type === 'project'}>
              <div
                onClick={() => {
                  if (menu().group) handleNewSession(menu().group!);
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 5px; cursor: pointer; color: #3f3f46; display: flex; align-items: center; gap: 6px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <span>➕</span>
                <span>在此项目下新建会话</span>
              </div>
              <div
                onClick={() => {
                  if (menu().group) {
                    setEditingItem({ type: 'project', currentName: menu().group!, group: menu().group });
                    setRenameInput(menu().group!);
                  }
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 5px; cursor: pointer; color: #3f3f46; display: flex; align-items: center; gap: 6px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <span>✏️</span>
                <span>重命名项目</span>
              </div>
              <div
                onClick={() => menu().group && handleDeleteProject(menu().group!)}
                style="padding: 6px 10px; border-radius: 5px; cursor: pointer; color: #ef4444; display: flex; align-items: center; gap: 6px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#fee2e2')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <span>🗑️</span>
                <span>删除项目及所有会话</span>
              </div>
            </Show>
          </div>
        )}
      </Show>

      {/* 重命名模态框 */}
      <Show when={editingItem()}>
        <div
          onClick={() => setEditingItem(null)}
          style="position: fixed; inset: 0; background: rgba(0,0,0,0.3); backdrop-filter: blur(2px); z-index: 1000; display: flex; align-items: center; justify-content: center;"
        >
          <div
            onClick={(e) => e.stopPropagation()}
            style="width: 360px; background: #ffffff; border-radius: 12px; padding: 20px; box-shadow: 0 10px 30px rgba(0,0,0,0.15); display: flex; flex-direction: column; gap: 14px;"
          >
            <div style="font-size: 14px; font-weight: 600; color: #0f172a;">
              {editingItem()?.type === 'session' ? '重命名会话' : '重命名项目'}
            </div>
            <input
              type="text"
              value={renameInput()}
              onInput={(e) => setRenameInput(e.currentTarget.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleConfirmRename()}
              placeholder="请输入新名称..."
              autofocus
              style="width: 100%; padding: 8px 12px; border: 1px solid #cbd5e1; border-radius: 6px; font-size: 13px; outline: none;"
            />
            <div style="display: flex; justify-content: flex-end; gap: 8px;">
              <button
                onClick={() => setEditingItem(null)}
                style="padding: 6px 12px; border: 1px solid #e2e8f0; background: #ffffff; border-radius: 6px; font-size: 12.5px; color: #64748b; cursor: pointer;"
              >
                取消
              </button>
              <button
                onClick={handleConfirmRename}
                style="padding: 6px 16px; border: none; background: #0284c7; border-radius: 6px; font-size: 12.5px; font-weight: 500; color: #ffffff; cursor: pointer;"
              >
                保存
              </button>
            </div>
          </div>
        </div>
      </Show>

      {/* 侧边栏底部 Provider 齿轮入口 */}
      <div style="border-top: 1px solid #eae7e1; padding: 6px 10px; display: flex; align-items: center; position: relative;">
        <div
          id="sidebar-bottom-model-btn"
          onClick={(e) => {
            e.stopPropagation();
            props.ctx.events.emit('settings:open', { tab: 'models' });
          }}
          style="display: flex; align-items: center; gap: 7px; cursor: pointer; padding: 5px 10px; border-radius: 8px; transition: all 0.15s ease; user-select: none;"
          onMouseEnter={(e) => (e.currentTarget.style.background = '#e2e8f0')}
          onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          title="点击打开设置中心 (Settings)"
        >
          <IconSettings size={14} color="#18181b" />
          <span style="font-size: 13px; font-weight: 400; color: #18181b; max-width: 170px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
            {gatewayStore.activeProvider()?.name.split(' (')[0] || 'custom'}
          </span>
        </div>

        {/* 弹出 Provider 快速切换菜单 */}
        <Show when={showGatewayMenu()}>
          <div
            onClick={(e) => e.stopPropagation()}
            style="position: absolute; bottom: 44px; left: 10px; width: 230px; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 8px; box-shadow: 0 6px 20px rgba(0,0,0,0.12); padding: 6px; z-index: 100; display: flex; flex-direction: column; gap: 3px;"
          >
            <div style="font-size: 11px; font-weight: 600; color: #8e8b83; padding: 2px 6px;">
              切换 Provider (服务商)
            </div>
            <For each={gatewayStore.providers()}>
              {(p) => (
                <div
                  onClick={() => {
                    gatewayStore.setActiveProvider(p.id);
                    setShowGatewayMenu(false);
                    toast.info(`已切换当前 Provider 为: ${p.name.split(' (')[0]}`);
                  }}
                  style={`padding: 6px 8px; border-radius: 6px; font-size: 12.5px; cursor: pointer; display: flex; align-items: center; justify-content: space-between; ${
                    gatewayStore.activeProviderId() === p.id ? 'background: #f1f5f9; font-weight: 600; color: #0284c7;' : 'color: #334155;'
                  }`}
                  onMouseEnter={(e) => {
                    if (gatewayStore.activeProviderId() !== p.id) e.currentTarget.style.background = '#f8fafc';
                  }}
                  onMouseLeave={(e) => {
                    if (gatewayStore.activeProviderId() !== p.id) e.currentTarget.style.background = 'transparent';
                  }}
                >
                  <div style="display: flex; align-items: center; gap: 6px;">
                    <IconSettings size={13} />
                    <span>{p.name.split(' (')[0]}</span>
                  </div>
                  <Show when={gatewayStore.activeProviderId() === p.id}>
                    <span style="font-size: 10px; background: #dcfce7; color: #15803d; padding: 1px 5px; border-radius: 4px;">Active</span>
                  </Show>
                </div>
              )}
            </For>
            <div
              onClick={() => {
                setShowGatewayMenu(false);
                props.ctx.events.emit('settings:open', { tab: 'models' });
              }}
              style="border-top: 1px solid #f1f5f9; padding: 6px 8px; font-size: 12px; color: #ea580c; cursor: pointer; margin-top: 2px; display: flex; align-items: center; gap: 5px; font-weight: 500;"
            >
              <span>⚙️</span>
              <span>管理与添加 Provider...</span>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default SessionList;
