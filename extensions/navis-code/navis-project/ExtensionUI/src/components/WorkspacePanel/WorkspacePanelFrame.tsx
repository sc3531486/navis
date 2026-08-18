import { Component, JSX, Show } from 'solid-js';
import { PanelSurface } from '@/components/PanelSurface';
import CloseIcon from '@/components/Icon/CloseIcon';

interface WorkspacePanelFrameProps {
  title: string;
  active?: boolean;
  closeLabel?: string;
  closable?: boolean;
  onFocus?: () => void;
  onClose?: (event: MouseEvent) => void;
  children: JSX.Element;
}

interface WorkspacePanelLayoutProps {
  class?: string;
  children: JSX.Element;
}

interface WorkspacePanelSectionHeaderProps {
  title: string;
  action?: JSX.Element;
}

export const WorkspacePanelFrame: Component<WorkspacePanelFrameProps> = (props) => (
  <PanelSurface
    as="section"
    class={`navis-right-panel flex min-h-0 flex-1 flex-col ${props.active ? 'navis-right-panel-active' : ''}`}
    onClick={() => props.onFocus?.()}
  >
    <header class="navis-right-panel-header flex items-center text-[12px] font-medium text-[#242424]">
      <span class="min-w-0 flex-1 truncate">{props.title}</span>
      <Show when={props.closable ?? true}>
        <button
          type="button"
          class="navis-right-panel-close"
          aria-label={props.closeLabel ?? `关闭 ${props.title}`}
          title="关闭"
          onClick={(event) => props.onClose?.(event)}
        >
          <CloseIcon />
        </button>
      </Show>
    </header>
    <div class="navis-right-panel-body min-h-0 flex-1 overflow-hidden">
      {props.children}
    </div>
  </PanelSurface>
);

export const WorkspacePanelScrollArea: Component<WorkspacePanelLayoutProps> = (props) => (
  <div class={`navis-workspace-scroll-area ${props.class ?? ''}`.trim()}>
    {props.children}
  </div>
);

export const WorkspacePanelSectionHeader: Component<WorkspacePanelSectionHeaderProps> = (props) => (
  <div class="navis-workspace-section-header">
    <div class="navis-workspace-section-heading">{props.title}</div>
    {props.action}
  </div>
);

export const WorkspacePanelCard: Component<WorkspacePanelLayoutProps> = (props) => (
  <article class={`navis-workspace-card ${props.class ?? ''}`.trim()}>
    {props.children}
  </article>
);

export default WorkspacePanelFrame;
