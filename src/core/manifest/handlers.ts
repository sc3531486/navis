// 默认贡献点处理器：把清单声明接入插槽注册中心、命令桥、工具网关、Agent 管线。
import { contributionRegistry } from './ContributionRegistry';
import type {
  SlotContribution,
  CommandContribution,
  ToolContribution,
  PipelineHookContribution,
} from './types';
import { componentRegistry } from '../components/ComponentRegistry';
import { toolRegistry } from '../tools/ToolRegistry';
import { globalAgentPipeline, type PipelineHookName } from '../pipeline/AgentPipeline';
import { callRemote, coreRouteIpc } from '../tauri-bridge';
import type { NavisContext } from '../context';

export function installDefaultHandlers(ctx: NavisContext): void {
  // slots：注册插槽条目，组件按名从插件组件注册表延迟解析（绑定可能晚于声明）
  contributionRegistry.registerHandler('slots', (data, { pluginId }) => {
    for (const slot of (data as SlotContribution[]) ?? []) {
      ctx.registerSlot(slot.target, {
        id: slot.id,
        pluginId,
        priority: slot.priority,
        component: () => {
          const thunk = componentRegistry.get(pluginId, slot.component ?? slot.id);
          return thunk ? thunk() : null;
        },
      });
    }
  });

  // providesSlots：向系统登记新发布的插槽（供动态插槽树自描述、其他扩展挂载）
  contributionRegistry.registerHandler('providesSlots', (data, { pluginId }) => {
    const slots = (data as string[]) ?? [];
    ctx.emit('slots:provided', { pluginId, slots });
    console.info(`[Navis] Plugin '${pluginId}' provides slots: ${slots.join(', ')}`);
  });

  // commands：注册命令桩。
  // 有后端进程的插件 -> 路由 core_route_ipc 到进程（method = 命令 id）；
  // 无后端进程的插件 -> 路由 navis_dispatch_rpc 进程内 RPC。
  contributionRegistry.registerHandler('commands', (data, context) => {
    for (const cmd of (data as CommandContribution[]) ?? []) {
      const commandId = `${context.pluginId}:${cmd.id}`;
      if (context.manifest?.main) {
        ctx.registerCommand(commandId, async (args?: any) => {
          return coreRouteIpc(context.pluginId, cmd.id, args ?? {});
        });
      } else {
        ctx.registerCommand(commandId, async (args?: any) => {
          return callRemote(commandId, args ?? {});
        });
      }
    }
  });

  // tools：注册到统一工具网关（执行路由到声明它的插件进程）
  contributionRegistry.registerHandler('tools', (data, { pluginId }) => {
    for (const tool of (data as ToolContribution[]) ?? []) {
      toolRegistry.register({
        pluginId,
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
      });
    }
  });

  // pipelineHooks：把具名 handler 挂到 Agent 管线钩子上（由插件组件注册表解析）
  contributionRegistry.registerHandler('pipelineHooks', (data, { pluginId }) => {
    for (const h of (data as PipelineHookContribution[]) ?? []) {
      const hookName = h.hook as PipelineHookName;
      if (!(hookName in globalAgentPipeline.hooks)) {
        console.warn(`[Navis] Unknown pipeline hook '${h.hook}' from '${pluginId}'`);
        continue;
      }
      globalAgentPipeline.tap(hookName, `${pluginId}:${h.handler}`, (payload: any) => {
        const fn = componentRegistry.get(pluginId, h.handler);
        return fn ? fn(payload) : undefined;
      });
    }
  });
}