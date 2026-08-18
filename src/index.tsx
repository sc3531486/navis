/* @refresh reload */
import { render } from 'solid-js/web';
import { FrameworkLifecycle } from './bootstrap';
import { HostViewSurface } from './components/HostView';
import type { ProductDefinition, ProductManifest } from './product-contract';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';
import './styles/index.css';

/** 构建期收集所有产品清单；新增产品只需增加自己的 product.json。 */
const manifestModules = import.meta.glob('/extensions/*/product.json', {
  eager: true,
  query: '?raw',
  import: 'default',
}) as Record<string, string>;

/** 构建期收集产品入口，但只有被选中的产品才会被动态加载。 */
const productModules = import.meta.glob('/extensions/*/*-ui.tsx') as Record<
  string,
  () => Promise<unknown>
>;

function parseManifest(path: string, content: string): ProductManifest | null {
  try {
    const manifest = JSON.parse(content) as Partial<ProductManifest>;
    if (
      typeof manifest.id !== 'string' ||
      typeof manifest.name !== 'string' ||
      typeof manifest.version !== 'string' ||
      typeof manifest.entry !== 'string'
    ) {
      console.warn(`[Navis] 忽略无效产品清单: ${path}`);
      return null;
    }
    return manifest as ProductManifest;
  } catch (error) {
    console.warn(`[Navis] 产品清单解析失败: ${path}`, error);
    return null;
  }
}

function productEntryPath(manifestPath: string, entry: string): string {
  const productDirectory = manifestPath.slice(0, manifestPath.lastIndexOf('/'));
  const normalizedEntry = entry.replace(/^\.\//, '');
  return `${productDirectory}/${normalizedEntry}`;
}

async function loadProduct(): Promise<ProductDefinition | null> {
  const manifests = Object.entries(manifestModules)
    .map(([path, content]) => {
      const manifest = parseManifest(path, content);
      return manifest ? { path, manifest } : null;
    })
    .filter((item): item is { path: string; manifest: ProductManifest } => item !== null);

  if (manifests.length === 0) return null;

  const requestedId = new URLSearchParams(window.location.search).get('product');
  const selected =
    (requestedId && manifests.find(({ manifest }) => manifest.id === requestedId)) ||
    manifests.find(({ manifest }) => manifest.default) ||
    manifests[0];

  const entryPath = productEntryPath(selected.path, selected.manifest.entry);
  const importer = productModules[entryPath];
  if (!importer) {
    console.warn(`[Navis] 产品入口不存在: ${entryPath}`);
    return null;
  }

  const loaded = (await importer()) as {
    default?: ProductDefinition;
    product?: ProductDefinition;
  };
  const definition = loaded.product ?? loaded.default;
  if (!definition || definition.id !== selected.manifest.id) {
    console.warn(`[Navis] 产品入口契约不匹配: ${entryPath}`);
    return null;
  }
  return definition;
}

/** 无产品时显示的纯 Navis 白板。 */
function BlankHost() {
  return (
    <FrameworkLifecycle>
      <div class="flex h-screen w-screen items-center justify-center bg-white text-[#242424]">
        <HostViewSurface zone="main" title="Extensions" />
      </div>
    </FrameworkLifecycle>
  );
}

async function bootstrap() {
  const root = document.getElementById('root');
  if (!(root instanceof HTMLElement)) {
    throw new Error('Navis 根节点不存在');
  }

  const product = await loadProduct();
  const Product = product?.component;
  render(() => (Product ? <Product /> : <BlankHost />), root);
}

void bootstrap();
