import { rootContext } from './core/context';
import { bootstrap } from './core/bootstrap';
import { WhiteboardShell } from './app/WhiteboardShell';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';
import './styles/index.css';

async function main() {
  // 宿主启动：桥接 + 贡献分发 + 插件装载（src/index.tsx 不出现任何业务实现）
  await bootstrap(rootContext);

  // 渲染白板容器
  const root = document.getElementById('root');
  if (root) {
    root.innerHTML = '';
    const { render } = await import('solid-js/web');
    render(
      () => (
        <WhiteboardShell
          ctx={rootContext}
          brandTitle="Navis"
          brandIcon="/icons/NAVIS.png"
        />
      ),
      root,
    );
  }
}

main().catch(console.error);