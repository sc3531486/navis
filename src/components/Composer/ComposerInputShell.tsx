import { Component, For, Show } from 'solid-js';

import type { Command } from '../CommandPalette/store';
import CloseIcon from '../Icon/CloseIcon';
import { FolderIcon, SendIcon, StopIcon } from '../Icon';
import { SlashCommandDropdown } from '../SlashCommandDropdown';
import type { ComposerAttachment } from './useComposerAttachments';

interface ComposerInputShellProps {
  attachments: () => ComposerAttachment[];
  inputValue: () => string;
  placeholder: () => string;
  showSlashDropdown: () => boolean;
  slashQuery: () => string;
  slashCommands: () => Command[];
  sending: () => boolean;
  loading: () => boolean;
  setTextareaRef: (element: HTMLTextAreaElement) => void;
  onInput: (value: string, textarea: HTMLTextAreaElement) => void;
  onPaste: (event: ClipboardEvent) => void;
  onPromptHistoryKey: (event: KeyboardEvent) => boolean;
  onSubmit: () => void;
  onStop: () => void;
  onRemoveAttachment: (attachmentId: string) => void;
  onSlashSelect: (command: Command) => void;
  onSlashDismiss: () => void;
}

const ComposerInputShell: Component<ComposerInputShellProps> = (props) => (
  <>
    <Show when={props.attachments().length > 0}>
      <div class="navis-composer-attachments" aria-label="Attachments">
        <For each={props.attachments()}>
          {(attachment) => (
            <div class={`navis-composer-attachment is-${attachment.kind}`} title={attachment.detail}>
              <Show
                when={attachment.previewUrl}
                fallback={
                  <span class="navis-composer-attachment-icon" aria-hidden="true">
                    {attachment.kind === 'folder' ? <FolderIcon /> : <span>&lt;/&gt;</span>}
                  </span>
                }
              >
                {(previewUrl) => (
                  <img class="navis-composer-attachment-preview" src={previewUrl()} alt="" />
                )}
              </Show>
              <span class="navis-composer-attachment-main">
                <span class="navis-composer-attachment-name">{attachment.name}</span>
                <span class="navis-composer-attachment-detail">{attachment.kind === 'image' ? 'Image' : attachment.kind === 'folder' ? 'Folder' : 'File'}</span>
              </span>
              <button
                type="button"
                class="navis-composer-attachment-remove"
                aria-label={`Remove ${attachment.name}`}
                title="Remove"
                onClick={() => props.onRemoveAttachment(attachment.id)}
              >
                <CloseIcon class="is-small" />
              </button>
            </div>
          )}
        </For>
      </div>
    </Show>
    <div class="relative">
      <SlashCommandDropdown
        visible={props.showSlashDropdown()}
        query={props.slashQuery()}
        commands={props.slashCommands()}
        onSelect={props.onSlashSelect}
        onDismiss={props.onSlashDismiss}
      />
      <textarea
        ref={props.setTextareaRef}
        class="navis-composer-textarea w-full resize-none text-[13px] leading-5 outline-none"
        rows={1}
        placeholder={props.placeholder()}
        value={props.inputValue()}
        onInput={(event) => props.onInput(event.currentTarget.value, event.currentTarget)}
        onPaste={props.onPaste}
        onKeyDown={(event) => {
          if (props.onPromptHistoryKey(event)) return;
          if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            props.onSubmit();
          }
        }}
      />
      <button
        type="button"
        class="navis-send-button absolute flex h-8 w-8 items-center justify-center rounded-md"
        classList={{ 'is-running': props.sending() }}
        aria-label={props.sending() ? 'Stop response' : 'Send message'}
        title={props.sending() ? 'Stop response' : 'Send message'}
        disabled={props.loading() && !props.sending()}
        onClick={() => {
          if (props.sending()) {
            props.onStop();
            return;
          }
          props.onSubmit();
        }}
      >
        <Show when={props.sending()} fallback={<SendIcon />}>
          <StopIcon />
        </Show>
      </button>
    </div>
  </>
);

export default ComposerInputShell;
