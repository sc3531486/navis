import type {
  ChatAgentTimelinePart,
  SessionMessageStreamChunk,
  ToolApprovalRequest,
} from '@/lib/stream';
import {
  applyAgentTimelinePartDelta,
  agentTimelinePartText,
  mergeAgentTimelinePart,
} from '@agent-core/lib/agent-timeline';
import type { ChatMessage, ChatMessageAttachment } from './chat-message-types';
import { cancelledStatusPresentation, failedStatusPresentation, isStatusLive } from '@/lib/status';

function createAssistantShell(sessionId: string, part: ChatAgentTimelinePart): ChatMessage {
  return {
    id: part.messageId,
    sessionId,
    role: 'assistant',
    content: agentTimelinePartText(part),
    attachments: [],
    tokenCount: null,
    createdAt: new Date().toISOString(),
    agentTimelineParts: [part],
  };
}

function createPendingUserMessage(
  sessionId: string,
  part: ChatAgentTimelinePart,
  content: string,
  attachments: ChatMessageAttachment[],
): ChatMessage {
  return {
    id: part.turnId,
    sessionId,
    role: 'user',
    content,
    attachments,
    tokenCount: null,
    createdAt: new Date().toISOString(),
    agentTimelineParts: [],
  };
}

function updateMessageWithAgentTimelinePart(
  message: ChatMessage,
  part: ChatAgentTimelinePart,
): ChatMessage {
  const agentTimelineParts = mergeAgentTimelinePart(message.agentTimelineParts, part);
  const partText = agentTimelinePartText(part);
  return {
    ...message,
    content: partText ? partText : message.content,
    agentTimelineParts,
  };
}

function mergeAgentTimelineParts(
  existing: ChatAgentTimelinePart[],
  incoming: ChatAgentTimelinePart[],
): ChatAgentTimelinePart[] {
  let next = existing;
  for (const part of incoming) {
    next = mergeAgentTimelinePart(next, part);
  }
  return next.slice().sort((left, right) => left.sequence - right.sequence);
}

export function mergeSnapshotMessages(
  existing: ChatMessage[],
  incoming: ChatMessage[],
): ChatMessage[] {
  const existingById = new Map(existing.map((message) => [message.id, message]));
  const incomingIds = new Set(incoming.map((message) => message.id));
  const mergedIncoming = incoming.map((message) => {
    const current = existingById.get(message.id);
    if (!current) return message;
    return {
      ...current,
      ...message,
      agentTimelineParts: mergeAgentTimelineParts(
        current.agentTimelineParts,
        message.agentTimelineParts,
      ),
    };
  });
  const localOnly = existing.filter((message) => !incomingIds.has(message.id));
  return [...mergedIncoming, ...localOnly].sort(
    (left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt),
  );
}

export function upsertAgentTimelinePartIntoSnapshot(
  messages: ChatMessage[],
  sessionId: string,
  part: ChatAgentTimelinePart,
  pendingUserContent: string | null,
  pendingUserAttachments: ChatMessageAttachment[],
): { messages: ChatMessage[]; insertedPendingUser: boolean } {
  const index = messages.findIndex((message) => message.id === part.messageId);

  if (index < 0) {
    const pendingContent = pendingUserContent?.trim();
    const hasTurnUserMessage = messages.some((message) => message.id === part.turnId);
    const hasPendingUser = Boolean(pendingContent) || pendingUserAttachments.length > 0;
    const nextMessages = hasPendingUser && !hasTurnUserMessage
      ? [
          ...messages,
          createPendingUserMessage(sessionId, part, pendingContent ?? '', pendingUserAttachments),
        ]
      : messages;
    return {
      messages: [...nextMessages, createAssistantShell(sessionId, part)],
      insertedPendingUser: nextMessages !== messages,
    };
  }

  const next = messages.slice();
  next[index] = updateMessageWithAgentTimelinePart(next[index], part);
  return { messages: next, insertedPendingUser: false };
}

export function appendAgentTimelinePartDeltaToSnapshot(
  messages: ChatMessage[],
  delta: Extract<SessionMessageStreamChunk, { type: 'agentTimelinePartDelta' }>,
): { messages: ChatMessage[]; applied: boolean } {
  return applyAgentTimelinePartDelta(messages, delta);
}

export function markRunningAgentTimelinePartsInSnapshot(
  messages: ChatMessage[],
  activeTurnId: string | null,
  status: 'aborted' | 'error',
  detail?: string,
): ChatMessage[] {
  if (!activeTurnId) return messages;

  const completedAt = new Date().toISOString();
  const completePart = (part: ChatAgentTimelinePart): ChatAgentTimelinePart => {
    const startedAtMs = Date.parse(part.startedAt ?? part.createdAt);
    const completedAtMs = Date.parse(completedAt);
    const durationMs = Number.isNaN(startedAtMs)
      ? part.durationMs
      : Math.max(0, completedAtMs - startedAtMs);
    return {
      ...part,
      status,
      statusPresentation: status === 'error'
        ? failedStatusPresentation
        : cancelledStatusPresentation,
      detail: detail ?? part.detail,
      updatedAt: completedAt,
      completedAt,
      durationMs,
    };
  };
  const updatePart = (part: ChatAgentTimelinePart): ChatAgentTimelinePart =>
    part.turnId === activeTurnId &&
    isStatusLive(part.statusPresentation)
      ? completePart(part)
      : part;

  return messages.map((message) => ({
    ...message,
    agentTimelineParts: message.agentTimelineParts.map(updatePart),
  }));
}

export function isActionableToolApproval(
  sessionId: string,
  request: ToolApprovalRequest,
): boolean {
  return Boolean(request.requestId && request.sessionId === sessionId);
}
