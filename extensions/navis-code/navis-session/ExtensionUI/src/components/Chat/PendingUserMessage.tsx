import { Component, Show } from 'solid-js';

import type { ChatMessageAttachment } from '@session/stores/chat-messages';
import MessageAttachments from './MessageAttachments';

interface PendingUserMessageProps {
  content: string | null;
  attachments: ChatMessageAttachment[];
  onCopyContent: (event: ClipboardEvent, messageText: string) => void;
}

const PendingUserMessage: Component<PendingUserMessageProps> = (props) => (
  <Show when={Boolean(props.content) || props.attachments.length > 0}>
    <article class="navis-message is-user self-end">
      <MessageAttachments attachments={props.attachments} pending />
      <Show when={props.content}>
        {(content) => (
          <div
            class="navis-message-content navis-message-bubble navis-message-pending max-w-[760px] text-[14px]"
            onCopy={(event) => props.onCopyContent(event, content())}
          >
            <p>{content()}</p>
          </div>
        )}
      </Show>
    </article>
  </Show>
);

export default PendingUserMessage;
