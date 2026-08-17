import { Context, type Disposable, type Plugin } from '@cordisjs/core';

/**
 * 前端扩展组合根：基于 @cordisjs/core 的轻量 Cordis 宿主。
 *
 * 固定目录约定与后端一致：
 * - ExtensionUI/  前端扩展（本宿主承载其插件纤维）
 * - ExtensionBackend/  后端扩展（由 Rust 侧承载，不在此处注册）
 */
export type ExtensionCordisPlugin = Plugin<Context, any>;

export interface ExtensionCordisHost {
  readonly context: Context;
  provideService<T>(name: string, value: T): Disposable;
  getService<T>(name: string): T | undefined;
  requireService<T>(name: string): T;
  registerExtensionPlugin(extensionId: string, plugin: ExtensionCordisPlugin, config?: any): Disposable;
  disposeExtension(extensionId: string): void;
  disposeAll(): void;
}

export function createExtensionCordisContext(): ExtensionCordisHost {
  const context = new Context({ name: 'navis-extension-host' });
  const extensionDisposers = new Map<string, Disposable[]>();
  const serviceDisposers = new Map<string, Disposable>();

  function disposeExtension(extensionId: string): void {
    const disposers = extensionDisposers.get(extensionId);
    if (!disposers || disposers.length === 0) return;
    extensionDisposers.delete(extensionId);
    for (const dispose of [...disposers]) dispose();
  }

  function disposeAll(): void {
    for (const extensionId of [...extensionDisposers.keys()]) disposeExtension(extensionId);
  }

  function provideService<T>(name: string, value: T): Disposable {
    serviceDisposers.get(name)?.();

    let disposer: Disposable = () => {};
    const wrapped: Disposable = () => {
      if (serviceDisposers.get(name) === wrapped) serviceDisposers.delete(name);
      disposer();
    };

    disposer = context.set(name, value);
    serviceDisposers.set(name, wrapped);
    return wrapped;
  }

  function getService<T>(name: string): T | undefined {
    return context.get(name) as T | undefined;
  }

  function requireService<T>(name: string): T {
    const value = context.get(name);
    if (value === undefined) {
      throw new Error(`Cordis service "${name}" is not available`);
    }
    return value as T;
  }

  function registerExtensionPlugin(
    extensionId: string,
    plugin: ExtensionCordisPlugin,
    config?: any,
  ): Disposable {
    disposeExtension(extensionId);

    const scope = context.plugin(plugin, config);
    const disposer: Disposable = () => {
      scope.dispose();
    };

    const tracked = extensionDisposers.get(extensionId) ?? [];
    tracked.push(disposer);
    extensionDisposers.set(extensionId, tracked);
    return disposer;
  }

  return {
    context,
    provideService,
    getService,
    requireService,
    registerExtensionPlugin,
    disposeExtension,
    disposeAll,
  };
}


