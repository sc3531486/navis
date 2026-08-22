// Agent 中间件拦截管线（业务扩展层持有）：原生轻量 Waterfall 风格钩子链。
// 宿主仅提供基础事件机制；Agent 执行流与拦截逻辑由本扩展全权自持有。
type AsyncTap = (payload: any) => any | Promise<any>;
type SyncTap = (...args: any[]) => void;

class AsyncSeriesWaterfallHook {
  private taps: Array<{ name: string; fn: AsyncTap }> = [];
  tap(name: string, fn: AsyncTap) {
    this.taps.push({ name, fn });
  }
  async promise(init: any): Promise<any> {
    let value = init;
    for (const t of this.taps) {
      const r = await t.fn(value);
      if (r !== undefined) value = r;
    }
    return value;
  }
}

class AsyncSeriesBailHook {
  private taps: Array<{ name: string; fn: AsyncTap }> = [];
  tap(name: string, fn: AsyncTap) {
    this.taps.push({ name, fn });
  }
  async promise(init: any): Promise<any> {
    for (const t of this.taps) {
      const r = await t.fn(init);
      if (r !== undefined) return r;
    }
    return undefined;
  }
}

class SyncHook {
  private taps: Array<{ name: string; fn: SyncTap }> = [];
  tap(name: string, fn: SyncTap) {
    this.taps.push({ name, fn });
  }
  call(...args: any[]) {
    for (const t of this.taps) t.fn(...args);
  }
}

export type PipelineHookName =
  | 'assembleContext'
  | 'beforeModelCall'
  | 'onStreamChunk'
  | 'beforeToolExecute'
  | 'afterToolExecute';

export interface AgentTurnContext {
  messages: any[];
  systemPrompt?: string;
  tools?: any[];
  [key: string]: any;
}

/** Agent 执行流开放管线 */
export class AgentPipeline {
  public hooks = {
    /** 上下文组装阶段（可被知识检索、记忆注入拦截） */
    assembleContext: new AsyncSeriesWaterfallHook(),
    /** 模型调用前（可被安全护栏、Mock 插件拦截） */
    beforeModelCall: new AsyncSeriesWaterfallHook(),
    /** 流式 Chunk 到达 */
    onStreamChunk: new SyncHook(),
    /** 工具执行拦截（可被权限鉴权、二次确认插件熔断） */
    beforeToolExecute: new AsyncSeriesBailHook(),
    /** 工具结果处理（可被格式化、错误自愈拦截） */
    afterToolExecute: new AsyncSeriesWaterfallHook(),
  };

  tap(hookName: PipelineHookName, name: string, fn: AsyncTap | SyncTap) {
    this.hooks[hookName].tap(name, fn as any);
  }

  async executeTurn(sessionContext: AgentTurnContext): Promise<any> {
    const ctx = await this.hooks.assembleContext.promise(sessionContext);
    const callConfig = await this.hooks.beforeModelCall.promise({
      payload: ctx,
      tools: (ctx as any).tools ?? [],
    });
    return callConfig;
  }
}

export const agentPipeline = new AgentPipeline();
