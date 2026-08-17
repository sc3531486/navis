import type { UiExtensionView } from '@/lib/extension-ui';

export interface HostViewRendererProps {
  view: UiExtensionView;
  surface: string;
}
