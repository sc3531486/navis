// Navis Settings 扩展入口：提供设置弹窗与配置管理
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { SettingsModal } from './components/SettingsModal';

export const NavisSettingsExtension: NavisPlugin = {
  name: 'navis-settings',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-settings] Initializing Settings extension...');

    componentRegistry.bind('navis-settings', {
      SettingsDialog: () => <SettingsModal ctx={ctx} />,
      SettingsModal: () => <SettingsModal ctx={ctx} />,
    });

    ctx.views.register('overlay', {
      id: 'navis-settings.dialog',
      pluginId: 'navis-settings',
      priority: 100,
      component: () => <SettingsModal ctx={ctx} />,
    });

    ctx.commands.register('settings:open', () => {
      ctx.events.emit('settings:open', {});
    });
  },
};

export default NavisSettingsExtension;
