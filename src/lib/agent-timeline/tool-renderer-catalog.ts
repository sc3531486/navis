import { createSignal } from 'solid-js';
import type { Component } from 'solid-js';
import type { ChatAgentTimelinePart } from '../stream';

export type AgentTimelinePartRendererProps = {
  part: ChatAgentTimelinePart;
  nowMs: number;
  expanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
};
export type AgentTimelinePartRenderer = Component<AgentTimelinePartRendererProps>;

export interface ToolRendererMatch {
  displayKind?: string;
  tool?: string;
  gatewayTool?: string;
  renderer?: string;
  detailView?: string;
}

export interface ToolRendererRegistration {
  registrationId: string;
  extensionName: string;
  priority: number;
  match: ToolRendererMatch;
  renderer: AgentTimelinePartRenderer;
  registeredAt: number;
}

export interface ToolRendererRegistrationOptions {
  id?: string;
  priority?: number;
  match: ToolRendererMatch;
}

let registrationCounter = 0;

const [toolRendererCatalog, setToolRendererCatalog] = createSignal<ToolRendererRegistration[]>([]);

const normalized = (value: string | null | undefined): string =>
  (value ?? '').trim().toLowerCase();

const partRendererHint = (part: ChatAgentTimelinePart): { renderer: string; detailView: string } => {
  const hint = part.metadata?.rendererHint;
  if (hint && typeof hint === 'object' && 'renderer' in hint) {
    const rendererHint = hint as { renderer?: unknown; detailView?: unknown };
    const renderer = typeof rendererHint.renderer === 'string' ? rendererHint.renderer : '';
    const detailView = typeof rendererHint.detailView === 'string' ? rendererHint.detailView : '';
    return { renderer, detailView };
  }
  return { renderer: '', detailView: '' };
};

const partDisplayKind = (part: ChatAgentTimelinePart): string => {
  const displayKind = part.metadata?.displayKind;
  return typeof displayKind === 'string' ? displayKind : '';
};

function matchesPart(registration: ToolRendererRegistration, part: ChatAgentTimelinePart, kind: string): boolean {
  const match = registration.match;
  const rendererHint = partRendererHint(part);
  if (match.renderer && normalized(match.renderer) !== normalized(rendererHint.renderer)) return false;
  if (match.detailView && normalized(match.detailView) !== normalized(rendererHint.detailView)) return false;
  if (
    match.displayKind &&
    normalized(match.displayKind) !== normalized(kind) &&
    normalized(match.displayKind) !== normalized(partDisplayKind(part))
  ) {
    return false;
  }
  if (match.tool && normalized(match.tool) !== normalized(part.tool)) return false;
  if (match.gatewayTool && normalized(match.gatewayTool) !== normalized(part.gatewayTool)) return false;
  return Boolean(match.renderer || match.detailView || match.displayKind || match.tool || match.gatewayTool);
}

function sortRenderers(left: ToolRendererRegistration, right: ToolRendererRegistration): number {
  if (left.priority !== right.priority) return left.priority - right.priority;
  return left.registeredAt - right.registeredAt;
}

export function registerToolRenderer(
  extensionName: string,
  options: ToolRendererRegistrationOptions,
  renderer: AgentTimelinePartRenderer,
): string {
  const registrationId = options.id ?? `${extensionName}-tool-renderer-${++registrationCounter}`;
  const registration: ToolRendererRegistration = {
    registrationId,
    extensionName,
    priority: options.priority ?? 100,
    match: options.match,
    renderer,
    registeredAt: Date.now(),
  };

  setToolRendererCatalog((current) => [
    ...current.filter((item) => item.registrationId !== registrationId),
    registration,
  ].sort(sortRenderers));
  return registrationId;
}

export function unregisterToolRenderer(registrationId: string): void {
  setToolRendererCatalog((current) => current.filter((item) => item.registrationId !== registrationId));
}

export function unregisterExtensionToolRenderers(extensionName: string): void {
  setToolRendererCatalog((current) => current.filter((item) => item.extensionName !== extensionName));
}

export function resolveToolRenderer(
  part: ChatAgentTimelinePart,
  kind: string,
  fallback: AgentTimelinePartRenderer,
): AgentTimelinePartRenderer {
  return toolRendererCatalog().find((registration) => matchesPart(registration, part, kind))?.renderer ?? fallback;
}

export function useToolRendererCatalog() {
  return toolRendererCatalog;
}
