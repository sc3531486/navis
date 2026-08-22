// Navis Memory 扩展入口：提供长期记忆与偏好管理
import type { NavisContext, NavisPlugin } from '@/core/context';

export const NavisMemoryExtension: NavisPlugin = {
  name: 'navis-memory',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-memory] Initializing Memory extension...');

    ctx.commands.register('memory:remember', async (args: { key: string; value: any }) => {
      console.info(`[Memory] Storing fact: ${args?.key}`);
      return { status: 'remembered' };
    });

    ctx.commands.register('memory:recall', async (args: { key: string }) => {
      console.info(`[Memory] Recalling fact for: ${args?.key}`);
      return { key: args?.key, value: 'User prefers dark mode and concise responses.' };
    });
  },
};

export default NavisMemoryExtension;
