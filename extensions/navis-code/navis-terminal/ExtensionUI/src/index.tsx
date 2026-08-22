// Navis Terminal 扩展入口：提供终端与 PTY 管理
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { TerminalPanel } from './components/TerminalPanel';

export const NavisTerminalExtension: NavisPlugin = {
  name: 'navis-terminal',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-terminal] Initializing Terminal extension...');

    componentRegistry.bind('navis-terminal', {
      Terminal: () => <TerminalPanel ctx={ctx} />,
      TerminalPanel: () => <TerminalPanel ctx={ctx} />,
    });

    ctx.views.register('navis-code.statusbar', {
      id: 'navis-terminal.status',
      pluginId: 'navis-terminal',
      priority: 50,
      component: () => (
        <span style="font-size: 11px; color: #888; cursor: pointer;">
          💻 Terminal: Idle
        </span>
      ),
    });

    ctx.commands.register('terminal:create', () => {
      ctx.events.emit('terminal:created', { id: `term-${Date.now()}` });
    });
  },
};

export default NavisTerminalExtension;
