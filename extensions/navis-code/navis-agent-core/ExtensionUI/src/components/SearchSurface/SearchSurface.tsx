import { Dialog } from '@kobalte/core/dialog';
import { createEffect, onMount, type JSX } from 'solid-js';

export interface SearchSurfaceProps {
  open: boolean;
  title: string;
  description: string;
  placeholder: string;
  query: string;
  onOpenChange: (open: boolean) => void;
  onQueryChange: (query: string) => void;
  onKeyDown: (event: KeyboardEvent) => void;
  leadingAccessory?: JSX.Element;
  children: JSX.Element;
}

export function SearchSurface(props: SearchSurfaceProps): JSX.Element {
  let inputRef: HTMLInputElement | undefined;

  createEffect(() => {
    if (!props.open) return;
    props.query;
    requestAnimationFrame(() => inputRef?.focus());
  });

  onMount(() => {
    if (props.open) inputRef?.focus();
  });

  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange} modal={true}>
      <Dialog.Portal>
        <Dialog.Overlay class="navis-search-surface-overlay" />
        <div class="navis-search-surface-positioner">
          <Dialog.Content class="navis-search-surface" onOpenAutoFocus={(event) => event.preventDefault()}>
            <Dialog.Title class="sr-only">{props.title}</Dialog.Title>
            <Dialog.Description class="sr-only">{props.description}</Dialog.Description>

            <div class="navis-search-surface-input-row">
              <svg
                class="navis-search-surface-icon"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.8"
                aria-hidden="true"
              >
                <circle cx="10.5" cy="10.5" r="6.5" />
                <path d="M16 16l5 5" stroke-linecap="round" />
              </svg>
              {props.leadingAccessory}
              <input
                ref={inputRef}
                class="navis-search-surface-input"
                type="text"
                value={props.query}
                placeholder={props.placeholder}
                autocomplete="off"
                spellcheck={false}
                role="combobox"
                aria-label={props.title}
                aria-autocomplete="list"
                onInput={(event) => props.onQueryChange(event.currentTarget.value)}
                onKeyDown={props.onKeyDown}
              />
              <Dialog.CloseButton class="navis-search-surface-close" aria-label={`Close ${props.title}`}>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                  <path d="M5 5l14 14M19 5L5 19" stroke-linecap="round" />
                </svg>
              </Dialog.CloseButton>
            </div>

            <div class="navis-search-surface-body">{props.children}</div>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  );
}

