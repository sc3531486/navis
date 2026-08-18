import { createStore } from 'solid-js/store';
import type {
  ChatMessage,
  ChatMessageAttachment,
  ChatTurnRunStatus,
} from './chat-message-types';
import type { ToolApprovalRequest } from '@/lib/stream';

export const CHAT_MESSAGES_PAGE_SIZE = 60;

export const [chatMessageState, setChatMessageState] = createStore<{
  sessionId: string | null;
  messages: ChatMessage[];
  total: number;
  loading: boolean;
  loadingMore: boolean;
  sending: boolean;
  turnRunStatus: ChatTurnRunStatus;
  activeTurnId: string | null;
  pendingUserContent: string | null;
  pendingUserAttachments: ChatMessageAttachment[];
  pendingApproval: ToolApprovalRequest | null;
  streamingAssistantContent: string;
  activeGuidance: boolean;
  error: string | null;
}>({
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
