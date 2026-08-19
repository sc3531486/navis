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
  private services = new Map<string, any>();
  private listeners = new Map<string, Set<EventHandler>>();
  private commands = new Map<string, (args?: any) => void | Promise<void>>();

  provide<T>(name: string, service: T): void {
    this.services.set(name, service);
    this.emit(`service:${name}:ready`, service);
  }

  use<T>(name: string): T {
    const service = this.services.get(name);
    if (!service) {
      throw new Error(`[Navis Context] Service "${name}" is not registered.`);
    }
    return service as T;
  }

  has(name: string): boolean {
    return this.services.has(name);
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
    this.commands.set(id, handler);
    this.emit('command:registered', id);
    return () => {
      this.commands.delete(id);
      this.emit('command:unregistered', id);
    };
  }

  executeCommand(id: string, args?: any): Promise<void> | void {
    const cmd = this.commands.get(id);
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
}

export const rootContext = new NavisContext();