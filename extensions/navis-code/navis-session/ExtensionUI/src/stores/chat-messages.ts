import { invoke } from '@tauri-apps/api/core';
import { mergeSnapshotMessages } from './chat-message-reducer';
import {
  CHAT_MESSAGES_PAGE_SIZE,
  chatMessageState,
  setChatMessageState,
} from './chat-message-state';
import { stopActiveChatStream } from './chat-turn-stream';
import type { UiSessionMessages } from './chat-message-types';

export {
  chatMessageState,
  setChatMessageState,
} from './chat-message-state';
export {
  respondToolApproval,
  sendChatMessage,
  stopChatMessage,
} from './chat-turn-stream';
export type {
  ChatMessage,
  ChatMessageAttachment,
  ChatMessageRole,
  ChatTurnRunStatus,
  SendChatMessageOptions,
} from './chat-message-types';

export async function loadChatMessages(sessionId: string | null): Promise<void> {
  if (!sessionId) {
    stopActiveChatStream();
    setChatMessageState({
      sessionId: null,
      messages: [],
      total: 0,
      loading: false,
      loadingMore: false,
      sending: false,
      turnRunStatus: 'idle',
      activeTurnId: null,
      pendingUserContent: null,
      pendingUserAttachments: [],
      pendingApproval: null,
      streamingAssistantContent: '',
      activeGuidance: false,
      error: null,
    });
    return;
  }

  if (chatMessageState.sessionId !== sessionId) {
    stopActiveChatStream();
  }

  setChatMessageState({
    sessionId,
    loading: true,
    loadingMore: false,
    error: null,
    pendingApproval: null,
    activeGuidance: false,
  });

  try {
    const result = await invoke<UiSessionMessages>('ui_list_session_messages', {
      payload: { sessionId, limit: CHAT_MESSAGES_PAGE_SIZE, latest: true },
    });
    if (chatMessageState.sessionId !== sessionId) return;
    setChatMessageState({
      sessionId,
      messages: result.messages,
      total: result.total,
      loading: false,
      loadingMore: false,
      error: null,
    });
  } catch (error) {
    if (chatMessageState.sessionId !== sessionId) return;
    setChatMessageState({
      loading: false,
      loadingMore: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

export async function loadOlderChatMessages(): Promise<boolean> {
  const sessionId = chatMessageState.sessionId;
  if (!sessionId || chatMessageState.loading || chatMessageState.loadingMore) return false;

  const loaded = chatMessageState.messages.length;
  const remaining = Math.max(0, chatMessageState.total - loaded);
  if (remaining === 0) return false;

  const limit = Math.min(CHAT_MESSAGES_PAGE_SIZE, remaining);
  const offset = Math.max(0, remaining - limit);

  setChatMessageState({ loadingMore: true, error: null });
  try {
    const result = await invoke<UiSessionMessages>('ui_list_session_messages', {
      payload: { sessionId, limit, offset },
    });
    if (chatMessageState.sessionId !== sessionId) return false;
    setChatMessageState({
      messages: mergeSnapshotMessages(chatMessageState.messages, result.messages),
      total: result.total,
      loadingMore: false,
      error: null,
    });
    return result.messages.length > 0;
  } catch (error) {
    if (chatMessageState.sessionId !== sessionId) return false;
    setChatMessageState({
      loadingMore: false,
      error: error instanceof Error ? error.message : String(error),
    });
    return false;
  }
}
