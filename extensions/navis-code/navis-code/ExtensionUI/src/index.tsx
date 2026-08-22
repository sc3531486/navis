import { createSignal, onMount, onCleanup, Show } from 'solid-js';
import type { NavisContext, NavisPlugin } from '@/core/context';
import { DynamicSlot } from '@/core/slots/DynamicSlot';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { toast } from '@/core/toast/ToastStore';
import { CommandPalette } from './components/CommandPalette';

// 产品级主布局：对标 Claude Code / Cowork 的极简暖白色桌面外壳
const StudioLayout = (props: { ctx: NavisContext }) => {
  const [sidebarOpen, setSidebarOpen] = createSignal(true);
  const [activeMode, setActiveMode] = createSignal<'cowork' | 'code'>('cowork');
  const [splitView, setSplitView] = createSignal(false);

  const handleWindowMinimize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    } catch (_) {
      toast.info('窗口已最小化 (浏览器预览)');
    }
  };

  const handleWindowMaximize = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().toggleMaximize();
    } catch (_) {
      if (document.fullscreenElement) {
        document.exitFullscreen?.();
      } else {
        document.documentElement.requestFullscreen?.();
      }
      toast.info('切换最大化/全屏');
    }
  };

  const handleWindowClose = async () => {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch (_) {
      toast.warning('关闭窗口 (浏览器环境)');
    }
  };

  const handleSplitToggle = () => {
    setSplitView(!splitView());
    toast.info(splitView() ? '已开启双栏分屏视图' : '已恢复单视口视图');
  };

  onMount(() => {
    const unsubMode = props.ctx.events.on('navis:mode:change', (payload: any) => {
      if (payload?.mode) {
        setActiveMode(payload.mode);
      }
    });

    onCleanup(() => unsubMode());
  });

  return (
    <div style="display: flex; flex-direction: column; width: 100vw; height: 100vh; overflow: hidden; background: #ffffff; color: #2d2b28; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;">
      {/* 顶部标题栏与窗口控制区 */}
      <div
        data-tauri-drag-region
        style="height: 38px; background: #ffffff; border-bottom: 1px solid #eae7e1; display: flex; align-items: center; justify-content: space-between; padding: 0 14px; user-select: none; z-index: 10;"
      >
        {/* 左侧导航与控制图标 */}
        <div style="display: flex; align-items: center; gap: 6px;">
          <button
            onClick={() => {
              setSidebarOpen(!sidebarOpen());
              toast.info(sidebarOpen() ? '已展开侧边栏' : '已收起侧边栏');
            }}
            style="background: transparent; border: none; font-size: 15px; color: #5a5750; cursor: pointer; padding: 4px 6px; border-radius: 4px; display: flex; align-items: center; justify-content: center;"
            title="切换侧边栏"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            ☰
          </button>
          <button
            onClick={handleSplitToggle}
            style={`border: none; font-size: 14px; cursor: pointer; padding: 4px 6px; border-radius: 4px; display: flex; align-items: center; justify-content: center; ${
              splitView() ? 'background: #eceae4; color: #1e1d1b;' : 'background: transparent; color: #5a5750;'
            }`}
            title="分屏视图"
            onMouseEnter={(e) => {
              if (!splitView()) e.currentTarget.style.background = '#f0eee8';
            }}
            onMouseLeave={(e) => {
              if (!splitView()) e.currentTarget.style.background = 'transparent';
            }}
          >
            ◫
          </button>
          <button
            onClick={() => props.ctx.commands.execute('command:palette')}
            style="background: transparent; border: none; font-size: 13px; color: #5a5750; cursor: pointer; padding: 4px 6px; border-radius: 4px; display: flex; align-items: center; justify-content: center;"
            title="全局搜索 (Ctrl+P)"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            🔍
          </button>
          <div style="width: 1px; height: 14px; background: #e7e4dc; margin: 0 2px;" />
          <button
            onClick={() => toast.info('后退到上一会话')}
            style="background: transparent; border: none; font-size: 13px; color: #8e8b83; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
            title="后退"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            ←
          </button>
          <button
            onClick={() => toast.info('前进到下一会话')}
            style="background: transparent; border: none; font-size: 13px; color: #8e8b83; cursor: pointer; padding: 2px 6px; border-radius: 4px;"
            title="前进"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            →
          </button>
        </div>

        {/* 中间窗口拖拽区域 */}
        <div
          data-tauri-drag-region
          style="flex: 1; height: 100%; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 500; color: #8e8b83; cursor: default;"
        >
          <span>Navis Code · {activeMode() === 'cowork' ? 'Cowork Workspace' : 'Code Studio'}</span>
        </div>

        {/* 右侧窗口最小化/最大化/关闭按钮 */}
        <div style="display: flex; align-items: center; gap: 2px;">
          <button
            onClick={handleWindowMinimize}
            style="background: transparent; border: none; color: #76736c; padding: 4px 8px; border-radius: 4px; font-size: 12px; cursor: pointer;"
            title="最小化"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            ─
          </button>
          <button
            onClick={handleWindowMaximize}
            style="background: transparent; border: none; color: #76736c; padding: 4px 8px; border-radius: 4px; font-size: 12px; cursor: pointer;"
            title="最大化"
            onMouseEnter={(e) => (e.currentTarget.style.background = '#f0eee8')}
            onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
          >
            □
          </button>
          <button
            onClick={handleWindowClose}
            style="background: transparent; border: none; color: #76736c; padding: 4px 8px; border-radius: 4px; font-size: 14px; cursor: pointer;"
            title="关闭"
            onMouseEnter={(e) => {
              e.currentTarget.style.background = '#ef4444';
              e.currentTarget.style.color = '#ffffff';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = '#76736c';
            }}
          >
            ✕
          </button>
        </div>
      </div>

      {/* 主体工作区（左侧边栏 + 中央主视口） */}
      <div style="flex: 1; display: flex; overflow: hidden; min-height: 0;">
        {/* 左侧边栏：容纳 SessionList / Explorer */}
        <div
          style={`width: 250px; min-width: 230px; max-width: 320px; background: #f7f6f2; border-right: 1px solid #eae7e1; display: ${
            sidebarOpen() ? 'flex' : 'none'
          }; flex-direction: column; overflow: hidden; height: 100%; min-height: 0;`}
        >
          <DynamicSlot
            name="navis-code.sidebar.left"
            class="navis-sidebar-slot"
            fallback={<div style="padding: 16px; color: #8e8b83; font-size: 12px;">加载侧边栏...</div>}
          />
        </div>

        {/* 中央主视口：容纳 Timeline 欢迎页与 Composer 悬浮卡片 */}
        <div style="flex: 1; display: flex; flex-direction: row; background: #ffffff; overflow: hidden; position: relative; height: 100%; min-height: 0;">
          <div style="flex: 1; display: flex; flex-direction: column; height: 100%; min-height: 0; position: relative; overflow: hidden;">
            <DynamicSlot
              name="navis-code.viewport.main"
              class="navis-main-slot"
              fallback={<div style="padding: 32px; color: #8e8b83; font-size: 13px;">加载主工作区...</div>}
            />
          </div>

          {/* 分屏视图：右侧编辑器辅助视口 */}
          <Show when={splitView()}>
            <div style="width: 45%; border-left: 1px solid #eae7e1; display: flex; flex-direction: column; height: 100%; min-height: 0; background: #faf9f6;">
              <DynamicSlot
                name="navis-code.viewport.editor"
                fallback={
                  <div style="padding: 24px; color: #8e8b83; font-size: 13px; display: flex; flex-direction: column; gap: 8px;">
                    <b style="color: #2d2b28;">代码编辑器 (Editor Split)</b>
                    <span>支持 Monaco / CodeMirror 协同渲染与实时 Diff。</span>
                  </div>
                }
              />
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
};

const DialogHost = () => <div class="navis-dialog-host" />;

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Initializing Claude Code / Cowork layout...');

    // 绑定清单 slots 引用的具名组件
    componentRegistry.bind('navis-code', {
      StudioLayout: () => <StudioLayout ctx={ctx} />,
      DialogHost: () => <DialogHost />,
      CommandPalette: () => <CommandPalette ctx={ctx} />,
    });

    // 注册产品壳根视口投影
    ctx.views.register('root', {
      id: 'navis-code.layout.studio',
      pluginId: 'navis-code',
      priority: 10,
      component: () => <StudioLayout ctx={ctx} />,
    });

    // 注册全局命令面板到浮层插槽
    ctx.views.register('overlay', {
      id: 'navis-code.command-palette',
      pluginId: 'navis-code',
      priority: 90,
      component: () => <CommandPalette ctx={ctx} />,
    });
  },
};

export default NavisCodeExtension;