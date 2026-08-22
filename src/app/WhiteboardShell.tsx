import { Component, type JSX } from 'solid-js';
import { NavisContext } from '../core/context';
import { DynamicSlot } from '../core/slots/DynamicSlot';
import { ToastContainer } from '../core/toast/ToastContainer';
import './WhiteboardShell.css';

interface WhiteboardShellProps {
  ctx: NavisContext;
  brandTitle?: string;
  brandIcon?: string;
}

export const WhiteboardShell: Component<WhiteboardShellProps> = (props) => {
  const brandTitle = () => props.brandTitle ?? 'Navis Whiteboard';
  const brandIcon = () => props.brandIcon ?? '/icons/NAVIS.png';

  return (
    <div class="navis-whiteboard-shell">
      <DynamicSlot
        name="root"
        class="navis-root-viewport"
        fallback={
          <div class="navis-empty-canvas">
            <div class="navis-canvas-card">
              <img src={brandIcon()} alt="Navis Logo" class="navis-canvas-logo" />
              <h1 class="navis-canvas-title">{brandTitle()}</h1>
              <p class="navis-canvas-desc">
                通用应用白板运行时已就绪。当前未挂载业务插件。
              </p>
              <div class="navis-canvas-hints">
                <span>可通过插件向 <code>root</code> 或自定义命名空间动态注入 UI 与业务能力。</span>
              </div>
            </div>
          </div>
        }
      />
      <DynamicSlot name="overlay" class="navis-overlay-layer" />
      <ToastContainer />
    </div>
  );
};

export default WhiteboardShell;