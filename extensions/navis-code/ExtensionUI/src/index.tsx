import type { NavisContext, NavisPlugin } from '../../../../src/core/context';
import { DynamicSlot } from '../../../../src/core/slots/DynamicSlot';
import { componentRegistry } from '../../../../src/core/components/ComponentRegistry';

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

// 产品级布局：嵌套 DynamicSlot 树，为其他扩展提供可挂载的子插槽
const StudioLayout = (props: { ctx: NavisContext }) => {
  void props.ctx;
  return (
    <div class="navis-code-studio-root">
      <div class="navis-code-body-grid">
        <DynamicSlot name="navis-code.sidebar.left" class="navis-code-sidebar-container" />
        <DynamicSlot name="navis-code.viewport.main" class="navis-code-main-container" />
      </div>
      <DynamicSlot name="navis-code.statusbar" class="navis-code-statusbar-container" />
    </div>
  );
};

const DialogHost = () => <div class="navis-dialog-host" />;

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Binding studio layout components...');

    // 绑定清单 slots 引用的具名组件（root/overlay 插槽由贡献分发阶段注册）
    componentRegistry.bind('navis-code', {
      StudioLayout: () => <StudioLayout ctx={ctx} />,
      DialogHost: () => <DialogHost />,
    });

    // 注册产品自身提供的子插槽内容（动态创建的子插槽）
    ctx.views.register('navis-code.sidebar.left', {
      id: 'navis-code.sidebar',
      pluginId: 'navis-code',
      priority: 10,
      component: () => <SidebarPlaceholder />,
    });
    ctx.views.register('navis-code.viewport.main', {
      id: 'navis-code.main',
      pluginId: 'navis-code',
      priority: 10,
      component: () => <MainPlaceholder />,
    });
    ctx.views.register('navis-code.statusbar', {
      id: 'navis-code.statusbar',
      pluginId: 'navis-code',
      priority: 10,
      component: () => <StatusbarPlaceholder />,
    });

    // 命令（冒号命名与清单 commandId 对齐）
    ctx.commands.register('navis-code:new-session', () => {
      ctx.events.emit('session:create', { timestamp: Date.now() });
    });
    ctx.commands.register('navis-code:open-settings', () => {
      ctx.events.emit('settings:open', {});
    });
  },
};

export default NavisCodeExtension;