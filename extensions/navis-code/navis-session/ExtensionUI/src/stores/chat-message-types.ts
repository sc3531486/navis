import type { ChatAgentTimelinePart, StreamTermination } from '@/lib/stream';

export type ChatMessageRole = 'user' | 'assistant' | 'system' | 'tool';

export interface ChatMessageAttachment {
  kind: 'image' | 'file';
  name: string;
  mimeType?: string | null;
  sizeBytes?: number | null;
  dataBase64?: string | null;
  textContent?: string | null;
  isTruncated?: boolean | null;
  modelReadable?: boolean | null;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: ChatMessageRole;
  content: string;
  attachments: ChatMessageAttachment[];
  tokenCount?: number | null;
  createdAt: string;
  agentTimelineParts: ChatAgentTimelinePart[];
}

export type ChatTurnRunStatus = 'idle' | 'running' | 'completed' | 'error' | 'aborted';

export interface UiSessionMessages {
  messages: ChatMessage[];
  total: number;
}

export interface SendChatMessageOptions {
  displayContent?: string;
  attachments?: ChatMessageAttachment[];
  onTermination?: (termination: StreamTermination) => void;
}
