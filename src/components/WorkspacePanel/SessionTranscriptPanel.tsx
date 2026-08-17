import { invoke } from '@tauri-apps/api/core';
import { EmptyState } from '../ui/EmptyState';
import { Component, Show, createMemo, createResource, createSignal, onCleanup } from 'solid-js';
import { chatMessageState } from '../../stores/chat-messages';
import { getSession } from '../../stores/session-tree';
import { transcriptViewClass, type TranscriptView } from '../../lib/transcript-view';
import ConversationTranscript from '../Chat/ConversationTranscript';
import type { RightWorkspacePanel } from '../../stores/app';
import {
  type UiSessionMessages,
} from './shared';
import { WorkspacePanelScrollArea } from './WorkspacePanelFrame';

const SessionTranscriptPanel: Component<{ panel: RightWorkspacePanel }> = (props) => {
  const transcriptView = (): TranscriptView => getSession(props.panel.sessionId ?? null)?.transcriptView ?? 'standard';
  const [now, setNow] = createSignal(Date.now());
  const [refreshTick, setRefreshTick] = createSignal(0);
  const refreshTimer = window.setInterval(() => {
    setNow(Date.now());
    setRefreshTick((tick) => tick + 1);
  }, 1_500);
  onCleanup(() => window.clearInterval(refreshTimer));

  const [messages] = createResource(
    () => {
      const sessionId = props.panel.sessionId;
      if (!sessionId) return null;
      const activeRevision =
        chatMessageState.sessionId === sessionId
          ? `${chatMessageState.total}:${chatMessageState.messages.length}:${chatMessageState.loading}`
          : 'inactive';
      return { sessionId, activeRevision, refreshTick: refreshTick() };
    },
    async ({ sessionId }) => invoke<UiSessionMessages>('ui_list_session_messages', {
      payload: {
        sessionId,
        limit: 100,
        latest: true,
      },
    }),
  );
  const messagesPayload = createMemo(() => messages.latest ?? messages());

  return (
    <div class={`navis-workspace-session ${transcriptViewClass(transcriptView())}`}>
      <Show when={props.panel.sessionId} fallback={
        <EmptyState
          title={props.panel.title}
          body="This session panel is missing a session ID, so Navis Go cannot read backend messages."
        />
      }>
        <Show when={!messages.loading || messagesPayload()} fallback={<EmptyState title="Loading session" body="Reading current session messages from the backend." />}>
          <Show
            when={!messages.error}
            fallback={<EmptyState title="Failed to load session" body={String(messages.error)} />}
          >
            <Show
              when={(messagesPayload()?.messages.length ?? 0) > 0}
              fallback={<EmptyState title="No session messages" body="The current session has no messages yet. New messages will sync here after you send them." />}
            >
              <div class="navis-workspace-session-count">
                {messagesPayload()?.total ?? 0} messages
              </div>
              <WorkspacePanelScrollArea class="navis-workspace-session-list">
                <ConversationTranscript
                  messages={messagesPayload()?.messages ?? []}
                  transcriptView={transcriptView()}
                  nowMs={now()}
                  showRoleLabel={false}
                />
              </WorkspacePanelScrollArea>
            </Show>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

export default SessionTranscriptPanel;
