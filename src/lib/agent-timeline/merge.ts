import type { ChatAgentTimelinePart } from '../stream';

export function mergeAgentTimelinePart(parts: ChatAgentTimelinePart[], part: ChatAgentTimelinePart): ChatAgentTimelinePart[] {
  const index = parts.findIndex((item) => item.partId === part.partId);
  if (index < 0) return [...parts, part];
  const next = parts.slice();
  const current = next[index];
  const merged = { ...current, ...part };
  if (current.kind === 'text' && part.kind === 'text') {
    const currentText = current.text ?? '';
    const nextText = part.text ?? '';
    merged.text = nextText.length >= currentText.length ? nextText : currentText;
  }
  next[index] = merged;
  return next;
}
