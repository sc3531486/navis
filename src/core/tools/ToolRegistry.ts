// 统一工具网关：收集所有扩展声明的工具元数据。
// 工具执行统一走 core_route_ipc（路由到声明它的插件进程），宿主不硬编码任何工具。
import { coreRouteIpc } from '../tauri-bridge';

export interface NavisTool {
  pluginId: string;
  name: string;
  description?: string;
  parameters?: any;
}

class ToolRegistry {
  private tools = new Map<string, NavisTool[]>();

  register(tool: NavisTool): void {
    const list = this.tools.get(tool.name) ?? [];
    list.push(tool);
    this.tools.set(tool.name, list);
  }

  get(name: string): NavisTool | undefined {
    return this.tools.get(name)?.[0];
  }

  /** 执行工具：路由到声明它的插件后端进程 */
  async invoke(name: string, params: any): Promise<any> {
    const tool = this.get(name);
    if (!tool) {
      throw new Error(`[Navis Tools] Tool "${name}" not found`);
    }
    return coreRouteIpc(tool.pluginId, `tool.${name}`, params);
  }

  list(): NavisTool[] {
    return [...this.tools.values()].flat();
  }
}

export const toolRegistry = new ToolRegistry();