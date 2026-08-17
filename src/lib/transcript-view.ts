export type TranscriptView = 'standard' | 'compact' | 'raw';

export interface TranscriptMessageSnapshot {
  id: string;
  role: string;
  createdAt: string;
  tokenCount?: number | null;
  content: string;
  agentTimelineParts?: unknown[];
}

export function transcriptViewClass(view: TranscriptView): string {
  if (view === 'compact') return 'is-compact';
  if (view === 'raw') return 'is-raw';
  return 'is-standard';
}

export function rawTranscriptMessage(message: TranscriptMessageSnapshot): string {
  return JSON.stringify(
    {
      id: message.id,
      role: message.role,
      createdAt: message.createdAt,
      tokenCount: message.tokenCount ?? null,
      content: message.content,
      agentTimelineParts: message.agentTimelineParts ?? [],
    },
    null,
    2,
  );
}
