import { Accessor, createEffect, createSignal } from 'solid-js';

import { chatMessageState, loadOlderChatMessages } from '@session/stores/chat-messages';

interface UseChatInfiniteScrollOptions {
  getContainer: () => HTMLElement | undefined;
  activeSessionId: Accessor<string | null>;
  taskCount: Accessor<number>;
}

export function useChatInfiniteScroll(options: UseChatInfiniteScrollOptions) {
  let previousSending = false;
  let lastAutoScrollRevision = '';

  const isNearBottom = () => {
    const container = options.getContainer();
    if (!container) return true;
    return container.scrollHeight - container.scrollTop - container.clientHeight < 96;
  };

  const hasOlderMessages = () => chatMessageState.messages.length < chatMessageState.total;

  const scrollToBottom = () => {
    window.requestAnimationFrame(() => {
      const container = options.getContainer();
      if (!container) return;
      container.scrollTop = container.scrollHeight;
    });
  };

  const [stickToBottom, setStickToBottom] = createSignal(true);

  const loadOlderMessagesAtTop = async () => {
    const container = options.getContainer();
    if (!container || !hasOlderMessages() || chatMessageState.loadingMore) return;

    const previousScrollHeight = container.scrollHeight;
    const previousScrollTop = container.scrollTop;
    const loaded = await loadOlderChatMessages();
    if (!loaded) return;

    window.requestAnimationFrame(() => {
      const current = options.getContainer();
      if (!current) return;
      current.scrollTop = current.scrollHeight - previousScrollHeight + previousScrollTop;
    });
  };

  const handleMessagesScroll = () => {
    setStickToBottom(isNearBottom());
    const container = options.getContainer();
    if (container && container.scrollTop < 120) {
      void loadOlderMessagesAtTop();
    }
  };

  const jumpToLatest = () => {
    setStickToBottom(true);
    scrollToBottom();
  };

  createEffect(() => {
    const sending = chatMessageState.sending;
    if (sending && !previousSending) {
      setStickToBottom(true);
      scrollToBottom();
    }
    previousSending = sending;
  });

  createEffect(() => {
    options.activeSessionId();
    setStickToBottom(true);
    scrollToBottom();
  });

  createEffect(() => {
    const revision = [
      chatMessageState.sessionId,
      chatMessageState.messages.length,
      chatMessageState.pendingUserContent ?? '',
      chatMessageState.streamingAssistantContent.length,
      chatMessageState.sending,
      chatMessageState.turnRunStatus,
      chatMessageState.loading,
      options.taskCount(),
    ].join(':');

    if (revision !== lastAutoScrollRevision && stickToBottom()) {
      lastAutoScrollRevision = revision;
      scrollToBottom();
    }
  });

  return {
    stickToBottom,
    handleMessagesScroll,
    jumpToLatest,
  };
}
