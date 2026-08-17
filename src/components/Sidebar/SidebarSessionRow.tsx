import { Component, Show } from 'solid-js';
import { FloatingMenu } from '../Menu';
import type { MenuActionItem } from '../../stores/menu';
import type { SidebarSession } from '../../stores/session-tree';

export const SidebarSessionRow: Component<{
  session: SidebarSession;
  menuId: string;
  selected: boolean;
  contextActive: boolean;
  menuItems: MenuActionItem[];
  availableWorktreeNames: string[];
  currentWorktreeName?: string;
  menuIconUrl: string;
  runningIconUrl: string;
  completedTaskIconUrl: string;
  getSubmenuItems: (item: MenuActionItem) => MenuActionItem[];
  onSelectSession: () => void;
  onOpenMenu: () => void;
  onToggleMenu: () => void;
  onMenuSelect: (item: MenuActionItem) => Promise<void>;
}> = (props) => (
  <div
    class={`navis-sidebar-session h-7 w-full rounded-md text-left ${
      props.selected ? 'is-selected' : ''
    } ${props.contextActive ? 'is-context-active' : ''}`}
    data-menu-anchor={props.menuId}
    onContextMenu={(event) => {
      event.preventDefault();
      props.onOpenMenu();
    }}
  >
    <span class="navis-selection-ring">
      <Show when={props.selected}>
        <span class="navis-selection-dot" />
      </Show>
    </span>
    <button
      type="button"
      class="min-w-0 flex-1 truncate text-left"
      onClick={props.onSelectSession}
    >
      {props.session.name}
    </button>
    <Show when={props.session.unread}>
      <span class="navis-session-unread-dot" aria-label="未读" />
    </Show>
    <Show when={props.session.pinned}>
      <span class="navis-session-pin" aria-label="已固定">Pinned</span>
    </Show>
    <button
      type="button"
      class={`navis-sidebar-more-button navis-session-more-button ${
        props.session.hasRunningTask ? 'is-running' : ''
      } ${props.session.hasCompletedTask ? 'has-completed-task' : ''}`}
      aria-label={`${props.session.name} session menu`}
      aria-expanded={props.contextActive}
      onClick={(event) => {
        event.stopPropagation();
        props.onToggleMenu();
      }}
    >
      <span
        class="navis-sidebar-action-icon navis-sidebar-menu-icon"
        style={{ '--navis-sidebar-action-icon-url': `url("${props.menuIconUrl}")` }}
        aria-hidden="true"
      />
      <Show when={props.session.hasRunningTask}>
        <span
          class="navis-session-running-action-icon"
          style={{ '--navis-session-running-icon-url': `url("${props.runningIconUrl}")` }}
          aria-hidden="true"
        />
      </Show>
      <Show when={!props.session.hasRunningTask && props.session.hasCompletedTask}>
        <span
          class="navis-session-completed-action-icon"
          style={{ '--navis-session-completed-icon-url': `url("${props.completedTaskIconUrl}")` }}
          aria-hidden="true"
        />
      </Show>
    </button>
    <Show when={props.contextActive}>
      <FloatingMenu
        items={props.menuItems}
        triggerLabel="会话更多菜单"
        align="right"
        width={178}
        getSubmenuItems={props.getSubmenuItems}
        onSelect={props.onMenuSelect}
      />
    </Show>
  </div>
);
