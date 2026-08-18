import { chatMessageState } from '@session/stores/chat-messages';
import { isStatusLive } from '@/lib/status';

export type AgentRuntimeStatus =
  | 'idle'
  | 'thinking'
  | 'tool_calling'
  | 'waiting_permission'
  | 'streaming'
  | 'error';

export function agentRuntimeStatus(): AgentRuntimeStatus {
  if (chatMessageState.error) return 'error';
  if (chatMessageState.pendingApproval) return 'waiting_permission';

  const hasRunningToolCall = chatMessageState.messages.some((message) =>
    message.agentTimelineParts.some((part) => part.kind === 'tool' && isStatusLive(part.statusPresentation)),
  );
  if (hasRunningToolCall) return 'tool_calling';

  if (chatMessageState.streamingAssistantContent) return 'streaming';
  if (chatMessageState.turnRunStatus === 'running' || chatMessageState.loading) return 'thinking';

  return 'idle';
}
