import { Component, For, Show } from 'solid-js';
import CloseIcon from '@/components/Icon/CloseIcon';
import HostViewRenderer from './HostViewRenderer';
import {
  activeHostViewForZone,
  closeHostView,
  focusHostView,
  hostViewsForZone,
  isHostViewClosable,
  type HostViewInstance,
} from '@/stores/host';
import { getHostViewSurfaceDescriptor } from './registry';

interface HostViewSurfaceProps {
  zone?: string;
  placement?: string;
  title?: string;
}

const HostViewTab: Component<{ view: HostViewInstance; active: boolean }> = (props) => (
  <div
    class={`navis-host-view-tab ${props.active ? 'is-active' : ''}`}
    onClick={() => focusHostView(props.view.id)}
  >
    <span>{props.view.name}</span>
    <Show when={isHostViewClosable(props.view)}>
      <button
        type="button"
        class="navis-host-view-tab-close"
        aria-label={`Close ${props.view.name}`}
        onClick={(event) => {
          event.stopPropagation();
          closeHostView(props.view.id);
        }}
      >
        <CloseIcon />
      </button>
    </Show>
  </div>
);

const HostViewSurface: Component<HostViewSurfaceProps> = (props) => {
  const zone = () => props.zone ?? props.placement ?? '';
  const views = () => hostViewsForZone(zone());
  const activeView = () => activeHostViewForZone(zone());
  const descriptor = () => getHostViewSurfaceDescriptor(zone());

  return (
    <Show when={descriptor() && views().length > 0 && activeView()}>
      {(view) => (
        <section class={`navis-host-view-surface navis-host-view-surface-${descriptor()!.id}`}>
          <header class="navis-host-view-surface-header">
            <div class="navis-host-view-surface-title">{props.title ?? view().name}</div>
            <Show when={views().length > 1}>
              <div class="navis-host-view-tabs">
                <For each={views()}>
                  {(candidate) => <HostViewTab view={candidate} active={candidate.id === view().id} />}
                </For>
              </div>
            </Show>
            <Show when={isHostViewClosable(view())}>
              <button
                type="button"
                class="navis-host-view-surface-close"
                aria-label={`Close ${view().name}`}
                onClick={() => closeHostView(view().id)}
              >
                <CloseIcon />
              </button>
            </Show>
          </header>
          <div class="navis-host-view-surface-body">
            <HostViewRenderer view={view()} surface={zone()} />
          </div>
        </section>
      )}
    </Show>
  );
};

export default HostViewSurface;
