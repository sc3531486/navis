import { For, Show, createMemo } from 'solid-js';
import type { Component } from 'solid-js';

type MessageContentBlock =
  | { kind: 'text'; text: string }
  | { kind: 'code'; language: string; code: string };

const textBlocks = (text: string): MessageContentBlock[] =>
  text
    .replace(/\r\n/g, '\n')
    .split(/\n{2,}/)
    .map((part) => part.trim())
    .filter((part) => part.length > 0)
    .map((part) => ({ kind: 'text', text: part }));

const trimCodeFenceContent = (code: string): string =>
  code.replace(/^\s*\n/, '').replace(/\n\s*$/, '');

const parseMessageContentBlocks = (content: string): MessageContentBlock[] => {
  const blocks: MessageContentBlock[] = [];
  const fencePattern = /```([^\n\r`]*)\r?\n([\s\S]*?)```/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = fencePattern.exec(content)) !== null) {
    blocks.push(...textBlocks(content.slice(cursor, match.index)));
    blocks.push({
      kind: 'code',
      language: match[1]?.trim() ?? '',
      code: trimCodeFenceContent(match[2] ?? ''),
    });
    cursor = match.index + match[0].length;
  }

  blocks.push(...textBlocks(content.slice(cursor)));
  return blocks.length > 0 ? blocks : [{ kind: 'text', text: content }];
};

const MessageContentRenderer: Component<{ content: string }> = (props) => {
  const blocks = createMemo(() => parseMessageContentBlocks(props.content));
  return (
    <For each={blocks()}>
      {(block) => (
        block.kind === 'code' ? (
          <pre class="navis-message-code-block">
            <Show when={block.language}>
              {(language) => <span class="navis-message-code-language">{language()}</span>}
            </Show>
            <code>{block.code}</code>
          </pre>
        ) : (
          <p class="navis-message-paragraph">{block.text}</p>
        )
      )}
    </For>
  );
};

export default MessageContentRenderer;
