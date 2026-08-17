import { Component, Show } from 'solid-js';
import {
  isTextAgentTimelinePart,
  timelineTextContent,
  type AgentTimelinePartGroup,
} from '@/lib/agent-timeline';
import { AgentTimelineView } from '../AgentTimeline/AgentTimelineView';
import MessageContentRenderer from '../ui/MessageContentRenderer';
import AgentBatchSummary from './AgentBatchSummary';
import MessageAttachments from './MessageAttachments';
import { rawTranscriptMessage } from '../../lib/transcript-view';
import type { TranscriptView } from '../../lib/transcript-view';
import type { ChatMessage } from '../../stores/chat-messages';
import type { TaskProjection } from '../../stores/task-projection';
import type { ChatAgentTimelinePart } from '../../lib/stream';

type ConversationMessageProps = {
  message: ChatMessage;
  index?: number;
  transcriptView: TranscriptView;
  nowMs: number;
  activeGuidance?: boolean;
  activeTurnId?: string | null;
  expandedGroups: Set<string>;
  manuallyToggledGroups: Set<string>;
  expandedParts: Set<string>;
  onGroupExpandedChange: (group: AgentTimelinePartGroup, expanded: boolean) => void;
  onPartExpandedChange: (part: ChatAgentTimelinePart, expanded: boolean) => void;
  onCopyContent?: (event: ClipboardEvent, content: string) => void;
  showRoleLabel?: boolean;
  tasks?: TaskProjection[];
};

const sortedParts = (message: ChatMessage): ChatAgentTimelinePart[] =>
  message.agentTimelineParts.slice().sort((left, right) => left.sequence - right.sequence);

const visibleParts = (message: ChatMessage): ChatAgentTimelinePart[] =>
  sortedParts(message).filter((part) => !isTextAgentTimelinePart(part));

const timelineTextForMessage = (message: ChatMessage): string =>
  sortedParts(message)
    .filter(isTextAgentTimelinePart)
    .map((part) => part.text ?? part.detail ?? part.summary ?? '')
    .join('');

const messageContent = (message: ChatMessage): string =>
  message.content || timelineTextForMessage(message);

const hasTimelineText = (message: ChatMessage): boolean =>
  sortedParts(message).some((part) => isTextAgentTimelinePart(part) && timelineTextContent(part).length > 0);

const isAssistantWorkMessage = (message: ChatMessage): boolean =>
  message.role === 'assistant' && visibleParts(message).length > 0;

const standaloneContent = (message: ChatMessage): string => {
  const content = messageContent(message);
  if (!isAssistantWorkMessage(message)) return content;
  return hasTimelineText(message) ? '' : content;
};

const messageRoleLabel = (message: ChatMessage): string => {
  if (message.role === 'assistant') return 'Navis Go';
  if (message.role === 'system') return 'System';
  if (message.role === 'tool') return 'Tool';
  return '';
};

export const ConversationMessage: Component<ConversationMessageProps> = (props) => {
  const workMessage = () => isAssistantWorkMessage(props.message);
  const content = () => standaloneContent(props.message);
  const attachments = () => props.message.attachments ?? [];

  return (
    <article
      class={`navis-message ${
        props.message.role === 'user'
          ? 'is-user self-end'
          : workMessage()
            ? 'is-navis is-working self-start'
            : 'is-navis self-start'
      }`}
    >
      <Show when={props.showRoleLabel !== false && props.message.role !== 'user' && !workMessage()}>
        <div class="navis-message-role text-[11px] text-[#8b8b8b]">{messageRoleLabel(props.message)}</div>
      </Show>
      <Show
        when={props.transcriptView !== 'raw'}
        fallback={<pre class="navis-message-raw">{rawTranscriptMessage(props.message)}</pre>}
      >
        <Show when={props.message.role === 'user' && attachments().length > 0}>
          <MessageAttachments attachments={attachments()} />
        </Show>
        <Show when={workMessage()}>
          <Show when={props.activeGuidance && props.message.agentTimelineParts.some((part) => part.turnId === props.activeTurnId)}>
            <div class="navis-guided-conversation-note">Guided conversation</div>
          </Show>
          <AgentTimelineView
            parts={sortedParts(props.message)}
            nowMs={props.nowMs}
            expandedGroups={props.expandedGroups}
            manuallyToggledGroups={props.manuallyToggledGroups}
            expandedParts={props.expandedParts}
            onGroupExpandedChange={props.onGroupExpandedChange}
            onPartExpandedChange={props.onPartExpandedChange}
          />
          <Show when={(props.tasks?.length ?? 0) > 0}>
            <AgentBatchSummary tasks={props.tasks ?? []} />
          </Show>
        </Show>
        <Show when={content()}>
          {(value) => (
            <div
              class={`navis-message-content max-w-[760px] text-[14px] ${
                props.message.role === 'user'
                  ? 'navis-message-bubble'
                  : 'navis-message-navis'
              }`}
              onCopy={(event) => props.onCopyContent?.(event, value())}
            >
              <MessageContentRenderer content={value()} />
            </div>
          )}
        </Show>
      </Show>
    </article>
  );
};
