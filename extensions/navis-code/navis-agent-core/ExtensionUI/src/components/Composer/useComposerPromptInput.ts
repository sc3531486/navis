import { createEffect, createSignal } from 'solid-js';
import type { Accessor, Setter } from 'solid-js';

export interface ComposerPromptInputOptions {
  inputValue: Accessor<string>;
  setInputValue: Setter<string>;
  promptHistory: Accessor<string[]>;
  focusToken: Accessor<number>;
}

const resizePromptInput = (textarea: HTMLTextAreaElement): void => {
  textarea.style.height = 'auto';
  textarea.style.height = `${Math.max(46, Math.min(textarea.scrollHeight, 160))}px`;
};

const isCaretOnFirstLine = (textarea: HTMLTextAreaElement): boolean =>
  textarea.value.slice(0, textarea.selectionStart).indexOf('\n') < 0;

const isCaretOnLastLine = (textarea: HTMLTextAreaElement): boolean =>
  textarea.value.slice(textarea.selectionEnd).indexOf('\n') < 0;

export function useComposerPromptInput(options: ComposerPromptInputOptions) {
  const [historyCursor, setHistoryCursor] = createSignal<number | null>(null);
  const [historyDraft, setHistoryDraft] = createSignal('');
  let textareaRef: HTMLTextAreaElement | undefined;

  const setTextareaRef = (element: HTMLTextAreaElement): void => {
    textareaRef = element;
  };

  const resizeInput = (textarea: HTMLTextAreaElement): void => {
    resizePromptInput(textarea);
  };

  const resetHistoryNavigation = (): void => {
    setHistoryCursor(null);
    setHistoryDraft('');
  };

  const replaceInputFromHistory = (value: string): void => {
    options.setInputValue(value);
    queueMicrotask(() => {
      if (!textareaRef) return;
      const cursor = textareaRef.value.length;
      textareaRef.setSelectionRange(cursor, cursor);
      resizePromptInput(textareaRef);
    });
  };

  const handlePromptHistoryKey = (event: KeyboardEvent): boolean => {
    if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return false;
    const textarea = event.currentTarget as HTMLTextAreaElement;
    if (event.key === 'ArrowUp' && !isCaretOnFirstLine(textarea)) return false;
    if (event.key === 'ArrowDown' && !isCaretOnLastLine(textarea)) return false;

    const history = options.promptHistory();
    if (history.length === 0) return false;

    event.preventDefault();
    const currentCursor = historyCursor();
    if (event.key === 'ArrowUp') {
      const nextCursor = currentCursor == null ? 0 : Math.min(history.length - 1, currentCursor + 1);
      if (currentCursor == null) setHistoryDraft(options.inputValue());
      setHistoryCursor(nextCursor);
      replaceInputFromHistory(history[nextCursor] ?? '');
      return true;
    }

    if (currentCursor == null) return true;
    const nextCursor = currentCursor - 1;
    if (nextCursor < 0) {
      setHistoryCursor(null);
      replaceInputFromHistory(historyDraft());
      return true;
    }
    setHistoryCursor(nextCursor);
    replaceInputFromHistory(history[nextCursor] ?? '');
    return true;
  };

  const focusInput = (): void => {
    textareaRef?.focus();
  };

  createEffect(() => {
    options.inputValue();
    if (textareaRef) resizePromptInput(textareaRef);
  });

  createEffect(() => {
    options.focusToken();
    if (!textareaRef) return;

    queueMicrotask(() => {
      if (!textareaRef) return;
      textareaRef.focus();
      const cursor = textareaRef.value.length;
      textareaRef.setSelectionRange(cursor, cursor);
      resizePromptInput(textareaRef);
    });
  });

  return {
    focusInput,
    handlePromptHistoryKey,
    resetHistoryNavigation,
    resizeInput,
    setTextareaRef,
  };
}
