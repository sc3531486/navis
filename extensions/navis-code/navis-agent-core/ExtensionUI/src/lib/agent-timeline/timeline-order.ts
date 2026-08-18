import type { ChatAgentTimelinePart } from '@/lib/stream';
import { isStatusLive } from '@/lib/status';

export type AgentTimelineItem =
  | { kind: 'text'; part: ChatAgentTimelinePart }
  | { kind: 'action'; part: ChatAgentTimelinePart }
  | { kind: 'finalizer'; part: ChatAgentTimelinePart };

export type AgentTimelinePartGroup = {
  id: string;
  label: string;
  detail?: string;
  parts: ChatAgentTimelinePart[];
};

export type AgentTimelineFlowItem =
  | { kind: 'text'; part: ChatAgentTimelinePart }
  | { kind: 'group'; group: AgentTimelinePartGroup; index: number }
  | { kind: 'finalizer'; part: ChatAgentTimelinePart };

export const isTextAgentTimelinePart = (part: ChatAgentTimelinePart): boolean => part.kind === 'text';

export const isToolPreludeTextPart = (part: ChatAgentTimelinePart): boolean =>
  isTextAgentTimelinePart(part) && part.source === 'gateway_tool_prelude';

export const isTurnPreludePart = (part: ChatAgentTimelinePart): boolean =>
  part.kind === 'reasoning' && part.source === 'turn_prelude';

export const isTurnFinalizerPart = (part: ChatAgentTimelinePart): boolean =>
  part.kind === 'summary' && part.source === 'turn_finalizer';

export const isActiveTimelinePart = (part: ChatAgentTimelinePart): boolean =>
  isStatusLive(part.statusPresentation);

export const isTimelineActionPart = (part: ChatAgentTimelinePart): boolean =>
  !isTextAgentTimelinePart(part) && !isTurnPreludePart(part) && !isTurnFinalizerPart(part);

export const timelineTextContent = (part: ChatAgentTimelinePart): string => {
  if (!isTextAgentTimelinePart(part)) return '';
  return (part.text ?? part.detail ?? '').trim();
};

export const isRenderableTimelineTextPart = (part: ChatAgentTimelinePart): boolean =>
  isTextAgentTimelinePart(part) && timelineTextContent(part).length > 0;

export const hasActiveTimelineActionPart = (parts: ChatAgentTimelinePart[]): boolean =>
  parts.some((part) => isTimelineActionPart(part) && isActiveTimelinePart(part));

export function visibleTurnPreludePart(
  parts: ChatAgentTimelinePart[],
  timelineItems: AgentTimelineItem[],
): ChatAgentTimelinePart | undefined {
  const prelude = parts.find((part) => isTurnPreludePart(part) && isActiveTimelinePart(part));
  if (!prelude) return undefined;
  const hasVisibleText = timelineItems.some((item) => item.kind === 'text');
  return hasVisibleText ? undefined : prelude;
}

export function buildAgentTimelineItems(parts: ChatAgentTimelinePart[]): AgentTimelineItem[] {
  const sorted = parts.slice().sort((left, right) => left.sequence - right.sequence);
  const finalizerPart = sorted.find(isTurnFinalizerPart);
  const items: AgentTimelineItem[] = [];

  for (const part of sorted) {
    if (isTurnPreludePart(part) || isTurnFinalizerPart(part)) continue;

    if (isTextAgentTimelinePart(part)) {
      if (!isRenderableTimelineTextPart(part)) continue;
      items.push({ kind: 'text', part });
      continue;
    }

    items.push({ kind: 'action', part });
  }

  if (finalizerPart) items.push({ kind: 'finalizer', part: finalizerPart });

  return items;
}

export function buildAgentTimelineFlowItems(
  items: AgentTimelineItem[],
): AgentTimelineFlowItem[] {
  const flow: AgentTimelineFlowItem[] = [];
  let currentGroup: AgentTimelinePartGroup | null = null;
  let currentGroupPushed = false;
  let groupIndex = 0;

  const pushCurrentGroup = () => {
    if (!currentGroup || currentGroupPushed) return;
    flow.push({ kind: 'group', group: currentGroup, index: groupIndex });
    groupIndex += 1;
    currentGroupPushed = true;
  };

  const createFallbackGroup = () => {
    currentGroup = {
      id: `step:${groupIndex + 1}`,
      label: `Step ${groupIndex + 1}`,
      parts: [],
    };
    currentGroupPushed = false;
  };

  for (const item of items) {
    if (item.kind === 'text') {
      const text = timelineTextContent(item.part);
      if (!text) continue;
      flow.push({ kind: 'text', part: item.part });
      currentGroup = {
        id: `${item.part.partId}:actions`,
        label: text,
        detail: item.part.detail ?? item.part.summary ?? undefined,
        parts: [],
      };
      currentGroupPushed = false;
      continue;
    }

    if (item.kind === 'action') {
      if (!currentGroup) createFallbackGroup();
      currentGroup!.parts.push(item.part);
      pushCurrentGroup();
      continue;
    }

    currentGroup = null;
    currentGroupPushed = false;
    flow.push(item);
  }

  return flow.filter((item) => item.kind !== 'group' || item.group.parts.length > 0);
}
