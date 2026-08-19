// 扩展组件注册中心：插件 UI 代码把具名组件绑定到 pluginId，供清单声明的
// slot.component / pipelineHooks.handler 按名字解析。框架无关，渲染层订阅更新。
export type ComponentThunk = (...args: any[]) => any;
export type NamedComponents = Record<string, ComponentThunk>;

class ComponentRegistry {
  private registry = new Map<string, NamedComponents>();
  private listeners = new Set<() => void>();

  bind(pluginId: string, components: NamedComponents): void {
    const existing = this.registry.get(pluginId) ?? {};
    this.registry.set(pluginId, { ...existing, ...components });
    this.notify();
  }

  get(pluginId: string, name: string): ComponentThunk | undefined {
    return this.registry.get(pluginId)?.[name];
  }

  has(pluginId: string, name: string): boolean {
    return this.registry.get(pluginId)?.[name] !== undefined;
  }

  list(pluginId: string): string[] {
    return Object.keys(this.registry.get(pluginId) ?? {});
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
        console.error('[ComponentRegistry] listener error:', err);
      }
    });
  }
}

export const componentRegistry = new ComponentRegistry();