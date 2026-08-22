// 递归自描述插槽引擎：框架无关的插槽注册中心。
// 组件以 thunk 形式注入，渲染层决定如何挂载（当前宿主为 SolidJS）。
export interface SlotContribution<T = unknown> {
  id: string;
  pluginId: string;
  priority: number;
  /** 组件工厂：调用一次返回渲染元素 */
  component: () => T;
  props?: Record<string, any>;
}

export type SlotRegistration = Omit<SlotContribution, 'priority'> & { priority?: number };

class SlotStore {
  private slots = new Map<string, SlotContribution[]>();
  private listeners = new Set<() => void>();

  /** 注册一个插槽贡献；返回注销函数 */
  register<T>(slotName: string, contribution: SlotRegistration): () => void {
    const list = this.slots.get(slotName) ?? [];
    const entry: SlotContribution = {
      ...contribution,
      priority: contribution.priority ?? 100,
    };
    const existingIndex = list.findIndex(
      (s) => s.id === entry.id && s.pluginId === entry.pluginId,
    );
    if (existingIndex !== -1) {
      list[existingIndex] = entry;
    } else {
      list.push(entry);
    }
    list.sort((a, b) => a.priority - b.priority);
    this.slots.set(slotName, list);
    this.notify();
    return () => {
      const idx = list.findIndex(
        (s) => s.id === entry.id && s.pluginId === entry.pluginId,
      );
      if (idx !== -1) {
        list.splice(idx, 1);
        this.notify();
      }
    };
  }

  /** 读取某插槽的全部贡献（按 priority 升序） */
  getContributions<T = unknown>(slotName: string): SlotContribution<T>[] {
    return (this.slots.get(slotName) ?? []) as SlotContribution<T>[];
  }

  has(slotName: string): boolean {
    return (this.slots.get(slotName)?.length ?? 0) > 0;
  }

  /** 全部插槽名（含已注册贡献的） */
  listSlots(): string[] {
    return [...this.slots.keys()];
  }

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    this.listeners.forEach((fn) => {
      try {
        fn();
      } catch (err) {
        console.error('[SlotStore] listener error:', err);
      }
    });
  }
}

/** 全局插槽注册中心单例 */
export const slotStore = new SlotStore();