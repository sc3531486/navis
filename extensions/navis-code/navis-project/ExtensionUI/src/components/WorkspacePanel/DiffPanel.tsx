import { invoke } from '@tauri-apps/api/core';
import { EmptyState } from '@/components/ui/EmptyState';
import { Component, For, Show, createMemo, createResource, createSignal } from 'solid-js';
import UnifiedDiffViewer from '@editor-ext/components/ui/UnifiedDiffViewer';
import { chatMessageState } from '@session/stores/chat-messages';
import { activeSessionId } from '@session/stores/session-tree';
import {
  type UiSessionGitDiff,
  type UiSessionChange,
  visibleSessionChanges,
} from './shared';

const DiffPanel: Component = () => {
  const [staged, setStaged] = createSignal(false);
  const [creatingRepo, setCreatingRepo] = createSignal(false);
  const [diffResult, { refetch }] = createResource(
    () => {
      const sessionId = activeSessionId();
      if (!sessionId) return null;
      return { sessionId, staged: staged() };
    },
    ({ sessionId, staged }) => invoke<UiSessionGitDiff>('ui_get_session_git_diff', {
      payload: {
        sessionId,
        staged,
      },
    }),
  );
  const [sessionChanges, { refetch: refetchSessionChanges }] = createResource(
    () => {
      const sessionId = activeSessionId();
      if (!sessionId) return null;
      return {
        sessionId,
        activeTurnId: chatMessageState.activeTurnId,
        messageCount: chatMessageState.messages.length,
      };
    },
    ({ sessionId }) => invoke<UiSessionChange[]>('ui_list_session_changes', {
      payload: {
        sessionId,
      },
    }),
  );
  const visibleChanges = createMemo(() => visibleSessionChanges(sessionChanges() ?? []));
  const createRepo = async () => {
    const sessionId = activeSessionId();
    if (!sessionId || creatingRepo()) return;
    setCreatingRepo(true);
    try {
      await invoke<UiSessionGitDiff>('ui_create_session_git_repo', {
        payload: { sessionId },
      });
      refetch();
      refetchSessionChanges();
    } finally {
      setCreatingRepo(false);
    }
  };

  return (
    <div class="navis-workspace-diff">
      <header class="navis-workspace-diff-toolbar">
        <div class="navis-workspace-diff-tabs" role="tablist" aria-label="Diff scope">
          <button
            type="button"
            role="tab"
            aria-selected={!staged()}
            class={!staged() ? 'is-active' : ''}
            onClick={() => setStaged(false)}
          >
            Unstaged
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={staged()}
            class={staged() ? 'is-active' : ''}
            onClick={() => setStaged(true)}
          >
            Staged
          </button>
        </div>
        <button
          type="button"
          class="navis-workspace-inline-action"
          onClick={() => {
            refetch();
            refetchSessionChanges();
          }}
        >
          Refresh
        </button>
      </header>

      <Show
        when={activeSessionId()}
        fallback={<EmptyState title="No session selected" body="Select a session with a Worktree so the Diff panel can read its real Git diff." />}
      >
        <Show when={!diffResult.loading} fallback={<EmptyState title="Loading Diff" body="Reading Git diff from the current session Worktree." />}>
          <Show
            when={!diffResult.error}
            fallback={<EmptyState title="Failed to load Diff" body={String(diffResult.error)} />}
          >
            <Show when={visibleChanges().length > 0}>
              <section class="navis-workspace-diff-turn" aria-label="Session file changes">
                <header>
                  <span>{chatMessageState.activeTurnId ? 'Current turn changes' : 'Recent session changes'}</span>
                  <span>{visibleChanges().length} files</span>
                </header>
                <For each={visibleChanges()}>
                  {(change) => (
                    <article class="navis-workspace-diff-turn-file" title={change.absolutePath}>
                      <div>
                        <strong>{change.relativePath ?? change.absolutePath}</strong>
                        <span>{change.toolName} · {change.operation} · {change.status}</span>
                      </div>
                      <span class="is-add">+{change.insertions}</span>
                      <span class="is-delete">-{change.deletions}</span>
                    </article>
                  )}
                </For>
              </section>
            </Show>
            <Show when={diffResult() && !diffResult()!.isRepo}>
              <div class="navis-workspace-diff-repo-card">
                <div>
                  <strong>No Git repository</strong>
                  <p>{diffResult()?.worktreeRoot}</p>
                </div>
                <Show
                  when={diffResult()?.canCreateRepo}
                  fallback={<span class="navis-workspace-diff-muted">Repository creation unavailable for this Worktree.</span>}
                >
                  <button
                    type="button"
                    class="navis-workspace-inline-action"
                    disabled={creatingRepo()}
                    onClick={() => void createRepo()}
                  >
                    {creatingRepo() ? 'Creating...' : 'Create Git repo'}
                  </button>
                </Show>
              </div>
            </Show>
            <Show when={diffResult()?.isRepo}>
              <Show
                when={(diffResult()?.diff.length ?? 0) > 0 || (diffResult()?.fileChanges.length ?? 0) > 0}
                fallback={<EmptyState title="No changes" body={`${staged() ? 'The staged area' : 'The working tree'} has no Git diff to display.`} />}
              >
                <div class="navis-workspace-diff-summary">
                  <span>{diffResult()?.filesChanged ?? 0} files</span>
                  <span class="is-add">+{diffResult()?.insertions ?? 0}</span>
                  <span class="is-delete">-{diffResult()?.deletions ?? 0}</span>
                  <span class="navis-workspace-diff-root" title={diffResult()?.worktreeRoot}>
                    {diffResult()?.worktreeRoot}
                  </span>
                </div>
                <Show when={(diffResult()?.fileChanges.length ?? 0) > 0}>
                  <div class="navis-workspace-diff-files" aria-label="Changed files">
                    <For each={diffResult()?.fileChanges ?? []}>
                      {(file) => (
                        <button type="button" class={`navis-workspace-diff-file is-${file.status}`}>
                          <span class="navis-workspace-diff-file-status">{file.status}</span>
                          <span class="navis-workspace-diff-file-path">{file.path}</span>
                          <span class="navis-workspace-diff-file-stat is-add">+{file.insertions}</span>
                          <span class="navis-workspace-diff-file-stat is-delete">-{file.deletions}</span>
                        </button>
                      )}
                    </For>
                  </div>
                </Show>
                <UnifiedDiffViewer diff={diffResult()?.diff ?? ''} class="navis-workspace-diff-code" ariaLabel="Git diff" />
              </Show>
            </Show>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

export default DiffPanel;
