import { Component, Show } from 'solid-js';

import { isMenuOpen, toggleMenu } from '../../stores/menu';
import { worktreeLabel } from '../../stores/composer-worktree';
import type { RecentWorktree } from '../../stores/project';
import { ChevronDown, FolderIcon, FolderPlusIcon, ScreenIcon } from '../Icon';
import { ComposerWorktreeMenu } from './ComposerMenus';

interface ComposerWorktreeSelectorProps {
  currentWorktreeRoot: () => string | null;
  recentWorktrees: () => RecentWorktree[];
  onSelect: (worktreeRoot: string | null) => void;
  onChooseNew: () => void;
}

const ComposerWorktreeSelector: Component<ComposerWorktreeSelectorProps> = (props) => {
  const currentLabel = () => worktreeLabel(props.currentWorktreeRoot());

  return (
    <div class="navis-worktree-selector-anchor" data-menu-anchor="composer-worktrees">
      <button
        type="button"
        class={`navis-worktree-chip is-current ${props.currentWorktreeRoot() ? '' : 'is-active'}`}
        aria-label={`Current worktree: ${currentLabel()}`}
        aria-expanded={isMenuOpen('composer-worktrees')}
        title={props.currentWorktreeRoot() ?? 'No worktree folder'}
        onClick={() => toggleMenu('composer-worktrees')}
      >
        <Show when={props.currentWorktreeRoot()} fallback={<ScreenIcon />}>
          <FolderIcon />
        </Show>
        <span>{currentLabel()}</span>
        <ChevronDown />
      </button>
      <button
        type="button"
        class="navis-worktree-folder-add"
        aria-label="Open recent worktrees"
        title="Recent worktrees"
        aria-expanded={isMenuOpen('composer-worktrees')}
        onClick={() => toggleMenu('composer-worktrees')}
      >
        <FolderPlusIcon />
      </button>
      <Show when={isMenuOpen('composer-worktrees')}>
        <ComposerWorktreeMenu
          currentPath={props.currentWorktreeRoot()}
          recentWorktrees={props.recentWorktrees()}
          onSelect={props.onSelect}
          onChooseNew={props.onChooseNew}
        />
      </Show>
    </div>
  );
};

export default ComposerWorktreeSelector;
