// Navis Task 扩展入口：提供任务看板与执行计划跟踪
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { TaskBoard } from './components/TaskBoard';

export const NavisTaskExtension: NavisPlugin = {
  name: 'navis-task',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-task] Initializing Task extension...');

    componentRegistry.bind('navis-task', {
      TaskPanel: () => <TaskBoard ctx={ctx} />,
      TaskBoard: () => <TaskBoard ctx={ctx} />,
    });

    ctx.views.register('navis-code.viewport.task', {
      id: 'navis-task.board',
      pluginId: 'navis-task',
      priority: 40,
      component: () => <TaskBoard ctx={ctx} />,
    });

    ctx.commands.register('task:add', (args: { title: string }) => {
      console.info(`[Task] Adding task: ${args?.title}`);
    });
  },
};

export default NavisTaskExtension;
