// Agent 中间件拦截管线：原生轻量 Tapable 风格钩子链（零依赖）。
// 宿主只提供机制；Agent 循环由扩展实现，通过钩子注入拦截逻辑（反思、压缩、鉴权等）。
type AsyncTap = (payload: any) => any | Promise<any>;
type SyncTap = (...args: any[]) => void;

class AsyncSeriesWaterfallHook {
  private taps: Array<{ name: string; fn: AsyncTap }> = [];
  tap(name: string, fn: AsyncTap) {
    this.taps.push({ name, fn });
  }
  /** 串行执行，前一结果作为后一输入（undefined 沿用） */
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
  /** 串行执行，任一返回非 undefined 立即短路并返回该值 */
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
    /** 上下文组装阶段（可被压缩、知识检索插件拦截） */
    assembleContext: new AsyncSeriesWaterfallHook(),
    /** 模型调用前（可被安全护栏、Mock 插件拦截） */
    beforeModelCall: new AsyncSeriesWaterfallHook(),
    /** 流式 Chunk 到达 */
    onStreamChunk: new SyncHook(),
    /** 工具执行拦截（可被鉴权、二次确认插件熔断或替换） */
    beforeToolExecute: new AsyncSeriesBailHook(),
    /** 工具结果处理（可被格式化、错误自愈插件拦截） */
    afterToolExecute: new AsyncSeriesWaterfallHook(),
  };

  /** 按钩子名注册拦截器 */
  tap(hookName: PipelineHookName, name: string, fn: AsyncTap | SyncTap) {
    this.hooks[hookName].tap(name, fn as any);
  }

  /** 执行一轮对话：上下文组装 -> 模型调用准备 -> 流式与工具循环（由扩展实现细节） */
  async executeTurn(sessionContext: AgentTurnContext): Promise<any> {
    const ctx = await this.hooks.assembleContext.promise(sessionContext);
    const callConfig = await this.hooks.beforeModelCall.promise({
      payload: ctx,
      tools: (ctx as any).tools ?? [],
    });
    return callConfig;
  }
}

/** 全局 Agent 管线单例 */
export const globalAgentPipeline = new AgentPipeline();