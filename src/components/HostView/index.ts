export { default as HostViewRenderer } from './HostViewRenderer';
export { default as HostViewSurface } from './HostViewSurface';
export type { HostViewRendererProps } from './types';
export type { UiExtensionView } from '@/lib/extension-ui';
export { getHostViewRendererDescriptor, getHostViewSurfaceDescriptor, registerHostViewRenderer, registerHostViewSurface } from './registry';
export type { HostViewRendererDescriptor, HostViewSurfaceDescriptor, HostViewSurfaceKind } from './registry';
