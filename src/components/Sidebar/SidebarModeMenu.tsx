import { Component, For, Show } from 'solid-js';
import { getWorkModeDisplayName, type RegisteredWorkMode } from '../../stores/extension';
import type { ModeTab, SidebarMenuItem } from './sidebar-model';

export const SidebarModeMenu: Component<{
  activeTab: ModeTab;
  builtinMenu: SidebarMenuItem[];
  customModes: RegisteredWorkMode[];
  selectedCustomMode: (mode: RegisteredWorkMode) => boolean;
  onBuiltinMenuSelect: (item: SidebarMenuItem) => void;
  onSelectCustomMode: (mode: RegisteredWorkMode) => void;
  onOpenModeExtensions: () => void;
}> = (props) => (
  <section class="navis-sidebar-menu">
    <div class="navis-sidebar-card">
      <Show when={props.activeTab !== 'custom'}>
        <For each={props.builtinMenu}>
          {(item) => (
            <button
              type="button"
              class="navis-sidebar-row flex h-[30px] w-full items-center text-left text-[12px] hover:text-[#000]"
              onClick={() => props.onBuiltinMenuSelect(item)}
            >
              <span class="navis-sidebar-marker text-center text-[#242424]">{item.marker}</span>
              <span>{item.label}</span>
            </button>
          )}
        </For>
      </Show>
      <Show when={props.activeTab === 'custom'}>
        <Show
          when={props.customModes.length > 0}
          fallback={
            <button
              type="button"
              class="navis-sidebar-row flex h-[30px] w-full items-center text-left text-[12px] hover:text-[#000]"
              onClick={props.onOpenModeExtensions}
            >
              <span class="navis-sidebar-marker text-center text-[#242424]">+</span>
              <span class="min-w-0 flex-1 truncate">Add mode extensions...</span>
            </button>
          }
        >
          <div class="navis-sidebar-custom-modes" aria-label="Custom modes">
            <For each={props.customModes}>
              {(mode) => (
                <button
                  type="button"
                  class={`navis-sidebar-row navis-sidebar-custom-mode flex h-[30px] w-full items-center text-left text-[12px] hover:text-[#000] ${
                    props.selectedCustomMode(mode) ? 'is-selected' : ''
                  }`}
                  onClick={() => props.onSelectCustomMode(mode)}
                >
                  <span class="navis-selection-ring">
                    <Show when={props.selectedCustomMode(mode)}>
                      <span class="navis-selection-dot" />
                    </Show>
                  </span>
                  <span class="min-w-0 flex-1 truncate">{getWorkModeDisplayName(mode)}</span>
                </button>
              )}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  </section>
);
