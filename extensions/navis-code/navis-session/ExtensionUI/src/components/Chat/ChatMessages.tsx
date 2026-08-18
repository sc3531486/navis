import { Component, Show, createSignal, onCleanup } from 'solid-js';

import ConversationTranscript from './ConversationTranscript';
import { registerBuiltinAgentTimelineRenderers } from '@agent-core/components/AgentTimeline/builtin-agent-timeline-renderers';
import { transcriptViewClass } from '@/lib/transcript-view';
import {
  chatMessageState,
} from '@session/stores/chat-messages';
import { activeSession, activeSessionId } from '@session/stores/session-tree';
import type { ChatMessage } from '@session/stores/chat-messages';
import PendingUserMessage from './PendingUserMessage';
import PendingAssistantMessage from './PendingAssistantMessage';
import { useChatInfiniteScroll } from './useChatInfiniteScroll';
import { useMessageTaskProjection } from './useMessageTaskProjection';

registerBuiltinAgentTimelineRenderers();

const normalizeCopiedMessageText = (value: string): string =>
  value.replace(/\s+/g, ' ').trim();

const selectionTouchesElement = (element: HTMLElement, selection: Selection): boolean => {
  const anchor = selection.anchorNode;
  const focus = selection.focusNode;
  return Boolean(
    (anchor && element.contains(anchor)) ||
      (focus && element.contains(focus)),
  );
};

const handleMessageContentCopy = (event: ClipboardEvent, messageText: string): void => {
  const clipboard = event.clipboardData;
  if (!clipboard) return;

  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || !selectionTouchesElement(event.currentTarget as HTMLElement, selection)) {
    return;
  }

  const selectedText = selection.toString();
  const text =
    normalizeCopiedMessageText(selectedText) === normalizeCopiedMessageText(messageText)
      ? messageText
      : selectedText;

  clipboard.setData('text/plain', text);
  event.preventDefault();
};

const hasVisibleAssistantActivity = (messages: ChatMessage[], activeTurnId: string | null): boolean => {
  if (!activeTurnId) return false;
  return messages.some((message) =>
    message.role === 'assistant' &&
    message.agentTimelineParts.some((part) =>
      part.turnId === activeTurnId &&
      (part.kind !== 'text' || Boolean((part.text ?? part.detail ?? part.summary ?? '').trim())),
    ),
  );
};

const ChatMessages: Component = () => {
  let scrollContainerRef: HTMLElement | undefined;
  const transcriptView = () => activeSession()?.transcriptView ?? 'standard';
  const [now, setNow] = createSignal(Date.now());
  const timer = window.setInterval(() => setNow(Date.now()), 1000);
  onCleanup(() => window.clearInterval(timer));

  const taskProjection = useMessageTaskProjection({
    activeSessionId,
    messages: () => chatMessageState.messages,
  });
  const scroll = useChatInfiniteScroll({
    getContainer: () => scrollContainerRef,
    activeSessionId,
    taskCount: () => taskProjection.projectedSubagentTasks().length,
  });
  const showPendingAssistant = () =>
    chatMessageState.sending &&
    !hasVisibleAssistantActivity(chatMessageState.messages, chatMessageState.activeTurnId);

  return (
    <section
      ref={scrollContainerRef}
      class={`navis-chat-messages ${transcriptViewClass(transcriptView())} relative flex min-h-0 flex-1 flex-col overflow-y-auto`}
      onScroll={scroll.handleMessagesScroll}
    >
      <div class="navis-chat-list flex w-full flex-col">
        <Show
          when={chatMessageState.messages.length > 0 || chatMessageState.sending || chatMessageState.pendingUserContent || chatMessageState.error}
          fallback={
            <div class="navis-chat-empty" role="status">
              {chatMessageState.loading
                ? 'Loading session messages'
                : chatMessageState.error
                  ? `Failed to load messages: ${chatMessageState.error}`
                  : activeSessionId()
                    ? 'This session has no messages yet'
                    : 'No session selected'}
            </div>
          }
        >
          <Show when={chatMessageState.loadingMore}>
            <div class="navis-chat-loading-older" role="status">Loading earlier messages</div>
          </Show>
          <Show when={chatMessageState.messages.length > 0}>
            <ConversationTranscript
              messages={chatMessageState.messages}
              transcriptView={transcriptView()}
              nowMs={now()}
              activeGuidance={chatMessageState.activeGuidance}
              activeTurnId={chatMessageState.activeTurnId}
              tasksByMessageId={taskProjection.tasksByMessageId()}
              onCopyContent={handleMessageContentCopy}
            />
          </Show>
          <PendingUserMessage
            content={chatMessageState.pendingUserContent}
            attachments={chatMessageState.pendingUserAttachments}
            onCopyContent={handleMessageContentCopy}
          />
          <Show when={showPendingAssistant()}>
            <PendingAssistantMessage />
          </Show>
        </Show>
      </div>
      <Show when={!scroll.stickToBottom() && (chatMessageState.messages.length > 0 || chatMessageState.sending)}>
        <button
          type="button"
          class="navis-chat-jump-latest"
          aria-label="Jump to latest message"
          title="Jump to latest"
          onClick={scroll.jumpToLatest}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9">
            <path d="M12 5v13M6.5 12.5 12 18l5.5-5.5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </Show>
    </section>
  );
};

export default ChatMessages;
