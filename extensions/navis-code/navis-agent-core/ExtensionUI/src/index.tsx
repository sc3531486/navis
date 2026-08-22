// Navis Agent Core 业务扩展入口
import { createSignal } from 'solid-js';
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { Composer } from './components/Composer';
import { Timeline } from './components/Timeline';
import { ContextDrawer } from './components/ContextDrawer';
import { agentPipeline } from './pipeline/AgentPipeline';
import { toast } from '@/core/toast/ToastStore';

const AgentWorkspace = (props: { ctx: NavisContext }) => {
  const [sessionTitle] = createSignal('Repeated Chinese Greetings');

  return (
    <div style="display: flex; flex-direction: row; height: 100%; width: 100%; background: #ffffff; overflow: hidden;">
      {/* 中央主视口 (Header + 消息滚动流 + 底部 Composer 输入框) */}
      <div style="flex: 1; display: flex; flex-direction: column; height: 100%; min-width: 0; overflow: hidden; background: #ffffff;">
        {/* 1. 顶部面包屑与标题栏 */}
        <div
          style="height: 42px; border-bottom: 1px solid #f1f5f9; display: flex; align-items: center; justify-content: space-between; padding: 0 20px; user-select: none; background: #ffffff; flex-shrink: 0;"
        >
          <div style="display: flex; align-items: center; gap: 8px; font-size: 13px; color: #475569;">
            <span style="font-weight: 500; color: #1e293b;">Navis Go</span>
            <span style="color: #94a3b8;">/</span>
            <span style="color: #64748b;">{sessionTitle()}</span>
          </div>

          <div style="display: flex; align-items: center; gap: 8px;">
            <button
              onClick={() => toast.info('Navis IDE 集成套件已就绪')}
              style="display: flex; align-items: center; gap: 5px; padding: 4px 10px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 6px; font-size: 12px; font-weight: 500; color: #334155; cursor: pointer;"
              onMouseEnter={(e) => (e.currentTarget.style.background = '#f1f5f9')}
              onMouseLeave={(e) => (e.currentTarget.style.background = '#f8fafc')}
            >
              <span>🚀</span>
              <span>安装 IDE</span>
            </button>
            <button
              onClick={() => props.ctx.events.emit('settings:open', { tab: 'models' })}
              style="background: transparent; border: none; color: #64748b; padding: 4px 6px; border-radius: 4px; cursor: pointer; font-size: 14px;"
              title="更多选项"
            >
              ⋮
            </button>
          </div>
        </div>

        {/* 2. 消息流滚动视口 */}
        <div
          id="timeline-scroll-container"
          style="flex: 1; overflow-y: auto; min-height: 0; padding: 20px 28px 12px; display: flex; flex-direction: column; align-items: center; overscroll-behavior: contain;"
        >
          <Timeline ctx={props.ctx} />
        </div>

        {/* 3. 底部固定 Composer 输入控制栏 (绝不沉底遮挡，始终完整展示) */}
        <div style="width: 100%; padding: 4px 28px 16px; flex-shrink: 0; display: flex; justify-content: center; background: #ffffff;">
          <Composer ctx={props.ctx} />
        </div>
      </div>

      {/* 右侧上下文与交付件抽屉 */}
      <ContextDrawer ctx={props.ctx} />
    </div>
  );
};

export const NavisAgentCoreExtension: NavisPlugin = {
  name: 'navis-agent-core',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-agent-core] Initializing Agent Core extension...');

    componentRegistry.bind('navis-agent-core', {
      Composer: () => <Composer ctx={ctx} />,
      Timeline: () => <Timeline ctx={ctx} />,
      ContextDrawer: () => <ContextDrawer ctx={ctx} />,
      AgentWorkspace: () => <AgentWorkspace ctx={ctx} />,
    });

    ctx.views.register('navis-code.viewport.main', {
      id: 'navis-agent-core.workspace',
      pluginId: 'navis-agent-core',
      priority: 100,
      component: () => <AgentWorkspace ctx={ctx} />,
    });

    ctx.services.provide('agentPipeline', agentPipeline);

    ctx.commands.register('agent:run', async (payload: any) => {
      ctx.events.emit('agent:turn:start', payload ?? {});
    });

    ctx.commands.register('agent:status', () => {
      return { status: 'idle', activeAgents: 1 };
    });
  },
};

export default NavisAgentCoreExtension;
