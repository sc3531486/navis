import { Component, createSignal, createMemo, For, Show, onMount, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { gatewayStore } from '@extensions/shared/navis-ai-platform/ExtensionUI/src/store/GatewayStore';
import {
  IconPlus,
  IconSettings,
  IconPrompt,
  IconChevronRight,
  IconChevronDown,
  IconSparkles,
  IconTrash,
} from '@/components/icons';

// ══════════════════════════════════════════════════════════════════════════
// 1:1 对齐图二、图三的精致纯矢量线性图标 (Monochrome Linear SVG Icons)
// ══════════════════════════════════════════════════════════════════════════
const IconPin = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <line x1="12" y1="17" x2="12" y2="22"></line>
    <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
  </svg>
);

const IconEditLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M12 20h9"></path>
    <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path>
  </svg>
);

const IconFolderLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
  </svg>
);

const IconArchiveLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <rect x="2" y="3" width="20" height="5" rx="1"></rect>
    <path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"></path>
    <path d="M10 12h4"></path>
  </svg>
);

const IconRemoveLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <line x1="18" y1="6" x2="6" y2="18"></line>
    <line x1="6" y1="6" x2="18" y2="18"></line>
  </svg>
);

const IconEyeLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
    <circle cx="12" cy="12" r="3"></circle>
  </svg>
);

const IconShareLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path>
    <polyline points="16 6 12 2 8 6"></polyline>
    <line x1="12" y1="2" x2="12" y2="15"></line>
  </svg>
);

const IconCopyLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
  </svg>
);

const IconOpenNewWindowLinear = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
    <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
    <polyline points="15 3 21 3 21 9"></polyline>
    <line x1="10" y1="14" x2="21" y2="3"></line>
  </svg>
);

interface SessionItem {
  id: string;
  title: string;
  group: string;
  updatedAt: string;
  active?: boolean;
}

const STORAGE_KEY = 'navis_sessions_list_v2';
const COLLAPSED_STORAGE_KEY = 'navis_collapsed_projects_v1';

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

  // 折叠/展开的项目分组集合
  const getInitialCollapsed = (): Record<string, boolean> => {
    try {
      const saved = localStorage.getItem(COLLAPSED_STORAGE_KEY);
      if (saved) return JSON.parse(saved);
    } catch (_) {}
    return {};
  };
  const [collapsedGroups, setCollapsedGroupsState] = createSignal<Record<string, boolean>>(getInitialCollapsed());

  const toggleGroupCollapse = (group: string, e?: MouseEvent) => {
    if (e) e.stopPropagation();
    setCollapsedGroupsState((prev) => {
      const next = { ...prev, [group]: !prev[group] };
      try {
        localStorage.setItem(COLLAPSED_STORAGE_KEY, JSON.stringify(next));
      } catch (_) {}
      return next;
    });
  };

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

  // 结构化聚合 Project 分组，保证同一项目分组的所有会话严格归并在一起，彻底解决新建会话产生重复 Project 的 Bug
  const groupedProjects = createMemo(() => {
    const map = new Map<string, SessionItem[]>();
    for (const s of sessions()) {
      const list = map.get(s.group) || [];
      list.push(s);
      map.set(s.group, list);
    }
    return Array.from(map.entries()).map(([groupName, items]) => ({
      name: groupName,
      sessions: items,
    }));
  });

  // 右键菜单状态
  const [contextMenu, setContextMenu] = createSignal<{
    type: 'session' | 'project';
    id?: string;
    group?: string;
    x: number;
    y: number;
  } | null>(null);

  // 重命名状态
  const [editingItem, setEditingItem] = createSignal<{
    type: 'session' | 'project';
    id?: string;
    currentName: string;
    group?: string;
  } | null>(null);
  const [renameInput, setRenameInput] = createSignal('');

  const handleTabChange = (mode: 'cowork' | 'code') => {
    setActiveTab(mode);
    props.ctx.events.emit('navis:mode:change', { mode });
    toast.info(`已切换至 ${mode === 'cowork' ? 'Cowork 协同模式' : 'Code 开发模式'}`);
  };

  /** 新建会话：插入到指定 project 内部，绝对不产生重复 project */
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
    // 确保所属分组展开
    setCollapsedGroupsState((prev) => ({ ...prev, [targetGroup]: false }));

    setSessions((prev) => {
      const deactivated: SessionItem[] = prev.map((s) => ({ ...s, active: false }));
      const firstIdx = deactivated.findIndex((s) => s.group === targetGroup);
      if (firstIdx >= 0) {
        const next: SessionItem[] = [...deactivated];
        next.splice(firstIdx, 0, newSess);
        return next;
      }
      return [newSess, ...deactivated];
    });

    props.ctx.events.emit('session:created', { id: newId, title: newTitle, group: targetGroup });
    props.ctx.events.emit('session:switched', { id: newId, title: newTitle, group: targetGroup });
    toast.success(`已在「${targetGroup}」下新建会话`);
  };

  /** 选中切换会话并联动主时间线 */
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

  onMount(() => {
    const handleClickOutside = () => {
      setShowGatewayMenu(false);
      setContextMenu(null);
    };
    window.addEventListener('click', handleClickOutside);
    onCleanup(() => window.removeEventListener('click', handleClickOutside));
  });

  return (
    <div style="display: flex; flex-direction: column; height: 100%; min-height: 0; background: #f8f8f7; color: #2d2b28; font-size: 13px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', sans-serif; position: relative;">
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
      <div style="flex: 1; overflow-y: auto; padding: 0 8px 12px; display: flex; flex-direction: column; gap: 6px; overscroll-behavior: contain;">
        <For each={groupedProjects()}>
          {(proj) => {
            const isCollapsed = () => !!collapsedGroups()[proj.name];

            return (
              <div style="display: flex; flex-direction: column; gap: 1px;">
                {/* 项目/分组表头 (点击支持折叠/展开，带删除项目与新建会话按钮) */}
                <div
                  id={`project-header-${proj.name}`}
                  onClick={() => toggleGroupCollapse(proj.name)}
                  onMouseEnter={(e) => {
                    setHoveredGroup(proj.name);
                    e.currentTarget.style.background = '#f0eee8';
                  }}
                  onMouseLeave={(e) => {
                    setHoveredGroup(null);
                    e.currentTarget.style.background = 'transparent';
                  }}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setContextMenu({ type: 'project', group: proj.name, x: e.clientX, y: e.clientY });
                  }}
                  style="padding: 6px 8px 4px; font-size: 11.5px; font-weight: 600; color: #71717a; text-transform: uppercase; letter-spacing: 0.04em; display: flex; align-items: center; justify-content: space-between; border-radius: 6px; cursor: pointer; user-select: none; transition: background 0.1s ease;"
                >
                  <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {/* 折叠箭头指示器 */}
                    <span
                      style={`display: flex; align-items: center; transform: ${
                        isCollapsed() ? 'rotate(-90deg)' : 'rotate(0deg)'
                      }; transition: transform 0.15s ease; color: #a1a1aa;`}
                    >
                      <IconChevronDown size={11} />
                    </span>
                    <IconFolderLinear />
                    <span style="overflow: hidden; text-overflow: ellipsis;">{proj.name}</span>
                  </div>

                  {/* 项目悬停快捷操作：新建会话 + 删除项目 */}
                  <div
                    style={`display: flex; align-items: center; gap: 3px; opacity: ${
                      hoveredGroup() === proj.name ? 1 : 0
                    }; transition: opacity 0.15s ease;`}
                  >
                    <button
                      id={`new-session-in-project-${proj.name}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleNewSession(proj.name);
                      }}
                      style="background: transparent; border: none; padding: 2px 4px; border-radius: 4px; color: #71717a; cursor: pointer; display: flex; align-items: center;"
                      title={`在「${proj.name}」下新建会话`}
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#e4e4e7')}
                      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    >
                      <IconPlus size={12} />
                    </button>

                    <button
                      id={`delete-project-${proj.name}`}
                      onClick={(e) => handleDeleteProject(proj.name, e)}
                      style="background: transparent; border: none; padding: 2px 4px; border-radius: 4px; color: #71717a; cursor: pointer; display: flex; align-items: center; transition: color 0.15s ease;"
                      title={`删除项目「${proj.name}」及其所有会话`}
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

                {/* 所属会话列表 */}
                <Show when={!isCollapsed()}>
                  <div style="display: flex; flex-direction: column; gap: 1px;">
                    <For each={proj.sessions}>
                      {(sess) => (
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
                            sess.active ? 'background: #eae7e1; color: #18181b; font-weight: 500;' : 'background: transparent; color: #3f3f46; font-weight: 400;'
                          }`}
                        >
                          <div style="display: flex; align-items: center; gap: 6px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; flex: 1;">
                            <span style={`width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; ${sess.active ? 'background: #c2410c;' : 'background: transparent;'}`} />
                            <span style="overflow: hidden; text-overflow: ellipsis; font-size: 13px;">{sess.title}</span>
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
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>

      {/* ══════════════════════════════════════════════════════════════════════════
          右键上下文菜单 (1:1 像素级对齐图二、图三字体、阴影与纯黑白矢量线性图标)
         ══════════════════════════════════════════════════════════════════════════ */}
      <Show when={contextMenu()}>
        {(menu) => (
          <div
            onClick={(e) => e.stopPropagation()}
            style={`position: fixed; top: ${menu().y}px; left: ${menu().x}px; width: 175px; background: #ffffff; border: 1px solid rgba(0, 0, 0, 0.09); border-radius: 9px; box-shadow: 0 4px 18px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.04); padding: 4px; z-index: 1000; display: flex; flex-direction: column; gap: 1px; font-size: 13px; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif; -webkit-font-smoothing: antialiased; color: #1f2937; user-select: none;`}
          >
            {/* 项目右键菜单 (1:1 对齐图二) */}
            <Show when={menu().type === 'project'}>
              <div
                onClick={() => {
                  toast.info(`已置顶项目: ${menu().group}`);
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconPin />
                <span>置顶</span>
              </div>

              <div
                onClick={() => {
                  if (menu().group) {
                    setEditingItem({ type: 'project', currentName: menu().group!, group: menu().group });
                    setRenameInput(menu().group!);
                  }
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconEditLinear />
                <span>编辑</span>
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                onClick={() => {
                  toast.info(`已在资源管理器中定位: ${menu().group}`);
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconFolderLinear />
                <span>在资源管理器中打开</span>
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                onClick={() => {
                  toast.info(`已归档项目: ${menu().group}`);
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconArchiveLinear />
                <span>归档聊天</span>
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                id="context-menu-delete-project"
                onClick={() => menu().group && handleDeleteProject(menu().group!)}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; color: #1f2937; font-size: 13px;"
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = '#f3f4f6';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                }}
              >
                <IconRemoveLinear />
                <span>移除项目</span>
              </div>
            </Show>

            {/* 会话右键菜单 (1:1 对齐图三) */}
            <Show when={menu().type === 'session'}>
              <div
                onClick={() => {
                  toast.info('已置顶会话');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconPin />
                <span>置顶</span>
              </div>

              <div
                onClick={() => {
                  const target = sessions().find((s) => s.id === menu().id);
                  if (target) {
                    setEditingItem({ type: 'session', id: target.id, currentName: target.title });
                    setRenameInput(target.title);
                  }
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconEditLinear />
                <span>重命名</span>
              </div>

              <div
                onClick={() => {
                  toast.info('已标记为未读');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconEyeLinear />
                <span>标记为未读</span>
              </div>

              <div
                onClick={() => {
                  toast.info('已归档');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconArchiveLinear />
                <span>归档</span>
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                onClick={() => {
                  toast.info('移动至项目');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <div style="display: flex; align-items: center; gap: 9px;">
                  <IconFolderLinear />
                  <span>项目</span>
                </div>
                <IconChevronRight size={12} color="#9ca3af" />
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                onClick={() => {
                  toast.info('已生成分享链接');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconShareLinear />
                <span>分享</span>
              </div>

              <div
                onClick={() => {
                  toast.info('已复制会话内容');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <div style="display: flex; align-items: center; gap: 9px;">
                  <IconCopyLinear />
                  <span>复制</span>
                </div>
                <IconChevronRight size={12} color="#9ca3af" />
              </div>

              {/* 分割线 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />

              <div
                onClick={() => {
                  toast.info('已在新窗口中打开');
                  setContextMenu(null);
                }}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; font-size: 13px; color: #1f2937;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#f3f4f6')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconOpenNewWindowLinear />
                <span>在新窗口中打开</span>
              </div>

              {/* 删除会话 */}
              <div style="height: 1px; background: #e5e7eb; margin: 3px 0;" />
              <div
                id="context-menu-delete-session"
                onClick={() => menu().id && handleDeleteSession(menu().id!)}
                style="padding: 6px 10px; border-radius: 6px; cursor: pointer; display: flex; align-items: center; gap: 9px; color: #ef4444; font-size: 13px;"
                onMouseEnter={(e) => (e.currentTarget.style.background = '#fee2e2')}
                onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
              >
                <IconRemoveLinear />
                <span>删除会话</span>
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
