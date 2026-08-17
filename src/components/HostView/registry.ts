import type { Component } from 'solid-js';
import HtmlSandboxRenderer from './HtmlSandboxRenderer';
import HostPanelRenderer from './HostPanelRenderer';
import type { HostViewRendererProps } from './types';

export type HostViewSurfaceKind = 'rightWorkspace' | 'inline' | 'settings' | 'dialog';

export interface HostViewRendererDescriptor {
  id: string;
  component: Component<HostViewRendererProps>;
}

export interface HostViewSurfaceDescriptor {
  id: string;
  kind: HostViewSurfaceKind;
}

const createRendererDescriptor = (
  id: string,
  component: Component<HostViewRendererProps>,
): HostViewRendererDescriptor => Object.freeze({ id, component });

const HOST_PANEL_RENDERER = createRendererDescriptor('host:panel', HostPanelRenderer);
const HTML_SANDBOX_RENDERER = createRendererDescriptor('html:sandbox', HtmlSandboxRenderer);

const hostViewRendererRegistry = new Map<string, HostViewRendererDescriptor>([
  [HOST_PANEL_RENDERER.id, HOST_PANEL_RENDERER],
  [HTML_SANDBOX_RENDERER.id, HTML_SANDBOX_RENDERER],
]);

const hostViewSurfaceRegistry = new Map<string, HostViewSurfaceDescriptor>([
  ['rightWorkspace', { id: 'rightWorkspace', kind: 'rightWorkspace' }],
  ['chatAside', { id: 'chatAside', kind: 'inline' }],
  ['bottomDrawer', { id: 'bottomDrawer', kind: 'inline' }],
  ['settingsSection', { id: 'settingsSection', kind: 'settings' }],
  ['dialog', { id: 'dialog', kind: 'dialog' }],
]);

function normalizeZone(zone: string): string {
  return zone.trim();
}

function dynamicZoneKind(zone: string): HostViewSurfaceKind | null {
  if (!zone.includes(':')) return null;
  return /\s/.test(zone) ? null : 'inline';
}

export function registerHostViewRenderer(descriptor: HostViewRendererDescriptor): () => void {
  const previous = hostViewRendererRegistry.get(descriptor.id);
  hostViewRendererRegistry.set(descriptor.id, Object.freeze({ ...descriptor }));
  return () => {
    if (previous) hostViewRendererRegistry.set(descriptor.id, previous);
    else hostViewRendererRegistry.delete(descriptor.id);
  };
}

export function registerHostViewSurface(descriptor: HostViewSurfaceDescriptor): () => void {
  const id = normalizeZone(descriptor.id);
  const previous = hostViewSurfaceRegistry.get(id);
  hostViewSurfaceRegistry.set(id, Object.freeze({ ...descriptor, id }));
  return () => {
    if (previous) hostViewSurfaceRegistry.set(id, previous);
    else hostViewSurfaceRegistry.delete(id);
  };
}

export function getHostViewRendererDescriptor(renderer: string): HostViewRendererDescriptor | undefined {
  return hostViewRendererRegistry.get(renderer);
}

export function getHostViewSurfaceDescriptor(zone: string): HostViewSurfaceDescriptor | undefined {
  const normalized = normalizeZone(zone);
  const registered = hostViewSurfaceRegistry.get(normalized);
  if (registered) return registered;
  const kind = dynamicZoneKind(normalized);
  return kind ? { id: normalized, kind } : undefined;
}
