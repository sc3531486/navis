import { Component } from 'solid-js';
import type { BuiltinMode, ModeTab } from './sidebar-model';

export const SidebarModeTabs: Component<{
  activeTab: ModeTab;
  onSelectBuiltinMode: (mode: BuiltinMode) => void;
  onSelectCustomTab: () => void;
}> = (props) => (
  <section class="navis-sidebar-mode">
    <div class="navis-mode-tabs grid grid-cols-3" role="tablist">
      <button
        type="button"
        role="tab"
        aria-selected={props.activeTab === 'cowork'}
        class={`navis-mode-tab rounded-md text-[12px] ${props.activeTab === 'cowork' ? 'is-active' : ''}`}
        onClick={() => props.onSelectBuiltinMode('cowork')}
      >
        <span class="navis-mode-tab-mark" aria-hidden="true">⌘</span>
        <span>Cowork</span>
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={props.activeTab === 'code'}
        class={`navis-mode-tab rounded-md text-[12px] ${props.activeTab === 'code' ? 'is-active' : ''}`}
        onClick={() => props.onSelectBuiltinMode('code')}
      >
        <span class="navis-mode-tab-mark" aria-hidden="true">&lt;/&gt;</span>
        <span>Code</span>
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={props.activeTab === 'custom'}
        class={`navis-mode-tab rounded-md text-[12px] ${props.activeTab === 'custom' ? 'is-active' : ''}`}
        onClick={props.onSelectCustomTab}
      >
        <span class="navis-mode-tab-mark" aria-hidden="true">◇</span>
        <span>Custom</span>
      </button>
    </div>
  </section>
);
