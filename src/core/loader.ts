// 运行时扩展装载器：开发期通过 Vite import.meta.glob 自动发现并装载扩展 UI 入口。
// 支持按产品套件与共享目录多层分组（extensions/**/ExtensionUI/src/index.tsx），无需修改任何宿主源码。
import type { NavisPlugin } from './context';

// 递归扫描 extensions/**/ExtensionUI/src/index.ts(x)，导出默认 NavisPlugin
const modules = import.meta.glob('../../extensions/**/ExtensionUI/src/index.{ts,tsx}', {
  eager: false,
});

export async function loadExtensions(): Promise<NavisPlugin[]> {
  const plugins: NavisPlugin[] = [];
  const paths = Object.keys(modules).sort();
  for (const path of paths) {
    try {
      const mod = await modules[path]();
      const candidate = (mod as any).default ?? (mod as any).NavisPlugin ?? (mod as any).plugin;
      if (candidate && typeof candidate.apply === 'function') {
        plugins.push(candidate as NavisPlugin);
      } else {
        console.warn(`[Navis Loader] No NavisPlugin export found in ${path}`);
      }
    } catch (err) {
      console.error(`[Navis Loader] Failed to load extension UI ${path}:`, err);
    }
  }
  return plugins;
}