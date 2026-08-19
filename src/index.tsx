import { rootContext } from './core/context';
import { initBridge } from './core/tauri-bridge';
import { WhiteboardShell } from './app/WhiteboardShell';
import { NavisCodeExtension } from '../extensions/navis-code/ExtensionUI/src/index';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';
import './styles/index.css';

async function bootstrap() {
  // 1. 初始化前后端通信桥梁
  await initBridge(rootContext);

  // 2. 挂载业务插件
  await rootContext.plugin(NavisCodeExtension);

  // 3. 渲染白板容器
  const root = document.getElementById('root');
  if (root) {
    root.innerHTML = '';
    const { render } = await import('solid-js/web');
    render(
      () => (
        <WhiteboardShell
          ctx={rootContext}
          brandTitle="Navis Code Studio"
          brandIcon="/icons/NAVIS.png"
        />
      ),
      root,
    );
  }
}

bootstrap().catch(console.error);
