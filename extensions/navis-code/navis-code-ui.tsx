/* Navis Code 产品入口：只组合 Navis 宿主与产品扩展。 */
import { onMount, type ParentProps } from 'solid-js';
import { FrameworkLifecycle } from '@/bootstrap';
import { loadGatewayCatalog } from '@project-ext/stores/gateway';
import { loadSlashCommands } from '@agent-core/stores/slash-commands';
import AppRoutes from './ExtensionUI/src/router';
import type { ProductDefinition } from '@/product-contract';
import '@/theme/variables.css';
import '@/theme/light.css';
import '@/theme/dark.css';
import '@/styles/index.css';
import '@session/styles/chatMessages/index.css';
import '@session/styles/leftSidebar/index.css';
import '@session/styles/startWorkspace/index.css';
import '@agent-core/styles/composer/index.css';
import '@agent-core/styles/search-surface.css';
import '@agent-core/styles/plan-panel.css';
import '@settings-ext/styles/settings/index.css';

function NavisCodeLifecycle(props: ParentProps) {
  onMount(() => {
    void loadGatewayCatalog();
    void loadSlashCommands();
  });
  return props.children;
}

function NavisCodeApp() {
  return (
    <FrameworkLifecycle>
      <NavisCodeLifecycle>
        <AppRoutes />
      </NavisCodeLifecycle>
    </FrameworkLifecycle>
  );
}

export const product: ProductDefinition = {
  id: 'navis-code',
  component: NavisCodeApp,
};

export default product;
