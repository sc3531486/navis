import { Component, For, Show, createSignal } from 'solid-js';
import {
  agentTimelineGroupExpansionKey,
  agentTimelinePartExpansionKey,
} from '../AgentTimeline/AgentTimelineView';
import { transcriptViewClass, type TranscriptView } from '../../lib/transcript-view';
import type { AgentTimelinePartGroup } from '../../lib/agent-timeline';
import type { ChatAgentTimelinePart } from '../../lib/stream';
import type { ChatMessage } from '../../stores/chat-messages';
import type { TaskProjection } from '../../stores/task-projection';
import { ConversationMessage } from './ConversationMessage';

interface ConversationTranscriptProps {
  messages: ChatMessage[];
  transcriptView: TranscriptView;
  nowMs: number;
  class?: string;
  showRoleLabel?: boolean;
  activeGuidance?: boolean;
  activeTurnId?: string | null;
  tasksByMessageId?: Map<string, TaskProjection[]>;
  onCopyContent?: (event: ClipboardEvent, content: string) => void;
}

const ConversationTranscript: Component<ConversationTranscriptProps> = (props) => {
  const [expandedAgentGroups, setExpandedAgentGroups] = createSignal<Set<string>>(new Set());
  const [manuallyToggledAgentGroups, setManuallyToggledAgentGroups] = createSignal<Set<string>>(new Set());
  const [expandedAgentTimelineParts, setExpandedAgentTimelineParts] = createSignal<Set<string>>(new Set());

  const setAgentGroupExpanded = (group: AgentTimelinePartGroup, expanded: boolean): void => {
    const key = agentTimelineGroupExpansionKey(group);
    setManuallyToggledAgentGroups((current) => new Set(current).add(key));
    setExpandedAgentGroups((current) => {
      const next = new Set(current);
      if (expanded) next.add(key);
      else next.delete(key);
      return next;
    });
  };

  const setAgentTimelinePartExpanded = (part: ChatAgentTimelinePart, expanded: boolean): void => {
    const key = agentTimelinePartExpansionKey(part);
    setExpandedAgentTimelineParts((current) => {
      const next = new Set(current);
      if (expanded) next.add(key);
      else next.delete(key);
      return next;
    });
  };

  return (
    <div class={`navis-conversation-transcript ${transcriptViewClass(props.transcriptView)} ${props.class ?? ''}`.trim()}>
      <For each={props.messages}>
        {(message, index) => (
          <ConversationMessage
            message={message}
            index={index()}
            transcriptView={props.transcriptView}
            nowMs={props.nowMs}
            activeGuidance={props.activeGuidance}
            activeTurnId={props.activeTurnId}
            tasks={props.tasksByMessageId?.get(message.id) ?? []}
            expandedGroups={expandedAgentGroups()}
            manuallyToggledGroups={manuallyToggledAgentGroups()}
            expandedParts={expandedAgentTimelineParts()}
            onGroupExpandedChange={setAgentGroupExpanded}
            onPartExpandedChange={setAgentTimelinePartExpanded}
            onCopyContent={props.onCopyContent}
            showRoleLabel={props.showRoleLabel}
          />
        )}
      </For>
      <Show when={props.messages.length === 0}>
        <div class="navis-conversation-transcript-empty">No messages</div>
      </Show>
    </div>
  );
};

export default ConversationTranscript;
