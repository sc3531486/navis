import { Component, createSignal, For, Show } from 'solid-js';
import type { NavisContext } from '@/core/context';

export interface DiffFilePayload {
  name: string;
  type?: 'diff' | 'image' | 'code' | 'doc';
  imageUrl?: string;
  diffLines?: Array<{
    leftNum?: number | string;
    rightNum?: number | string;
    type: 'unchanged' | 'deleted' | 'added' | 'header';
    content: string;
    hasPlusBadge?: boolean;
  }>;
}

const DEFAULT_AGENTS_DIFF: Array<{
  leftNum?: number | string;
  rightNum?: number | string;
  type: 'unchanged' | 'deleted' | 'added' | 'header';
  content: string;
  hasPlusBadge?: boolean;
}> = [
  { leftNum: 1, rightNum: 1, type: 'header', content: '# AGENTS.md' },
  { leftNum: 2, rightNum: 2, type: 'unchanged', content: '' },
  {
    leftNum: 3,
    type: 'deleted',
    content: '本文件是 `D:\\myworkspace\\Navis Go` 的开发约束。文档中的“框架”默认指通用 Navis 宿主；文档中的“产品”默认指由扩展组合出的 Navis Code，不能把二者混为一谈。',
  },
  {
    rightNum: 3,
    type: 'added',
    content: '本文件是 `D:\\myworkspace\\Navis Go` 的核心架构与开发约束。文档中的“框架”默认指通用 Navis 白板宿主与扩展运行时；文档中的“产品”默认指由产品清单组合出的具体应用形态（如 Navis Code、柜面系统、双录系统等），不能把二者混为一谈。',
    hasPlusBadge: true,
  },
  { leftNum: 4, rightNum: 4, type: 'unchanged', content: '' },
  { leftNum: 5, rightNum: 5, type: 'header', content: '## 项目定位' },
  { leftNum: 6, rightNum: 6, type: 'unchanged', content: '' },
  {
    leftNum: 7,
    type: 'deleted',
    content: 'Navis 是基于 Tauri 2 的通用桌面应用白板与扩展运行时。Navis 只负责窗口与宿主生命周期、扩展发现/加载/启停、能力注册、事件、权限、存储、IPC、流式通道和 UI 投影等通用机制。',
  },
  {
    rightNum: 7,
    type: 'added',
    content: 'Navis 是基于 Tauri 2 的通用桌面应用白板与扩展运行时（灵感源自 Cordis 扩展体系）。Navis 框架只负责窗口与宿主生命周期、扩展发现/加载/启停、IoC 服务容器（DI）、事件总线（emit/waterfall/serial/parallel）、响应式插槽树（DynamicSlot）、通用命令、沙箱权限、存储、多路复用 IPC 与流式通道等通用基础设施。',
    hasPlusBadge: true,
  },
  { leftNum: 8, rightNum: 8, type: 'unchanged', content: '' },
  {
    leftNum: 9,
    type: 'deleted',
    content: 'AI、Agent、会话、项目、编辑器、终端、知识库、记忆、任务、设置，以及银行柜面系统、双录系统等所有垂直业务，全部属于扩展，不得写入通用框架层。',
  },
  {
    rightNum: 9,
    type: 'added',
    content: 'AI、Agent、会话、项目、编辑器、终端、知识库、记忆、任务、设置，以及银行柜面系统、双录系统等所有垂直业务，**全部属于扩展（Extensions）**，严禁写入底层框架层（`src/` 与 `src-tauri/`）。',
  },
  { leftNum: 10, rightNum: 10, type: 'unchanged', content: '' },
  {
    leftNum: 11,
    type: 'deleted',
    content: 'Navis Code 是 Navis 的第一个产品，由 `extensions/navis-code/` 下的业务扩展和产品入口组合而成。未来增加其他产品时，应新增产品目录或独立扩展组合，不修改 Navis 的业务实现。',
  },
  {
    rightNum: 11,
    type: 'added',
    content: 'Navis Code 是在 Navis 框架上装配的第一个产品形态，由 `navis-code.json` 声明装配的套件扩展组合而成。未来增加其他产品形态（如银行柜面系统、双录系统）时，只需新增扩展目录并在其产品清单（如 `teller-system.json`）中声明装配，**无需修改 Navis 通用框架任何一行源码**。',
  },
  { leftNum: 12, rightNum: 12, type: 'unchanged', content: '' },
  { leftNum: 13, rightNum: 13, type: 'header', content: '## 常用命令' },
  { leftNum: 14, rightNum: 14, type: 'unchanged', content: '' },
  { leftNum: 15, rightNum: 15, type: 'unchanged', content: '```bash' },
  { leftNum: 16, rightNum: 16, type: 'unchanged', content: 'npm run dev' },
  { leftNum: 17, rightNum: 17, type: 'unchanged', content: 'npm run build' },
  { leftNum: 18, rightNum: 18, type: 'unchanged', content: 'npx tauri dev' },
  { leftNum: 19, rightNum: 19, type: 'unchanged', content: 'npx tauri build' },
  { leftNum: 20, rightNum: 20, type: 'unchanged', content: 'cd src-tauri && cargo check' },
  { leftNum: 21, rightNum: 21, type: 'unchanged', content: 'cd src-tauri && cargo test' },
  { leftNum: 22, rightNum: 22, type: 'unchanged', content: '```' },
];

const DEFAULT_PREVIEW_IMAGE =
  'data:image/svg+xml;utf8,' +
  encodeURIComponent(`
    <svg xmlns="http://www.w3.org/2000/svg" width="720" height="420" viewBox="0 0 720 420" fill="none">
      <defs>
        <linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stop-color="#0284c7" />
          <stop offset="50%" stop-color="#38bdf8" />
          <stop offset="100%" stop-color="#f97316" />
        </linearGradient>
      </defs>
      <rect width="720" height="420" rx="16" fill="#0f172a" />
      <rect x="24" y="24" width="672" height="372" rx="12" fill="url(#g1)" fill-opacity="0.15" stroke="#334155" stroke-width="2" />
      <circle cx="360" cy="180" r="64" fill="url(#g1)" />
      <text x="360" y="280" text-anchor="middle" fill="#f8fafc" font-size="20" font-weight="600" font-family="sans-serif">Navis Code Visual Artifact</text>
      <text x="360" y="310" text-anchor="middle" fill="#94a3b8" font-size="14" font-family="sans-serif">High-Resolution Workspace Snapshot Preview</text>
    </svg>
  `);

export const DiffViewer: Component<{
  ctx: NavisContext;
  file?: DiffFilePayload;
  onClose: () => void;
}> = (props) => {
  const currentFile = () => props.file || { name: 'AGENTS.md', type: 'diff' };
  const diffLines = () => props.file?.diffLines || DEFAULT_AGENTS_DIFF;
  const isImage = () => currentFile().type === 'image' || currentFile().name.endsWith('.png') || currentFile().name.includes('Media');

  // 图片放大灯箱状态
  const [imageZoomed, setImageZoomed] = createSignal(false);
  const [zoomScale, setZoomScale] = createSignal(1);

  return (
    <div style="flex: 1; display: flex; flex-direction: column; height: 100%; min-width: 0; background: #ffffff; overflow: hidden; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
      {/* 1. 顶部 Tab 栏 (1:1 对齐图四) */}
      <div style="height: 38px; background: #f8fafc; border-bottom: 1px solid #e2e8f0; display: flex; align-items: center; justify-content: space-between; padding: 0 12px; user-select: none; flex-shrink: 0;">
        <div style="display: flex; align-items: center; gap: 8px;">
          {/* 线性标签图标 */}
          <div style="display: flex; align-items: center; gap: 4px; color: #64748b;">
            <button
              style="background: transparent; border: none; padding: 4px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; color: inherit;"
              title="文档列表"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <rect x="3" y="3" width="18" height="18" rx="3"></rect>
                <line x1="7" y1="8" x2="17" y2="8"></line>
                <line x1="7" y1="12" x2="17" y2="12"></line>
                <line x1="7" y1="16" x2="13" y2="16"></line>
              </svg>
            </button>
            <button
              style="background: transparent; border: none; padding: 4px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; color: inherit;"
              title="新建文件"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                <polyline points="14 2 14 8 20 8"></polyline>
                <line x1="12" y1="18" x2="12" y2="12"></line>
                <line x1="9" y1="15" x2="15" y2="15"></line>
              </svg>
            </button>
          </div>

          {/* 活跃文件 Tab 药丸 */}
          <div style="display: flex; align-items: center; gap: 6px; background: #ffffff; border: 1px solid #e2e8f0; border-bottom: 2px solid #0284c7; padding: 4px 10px; border-radius: 6px 6px 0 0; font-size: 12px; font-weight: 500; color: #0f172a; height: 32px; margin-top: 5px;">
            <Show when={isImage()} fallback={<span>📄</span>}>
              <span>🖼️</span>
            </Show>
            <span style="max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
              {currentFile().name} <span style="color: #64748b; font-size: 11px;">(all agent edits)</span>
            </span>
            <button
              id="diff-viewer-close-btn"
              onClick={props.onClose}
              style="background: transparent; border: none; color: #94a3b8; cursor: pointer; padding: 1px 3px; font-size: 12px; border-radius: 3px; display: flex; align-items: center;"
              onMouseEnter={(e) => (e.currentTarget.style.color = '#0f172a')}
              onMouseLeave={(e) => (e.currentTarget.style.color = '#94a3b8')}
              title="关闭视图"
            >
              ✕
            </button>
          </div>
        </div>

        {/* 右侧窗口控件 */}
        <div style="display: flex; align-items: center; gap: 8px; color: #64748b;">
          <button
            style="background: transparent; border: none; color: inherit; cursor: pointer; padding: 2px 4px; font-size: 14px;"
            title="新建标签"
          >
            +
          </button>
          <button
            style="background: transparent; border: none; color: inherit; cursor: pointer; padding: 2px 4px; font-size: 13px;"
            title="窗口分栏"
          >
            ❐
          </button>
        </div>
      </div>

      {/* 2. 面包屑与快捷导航行 (1:1 对齐图四) */}
      <div style="height: 36px; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between; padding: 0 16px; font-size: 12px; color: #64748b; background: #ffffff; flex-shrink: 0;">
        <div style="display: flex; align-items: center; gap: 6px;">
          <span>Navis Go</span>
          <span>&gt;</span>
          <Show when={isImage()} fallback={<span>📄</span>}>
            <span>🖼️</span>
          </Show>
          <span style="font-weight: 600; color: #0f172a;">{currentFile().name}</span>
          <span style="color: #94a3b8; font-size: 11px;">(all agent edits)</span>
        </div>

        <div style="display: flex; align-items: center; gap: 10px;">
          <button style="background: transparent; border: none; color: #64748b; cursor: pointer;">⋮</button>
          <button style="background: transparent; border: none; color: #64748b; cursor: pointer;" title="复制">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            </svg>
          </button>
          <button style="background: transparent; border: none; color: #64748b; cursor: pointer;" title="分屏">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
              <rect x="3" y="3" width="18" height="18" rx="2"></rect>
              <line x1="9" y1="3" x2="9" y2="21"></line>
            </svg>
          </button>
        </div>
      </div>

      {/* 3. 主视图区域：Diff 差异对比 OR 图片查看器 (带放大支持) */}
      <div style="flex: 1; overflow-y: auto; background: #ffffff; display: flex; flex-direction: column; position: relative;">
        <Show
          when={!isImage()}
          fallback={
            /* ══════════════════════════════════════════════════════════════════════════
               图片 / 交付件查看器 (支持右上角放大与全屏灯箱)
               ══════════════════════════════════════════════════════════════════════════ */
            <div style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 24px; position: relative; background: #fafafa;">
              {/* 图片卡片容器 */}
              <div style="position: relative; max-width: 90%; max-height: 80%; background: #ffffff; border: 1px solid #e2e8f0; border-radius: 12px; padding: 12px; box-shadow: 0 8px 30px rgba(0,0,0,0.06); display: flex; flex-direction: column; align-items: center;">
                {/* 右上角放大按钮 (对齐要求) */}
                <div style="position: absolute; top: 12px; right: 12px; z-index: 10; display: flex; align-items: center; gap: 6px;">
                  <button
                    id="image-zoom-btn"
                    onClick={() => {
                      setImageZoomed(true);
                      setZoomScale(1.5);
                    }}
                    style="display: flex; align-items: center; gap: 4px; padding: 6px 12px; background: rgba(15, 23, 42, 0.85); color: #ffffff; border: none; border-radius: 6px; font-size: 12px; font-weight: 500; cursor: pointer; backdrop-filter: blur(4px); transition: background 0.15s ease;"
                    onMouseEnter={(e) => (e.currentTarget.style.background = '#0f172a')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'rgba(15, 23, 42, 0.85)')}
                    title="放大查看图片"
                  >
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                      <circle cx="11" cy="11" r="8"></circle>
                      <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                      <line x1="11" y1="8" x2="11" y2="14"></line>
                      <line x1="8" y1="11" x2="14" y2="11"></line>
                    </svg>
                    <span>放大</span>
                  </button>
                </div>

                <img
                  src={currentFile().imageUrl || DEFAULT_PREVIEW_IMAGE}
                  alt={currentFile().name}
                  style="max-width: 100%; max-height: 520px; border-radius: 8px; object-fit: contain;"
                />

                <div style="margin-top: 10px; font-size: 12px; color: #64748b; display: flex; align-items: center; gap: 12px;">
                  <span>🖼️ {currentFile().name}</span>
                  <span>• 1280 × 850 px</span>
                  <span>• 交付件</span>
                </div>
              </div>
            </div>
          }
        >
          {/* ══════════════════════════════════════════════════════════════════════════
             Diff 代码 / Markdown 差异渲染 (1:1 像素级还原图四红绿行与行号)
             ══════════════════════════════════════════════════════════════════════════ */}
          <div style="font-family: 'JetBrains Mono', 'Fira Code', Consolas, Monaco, monospace; font-size: 12.5px; line-height: 1.6; color: #1e293b;">
            <For each={diffLines()}>
              {(line) => {
                const isDel = line.type === 'deleted';
                const isAdd = line.type === 'added';
                const isHeader = line.type === 'header';

                return (
                  <div
                    style={`display: flex; align-items: stretch; width: 100%; border-bottom: 1px solid transparent; ${
                      isDel
                        ? 'background: #fee2e2; color: #991b1b;'
                        : isAdd
                        ? 'background: #dcfce7; color: #166534;'
                        : 'background: transparent; color: #334155;'
                    }`}
                  >
                    {/* 左侧行号 */}
                    <div style="width: 36px; padding: 2px 8px; text-align: right; color: #94a3b8; font-size: 11px; user-select: none; flex-shrink: 0; background: rgba(0,0,0,0.01);">
                      {line.leftNum || ''}
                    </div>

                    {/* 右侧行号 */}
                    <div style="width: 36px; padding: 2px 8px; text-align: right; color: #94a3b8; font-size: 11px; user-select: none; flex-shrink: 0; background: rgba(0,0,0,0.01);">
                      {line.rightNum || ''}
                    </div>

                    {/* 代码正文 */}
                    <div style="flex: 1; padding: 2px 12px; overflow-x: auto; white-space: pre-wrap; word-break: break-all; display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                      <span
                        style={
                          isHeader
                            ? 'font-weight: 700; color: #d97706;'
                            : isDel
                            ? 'color: #b91c1c;'
                            : isAdd
                            ? 'color: #15803d;'
                            : 'color: #1e293b;'
                        }
                      >
                        {line.content}
                      </span>

                      {/* 图四右侧蓝底加号标记 */}
                      <Show when={line.hasPlusBadge}>
                        <div
                          style="width: 18px; height: 18px; border-radius: 4px; background: #0284c7; color: #ffffff; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: bold; flex-shrink: 0; margin-right: 8px; user-select: none;"
                          title="已应用修改"
                        >
                          +
                        </div>
                      </Show>
                    </div>
                  </div>
                );
              }}
            </For>
          </div>
        </Show>
      </div>

      {/* ══════════════════════════════════════════════════════════════════════════
          全屏放大图片灯箱 Modal
         ══════════════════════════════════════════════════════════════════════════ */}
      <Show when={imageZoomed()}>
        <div
          onClick={() => setImageZoomed(false)}
          style="position: fixed; inset: 0; background: rgba(0, 0, 0, 0.85); backdrop-filter: blur(8px); z-index: 10000; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 20px;"
        >
          {/* 灯箱控制工具栏 */}
          <div
            onClick={(e) => e.stopPropagation()}
            style="position: absolute; top: 20px; right: 24px; display: flex; align-items: center; gap: 10px; background: rgba(30, 41, 59, 0.9); padding: 6px 14px; border-radius: 20px; color: #ffffff; font-size: 13px;"
          >
            <button
              onClick={() => setZoomScale((s) => Math.min(s + 0.25, 3))}
              style="background: transparent; border: none; color: #ffffff; cursor: pointer; font-size: 16px;"
              title="放大"
            >
              +
            </button>
            <span>{Math.round(zoomScale() * 100)}%</span>
            <button
              onClick={() => setZoomScale((s) => Math.max(s - 0.25, 0.5))}
              style="background: transparent; border: none; color: #ffffff; cursor: pointer; font-size: 16px;"
              title="缩小"
            >
              -
            </button>
            <button
              onClick={() => setZoomScale(1)}
              style="background: transparent; border: none; color: #94a3b8; cursor: pointer; font-size: 12px; margin-left: 4px;"
            >
              重置
            </button>
            <div style="width: 1px; height: 14px; background: #475569; margin: 0 4px;" />
            <button
              id="image-zoom-close-btn"
              onClick={() => setImageZoomed(false)}
              style="background: transparent; border: none; color: #ffffff; cursor: pointer; font-size: 16px;"
              title="退出全屏"
            >
              ✕
            </button>
          </div>

          <div
            onClick={(e) => e.stopPropagation()}
            style="max-width: 90vw; max-height: 85vh; overflow: auto; display: flex; align-items: center; justify-content: center;"
          >
            <img
              src={currentFile().imageUrl || DEFAULT_PREVIEW_IMAGE}
              alt={currentFile().name}
              style={`transform: scale(${zoomScale()}); transition: transform 0.2s ease-out; border-radius: 8px; box-shadow: 0 20px 50px rgba(0,0,0,0.5); max-width: 85vw; max-height: 80vh; object-fit: contain;`}
            />
          </div>
        </div>
      </Show>
    </div>
  );
};
