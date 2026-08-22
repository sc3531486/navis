import { Component, createSignal, For, Show } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import {
  IconFolder,
  IconSparkles,
  IconCheck,
  IconChevronRight,
  IconChevronDown,
} from '@/components/icons';

export const ContextDrawer: Component<{ ctx: NavisContext }> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'tasks' | 'artifacts'>('artifacts');
  const [filesExpanded, setFilesExpanded] = createSignal(true);
  const [artifactsExpanded, setArtifactsExpanded] = createSignal(true);
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

  return (
    <div
      style="width: 240px; min-width: 220px; max-width: 280px; height: 100%; background: #ffffff; border-left: 1px solid #f0eee8; display: flex; flex-direction: column; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; user-select: none;"
    >
      {/* 顶部 Tab 图标切换栏 */}
      <div style="display: flex; align-items: center; justify-content: flex-start; gap: 4px; padding: 8px 12px; border-bottom: 1px solid #f4f2ee;">
        <button
          onClick={() => setActiveTab('tasks')}
          style={`padding: 5px 8px; border: none; border-radius: 6px; cursor: pointer; display: flex; align-items: center; font-size: 13px; ${
            activeTab() === 'tasks' ? 'background: #f4f2ee; color: #1e1d1b;' : 'background: transparent; color: #8e8b83;'
          }`}
          title="任务面板"
        >
          📋
        </button>
        <button
          onClick={() => setActiveTab('artifacts')}
          style={`padding: 5px 8px; border: none; border-radius: 6px; cursor: pointer; display: flex; align-items: center; font-size: 13px; ${
            activeTab() === 'artifacts' ? 'background: #f4f2ee; color: #1e1d1b;' : 'background: transparent; color: #8e8b83;'
          }`}
          title="交付件与上下文"
        >
          📄
        </button>
      </div>

      {/* 抽屉内容滚动区域 */}
      <div style="flex: 1; overflow-y: auto; padding: 8px 12px; display: flex; flex-direction: column; gap: 14px;">
        {/* 1. 子代理 */}
        <div
          onClick={() => toast.info('当前暂无活动子代理')}
          style="display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: #5a5750; cursor: pointer; padding: 4px 0;"
        >
          <span style="font-weight: 500;">子代理 <span style="color: #a19e95;">0</span></span>
          <IconChevronRight size={13} color="#a19e95" />
        </div>

        {/* 2. 文件已更改 */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setFilesExpanded(!filesExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: #5a5750; cursor: pointer; padding: 4px 0;"
          >
            <span style="font-weight: 500;">文件已更改 <span style="color: #a19e95;">100</span></span>
            <Show when={filesExpanded()} fallback={<IconChevronRight size={13} color="#a19e95" />}>
              <IconChevronDown size={13} color="#a19e95" />
            </Show>
          </div>

          <Show when={filesExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 2px;">
              <For each={changedFiles}>
                {(file) => (
                  <div
                    onClick={() => {
                      props.ctx.events.emit('editor:open', { path: `D:\\myworkspace\\Navis Go\\${file}` });
                      toast.info(`正在打开文件: ${file}`);
                    }}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #44403c; padding: 3px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f9f8f6')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">📄</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{file}</span>
                  </div>
                )}
              </For>
              <div
                onClick={() => props.ctx.commands.execute('command:palette')}
                style="font-size: 11px; color: #8e8b83; padding: 2px 6px; cursor: pointer;"
                onMouseEnter={(e) => (e.currentTarget.style.color = '#2563eb')}
                onMouseLeave={(e) => (e.currentTarget.style.color = '#8e8b83')}
              >
                See all (100)
              </div>
            </div>
          </Show>
        </div>

        {/* 3. 交付件列表 (Artifacts) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setArtifactsExpanded(!artifactsExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: #5a5750; cursor: pointer; padding: 4px 0;"
          >
            <span style="font-weight: 500;">交付件列表 (Artifacts) <span style="color: #a19e95;">139</span></span>
            <Show when={artifactsExpanded()} fallback={<IconChevronRight size={13} color="#a19e95" />}>
              <IconChevronDown size={13} color="#a19e95" />
            </Show>
          </div>

          <Show when={artifactsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 2px;">
              <For each={artifactsList}>
                {(art) => (
                  <div
                    onClick={() => toast.info(`查看交付件: ${art.title}`)}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #44403c; padding: 3px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f9f8f6')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">🖼️</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{art.title}</span>
                  </div>
                )}
              </For>
              <div
                onClick={() => toast.info('查看全部 139 个交付件')}
                style="font-size: 11px; color: #8e8b83; padding: 2px 6px; cursor: pointer;"
                onMouseEnter={(e) => (e.currentTarget.style.color = '#2563eb')}
                onMouseLeave={(e) => (e.currentTarget.style.color = '#8e8b83')}
              >
                See all (139)
              </div>
            </div>
          </Show>
        </div>

        {/* 4. 用户上传 (Uploads) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setUploadsExpanded(!uploadsExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: #5a5750; cursor: pointer; padding: 4px 0;"
          >
            <span style="font-weight: 500;">Uploads <span style="color: #a19e95;">14</span></span>
            <Show when={uploadsExpanded()} fallback={<IconChevronRight size={13} color="#a19e95" />}>
              <IconChevronDown size={13} color="#a19e95" />
            </Show>
          </div>

          <Show when={uploadsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 2px;">
              <For each={uploadsList}>
                {(up) => (
                  <div
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #44403c; padding: 3px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f9f8f6')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">🖼️</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{up.title}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* 5. 后台任务 */}
        <div
          onClick={() => toast.info('当前暂无后台任务运行')}
          style="display: flex; align-items: center; justify-content: space-between; font-size: 12.5px; color: #5a5750; cursor: pointer; padding: 4px 0;"
        >
          <span style="font-weight: 500;">后台任务 <span style="color: #a19e95;">0</span></span>
          <IconChevronRight size={13} color="#a19e95" />
        </div>
      </div>
    </div>
  );
};

export default ContextDrawer;
