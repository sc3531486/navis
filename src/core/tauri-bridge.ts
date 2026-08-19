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
