import { Component, createSignal, onMount, onCleanup, For, Show } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { callRemote } from '@/core/tauri-bridge';
import {
  IconChevronRight,
  IconChevronDown,
} from '@/components/icons';

export interface ChangedFileItem {
  name: string;
  path: string;
  breadcrumb?: string;
  type: 'diff' | 'code' | 'doc';
  status?: string;
}

export interface ArtifactItem {
  id: string;
  title: string;
  type: 'image' | 'doc' | 'script' | 'plan';
  path?: string;
  imageUrl?: string;
  timestamp: number;
}

export interface UploadItem {
  id: string;
  title: string;
  type: 'image' | 'file';
  size?: number;
  url?: string;
  timestamp: number;
}

export interface SubagentItem {
  id: string;
  name: string;
  status: 'running' | 'idle' | 'completed';
}

export interface TaskItem {
  id: string;
  name: string;
  status: 'running' | 'completed' | 'failed';
}

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

// 真实物理磁盘文件动态索引 (开发期/浏览器期通过 Vite glob 自动发现真实物理文件)
const localWorkspaceGlob = import.meta.glob(
  [
    '../../../../../extensions/**/*.{tsx,ts,json,md,css}',
    '../../../../../src/**/*.{tsx,ts,json,md,css}',
    '../../../../../*.{md,json,ts,toml}',
  ],
  { query: '?raw', eager: false }
);

export const ContextDrawer: Component<{ ctx: NavisContext }> = (props) => {
  const [activeTab, setActiveTab] = createSignal<'tasks' | 'artifacts'>('tasks');
  const [filesExpanded, setFilesExpanded] = createSignal(true);
  const [artifactsExpanded, setArtifactsExpanded] = createSignal(false);
  const [uploadsExpanded, setUploadsExpanded] = createSignal(false);
  const [subagentsExpanded, setSubagentsExpanded] = createSignal(false);
  const [tasksExpanded, setTasksExpanded] = createSignal(false);

  // ══════════════════════════════════════════════════════════════════════════
  // 全动态响应式状态（拒绝任何死数据，纯实时扫描与事件驱动）
  // ══════════════════════════════════════════════════════════════════════════
  const [changedFiles, setChangedFiles] = createSignal<ChangedFileItem[]>([]);
  const [artifacts, setArtifacts] = createSignal<ArtifactItem[]>([]);
  const [uploads, setUploads] = createSignal<UploadItem[]>([]);
  const [subagents, setSubagents] = createSignal<SubagentItem[]>([]);
  const [tasks, setTasks] = createSignal<TaskItem[]>([]);

  // 从 Git 与文件系统真实扫描已修改/已变动文件
  const refreshWorkspaceFiles = async () => {
    try {
      // 1. 尝试通过 git status 真实读取变动文件
      const res = await callRemote('core:shell:exec', { command: 'git status --porcelain' });
      if (res?.success && typeof res.stdout === 'string' && res.stdout.trim().length > 0) {
        const lines = res.stdout.split('\n').filter((l: string) => l.trim().length > 0 && /^[ MADRCU?!]{1,2}\s+/.test(l));
        const parsed: ChangedFileItem[] = lines.map((line: string) => {
          const status = line.slice(0, 2).trim();
          const filePath = line.slice(2).trim();
          const fileName = filePath.split('/').pop() || filePath;
          return {
            name: fileName,
            path: filePath,
            breadcrumb: filePath.includes('/') ? filePath.replace(/\//g, ' > ') : `Navis Go > 📄 ${fileName}`,
            type: fileName.endsWith('.md') ? 'diff' : 'code',
            status,
          };
        });
        if (parsed.length > 0) {
          setChangedFiles(parsed);
          return;
        }
      }

      // 2. 如果工作区当前干净无 Git 差异，扫描真实工作区顶层核心文件供查看
      const listRes = await callRemote('core:fs:list_dir', { path: '.' });
      if (listRes?.success && Array.isArray(listRes.entries)) {
        const realFiles: ChangedFileItem[] = listRes.entries
          .filter((e: any) => !e.is_dir && !e.name.startsWith('.'))
          .map((e: any) => ({
            name: e.name,
            path: e.name,
            breadcrumb: `Navis Go > 📄 ${e.name}`,
            type: e.name.endsWith('.md') ? 'diff' : 'code',
          }));
        if (realFiles.length > 0) {
          setChangedFiles(realFiles);
          return;
        }
      }

      // 3. 开发环境 / 浏览器环境动态扫描真实磁盘文件索引
      const globKeys = Object.keys(localWorkspaceGlob);
      if (globKeys.length > 0) {
        const discovered = globKeys
          .map((k) => {
            const clean = k.replace(/^(\.\.\/)+/, '');
            const name = clean.split('/').pop() || clean;
            return {
              name,
              path: clean,
              breadcrumb: clean.includes('/') ? clean.replace(/\//g, ' > ') : `Navis Go > 📄 ${name}`,
              type: name.endsWith('.md') ? ('diff' as const) : ('code' as const),
            };
          })
          .filter((f) => !f.name.startsWith('.'));
        if (discovered.length > 0) {
          setChangedFiles(discovered.slice(0, 10));
          return;
        }
      }
    } catch (e) {
      console.warn('[ContextDrawer] Error refreshing real files:', e);
    }
  };

  onMount(() => {
    // 初始扫描
    refreshWorkspaceFiles();

    // 监听实时文件创建/修改事件
    const unsubFileCreated = props.ctx.events.on('file:created', (payload: { path: string }) => {
      if (!payload?.path) return;
      const fileName = payload.path.split('/').pop() || payload.path;
      setChangedFiles((prev) => [
        {
          name: fileName,
          path: payload.path,
          breadcrumb: `Navis Go > 📄 ${fileName}`,
          type: fileName.endsWith('.md') ? 'diff' : 'code',
        },
        ...prev.filter((f) => f.path !== payload.path),
      ]);
    });

    const unsubFileModified = props.ctx.events.on('file:modified', (payload: { path: string }) => {
      if (!payload?.path) return;
      const fileName = payload.path.split('/').pop() || payload.path;
      setChangedFiles((prev) => [
        {
          name: fileName,
          path: payload.path,
          breadcrumb: `Navis Go > 📄 ${fileName}`,
          type: 'diff',
        },
        ...prev.filter((f) => f.path !== payload.path),
      ]);
    });

    // 监听实时交付件生成事件 (如 Agent 生成的文档、图片、脚本)
    const unsubArtifact = props.ctx.events.on('artifact:created', (payload: any) => {
      if (!payload?.name && !payload?.title) return;
      const title = payload.title || payload.name;
      const isImg = title.endsWith('.png') || title.endsWith('.jpg') || payload.type === 'image';
      setArtifacts((prev) => [
        {
          id: `art-${Date.now()}`,
          title,
          type: isImg ? 'image' : 'doc',
          path: payload.path,
          imageUrl: payload.imageUrl,
          timestamp: Date.now(),
        },
        ...prev,
      ]);
    });

    // 监听上传附件事件
    const unsubUpload = props.ctx.events.on('composer:attachment:added', (payload: any) => {
      if (!payload?.name) return;
      setUploads((prev) => [
        {
          id: `up-${Date.now()}`,
          title: payload.name,
          type: payload.type || 'file',
          size: payload.size,
          url: payload.url,
          timestamp: Date.now(),
        },
        ...prev,
      ]);
    });

    // 监听子代理生命周期
    const unsubSubagent = props.ctx.events.on('subagent:spawned', (payload: any) => {
      if (!payload?.name) return;
      setSubagents((prev) => [
        {
          id: payload.id || `sub-${Date.now()}`,
          name: payload.name,
          status: 'running',
        },
        ...prev,
      ]);
    });

    // 监听后台任务
    const unsubTask = props.ctx.events.on('task:started', (payload: any) => {
      if (!payload?.name) return;
      setTasks((prev) => [
        {
          id: payload.id || `task-${Date.now()}`,
          name: payload.name,
          status: 'running',
        },
        ...prev,
      ]);
    });

    onCleanup(() => {
      unsubFileCreated();
      unsubFileModified();
      unsubArtifact();
      unsubUpload();
      unsubSubagent();
      unsubTask();
    });
  });

  const openFileViewer = (
    name: string,
    type: 'diff' | 'code' | 'doc' | 'image' | 'script' | 'plan' | string = 'code',
    path?: string,
    breadcrumb?: string,
    imageUrl?: string
  ) => {
    props.ctx.events.emit('diff:open', {
      name,
      type,
      path: path || name,
      breadcrumb: breadcrumb || `Navis Go > 📄 ${name}`,
      imageUrl,
    });
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
          title="文档与任务概览"
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

      {/* 抽屉内容列表区域 (全部数字与列表项实时动态计算) */}
      <div style="flex: 1; overflow-y: auto; padding: 12px 14px; display: flex; flex-direction: column; gap: 14px;">
        {/* 1. 子代理 (动态计数) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setSubagentsExpanded(!subagentsExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>子代理</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                {subagents().length}
              </span>
            </div>
            <Show when={subagentsExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={subagentsExpanded() && subagents().length > 0}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={subagents()}>
                {(sub) => (
                  <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px;">
                    <span style="font-size: 11px;">🤖</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{sub.name}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* 2. 文件已更改 (动态计数与真实文件列表) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            id="context-drawer-files-changed-item"
            onClick={() => {
              setFilesExpanded(!filesExpanded());
              if (changedFiles().length > 0) {
                const first = changedFiles()[0];
                openFileViewer(first.name, first.type, first.path, first.breadcrumb);
              }
            }}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>文件已更改</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                {changedFiles().length}
              </span>
            </div>
            <Show when={filesExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={filesExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={changedFiles()}>
                {(f) => (
                  <div
                    id={`file-item-${f.name}`}
                    onClick={() => openFileViewer(f.name, f.type, f.path, f.breadcrumb)}
                    style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style="font-size: 11px;">📄</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{f.name}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>

        {/* 3. 交付件列表(Artifacts) (动态计数与交付件列表) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            id="context-drawer-artifacts-item"
            onClick={() => {
              setArtifactsExpanded(!artifactsExpanded());
              if (artifacts().length > 0) {
                const first = artifacts()[0];
                openFileViewer(first.title, first.type, first.path, `Navis Go > ${first.title}`, first.imageUrl);
              }
            }}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>交付件列表(Artifacts)</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                {artifacts().length}
              </span>
            </div>
            <Show when={artifactsExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={artifactsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <Show
                when={artifacts().length > 0}
                fallback={
                  <div style="font-size: 11.5px; color: #a1a1aa; padding: 4px 6px;">暂无生成交付件</div>
                }
              >
                <For each={artifacts()}>
                  {(art) => (
                    <div
                      id={`artifact-item-${art.title.replace(/[^a-zA-Z0-9]/g, '')}`}
                      onClick={() => openFileViewer(art.title, art.type, art.path, `Navis Go > ${art.title}`, art.imageUrl)}
                      style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    >
                      <span style="font-size: 11px;">{art.type === 'image' ? '🖼️' : '📄'}</span>
                      <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{art.title}</span>
                    </div>
                  )}
                </For>
              </Show>
            </div>
          </Show>
        </div>

        {/* 4. Uploads (动态计数与列表) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setUploadsExpanded(!uploadsExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>Uploads</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                {uploads().length}
              </span>
            </div>
            <Show when={uploadsExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={uploadsExpanded()}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <Show
                when={uploads().length > 0}
                fallback={
                  <div style="font-size: 11.5px; color: #a1a1aa; padding: 4px 6px;">暂无上传附件</div>
                }
              >
                <For each={uploads()}>
                  {(upload) => (
                    <div
                      onClick={() => openFileViewer(upload.title, upload.type === 'image' ? 'image' : 'doc', undefined, `Navis Go > ${upload.title}`)}
                      style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px; cursor: pointer;"
                      onMouseEnter={(e) => (e.currentTarget.style.background = '#f4f4f5')}
                      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    >
                      <span style="font-size: 11px;">{upload.type === 'image' ? '🖼️' : '📎'}</span>
                      <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{upload.title}</span>
                    </div>
                  )}
                </For>
              </Show>
            </div>
          </Show>
        </div>

        {/* 5. 后台任务 (动态计数) */}
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <div
            onClick={() => setTasksExpanded(!tasksExpanded())}
            style="display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: #3f3f46; cursor: pointer; padding: 4px 0;"
            onMouseEnter={(e) => (e.currentTarget.style.color = '#18181b')}
            onMouseLeave={(e) => (e.currentTarget.style.color = '#3f3f46')}
          >
            <div style="display: flex; align-items: center; gap: 8px;">
              <span>后台任务</span>
              <span style="background: #f4f4f5; color: #71717a; padding: 1px 7px; border-radius: 10px; font-size: 11.5px; font-weight: 500;">
                {tasks().length}
              </span>
            </div>
            <Show when={tasksExpanded()} fallback={<IconChevronRight size={13} color="#a1a1aa" />}>
              <IconChevronDown size={13} color="#a1a1aa" />
            </Show>
          </div>

          <Show when={tasksExpanded() && tasks().length > 0}>
            <div style="display: flex; flex-direction: column; gap: 4px; padding-left: 6px;">
              <For each={tasks()}>
                {(task) => (
                  <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: #4b5563; padding: 4px 6px; border-radius: 4px;">
                    <span style="font-size: 11px;">⚙️</span>
                    <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{task.name}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
};

export default ContextDrawer;
