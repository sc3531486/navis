import { Component, For, Show } from 'solid-js';
import {
  buildAgentTimelineFlowItems,
  buildAgentTimelineItems,
  resolveToolRenderer,
  timelineTextContent,
  type AgentTimelinePartRenderer,
  type AgentTimelinePartGroup,
} from '@/lib/agent-timeline';
import type { ChatAgentTimelinePart } from '@/lib/stream';
import MessageContentRenderer from '../ui/MessageContentRenderer';
import ShimmerText from '../ui/ShimmerText';
import { statusClass } from '../../lib/status';
import { isLiveTimelinePart } from './tool-kind';
import { AgentTraceGlyph } from './TraceIcon';

export const agentTimelinePartExpansionKey = (part: ChatAgentTimelinePart): string =>
  `${part.turnId}:${part.partId}`;

export const agentTimelineGroupExpansionKey = (group: AgentTimelinePartGroup): string => {
  const turnId = group.parts[0]?.turnId ?? 'turn';
  return `${turnId}:${group.id}`;
};

const exceptionalStatuses = new Set(['error', 'denied', 'aborted', 'interrupted']);

const normalizedToolKind = (part: ChatAgentTimelinePart): string => {
  if (part.kind === 'reasoning' || part.kind === 'permission' || part.kind === 'sidechain' || part.kind === 'summary') {
    return part.kind;
  }
  const raw = (part.gatewayTool ?? part.tool ?? part.kind ?? '').toLowerCase().replace(/[^a-z0-9_:-]+/g, '_');
  if (raw.includes('read')) return 'read';
  if (raw.includes('list') || raw.includes('ls')) return 'list';
  if (raw.includes('grep') || raw.includes('search') || raw.includes('glob')) return 'search';
  if (raw.includes('inspect')) return 'inspect';
  if (raw.includes('edit') || raw.includes('write') || raw.includes('patch')) return 'edit';
  if (raw.includes('terminal') || raw.includes('shell') || raw.includes('bash')) return 'terminal';
  return raw || 'generic';
};

const partTitle = (part: ChatAgentTimelinePart): string =>
  part.title?.trim() ||
  part.summary?.trim() ||
  part.tool?.trim() ||
  part.gatewayTool?.trim() ||
  part.kind ||
  'Agent part';

const FallbackTimelinePart: AgentTimelinePartRenderer = (props) => (
  <div class="navis-agent-trace-row-wrap">
    <div class={`navis-agent-trace-row ${statusClass(props.part.statusPresentation)}`}>
      <AgentTraceGlyph kind={normalizedToolKind(props.part)} />
      <span class="navis-agent-trace-label">{partTitle(props.part)}</span>
      <span class="navis-agent-trace-meta">{props.part.status ?? ''}</span>
      <span class="navis-agent-trace-duration is-empty" />
    </div>
  </div>
);

const AgentTimelineActionPart: AgentTimelinePartRenderer = (props) => {
  const kind = normalizedToolKind(props.part);
  const Renderer = resolveToolRenderer(props.part, kind, FallbackTimelinePart);
  return <Renderer {...props} />;
};

const groupHasLivePart = (group: AgentTimelinePartGroup): boolean =>
  group.parts.some(isLiveTimelinePart);

const groupHasExceptionalPart = (group: AgentTimelinePartGroup): boolean =>
  group.parts.some((part) => part.status && exceptionalStatuses.has(part.status));

const groupTitle = (group: AgentTimelinePartGroup): string => {
  const active = groupHasLivePart(group);
  return active ? `正在运行 ${group.parts.length} 条命令` : `已运行 ${group.parts.length} 条命令`;
};

const groupSummary = (group: AgentTimelinePartGroup): string => {
  const counts = new Map<string, number>();
  for (const part of group.parts) {
    const kind = normalizedToolKind(part);
    counts.set(kind, (counts.get(kind) ?? 0) + 1);
  }
  const label = (kind: string, count: number): string => {
    const noun = (() => {
      switch (kind) {
        case 'read': return 'read';
        case 'list': return 'list';
        case 'search': return 'search';
        case 'inspect': return 'inspect';
        case 'edit': return 'edit';
        case 'terminal': return 'command';
        case 'sidechain': return 'agent task';
        case 'permission': return 'approval';
        case 'error': return 'error';
        default: return 'tool part';
      }
    })();
    return `${count} ${count === 1 ? noun : `${noun}s`}`;
  };
  return [...counts.entries()]
    .map(([kind, count]) => label(kind, count))
    .join(' · ') || `${group.parts.length} tool ${group.parts.length === 1 ? 'part' : 'parts'}`;
};

const AgentTimelineTextPart: Component<{ part: ChatAgentTimelinePart }> = (props) => {
  const content = () => timelineTextContent(props.part);
  return (
    <Show when={content()}>
      {(value) => (
        <div class="navis-agent-message-text">
          <MessageContentRenderer content={value()} />
        </div>
      )}
    </Show>
  );
};

const AgentTimelineGroupView: Component<{
  group: AgentTimelinePartGroup;
  nowMs: number;
  defaultExpanded: boolean;
  expandedGroups: Set<string>;
  manuallyToggledGroups: Set<string>;
  expandedParts: Set<string>;
  onGroupExpandedChange: (group: AgentTimelinePartGroup, expanded: boolean) => void;
  onPartExpandedChange: (part: ChatAgentTimelinePart, expanded: boolean) => void;
}> = (props) => {
  const key = () => agentTimelineGroupExpansionKey(props.group);
  const expanded = () =>
    props.manuallyToggledGroups.has(key())
      ? props.expandedGroups.has(key())
      : props.defaultExpanded;

  return (
    <div class={`navis-agent-trace-step-group ${expanded() ? 'is-expanded' : ''}`}>
      <button
        type="button"
        class="navis-agent-trace-step-group-row"
        aria-expanded={expanded()}
        title={props.group.detail ?? props.group.label}
        onClick={() => props.onGroupExpandedChange(props.group, !expanded())}
      >
        <span class="navis-agent-trace-step-group-mark" aria-hidden="true">
          <AgentTraceGlyph kind="integration" />
        </span>
        <span class="navis-agent-trace-step-group-copy">
          <ShimmerText
            active={groupHasLivePart(props.group)}
            class="navis-agent-trace-step-group-label"
            peakColor="#242424"
          >
            {groupTitle(props.group)}
          </ShimmerText>
        </span>
        <span class={`navis-agent-trace-step-group-chevron ${expanded() ? 'is-open' : ''}`} aria-hidden="true" />
      </button>
      <Show when={expanded()}>
        <div class="navis-agent-trace-step-group-detail">{groupSummary(props.group)}</div>
        <div class="navis-agent-trace-step-group-items">
          <For each={props.group.parts}>
            {(part) => (
              <AgentTimelineActionPart
                part={part}
                nowMs={props.nowMs}
                expanded={props.expandedParts.has(agentTimelinePartExpansionKey(part))}
                onExpandedChange={(expanded) => props.onPartExpandedChange(part, expanded)}
              />
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export const AgentTimelineView: Component<{
  parts: ChatAgentTimelinePart[];
  nowMs: number;
  expandedGroups: Set<string>;
  manuallyToggledGroups: Set<string>;
  expandedParts: Set<string>;
  onGroupExpandedChange: (group: AgentTimelinePartGroup, expanded: boolean) => void;
  onPartExpandedChange: (part: ChatAgentTimelinePart, expanded: boolean) => void;
}> = (props) => {
  const flowItems = () => buildAgentTimelineFlowItems(buildAgentTimelineItems(props.parts));

  return (
    <Show when={flowItems().length > 0}>
      <div class="navis-agent-trace self-start" aria-live="polite">
        <For each={flowItems()}>
          {(item) =>
            item.kind === 'text'
              ? <AgentTimelineTextPart part={item.part} />
              : item.kind === 'group'
                ? (
                  <AgentTimelineGroupView
                    group={item.group}
                    nowMs={props.nowMs}
                    defaultExpanded={groupHasLivePart(item.group) || groupHasExceptionalPart(item.group)}
                    expandedGroups={props.expandedGroups}
                    manuallyToggledGroups={props.manuallyToggledGroups}
                    expandedParts={props.expandedParts}
                    onGroupExpandedChange={props.onGroupExpandedChange}
                    onPartExpandedChange={props.onPartExpandedChange}
                  />
                )
                : (
                  <div class="navis-agent-finalizer" title={item.part.detail ?? item.part.summary ?? 'Turn complete'}>
                    <AgentTraceGlyph kind="agent" />
                    <span class="navis-agent-finalizer-copy">{item.part.summary ?? 'Finished response'}</span>
                  </div>
                )
          }
        </For>
      </div>
    </Show>
  );
};
