// 泛型贡献点分发中心：schema-agnostic。
// 宿主不硬编码解析 contributes 的固定字段，任何插件都可注册自己关心的贡献点。
import type { ExtensionManifest } from './types';

export type ContributionContext = {
  pluginId: string;
  extensionPath?: string;
  manifest?: ExtensionManifest;
};

export type ContributionHandler = (
  data: any,
  context: ContributionContext,
) => void | Promise<void>;

class ContributionRegistry {
  private handlers = new Map<string, ContributionHandler>();

  /** 核心或其他插件注册自己关心的 Manifest 字段 */
  registerHandler(contributeKey: string, handler: ContributionHandler) {
    this.handlers.set(contributeKey, handler);
  }

  /** 扫描到任何 extension.json 时按字段名通用分发 */
  async dispatch(manifest: ExtensionManifest, context: ContributionContext) {
    const contributes: Record<string, any> = (manifest.contributes ?? {}) as any;
    for (const [key, data] of Object.entries(contributes)) {
      const handler = this.handlers.get(key);
      if (handler) {
        await handler(data, context);
      } else {
        console.warn(`[Navis] No handler registered for contribution point: ${key}`);
      }
    }
  }
}

export const contributionRegistry = new ContributionRegistry();