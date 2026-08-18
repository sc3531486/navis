import { createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from 'solid-js';
import { SearchSurface } from '@agent-core/components/SearchSurface';
import {
  activeSessionId,
  allSessions,
  createSession,
  loadSessionTree,
  selectSession,
  setSessionWorktreeRoot,
  type SidebarSession,
} from '@session/stores/session-tree';
import { loadRecentWorktrees, projectState, type RecentWorktree } from '@project-ext/stores/project';

type GlobalSearchItem =
  | { id: string; kind: 'chat'; label: string; detail: string; session: SidebarSession }
  | { id: string; kind: 'worktree'; label: string; detail: string; worktree: RecentWorktree };

const [isGlobalSearchOpen, setGlobalSearchOpen] = createSignal(false);

export const globalSearchAPI = {
  open(): void {
    setGlobalSearchOpen(true);
  },
  close(): void {
    setGlobalSearchOpen(false);
  },
};

function matches(query: string, values: Array<string | null | undefined>): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return values.some((value) => value?.toLowerCase().includes(normalized));
}

export function GlobalSearchPalette(): JSX.Element {
  const [query, setQuery] = createSignal('');
  const [selectedIndex, setSelectedIndex] = createSignal(0);

  onMount(() => {
    void loadSessionTree();
    void loadRecentWorktrees(20);
  });

  const items = createMemo<GlobalSearchItem[]>(() => {
    const q = query();
    const chats: GlobalSearchItem[] = allSessions()
      .filter((session) => matches(q, [session.name, session.worktreeRoot, session.providerId, session.modelId]))
      .map((session) => ({
        id: `chat:${session.id}`,
        kind: 'chat',
        label: session.name || 'Untitled chat',
        detail: session.worktreeRoot ?? 'Chat',
        session,
      }));

    const worktrees: GlobalSearchItem[] = projectState.recentWorktrees
      .filter((worktree) => matches(q, [worktree.name, worktree.path]))
      .map((worktree) => ({
        id: `worktree:${worktree.id}`,
        kind: 'worktree',
        label: worktree.name,
        detail: worktree.path,
        worktree,
      }));

    return [...chats, ...worktrees].slice(0, 40);
  });

  const close = (): void => {
    setGlobalSearchOpen(false);
    setQuery('');
    setSelectedIndex(0);
  };

  const executeItem = async (item: GlobalSearchItem): Promise<void> => {
    close();
    if (item.kind === 'chat') {
      await selectSession(item.session.id);
      return;
    }

    const sessionId = activeSessionId() ?? (await createSession('code', item.worktree.name));
    if (sessionId) {
      await setSessionWorktreeRoot(sessionId, item.worktree.path);
    }
  };

  const handleKeyDown = (event: KeyboardEvent): void => {
    const count = items().length;
    if (event.key === 'Escape') {
      close();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setSelectedIndex((index) => (count === 0 ? 0 : (index + 1) % count));
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      setSelectedIndex((index) => (count === 0 ? 0 : (index - 1 + count) % count));
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const item = items()[selectedIndex()];
      if (item) void executeItem(item);
    }
  };

  onCleanup(() => setGlobalSearchOpen(false));

  return (
    <SearchSurface
      open={isGlobalSearchOpen()}
      title="Global search"
      description="Search chats and worktrees. Use arrow keys to navigate and Enter to open."
      placeholder="Search chats and worktrees"
      query={query()}
      onOpenChange={(open) => (open ? setGlobalSearchOpen(true) : close())}
      onQueryChange={(value) => {
        setQuery(value);
        setSelectedIndex(0);
      }}
      onKeyDown={handleKeyDown}
    >
      <div class="navis-global-search-list" role="listbox" aria-label="Search results">
        <Show
          when={items().length > 0}
          fallback={
            <div class="navis-search-empty">
              <div class="navis-search-empty-title">No matching chats or worktrees</div>
              <div class="navis-search-empty-detail">Try another name or Worktree path.</div>
            </div>
          }
        >
          <For each={items()}>
            {(item, index) => (
              <button
                type="button"
                class={`navis-global-search-item ${index() === selectedIndex() ? 'is-selected' : ''}`}
                role="option"
                aria-selected={index() === selectedIndex()}
                onMouseEnter={() => setSelectedIndex(index())}
                onClick={() => void executeItem(item)}
              >
                <span class="navis-global-search-glyph" aria-hidden="true">
                  {item.kind === 'chat' ? '</>' : '⌘'}
                </span>
                <span class="navis-global-search-copy">
                  <span class="navis-global-search-label">{item.label}</span>
                  <span class="navis-global-search-detail">{item.detail}</span>
                </span>
                <span class="navis-global-search-meta">
                  {index() === selectedIndex() ? 'Enter' : item.kind === 'chat' ? 'Chat' : 'Worktree'}
                </span>
              </button>
            )}
          </For>
        </Show>
      </div>
    </SearchSurface>
  );
}
