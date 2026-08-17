/**
 * ============================================================
 * navis-session 扩展前端 — 迁移过渡期 re-export 桥
 * ============================================================
 *
 * 实际文件仍保留在 src/components/Chat/ 和 src/stores/chat*
 * 后续 Phase 执行物理搬迁时，此处的 re-export 路径将同步更新。
 *
 * 归属说明：
 *   - Chat 组件 → 扩展 navis-session
 *   - chat-* stores → 扩展 navis-session
 *   - chatMessages CSS → 扩展 navis-session
 * ============================================================
 */

// ── Chat 组件 ────────────────────────────────────────────
export { default as AgentBatchSummary } from '@/components/Chat/AgentBatchSummary';
export { default as ChatHeader } from '@/components/Chat/ChatHeader';
export { default as ChatMessages } from '@/components/Chat/ChatMessages';
export { default as ConversationMessage } from '@/components/Chat/ConversationMessage';
export { default as ConversationTranscript } from '@/components/Chat/ConversationTranscript';
export { default as MessageAttachments } from '@/components/Chat/MessageAttachments';
export { panelActions } from '@/components/Chat/panel-actions';
export { default as PendingAssistantMessage } from '@/components/Chat/PendingAssistantMessage';
export { default as PendingUserMessage } from '@/components/Chat/PendingUserMessage';
export { useChatInfiniteScroll } from '@/components/Chat/useChatInfiniteScroll';
export { useMessageTaskProjection } from '@/components/Chat/useMessageTaskProjection';

// ── Chat Stores ──────────────────────────────────────────
export {
  chatMessageState,
  chatMessagesState,
  loadChatMessages,
} from '@/stores/chat-messages';

export type {
  ChatMessage,
  ChatMessageState,
} from '@/stores/chat-messages';

export {
  chatMessageReducer,
  applyChatMessageDelta,
} from '@/stores/chat-message-reducer';

export {
  chatTurnStreamState,
  setChatTurnStreamState,
} from '@/stores/chat-turn-stream';

export type {
  ChatTurnStream,
  ChatTurnStreamState,
} from '@/stores/chat-turn-stream';

export {
  chatMessageLoadingState,
  setChatMessageLoadingState,
} from '@/stores/chat-message-state';

export type {
  ChatMessageLoading,
} from '@/stores/chat-message-state';

export type {
  ChatMessageType,
} from '@/stores/chat-message-types';
