import type { NavisContext } from './context';

let invoke: ((cmd: string, args?: any) => Promise<any>) | null = null;

async function getInvoke() {
  if (!invoke) {
    const tauri = await import('@tauri-apps/api/core');
    invoke = tauri.invoke;
  }
  return invoke;
}

export async function initBridge(ctx: NavisContext): Promise<void> {
  // 注册命令桥：前端 executeCommand 自动调用后端 RPC
  const originalExecute = ctx.executeCommand.bind(ctx);
  ctx.executeCommand = async (id: string, args?: any) => {
    // 先尝试本地命令
    try {
      await originalExecute(id, args);
      return;
    } catch (_) {
      // 本地没有，调用后端
    }
    try {
      const invokeFn = await getInvoke();
      const result = await invokeFn('navis_dispatch_rpc', {
        route: id,
        payload: args || {},
      });
      ctx.emit(`command:${id}:result`, result);
    } catch (err) {
      console.error(`[Tauri Bridge] Command "${id}" failed:`, err);
    }
  };

  // 注册 Tauri 事件监听桥
  try {
    const tauri = await import('@tauri-apps/api/event');
    await tauri.listen('navis:event', (event: any) => {
      ctx.emit(event.payload?.type || 'unknown', event.payload?.data);
    });
  } catch (_) {
    console.warn('[Tauri Bridge] Event listener not available');
  }

  console.info('[Tauri Bridge] Initialized');
}

export async function listExtensions(): Promise<any[]> {
  try {
    const invokeFn = await getInvoke();
    return await invokeFn('navis_list_extensions');
  } catch (_) {
    return [];
  }
}

export async function listRoutes(): Promise<string[]> {
  try {
    const invokeFn = await getInvoke();
    return await invokeFn('navis_list_routes');
  } catch (_) {
    return [];
  }
}

/** 直接调用后端进程内动态 RPC 路由（命令桥底层） */
export async function callRemote(route: string, payload?: any): Promise<any> {
  const invokeFn = await getInvoke();
  return invokeFn('navis_dispatch_rpc', { route, payload: payload ?? {} });
}

/** 统一通信协议：{ extension, action, data }，宿主按目标扩展自动路由 */
export async function navisDispatch(
  extension: string,
  action: string,
  data?: any,
): Promise<any> {
  const invokeFn = await getInvoke();
  return invokeFn('navis_dispatch', { extension, action, data: data ?? {} });
}

/** 当前激活的产品配置 */
export async function activeProduct(): Promise<any> {
  try {
    const invokeFn = await getInvoke();
    return await invokeFn('navis_active_product');
  } catch (_) {
    return null;
  }
}

/** 通用 IPC 路由：同步请求路由到插件进程 */
export async function coreRouteIpc(
  pluginId: string,
  method: string,
  params: any,
): Promise<any> {
  const invokeFn = await getInvoke();
  return invokeFn('core_route_ipc', { pluginId, method, params });
}

/** 通用 IPC 路由：流式请求路由到插件进程，事件经 Channel 推送 */
export async function coreRouteStream(
  pluginId: string,
  method: string,
  params: any,
  onEvent: (event: any) => void,
): Promise<void> {
  const invokeFn = await getInvoke();
  const tauri = await import('@tauri-apps/api/core');
  const { Channel } = tauri;
  const channel = new Channel<any>();
  channel.onmessage = onEvent;
  await invokeFn('core_route_stream', { pluginId, method, params, onEvent: channel });
}

/** 运行中的插件进程清单 */
export async function listProcesses(): Promise<string[]> {
  try {
    const invokeFn = await getInvoke();
    return await invokeFn('navis_list_processes');
  } catch (_) {
    return [];
  }
}

/** 沙箱权限审计日志 */
export async function auditLog(): Promise<any[]> {
  try {
    const invokeFn = await getInvoke();
    return await invokeFn('navis_audit_log');
  } catch (_) {
    return [];
  }
}
