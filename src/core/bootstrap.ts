// 宿主启动生命周期：桥接 -> 安装默认贡献处理器 -> 清单发现与分发 -> 装载 UI 插件。
import type { NavisContext } from './context';
import { initBridge, listExtensions } from './tauri-bridge';
import { installDefaultHandlers } from './manifest/handlers';
import { contributionRegistry } from './manifest/ContributionRegistry';
import type { ExtensionManifest } from './manifest/types';
import { loadExtensions } from './loader';

export async function bootstrap(ctx: NavisContext): Promise<void> {
  // 1. 初始化前后端通信桥梁
  await initBridge(ctx);

  // 2. 安装宿主默认贡献处理器（slots/commands/tools/pipelineHooks）
  installDefaultHandlers(ctx);

  // 3. 发现扩展并分发贡献点（后端返回完整清单，含 contributes 全量结构）
  const manifests = (await listExtensions()) as unknown as ExtensionManifest[];
  for (const manifest of manifests) {
    await contributionRegistry.dispatch(manifest, {
      pluginId: manifest.id,
      extensionPath: manifest.main ?? '',
      manifest,
    });
  }

  // 4. 装载扩展 UI 插件（import.meta.glob 静态打包，动态挂载）
  const plugins = await loadExtensions();
  for (const plugin of plugins) {
    await ctx.plugin(plugin);
  }
}