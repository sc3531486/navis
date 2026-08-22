// 通用宿主默认贡献点处理器：把清单声明接入通用插槽注册中心与命令桥。
// 垂直业务贡献点（如 tools/pipelineHooks）由对应扩展插件在运行时自行调用 contributionRegistry.registerHandler 注册。
import { contributionRegistry } from './ContributionRegistry';
import type { SlotContribution, CommandContribution } from './types';
import { componentRegistry } from '../components/ComponentRegistry';
import { navisDispatch } from '../tauri-bridge';
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

  // commands：注册命令桩，执行统一走 navis_dispatch（宿主自动路由到进程内或插件进程）
  contributionRegistry.registerHandler('commands', (data, context) => {
    for (const cmd of (data as CommandContribution[]) ?? []) {
      const commandId = `${context.pluginId}:${cmd.id}`;
      ctx.registerCommand(commandId, async (args?: any) => {
        return navisDispatch(context.pluginId, cmd.id, args ?? {});
      });
    }
  });
}