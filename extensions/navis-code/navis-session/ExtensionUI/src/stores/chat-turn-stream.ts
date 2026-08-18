import { invoke } from '@tauri-apps/api/core';
import {
  runChannelStream,
  type ChatAgentTimelinePart,
  type SessionMessageStreamChunk,
  type StreamRunController,
  type StreamTermination,
  type ToolApprovalDecision,
} from '@/lib/stream';
import {
  appendAgentTimelinePartDeltaToSnapshot,
  isActionableToolApproval,
  markRunningAgentTimelinePartsInSnapshot,
  mergeSnapshotMessages,
  upsertAgentTimelinePartIntoSnapshot,
} from './chat-message-reducer';
import { chatMessageState, setChatMessageState } from './chat-message-state';
import type { SendChatMessageOptions } from './chat-message-types';

let activeStreamController: StreamRunController | null = null;

export function stopActiveChatStream(): void {
  activeStreamController?.stop('Superseded by a new chat stream');
  activeStreamController = null;
}

function upsertAgentTimelinePartIntoMessages(sessionId: string, part: ChatAgentTimelinePart): void {
  let insertedPendingUser = false;
  setChatMessageState('messages', (messages) => {
    const result = upsertAgentTimelinePartIntoSnapshot(
      messages,
      sessionId,
      part,
      chatMessageState.pendingUserContent,
      chatMessageState.pendingUserAttachments,
    );
    insertedPendingUser = result.insertedPendingUser;
    return result.messages;
  });
  if (insertedPendingUser) {
    setChatMessageState('pendingUserContent', null);
    setChatMessageState('pendingUserAttachments', []);
  }
}

function appendAgentTimelinePartDeltaToMessages(
  delta: Extract<SessionMessageStreamChunk, { type: 'agentTimelinePartDelta' }>,
): boolean {
  let applied = false;
  setChatMessageState('messages', (messages) => {
    const result = appendAgentTimelinePartDeltaToSnapshot(messages, delta);
    applied = result.applied;
    return result.messages;
  });
  return applied;
}

function markRunningAgentTimelineParts(status: 'aborted' | 'error', detail?: string): void {
  setChatMessageState('messages', (messages) =>
    markRunningAgentTimelinePartsInSnapshot(
      messages,
      chatMessageState.activeTurnId,
      status,
      detail,
    ),
  );
}

function completeTransientState(sessionId: string | null): void {
  activeStreamController = null;
  setChatMessageState({
    sessionId,
    sending: false,
    turnRunStatus: 'completed',
    activeTurnId: null,
    pendingUserContent: null,
    pendingUserAttachments: [],
    pendingApproval: null,
    streamingAssistantContent: '',
    activeGuidance: false,
  });
}

function handleStreamTermination(
  sessionId: string,
  termination: StreamTermination,
  options: SendChatMessageOptions,
): void {
  if (chatMessageState.sessionId !== sessionId) return;
  if (termination.kind === 'completed') {
    completeTransientState(sessionId);
    options.onTermination?.(termination);
    return;
  }

  if (termination.kind === 'stopped' || termination.kind === 'cancelled') {
    markRunningAgentTimelineParts('aborted', termination.reason ?? 'Stopped by user');
    activeStreamController = null;
    setChatMessageState({
      sending: false,
      turnRunStatus: 'aborted',
      loading: false,
      activeTurnId: null,
      pendingApproval: null,
      pendingUserAttachments: [],
      streamingAssistantContent: '',
      activeGuidance: false,
    });
    options.onTermination?.(termination);
    return;
  }

  const error = termination.error;
  markRunningAgentTimelineParts('error', error.message);
  activeStreamController = null;
  setChatMessageState({
    sending: false,
    turnRunStatus: 'error',
    loading: false,
    pendingApproval: null,
    pendingUserAttachments: [],
    streamingAssistantContent: '',
    activeGuidance: false,
    error: error.message,
  });
  options.onTermination?.(termination);
}

function handleStreamChunk(sessionId: string, chunk: SessionMessageStreamChunk): void {
  switch (chunk.type) {
    case 'messages':
      setChatMessageState({
        sessionId,
        messages: mergeSnapshotMessages(chatMessageState.messages, chunk.messages),
        total: chunk.total,
        pendingUserContent: null,
        pendingUserAttachments: [],
        streamingAssistantContent: '',
        activeGuidance: false,
      });
      return;
    case 'agentTimelinePart':
      if (!chunk.part.messageId) {
        setChatMessageState('error', 'Protocol error: agentTimelinePart.messageId is required');
        return;
      }
      upsertAgentTimelinePartIntoMessages(sessionId, chunk.part);
      if (chunk.part.kind === 'text') {
        setChatMessageState('streamingAssistantContent', chunk.part.text ?? '');
      }
      return;
    case 'agentTimelinePartDelta':
      if (!chunk.messageId || !chunk.partId) {
        setChatMessageState('error', 'Protocol error: agentTimelinePartDelta messageId/partId is required');
        return;
      }
      if (!appendAgentTimelinePartDeltaToMessages(chunk)) {
        setChatMessageState('error', 'Protocol error: agentTimelinePartDelta target step is missing');
        return;
      }
      if (chunk.field === 'text') {
        setChatMessageState('streamingAssistantContent', (value) => `${value}${chunk.delta}`);
      }
      return;
    case 'toolApproval':
      if (!isActionableToolApproval(sessionId, chunk.request)) {
        setChatMessageState('error', 'Protocol error: toolApproval request is not actionable');
        return;
      }
      setChatMessageState('pendingApproval', chunk.request);
      return;
  }
}

export async function sendChatMessage(
  sessionId: string,
  content: string,
  options: SendChatMessageOptions = {},
): Promise<void> {
  const text = content.trim();
  const attachments = options.attachments ?? [];
  if (!text && attachments.length === 0) return;
  const displayText = options.displayContent !== undefined ? options.displayContent.trim() : text;

  stopActiveChatStream();
  setChatMessageState({
    sessionId,
    sending: true,
    turnRunStatus: 'running',
    loading: false,
    activeTurnId: null,
    pendingUserContent: displayText,
    pendingUserAttachments: attachments,
    pendingApproval: null,
    streamingAssistantContent: '',
    activeGuidance: displayText.includes('Guidance from queued task:'),
    error: null,
  });

  activeStreamController = runChannelStream<SessionMessageStreamChunk>({
    command: 'ui_stream_session_message',
    args: { payload: { sessionId, content: text, displayContent: displayText, attachments } },
    onChunk: (chunk) => {
      if (chatMessageState.sessionId !== sessionId) return;
      if (chunk.type === 'agentTimelinePart') {
        setChatMessageState('activeTurnId', chunk.part.turnId);
      } else if (chunk.type === 'agentTimelinePartDelta') {
        setChatMessageState('activeTurnId', chunk.turnId);
      }
      handleStreamChunk(sessionId, chunk);
    },
    onTermination: (termination) => {
      handleStreamTermination(sessionId, termination, options);
    },
  });
}

export function stopChatMessage(): void {
  if (!activeStreamController) return;
  activeStreamController.stop('Stopped by user');
}

export async function respondToolApproval(
  requestId: string,
  decision: ToolApprovalDecision,
): Promise<void> {
  const accepted = await invoke<boolean>('ui_respond_tool_approval', {
    payload: { requestId, decision },
  });
  if (!accepted) {
    setChatMessageState('pendingApproval', (approval) =>
      approval?.requestId === requestId ? null : approval,
    );
    throw new Error('Tool approval request is no longer active');
  }
  setChatMessageState('pendingApproval', null);
}
