/**
 * SlashCommandDropdown - 输入框上方的 Slash 命令下拉列表
 *
 * 在输入框中输入 "/" 时显示，列出所有可用的 slash commands。
 * 支持键盘导航（↑↓ Enter Escape）和模糊过滤。
 */
import { Component, For, Show, createMemo, createSignal, onCleanup, onMount } from 'solid-js';
import { commandPaletteState, type Command } from '../components/CommandPalette/store';

interface SlashCommandDropdownProps {
  visible: boolean;
  query: string;
  commands: Command[];
  onSelect: (command: Command) => void;
  onDismiss: () => void;
}

const ICONS: Record<string, string> = {
  'builtin:command': '🔗',
  'builtin:skill': '⚡',
  'extension:command': '🔌',
  'extension:skill': '🧩',
};

function getIcon(command: Command): string {
  const key = `${command.source === 'skill' ? 'builtin' : command.source}:${command.source === 'command' ? 'command' : 'skill'}`;
  return ICONS[key] ?? '📋';
}

export const SlashCommandDropdown: Component<SlashCommandDropdownProps> = (props) => {
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  let containerRef: HTMLDivElement | undefined;

  const filtered = createMemo(() => {
    const q = props.query.toLowerCase().replace(/^\//, '').trim();
    if (!q) return props.commands;
    return props.commands.filter(
      (c) =>
        c.label.toLowerCase().includes(q) ||
        (c.description?.toLowerCase().includes(q) ?? false) ||
        c.tags?.some((t) => t.toLowerCase().includes(q)),
    );
  });

  // 重置选中项
  const resetSelection = () => setSelectedIndex(0);

  // 键盘导航
  const handleKeyDown = (e: KeyboardEvent) => {
    if (!props.visible) return;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, filtered().length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (filtered()[selectedIndex()]) {
          props.onSelect(filtered()[selectedIndex()]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        props.onDismiss();
        break;
    }
  };

  // 点击外部关闭
  const handleClickOutside = (e: MouseEvent) => {
    if (containerRef && !containerRef.contains(e.target as Node)) {
      props.onDismiss();
    }
  };

  onMount(() => {
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleClickOutside);
  });

  onCleanup(() => {
    document.removeEventListener('keydown', handleKeyDown);
    document.removeEventListener('mousedown', handleClickOutside);
  });

  // 当 query 变化时重置选中项
  const prevQuery = createMemo(() => props.query);
  if (prevQuery() !== props.query) {
    resetSelection();
  }

  return (
    <Show when={props.visible && filtered().length > 0}>
      <div
        ref={containerRef}
        class="slash-command-dropdown"
        role="listbox"
        aria-label="Slash commands"
      >
        <For each={filtered()}>
          {(command, index) => (
            <div
              class={`slash-command-item ${index() === selectedIndex() ? 'is-selected' : ''}`}
              role="option"
              aria-selected={index() === selectedIndex()}
              onMouseEnter={() => setSelectedIndex(index())}
              onClick={() => props.onSelect(command)}
            >
              <span class="slash-command-icon">{getIcon(command)}</span>
              <div class="slash-command-text">
                <span class="slash-command-name">{command.label}</span>
                <Show when={command.description}>
                  <span class="slash-command-desc">{command.description}</span>
                </Show>
              </div>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
};
