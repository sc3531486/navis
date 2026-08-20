import type { JSX } from 'solid-js';
import { slotStore, type SlotContribution } from './slots/SlotStore';

export type EventHandler<T = any> = (payload: T) => void | Promise<void>;

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
  private commandsMap = new Map<string, (args?: any) => void | Promise<void>>();

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

  registerCommand(id: string, handler: (args?: any) => void | Promise<void>): () => void {
    this.commandsMap.set(id, handler);
    this.emit('command:registered', id);
    return () => {
      this.commandsMap.delete(id);
      this.emit('command:unregistered', id);
    };
  }

  executeCommand(id: string, args?: any): Promise<void> | void {
    const cmd = this.commandsMap.get(id);
    if (cmd) {
      return cmd(args);
    }
    console.warn(`[Navis Context] Command "${id}" not found.`);
  }

  async plugin(plugin: NavisPlugin): Promise<void> {
    console.info(`[Navis Engine] Applying plugin: ${plugin.name}`);
    await plugin.apply(this);
    this.emit(`plugin:${plugin.name}:mounted`);
  }

  // 命名空间别名：对齐「前端 Context DI」契约（views.register / events.on）。
  // 扩展统一通过 ctx.views / ctx.commands / ctx.events / ctx.services 编程；
  // 上层扁平方法保留作为宿主内部与历史兼容入口。
  readonly views = {
    register: (target: string, item: SlotItem) => this.registerSlot(target, item),
    items: (target: string) => this.getSlotItems(target),
  };

  readonly commands = {
    register: (id: string, handler: (args?: any) => void | Promise<void>) =>
      this.registerCommand(id, handler),
    execute: (id: string, args?: any) => this.executeCommand(id, args),
  };

  readonly events = {
    on: (event: string, handler: EventHandler) => this.on(event, handler),
    emit: (event: string, payload?: any) => this.emit(event, payload),
  };

  readonly services = {
    provide: <T>(name: string, service: T) => this.provide(name, service),
    use: <T>(name: string) => this.use<T>(name),
    has: (name: string) => this.has(name),
  };
}

export const rootContext = new NavisContext();