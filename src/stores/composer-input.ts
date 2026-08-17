import { createSignal } from 'solid-js';

const PROMPT_HISTORY_KEY = 'navis.composer.promptHistory.v1';
const MAX_PROMPT_HISTORY_ITEMS = 80;

export const [composerInputValue, setComposerInputValue] = createSignal('');
export const [composerInputFocusToken, setComposerInputFocusToken] = createSignal(0);

function readPromptHistoryStorage(): string[] {
  if (typeof window === 'undefined') return [];
  try {
    const raw = window.localStorage.getItem(PROMPT_HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed)
      ? parsed.filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
      : [];
  } catch {
    return [];
  }
}

function writePromptHistoryStorage(items: string[]): void {
  if (typeof window === 'undefined') return;
  window.localStorage.setItem(PROMPT_HISTORY_KEY, JSON.stringify(items.slice(0, MAX_PROMPT_HISTORY_ITEMS)));
}

export function composerPromptHistory(): string[] {
  return readPromptHistoryStorage();
}

export function rememberComposerPrompt(prompt: string): void {
  const normalized = prompt.trim();
  if (!normalized) return;
  const next = [
    normalized,
    ...readPromptHistoryStorage().filter((item) => item !== normalized),
  ].slice(0, MAX_PROMPT_HISTORY_ITEMS);
  writePromptHistoryStorage(next);
}

export function requestComposerInputFocus(): void {
  setComposerInputFocusToken((current) => current + 1);
}

export function appendComposerInputText(text: string): void {
  if (!text.trim()) return;

  setComposerInputValue((current) => {
    const nextText = text.trimStart();
    if (!current.trim()) return nextText;
    if (/\s$/.test(current)) return `${current}${nextText}`;
    return `${current} ${nextText}`;
  });
  requestComposerInputFocus();
}

export function insertComposerSlashTrigger(trigger: string): void {
  appendComposerInputText(`${trigger.trim()} `);
}
