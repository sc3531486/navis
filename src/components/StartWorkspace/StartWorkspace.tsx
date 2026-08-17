import { Component, JSX, Show } from 'solid-js';

import { agentState } from '../../stores/agent';
import { appState } from '../../stores/app';
import { activeSession } from '../../stores/session-tree';
import { resolvedComposerModelSelection } from '../../stores/composer-session';
import { modelLabel } from '../../stores/composer-menu';
import { worktreeLabel } from '../../stores/composer-worktree';
import { gatewayState } from '../../stores/gateway';

interface StartWorkspaceProps {
  composer: (variant: 'start-session' | 'start-task') => JSX.Element;
}

const StartWorkspace: Component<StartWorkspaceProps> = (props) => {
  const sessionMode = () => {
    const activeMode = activeSession()?.mode;
    if (activeMode) return activeMode;
    const workMode = agentState.workMode;
    return workMode.type === 'custom' ? `custom:${workMode.runtimeId}` : workMode.type;
  };
  const isCowork = () => sessionMode() === 'cowork';
  const startKind = () => appState.pendingStartKind ?? (isCowork() ? 'task' : 'session');
  const modeLabel = () => {
    if (sessionMode() === 'cowork') return 'Cowork';
    if (sessionMode() === 'code') return 'Code';
    return 'Custom mode';
  };
  const startWorktreeLabel = () => worktreeLabel(activeSession()?.worktreeRoot?.trim() || null);
  const currentModelLabel = () => modelLabel(gatewayState.models, resolvedComposerModelSelection());

  return (
    <main class={`navis-start-workspace is-${startKind()}`}>
      <Show
        when={startKind() === 'task'}
        fallback={
          <section class="navis-start-session-page" aria-label="Create session">
            <div class="navis-start-session-main">
              <div class="navis-start-heading">
                <span class="navis-start-mark" aria-hidden="true" />
                <div>
                  <h1>What’s up next?</h1>
                </div>
              </div>
              <div class="navis-start-session-overview" aria-label="Session start overview">
                <div>
                  <span>Mode</span>
                  <strong>{modeLabel()}</strong>
                </div>
                <div>
                  <span>Project</span>
                  <strong>{startWorktreeLabel()}</strong>
                </div>
                <div>
                  <span>Model</span>
                  <strong>{currentModelLabel()}</strong>
                </div>
              </div>
            </div>
            {props.composer('start-session')}
          </section>
        }
      >
        <section class="navis-start-task-page" aria-label="Create task">
          <div class="navis-start-heading">
            <span class="navis-start-mark" aria-hidden="true" />
            <div>
              <h1>Let's knock something off your list</h1>
            </div>
          </div>
          {props.composer('start-task')}
        </section>
      </Show>
    </main>
  );
};

export default StartWorkspace;
