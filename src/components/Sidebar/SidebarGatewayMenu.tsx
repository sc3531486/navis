import { Component, Show } from 'solid-js';
import { FloatingMenu } from '../Menu';
import type { MenuActionItem } from '../../stores/menu';

export const SidebarGatewayMenu: Component<{
  isOpen: boolean;
  gatewayIconUrl: string;
  disclosureIconUrl: string;
  items: MenuActionItem[];
  selectedCommands: string[];
  getSubmenuItems: (item: MenuActionItem) => MenuActionItem[];
  onToggle: () => void;
  onSelect: (item: MenuActionItem) => Promise<void>;
}> = (props) => (
  <section class="navis-sidebar-gateway relative" data-menu-anchor="gateway">
    <button
      type="button"
      class="navis-gateway-button flex w-full items-center text-left text-[12px]"
      title="Open Navis Go menu"
      aria-expanded={props.isOpen}
      onClick={props.onToggle}
    >
      <img class="navis-gateway-icon" src={props.gatewayIconUrl} alt="" aria-hidden="true" />
      <span class="min-w-0 flex-1 truncate">Gateway</span>
      <img class="navis-gateway-chevron" src={props.disclosureIconUrl} alt="" aria-hidden="true" />
    </button>
    <Show when={props.isOpen}>
      <FloatingMenu
        items={props.items}
        triggerLabel="Navis Go"
        placement="above"
        width={204}
        selectedCommands={props.selectedCommands}
        getSubmenuItems={props.getSubmenuItems}
        onSelect={props.onSelect}
      />
    </Show>
  </section>
);
