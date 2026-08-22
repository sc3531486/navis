// Navis Project 扩展入口：提供项目工作区与文件树
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { ProjectTree } from './components/ProjectTree';

export const NavisProjectExtension: NavisPlugin = {
  name: 'navis-project',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-project] Initializing Project extension...');

    componentRegistry.bind('navis-project', {
      ProjectTree: () => <ProjectTree ctx={ctx} />,
    });

    ctx.views.register('navis-code.sidebar.project', {
      id: 'navis-project.tree',
      pluginId: 'navis-project',
      priority: 10,
      component: () => <ProjectTree ctx={ctx} />,
    });

    ctx.commands.register('project:open-folder', () => {
      ctx.events.emit('project:folder-opened', { path: 'D:/myworkspace/Navis Go' });
    });
  },
};

export default NavisProjectExtension;
