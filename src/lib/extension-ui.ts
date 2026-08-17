/**
 * UI view contract shared by extension contribution projections and HostView
 * renderers. The host owns zone/placement lifecycle; renderers consume this
 * backend-aligned view description.
 */
export interface UiExtensionContributionCounts {
  workModes: number;
  views: number;
  menus: number;
  commands: number;
  keybindings: number;
  triggers: number;
  mcpServers: number;
  providers: number;
  zones: number;
  scripts: number;
  toolbarItems: number;
  statusbarItems: number;
  inlineExtensions: number;
  configuration: number;
}


export interface UiZone {
  id: string;
  name: string;
  extensionId: string | null;
  anchorParent: string | null;
  anchorPosition: string | null;
}

export interface UiExtensionScript {
  extensionId: string;
  scriptId: string;
  entry: string;
  resourcePath: string | null;
  runOn: string[];
}

export interface UiExtensionLocale {
  extensionId: string;
  lang: string;
  entry: string;
  resourcePath: string | null;
}

export interface UiExtensionDiscoveryResult {
  extensionId: string;
  extensionName: string;
  provides: string[];
  views: string[];
  commands: string[];
  scripts: string[];
}

export interface UiExtensionPointRegistration {
  extensionId: string;
  kind: 'toolbar' | 'statusbar' | 'inline' | 'configuration' | string;
  id: string;
  label: string | null;
  command: string | null;
  target: string | null;
  group: string | null;
  when: string | null;
  data: unknown;
}

export interface UiExtensionView {
  extensionId: string;
  extensionName: string;
  extensionDescription: string;
  viewId: string;
  name: string;
  icon: string | null;
  /** Open HostView zone. */
  zone: string;
  /** Deprecated compatibility alias mirrored by the backend. */
  placement: string;
  renderer: string;
  entry: string | null;
  resourcePath: string | null;
  config: unknown;
  allowClose: boolean;
  defaultVisible: boolean;
  contributionCounts: UiExtensionContributionCounts;
}

export function viewZone(view: Pick<UiExtensionView, 'zone' | 'placement'>): string {
  return view.zone || view.placement;
}
