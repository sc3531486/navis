import { Component, Show } from 'solid-js';
import { getHostViewRendererDescriptor } from './registry';
import type { HostViewRendererProps } from './types';

const resolveHostViewRenderer = (props: HostViewRendererProps): Component<HostViewRendererProps> | undefined =>
  getHostViewRendererDescriptor(props.view.renderer)?.component;

const HostViewRenderer: Component<HostViewRendererProps> = (props) => (
  <Show
    when={resolveHostViewRenderer(props)}
    fallback={<div class="navis-host-view-empty"><strong>Renderer unavailable</strong><span>{props.view.renderer}</span></div>}
  >
    {(descriptor) => {
      const Renderer = descriptor();
      return <Renderer view={props.view} surface={props.surface} />;
    }}
  </Show>
);

export default HostViewRenderer;
