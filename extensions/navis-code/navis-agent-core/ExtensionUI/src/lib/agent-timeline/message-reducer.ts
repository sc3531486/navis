import type { ChatAgentTimelinePart, AgentTimelinePartDeltaStreamChunk } from '@/lib/stream';

export interface ChatMessageTimelineState {
  id: string;
  content: string;
  agentTimelineParts: ChatAgentTimelinePart[];
}

export function agentTimelinePartText(part: ChatAgentTimelinePart): string {
  if (part.kind !== 'text') return '';
  if (part.source === 'gateway_tool_prelude') return '';
  return part.text ?? part.detail ?? part.summary ?? '';
}

function appendDelta(part: ChatAgentTimelinePart, delta: AgentTimelinePartDeltaStreamChunk): ChatAgentTimelinePart {
  if (delta.field === 'text') return { ...part, text: `${part.text ?? ''}${delta.delta}` };
  if (delta.field === 'detail') return { ...part, detail: `${part.detail ?? ''}${delta.delta}` };
  return { ...part, summary: `${part.summary ?? ''}${delta.delta}` };
}

export function applyAgentTimelinePartDelta<TMessage extends ChatMessageTimelineState>(
  messages: TMessage[],
  delta: AgentTimelinePartDeltaStreamChunk,
): { messages: TMessage[]; applied: boolean } {
  let applied = false;
  const nextMessages = messages.map((message) => {
    if (message.id !== delta.messageId) return message;
    let matchedPart: ChatAgentTimelinePart | null = null;
    const agentTimelineParts = message.agentTimelineParts.map((part) => {
      if (part.partId !== delta.partId) return part;
      applied = true;
      matchedPart = appendDelta(part, delta);
      return matchedPart;
    });
    if (!matchedPart) return message;
    return {
      ...message,
      content: agentTimelinePartText(matchedPart) || message.content,
      agentTimelineParts,
    } as TMessage;
  });
  return { messages: nextMessages, applied };
}
