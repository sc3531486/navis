/* @refresh reload */
import { render } from 'solid-js/web';
import AppRoutes from './router';
import './theme/variables.css';
import './theme/light.css';
import './theme/dark.css';

const root = document.getElementById('root');

if (import.meta.env.DEV && !(root instanceof HTMLElement)) {
  throw new Error(
    'Root element not found. Did you forget to add it to index.html? Maybe the id attribute got misspelled?',
  );
}

render(() => <AppRoutes />, root!);
