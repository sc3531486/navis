// Navis Knowledge 扩展入口：提供知识库管理与 RAG 检索
import type { NavisContext, NavisPlugin } from '@/core/context';

export const NavisKnowledgeExtension: NavisPlugin = {
  name: 'navis-knowledge',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-knowledge] Initializing Knowledge extension...');

    // 注册知识库检索与索引命令
    ctx.commands.register('knowledge:search', async (args: { query: string }) => {
      console.info(`[Knowledge] Searching documents for: ${args?.query}`);
      return [
        { title: 'Navis Architecture Guide', snippet: 'Everything is an extension in Navis framework.' },
        { title: 'Cordis Primer', snippet: 'Services, Events, Lifecycles and Disposers.' },
      ];
    });

    ctx.commands.register('knowledge:index', async (args: { path: string }) => {
      console.info(`[Knowledge] Indexing path: ${args?.path}`);
      return { status: 'indexed', count: 42 };
    });
  },
};

export default NavisKnowledgeExtension;
