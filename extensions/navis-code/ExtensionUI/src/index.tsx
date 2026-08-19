import { NavisContext, NavisPlugin } from '../../../../src/core/context';
import { SlotRenderer } from '../../../../src/core/SlotRenderer';

// 占位组件（后续替换为实际业务组件）
const SidebarPlaceholder = () => (
  <div style="padding: 12px; color: #9ca3af; font-size: 12px;">
    <div style="margin-bottom: 8px; color: #e5e7eb; font-weight: 600;">Explorer</div>
    <div style="opacity: 0.6;">No folders opened</div>
  </div>
);

const MainPlaceholder = () => (
  <div style="flex: 1; display: flex; align-items: center; justify-content: center; color: #6b7280;">
    <div style="text-align: center;">
      <div style="font-size: 48px; margin-bottom: 16px; opacity: 0.3;">✦</div>
      <div style="font-size: 14px;">Ready to code</div>
    </div>
  </div>
);

const StatusbarPlaceholder = () => (
  <div style="font-size: 11px; color: #6b7280; display: flex; gap: 12px;">
    <span>Ready</span>
    <span>•</span>
    <span>Navis Code v1.0</span>
  </div>
);

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Registering Studio layout into root slot...');

    // 1. root 插槽：多栏子插槽树
    ctx.registerSlot('root', {
      id: 'navis-code.layout.root',
      priority: 10,
      component: () => (
        <div class="navis-code-studio-root">
          <div class="navis-code-body-grid">
            <SlotRenderer
              ctx={ctx}
              target="navis-code.sidebar.left"
              class="navis-code-sidebar-container"
            />
            <SlotRenderer
              ctx={ctx}
              target="navis-code.viewport.main"
              class="navis-code-main-container"
            />
          </div>
          <SlotRenderer
            ctx={ctx}
            target="navis-code.statusbar"
            class="navis-code-statusbar-container"
          />
        </div>
      )
    });

    // 2. 注入子插槽组件
    ctx.registerSlot('navis-code.sidebar.left', {
      id: 'navis-code.sidebar',
      priority: 10,
      component: () => <SidebarPlaceholder />
    });

    ctx.registerSlot('navis-code.viewport.main', {
      id: 'navis-code.main',
      priority: 10,
      component: () => <MainPlaceholder />
    });

    ctx.registerSlot('navis-code.statusbar', {
      id: 'navis-code.statusbar',
      priority: 10,
      component: () => <StatusbarPlaceholder />
    });

    // 3. 注册命令
    ctx.registerCommand('navis-code.new-session', () => {
      ctx.emit('session:create', { timestamp: Date.now() });
    });

    ctx.registerCommand('navis-code.open-settings', () => {
      ctx.emit('settings:open', {});
    });
  }
};

export default NavisCodeExtension;
