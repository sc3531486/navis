import { Component, Show } from 'solid-js';

import gatewayChevronUrl from '@session/assets/gateway-v.svg';
import { FloatingMenu } from '@navis-code/components/Menu';
import { RightWorkspaceHeader } from '@navis-code/components/RightWorkspaceHeader';
import { ScreenIcon } from '@navis-code/components/Icon';
import { closeMenu, getMenuItems, isMenuOpen, toggleMenu, type MenuActionItem } from '@/stores/menu';
import { executeSessionMenuItem, getSessionSubmenuItems } from '@session/stores/session-menu';
import { activeSession, activeSessionId } from '@session/stores/session-tree';
import { loadEditorSettings, settingsState } from '@settings-ext/stores/settings';

const ChatHeader: Component = () => {
  const currentSessionTitle = () => activeSession()?.name ?? 'General coding session';
  const currentTranscriptView = () => activeSession()?.transcriptView ?? 'standard';

  async function handleTitleMenuSelect(item: MenuActionItem): Promise<void> {
    const sessionId = activeSessionId();
    if (!sessionId) {
      closeMenu();
      return;
    }
    const subject = currentSessionTitle();
    closeMenu();
    await executeSessionMenuItem(item, {
      sessionId,
      sessionName: subject,
    });
  }

  function toggleTitleMenu(): void {
    if (!settingsState.loaded) void loadEditorSettings();
    toggleMenu('chat-title');
  }

  return (
    <header class="navis-chat-header flex h-[36px] flex-shrink-0 items-center">
      <div class="navis-chat-header-icon flex h-6 w-6 items-center justify-center rounded-md">
        <ScreenIcon />
      </div>
      <div class="relative min-w-0 flex items-center" data-menu-anchor="chat-title">
        <div class="truncate text-[13px] font-medium" title={`Transcript view: ${currentTranscriptView()}`}>
          Navis Go / {currentSessionTitle()}
        </div>
        <button
          type="button"
          class="navis-chat-title-button flex h-5 w-5 items-center justify-center rounded-md"
          aria-label="Session title menu"
          aria-expanded={isMenuOpen('chat-title')}
          title="Session title menu"
          onClick={toggleTitleMenu}
        >
          <span
            class={`navis-chat-title-chevron ${isMenuOpen('chat-title') ? 'is-open' : ''}`}
            style={{ '--navis-chevron-url': `url("${gatewayChevronUrl}")` }}
            aria-hidden="true"
          />
        </button>
        <Show when={isMenuOpen('chat-title')}>
          <FloatingMenu
            items={getMenuItems('ChatTitle')}
            triggerLabel="Session title menu"
            width={178}
            selectedCommands={[`session.transcriptView.${currentTranscriptView()}`]}
            getSubmenuItems={(item) =>
              getSessionSubmenuItems(item, {
                sessionId: activeSessionId() ?? 'chat-title',
                target: 'ChatTitle',
              })}
            onSelect={handleTitleMenuSelect}
          />
        </Show>
      </div>
      <div class="flex-1" />
      <RightWorkspaceHeader />
    </header>
  );
};

export default ChatHeader;
