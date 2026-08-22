// 通用 Navis 上下文容器（基于 Cordis 原语设计）
// 提供 IoC 服务依赖注入、类型化事件分发（emit/waterfall/serial/parallel）、插槽投影与命令注册。
import type { JSX } from 'solid-js';
import { slotStore, type SlotContribution } from './slots/SlotStore';

export type EventHandler<T = any> = (payload: T) => void | Promise<void>;
export type WaterfallHandler<T = any> = (payload: T, next: () => Promise<T>) => Promise<T> | T;
export type CommandHandler = (args?: any) => any | Promise<any>;

export interface SlotItem {
  id: string;
  pluginId?: string;
  priority?: number;
  component: () => JSX.Element;
}

export interface NavisPlugin {
  name: string;
  apply: (ctx: NavisContext) => void | Promise<void>;
}

export class NavisContext {
  private servicesMap = new Map<string, any>();
  private listeners = new Map<string, Set<EventHandler>>();
  private waterfalls = new Map<string, WaterfallHandler[]>();
  private commandsMap = new Map<string, CommandHandler>();
  private disposers: Array<() => void> = [];

  // ==================== IoC 服务容器 (Services DI) ====================

  provide<T>(name: string, service: T): void {
    this.servicesMap.set(name, service);
    this.emit(`service:${name}:ready`, service);
  }

  use<T>(name: string): T {
    const service = this.servicesMap.get(name);
    if (!service) {
      throw new Error(`[Navis Context] Service "${name}" is not registered.`);
    }
    return service as T;
  }

  has(name: string): boolean {
    return this.servicesMap.has(name);
  }

  // ==================== 事件总线 (Cordis Events) ====================

  on(event: string, handler: EventHandler): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(handler);
    return () => {
      this.listeners.get(event)?.delete(handler);
    };
  }

  emit(event: string, payload?: any): void {
    const handlers = this.listeners.get(event);
    if (handlers) {
      handlers.forEach((h) => {
        try {
          h(payload);
        } catch (err) {
          console.error(`[Navis Context] Error in handler for event "${event}":`, err);
        }
      });
    }
  }

  /** 串行异步执行所有监听器 */
  async serial(event: string, payload?: any): Promise<void> {
    const handlers = this.listeners.get(event);
    if (handlers) {
      for (const h of handlers) {
        await h(payload);
      }
    }
  }

  /** 并行异步执行所有监听器 */
  async parallel(event: string, payload?: any): Promise<void> {
    const handlers = this.listeners.get(event);
    if (handlers) {
      const promises = Array.from(handlers).map((h) => Promise.resolve(h(payload)));
      await Promise.all(promises);
    }
  }

  /** 注册 Waterfall 中间件处理器（下游通过 next() 逐级包装） */
  waterfallHook(event: string, handler: WaterfallHandler): () => void {
    if (!this.waterfalls.has(event)) {
      this.waterfalls.set(event, []);
    }
    const list = this.waterfalls.get(event)!;
    list.push(handler);
    return () => {
      const idx = list.indexOf(handler);
      if (idx !== -1) list.splice(idx, 1);
    };
  }

  /** 触发 Waterfall 中间件流水线 */
  async waterfall<T = any>(event: string, initialValue: T): Promise<T> {
    const handlers = this.waterfalls.get(event) ?? [];
    let index = 0;

    const dispatch = async (current: T): Promise<T> => {
      if (index >= handlers.length) {
        return current;
      }
      const handler = handlers[index++];
      return handler(current, () => dispatch(current));
    };

    return dispatch(initialValue);
  }

  /** 注册副作用及清理函数 */
  effect(fn: () => (() => void) | void): () => void {
    const cleanup = fn();
    const disposer = typeof cleanup === 'function' ? cleanup : () => {};
    this.disposers.push(disposer);
    return () => {
      const idx = this.disposers.indexOf(disposer);
      if (idx !== -1) this.disposers.splice(idx, 1);
      disposer();
    };
  }

  // ==================== 插槽与视图投影 (Views & Slots) ====================

  registerSlot(target: string, item: SlotItem): () => void {
    const unsub = slotStore.register(target, {
      id: item.id,
      pluginId: item.pluginId ?? 'host',
      priority: item.priority ?? 100,
      component: item.component as () => unknown,
    });
    this.emit(`slot:${target}:updated`, this.getSlotItems(target));
    return () => {
      unsub();
      this.emit(`slot:${target}:updated`, this.getSlotItems(target));
    };
  }

  getSlotItems(target: string): SlotItem[] {
    return slotStore.getContributions(target) as unknown as SlotItem[];
  }

  // ==================== 通用命令桥 (Commands) ====================

  registerCommand(id: string, handler: CommandHandler): () => void {
    this.commandsMap.set(id, handler);
    this.emit('command:registered', id);
    return () => {
      this.commandsMap.delete(id);
      this.emit('command:unregistered', id);
    };
  }

  executeCommand = async (id: string, args?: any): Promise<any> => {
    const cmd = this.commandsMap.get(id);
    if (cmd) {
      return cmd(args);
    }
    console.warn(`[Navis Context] Command "${id}" not found.`);
    return undefined;
  };

  // ==================== 插件装载 (Plugin) ====================

  async plugin(plugin: NavisPlugin): Promise<void> {
    console.info(`[Navis Engine] Applying plugin: ${plugin.name}`);
    await plugin.apply(this);
    this.emit(`plugin:${plugin.name}:mounted`);
  }

  // ==================== 命名空间别名 (Context DI 契约) ====================

  readonly views = {
    register: (target: string, item: SlotItem) => this.registerSlot(target, item),
    items: (target: string) => this.getSlotItems(target),
  };

  readonly commands = {
    register: (id: string, handler: CommandHandler) => this.registerCommand(id, handler),
    execute: async (id: string, args?: any) => this.executeCommand(id, args),
  };

  readonly events = {
    on: (event: string, handler: EventHandler) => this.on(event, handler),
    emit: (event: string, payload?: any) => this.emit(event, payload),
    serial: (event: string, payload?: any) => this.serial(event, payload),
    parallel: (event: string, payload?: any) => this.parallel(event, payload),
    waterfallHook: (event: string, handler: WaterfallHandler) => this.waterfallHook(event, handler),
    waterfall: <T = any>(event: string, initial: T) => this.waterfall<T>(event, initial),
  };

  readonly services = {
    provide: <T>(name: string, service: T) => this.provide(name, service),
    use: <T>(name: string) => this.use<T>(name),
    has: (name: string) => this.has(name),
  };
}

export const rootContext = new NavisContext();