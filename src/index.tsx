import { rootContext } from './core/context';
import { WhiteboardShell } from './app/WhiteboardShell';
import { NavisCodeExtension } from '../extensions/navis-code/ExtensionUI/src/index';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';
import './styles/index.css';

async function bootstrap() {
  await rootContext.plugin(NavisCodeExtension);

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
