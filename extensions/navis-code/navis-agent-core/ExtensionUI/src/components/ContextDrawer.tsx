import { Component, createSignal, For, Show } from 'solid-js';
import type { NavisContext } from '@/core/context';
import {
  IconChevronRight,
  IconChevronDown,
} from '@/components/icons';

// ══════════════════════════════════════════════════════════════════════════
// 1:1 对齐图三的顶部 Tab 纯矢量单色线性图标 (Monochrome Linear Tab Icons)
// ══════════════════════════════════════════════════════════════════════════
const IconMenuDocLinear = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="3"></rect>
    <line x1="7" y1="8" x2="17" y2="8"></line>
    <line x1="7" y1="12" x2="17" y2="12"></line>
    <line x1="7" y1="16" x2="13" y2="16"></line>
  </svg>
);

const IconFilePlusLinear = () => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
    <polyline points="14 2 14 8 20 8"></polyline>
    <line x1="12" y1="18" x2="12" y2="12"></line>
    <line x1="9" y1="15" x2="15" y2="15"></line>
  </svg>
);

export const ContextDrawer: Component<{ ctx: NavisContext }> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'tasks' | 'artifacts'>('tasks');
  const [filesExpanded, setFilesExpanded] = createSignal(false);
  const [artifactsExpanded, setArtifactsExpanded] = createSignal(false);
  const [uploadsExpanded, setUploadsExpanded] = createSignal(false);

  const changedFiles = [
    'AGENTS.md',
    'MIGRATION-PLAN.md',
    'ARCHITECTURE_REVIEW.md',
    'README.md',
    'CLAUDE.md',
  ];

  const artifactsList = [
    { title: 'Media (Today 10:31 AM)', type: 'image' },
    { title: 'Agent Turn Live 03...', type: 'doc' },
    { title: 'Verify Agent Promp...', type: 'script' },
    { title: 'Media (Today 10:31 AM)', type: 'image' },
  ];

  const uploadsList = [
    { title: 'Media (Today 10:21 AM)', type: 'image' },
    { title: 'Media (Today 10:17 AM)', type: 'image' },
    { title: 'Media (Today 10:10 AM)', type: 'image' },
  ];

  const openDiffViewer = (name = 'AGENTS.md', type = 'diff', imageUrl?: string) => {
    props.ctx.events.emit('diff:open', { name, type, imageUrl });
  };

  return (
    <div
      style="width: 240px; min-width: 220px; max-width: 280px; height: 100%; background: #ffffff; border-left: 1px solid #f0eee8; display: flex; flex-direction: column; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', sans-serif; -webkit-font-smoothing: antialiased; user-select: none;"
    >
      {/* 顶部 Tab 图标切换栏 (1:1 对齐图三) */}
      <div style="display: flex; align-items: center; justify-content: flex-start; gap: 4px; padding: 6px 10px; border-bottom: 1px solid #f4f2ee;">
        <button
          onClick={() => setActiveTab('tasks')}
          style={`padding: 5px 8px; border: none; border-radius: 6px; cursor: pointer; display: flex; align-items: center; font-size: 13px; transition: background 0.1s ease; ${
            activeTab() === 'tasks' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #71717a;'
          }`}
          title="文档概览"
        >
          <IconMenuDocLinear />
        </button>
        <button
          onClick={() => setActiveTab('artifacts')}
          style={`padding: 5px 8px; border: none; border-radius: 6px; cursor: pointer; display: flex; align-items: center; font-size: 13px; transition: background 0.1s ease; ${
            activeTab() === 'artifacts' ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #71717a;'
          }`}
          title="新建与添加"
        >
          <IconFilePlusLinear />
        </button>
      </div>

      {/* 抽屉内容列表区域 (1:1 像素级对齐图三药丸数字与排版) */}
      <div style="flex: 1; overflow-y: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 14px;">
        {/* 1. 子代理 0 > */}
        <div
          style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
          onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
          onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
        >
          <div style="display: flex; align-items: center; gap: 8px;">
            <span>子代理</span>
            <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
              0
            </span>
          </div>
          <IconChevronRight size={13} color="#a1a1aa" />
        </div>

        {/* 2. 文件已更改 107 > (点击展开文件，点击文件打开图四 Diff 视图) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            id="context-drawer-files-changed-item"
            onClick={() => {
              setFilesExpanded(!filesExpanded());
              openDiffViewer('AGENTS.md', 'diff');
            }}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>文件已更改</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                107
              </span>
            </div>
            <Show when={filesExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={filesExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={changedFiles}>
                {(file) => (
                  <div
                    id={`file-item-${file}`}
                    onClick={() => openDiffViewer(file, 'diff')}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">📄</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{file}</span>
                  </div>
                )}
              </For>
              <div
                onClick={() => openDiffViewer('AGENTS.md', 'diff')}
                style="font-size: 11px; color: #9ca3af; padding: 2px 6px; cursor: pointer;"
                onMouseEnter={(e) => (e.currentTarget.style.color = '#2563eb')}
                onMouseLeave={(e) => (e.currentTarget.style.color = '#9ca3af')}
              >
                See all (107)
              </div>
            </div>
          </Show>
        </div>

        {/* 3. 交付件列表(Artifacts) 278 > (支持查看并放大图片) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            id="context-drawer-artifacts-item"
            onClick={() => {
              setArtifactsExpanded(!artifactsExpanded());
              openDiffViewer('Media (Today 10:31 AM)', 'image');
            }}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>交付件列表(Artifacts)</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                278
              </span>
            </div>
            <Show when={artifactsExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={artifactsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={artifactsList}>
                {(art) => (
                  <div
                    id={`artifact-item-${art.title.replace(/[^a-zA-Z0-9]/g, '')}`}
                    onClick={() => openDiffViewer(art.title, art.type as any)}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">{art.type === 'image' ? '🖼️' : '📄'}</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{art.title}</span>
                  </div>
                )}
              </For>
              <div
                onClick={() => openDiffViewer('Media (Today 10:31 AM)', 'image')}
                style="font-size: 11px; color: #9ca3af; padding: 2px 6px; cursor: pointer;"
                onMouseEnter={(e) => (e.currentTarget.style.color = '#2563eb')}
                onMouseLeave={(e) => (e.currentTarget.style.color = '#9ca3af')}
              >
                See all (278)
              </div>
            </div>
          </Show>
        </div>

        {/* 4. Uploads 49 > */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => {
              setUploadsExpanded(!uploadsExpanded());
              openDiffViewer('Media (Today 10:21 AM)', 'image');
            }}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>Uploads</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                49
              </span>
            </div>
            <Show when={uploadsExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={uploadsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={uploadsList}>
                {(upload) => (
                  <div
                    onClick={() => openDiffViewer(upload.title, 'image')}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">🖼️</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{upload.title}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* 5. 后台任务 0 > */}
        <div
          style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
          onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
          onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
        >
          <div style="display: flex; align-items: center; gap: 8px;">
            <span>后台任务</span>
            <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
              0
            </span>
          </div>
          <IconChevronRight size={13} color="#a1a1aa" />
        </div>
      </div>
    </div>
  );
};

export default ContextDrawer;
