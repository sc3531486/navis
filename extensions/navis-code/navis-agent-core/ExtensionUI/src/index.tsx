// Navis Agent Core 业务扩展入口
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { Composer } from './components/Composer';
import { Timeline } from './components/Timeline';
import { agentPipeline } from './pipeline/AgentPipeline';

const AgentWorkspace = (props: { ctx: NavisContext }) => {
  return (
    <div style="display: flex; flex-direction: column; height: 100%; width: 100%; background: #ffffff; position: relative; overflow: hidden;">
      {/* 消息与欢迎画布区 */}
      <Timeline ctx={props.ctx} />

      {/* 底部悬浮 Composer 输入框 */}
      <Composer ctx={props.ctx} />
    </div>
  );
};

export const NavisAgentCoreExtension: NavisPlugin = {
  name: 'navis-agent-core',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-agent-core] Initializing Agent Core extension...');

    // 绑定具名组件供插槽延迟解析
    componentRegistry.bind('navis-agent-core', {
      Composer: () => <Composer ctx={ctx} />,
      Timeline: () => <Timeline ctx={ctx} />,
      AgentWorkspace: () => <AgentWorkspace ctx={ctx} />,
    });

    // 注册主视口视图投影
    ctx.views.register('navis-code.viewport.main', {
      id: 'navis-agent-core.workspace',
      pluginId: 'navis-agent-core',
      priority: 100, // 设为最高优先级，作为主工作区首选
      component: () => <AgentWorkspace ctx={ctx} />,
    });

    // 注入 Agent 服务
    ctx.services.provide('agentPipeline', agentPipeline);

    // 注册 Agent 相关命令
    ctx.commands.register('agent:run', async (payload: any) => {
      ctx.events.emit('agent:turn:start', payload ?? {});
    });

    ctx.commands.register('agent:status', () => {
      return { status: 'idle', activeAgents: 1 };
    });
  },
};

export default NavisAgentCoreExtension;
