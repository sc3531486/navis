import { Component, Show } from 'solid-js';
import { FloatingMenu } from '../Menu';
import type { MenuActionItem } from '../../stores/menu';
import type { SessionWorktree } from '../../stores/session-tree';

interface SidebarWorktreeTitleProps {
  worktree: SessionWorktree;
  menuId: string;
  isContextActive: boolean;
  isActive: boolean;
  hasRunningTask: boolean;
  menuItems: MenuActionItem[];
  projectFolderCloseUrl: string;
  projectFolderOpenUrl: string;
  disclosureIconUrl: string;
  menuIconUrl: string;
  editIconUrl: string;
  onToggleCollapsed: () => void;
  onOpenContextMenu: () => void;
  onToggleMenu: () => void;
  onRename: () => void;
  onMenuSelect: (item: MenuActionItem) => Promise<void>;
}

export const SidebarWorktreeTitle: Component<SidebarWorktreeTitleProps> = (props) => (
  <div
    class={`navis-sidebar-worktree-title w-full text-left ${
      props.isContextActive ? 'is-context-active' : ''
    } ${props.isActive ? 'is-active' : ''} ${props.hasRunningTask ? 'has-running-task' : ''}`}
    data-menu-anchor={props.menuId}
    onContextMenu={(event) => {
      event.preventDefault();
      props.onOpenContextMenu();
    }}
  >
    <button
      type="button"
      class="navis-sidebar-worktree-main min-w-0 flex-1 truncate text-left"
      onClick={props.onToggleCollapsed}
    >
      <span
        class="navis-sidebar-worktree-icon"
        style={{
          '--navis-project-icon-url': `url("${
            props.worktree.collapsed ? props.projectFolderCloseUrl : props.projectFolderOpenUrl
          }")`,
        }}
        aria-hidden="true"
      />
      <span class="min-w-0 truncate">{props.worktree.name}</span>
      <span
        class={`navis-sidebar-worktree-disclosure ${props.worktree.collapsed ? 'is-collapsed' : ''}`}
        style={{ '--navis-sidebar-disclosure-url': `url("${props.disclosureIconUrl}")` }}
        aria-hidden="true"
      />
    </button>
    <button
      type="button"
      class="navis-sidebar-more-button"
      aria-label={`${props.worktree.name} worktree menu`}
      aria-expanded={props.isContextActive}
      onClick={(event) => {
        event.stopPropagation();
        props.onToggleMenu();
      }}
    >
      <span
        class="navis-sidebar-action-icon"
        style={{ '--navis-sidebar-action-icon-url': `url("${props.menuIconUrl}")` }}
        aria-hidden="true"
      />
    </button>
    <button
      type="button"
      class="navis-sidebar-edit-button"
      aria-label={`Rename ${props.worktree.name} worktree`}
      title="Rename worktree"
      onClick={(event) => {
        event.stopPropagation();
        props.onRename();
      }}
    >
      <span
        class="navis-sidebar-action-icon"
        style={{ '--navis-sidebar-action-icon-url': `url("${props.editIconUrl}")` }}
        aria-hidden="true"
      />
    </button>
    <Show when={props.isContextActive}>
      <FloatingMenu
        items={props.menuItems}
        triggerLabel="Worktree"
        align="right"
        width={160}
        showSourceLabel={false}
        onSelect={props.onMenuSelect}
      />
    </Show>
  </div>
);
