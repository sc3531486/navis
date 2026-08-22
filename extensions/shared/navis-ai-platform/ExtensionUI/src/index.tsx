// Navis AI Platform 扩展入口：提供模型网关、MCP/LSP 管理与工具网关
import type { NavisContext, NavisPlugin } from '@/core/context';
import { contributionRegistry } from '@/core/manifest/ContributionRegistry';
import { toolRegistry } from './tools/ToolRegistry';

export const NavisAIPlatformExtension: NavisPlugin = {
  name: 'navis-ai-platform',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-ai-platform] Initializing AI Platform extension...');

    // 动态监听清单中的 tools 贡献点
    contributionRegistry.registerHandler('tools', (data, { pluginId }) => {
      for (const tool of (data as any[]) ?? []) {
        toolRegistry.register({
          pluginId,
          name: tool.name,
          description: tool.description,
          parameters: tool.parameters,
        });
      }
    });

    // 注入工具网关服务
    ctx.services.provide('toolRegistry', toolRegistry);

    // 注册 AI 相关通用命令
    ctx.commands.register('ai:models', () => {
      return [
        { id: 'claude-3-5-sonnet', name: 'Claude 3.5 Sonnet', provider: 'Anthropic' },
        { id: 'gpt-4o', name: 'GPT-4o', provider: 'OpenAI' },
        { id: 'deepseek-v3', name: 'DeepSeek V3', provider: 'DeepSeek' },
      ];
    });

    ctx.commands.register('tool:invoke', async (args: { name: string; params: any }) => {
      return toolRegistry.invoke(args.name, args.params);
    });
  },
};

export default NavisAIPlatformExtension;
