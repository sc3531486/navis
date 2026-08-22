// Navis Session 扩展入口：提供会话管理与侧边栏列表
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { SessionList } from './components/SessionList';

export const NavisSessionExtension: NavisPlugin = {
  name: 'navis-session',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-session] Initializing Session extension...');

    // 绑定组件
    componentRegistry.bind('navis-session', {
      SessionList: () => <SessionList ctx={ctx} />,
    });

    // 注册左侧栏视图投影
    ctx.views.register('navis-code.sidebar.left', {
      id: 'navis-session.list',
      pluginId: 'navis-session',
      priority: 30,
      component: () => <SessionList ctx={ctx} />,
    });

    ctx.commands.register('session:new', () => {
      ctx.events.emit('session:create', { timestamp: Date.now() });
    });
  },
};

export default NavisSessionExtension;
