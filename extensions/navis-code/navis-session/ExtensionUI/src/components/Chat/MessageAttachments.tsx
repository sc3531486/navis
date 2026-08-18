import { Component, For, Show } from 'solid-js';

import { PaperclipIcon } from '@navis-code/components/Icon';
import type { ChatMessageAttachment } from '@session/stores/chat-messages';

type MessageAttachmentsProps = {
  attachments: ChatMessageAttachment[];
  pending?: boolean;
};

const attachmentImageSrc = (attachment: ChatMessageAttachment): string | null => {
  if (attachment.kind !== 'image' || !attachment.dataBase64) return null;
  return `data:${attachment.mimeType || 'image/png'};base64,${attachment.dataBase64}`;
};

const attachmentKindLabel = (attachment: ChatMessageAttachment): string => {
  if (attachment.kind === 'image') return 'Image';
  if (attachment.modelReadable) return attachment.isTruncated ? 'Text file, truncated' : 'Text file';
  return 'File';
};

export const MessageAttachments: Component<MessageAttachmentsProps> = (props) => (
  <Show when={props.attachments.length > 0}>
    <div class={`navis-message-attachments ${props.pending ? 'is-pending' : ''}`} aria-label="Message attachments">
      <For each={props.attachments}>
        {(attachment) => {
          const imageSrc = () => attachmentImageSrc(attachment);
          return (
            <div class={`navis-message-attachment is-${attachment.kind}`} title={attachment.name}>
              <Show
                when={imageSrc()}
                fallback={
                  <span class="navis-message-attachment-file-icon" aria-hidden="true">
                    <PaperclipIcon />
                  </span>
                }
              >
                {(src) => (
                  <img
                    class="navis-message-attachment-image"
                    src={src()}
                    alt={attachment.name}
                    loading="lazy"
                  />
                )}
              </Show>
              <Show when={attachment.kind !== 'image'}>
                <span class="navis-message-attachment-meta">
                  <span class="navis-message-attachment-name">{attachment.name}</span>
                  <span class="navis-message-attachment-kind">{attachmentKindLabel(attachment)}</span>
                </span>
              </Show>
            </div>
          );
        }}
      </For>
    </div>
  </Show>
);

export default MessageAttachments;
