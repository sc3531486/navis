/* @refresh reload */
import { render } from 'solid-js/web';
import { getHotkeyManager } from './lib/hotkey';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';
import './styles/index.css';
import AppRoutes from './router';
import { init as initAppState, installAppStatePersistence } from './stores/app';
import { installMenuDismissHandlers, loadMenus } from './stores/menu';
import { loadExtensions } from './stores/extension';
import { installThemeLifecycle, restoreTheme } from './theme';

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error('Root element not found');
}

getHotkeyManager().init();
void loadMenus();
void loadExtensions();
installMenuDismissHandlers();
initAppState();
restoreTheme();

const Root = () => {
  installAppStatePersistence();
  installThemeLifecycle();
  return <AppRoutes />;
};

render(() => <Root />, root!);
