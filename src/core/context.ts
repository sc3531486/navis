import type { JSX } from 'solid-js';

export type EventHandler<T = any> = (payload: T) => void | Promise<void>;

export interface SlotItem {
  id: string;
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
  private slotRegistry = new Map<string, SlotItem[]>();
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
    if (!this.slotRegistry.has(target)) {
      this.slotRegistry.set(target, []);
    }
    const list = this.slotRegistry.get(target)!;
    list.push({ ...item, priority: item.priority ?? 100 });
    list.sort((a, b) => (a.priority ?? 100) - (b.priority ?? 100));
    this.emit(`slot:${target}:updated`, list);

    return () => {
      const idx = list.findIndex((s) => s.id === item.id);
      if (idx !== -1) {
        list.splice(idx, 1);
        this.emit(`slot:${target}:updated`, list);
      }
    };
  }

  getSlotItems(target: string): SlotItem[] {
    return this.slotRegistry.get(target) || [];
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