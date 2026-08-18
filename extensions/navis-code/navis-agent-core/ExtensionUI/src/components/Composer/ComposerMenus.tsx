import { Component, For, JSX, Show } from 'solid-js';

import {
  ChecklistIcon,
  ConnectorIcon,
  FolderIcon,
  FolderPlusIcon,
  MultiAgentIcon,
  PaperclipIcon,
  ExtensionIcon,
  ScreenIcon,
  SlashIcon,
  TargetIcon,
} from '@navis-code/components/Icon';
import CloseIcon from '@/components/Icon/CloseIcon';
import {
  BUILTIN_INPUT_PLUS_COMMANDS,
  REASONING_EFFORT_OPTIONS,
} from '@agent-core/stores/composer-menu';
import type { MenuActionItem } from '@/stores/menu';
import type { GatewayModel, GatewayModelSelection } from '@project-ext/stores/gateway';
import type { ReasoningEffort } from '@session/stores/session-tree';
import type { RecentWorktree } from '@project-ext/stores/project';

export interface ComposerQuickMenuProps {
  items: MenuActionItem[];
  selectedCommands: string[];
  planModeEnabled: boolean;
  multiAgentEnabled: boolean;
  goalTrackingEnabled: boolean;
  onSelect: (item: MenuActionItem) => void;
}

export const ComposerSwitch: Component<{ checked: boolean }> = (props) => (
  <span class={`navis-composer-switch ${props.checked ? 'is-on' : ''}`} aria-hidden="true">
    <span />
  </span>
);

export const ComposerRunningChip: Component<{
  label: string;
  ariaLabel: string;
  onClose: () => void;
  running?: boolean;
  children: JSX.Element;
}> = (props) => (
  <button
    type="button"
    class={`navis-composer-running-chip ${props.running ? 'is-running' : ''}`}
    aria-label={props.ariaLabel}
    title={props.label}
    onClick={props.onClose}
  >
    <span class="navis-running-icon">{props.children}</span>
    <span class="navis-running-close" aria-hidden="true"><CloseIcon class="is-small" /></span>
    <span>{props.label}</span>
  </button>
);

export const ComposerPlusMenuItem: Component<{
  item: MenuActionItem;
  role?: 'menuitem' | 'menuitemcheckbox';
  checked?: boolean;
  selected?: boolean;
  onSelect: (item: MenuActionItem) => void;
  children: JSX.Element;
}> = (props) => (
  <button
    type="button"
    role={props.role ?? 'menuitem'}
    aria-checked={props.role === 'menuitemcheckbox' ? props.checked : undefined}
    class={`navis-composer-plus-item ${props.selected ? 'is-selected' : ''}`}
    onClick={() => props.onSelect(props.item)}
  >
    {props.children}
  </button>
);

export const ComposerQuickMenu: Component<ComposerQuickMenuProps> = (props) => {
  const itemByCommand = (command: string) => props.items.find((item) => item.command === command);
  const extensionItems = () => props.items.filter((item) => item.extensionId && !BUILTIN_INPUT_PLUS_COMMANDS.has(item.command));
  const isSelected = (item: MenuActionItem) => props.selectedCommands.includes(item.command);

  return (
    <div class="navis-composer-plus-menu" role="menu">
      <Show when={itemByCommand('composer.addFiles')}>
        {(item) => (
          <ComposerPlusMenuItem item={item()} onSelect={props.onSelect}>
            <PaperclipIcon />
            <span>{item().label}</span>
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.addFolder')}>
        {(item) => (
          <ComposerPlusMenuItem item={item()} onSelect={props.onSelect}>
            <FolderIcon />
            <span>{item().label}</span>
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.insertSlashCommand')}>
        {(item) => (
          <ComposerPlusMenuItem item={item()} onSelect={props.onSelect}>
            <SlashIcon />
            <span>{item().label}</span>
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.addConnectors')}>
        {(item) => (
          <ComposerPlusMenuItem item={item()} onSelect={props.onSelect}>
            <ConnectorIcon />
            <span>{item().label}</span>
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.addExtensions')}>
        {(item) => (
          <ComposerPlusMenuItem item={item()} onSelect={props.onSelect}>
            <ExtensionIcon />
            <span>{item().label}</span>
          </ComposerPlusMenuItem>
        )}
      </Show>

      <div class="navis-composer-plus-divider" role="separator" />

      <Show when={itemByCommand('composer.togglePlanMode')}>
        {(item) => (
          <ComposerPlusMenuItem
            item={item()}
            role="menuitemcheckbox"
            checked={props.planModeEnabled}
            selected={isSelected(item())}
            onSelect={props.onSelect}
          >
            <ChecklistIcon />
            <span>{item().label}</span>
            <ComposerSwitch checked={props.planModeEnabled} />
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.toggleMultiAgent')}>
        {(item) => (
          <ComposerPlusMenuItem
            item={item()}
            role="menuitemcheckbox"
            checked={props.multiAgentEnabled}
            selected={isSelected(item())}
            onSelect={props.onSelect}
          >
            <MultiAgentIcon />
            <span>{item().label}</span>
            <ComposerSwitch checked={props.multiAgentEnabled} />
          </ComposerPlusMenuItem>
        )}
      </Show>
      <Show when={itemByCommand('composer.toggleGoalTracking')}>
        {(item) => (
          <ComposerPlusMenuItem
            item={item()}
            role="menuitemcheckbox"
            checked={props.goalTrackingEnabled}
            selected={isSelected(item())}
            onSelect={props.onSelect}
          >
            <TargetIcon />
            <span>{item().label}</span>
            <ComposerSwitch checked={props.goalTrackingEnabled} />
          </ComposerPlusMenuItem>
        )}
      </Show>

      <Show when={extensionItems().length > 0}>
        <div class="navis-composer-plus-divider" role="separator" />
        <For each={extensionItems()}>
          {(item) => (
            <button
              type="button"
              role="menuitem"
              class="navis-composer-plus-item"
              onClick={() => props.onSelect(item)}
            >
              <span class="navis-composer-plus-extension-dot" aria-hidden="true" />
              <span>{item.label}</span>
            </button>
          )}
        </For>
      </Show>
    </div>
  );
};

export const ComposerWorktreeMenu: Component<{
  currentPath: string | null;
  recentWorktrees: RecentWorktree[];
  onSelect: (path: string | null) => void;
  onChooseNew: () => void;
}> = (props) => (
  <div class="navis-worktree-menu" role="menu">
    <div class="navis-worktree-menu-title">Recent worktrees (last 10)</div>
    <button
      type="button"
      role="menuitemradio"
      aria-checked={!props.currentPath}
      class={`navis-worktree-menu-item ${!props.currentPath ? 'is-selected' : ''}`}
      onClick={() => props.onSelect(null)}
    >
      <ScreenIcon />
      <span class="navis-worktree-menu-name">Local</span>
      <span class="navis-worktree-menu-path">No worktree folder</span>
    </button>
    <Show
      when={props.recentWorktrees.length > 0}
      fallback={<div class="navis-worktree-menu-empty">No recent worktrees</div>}
    >
      <For each={props.recentWorktrees.slice(0, 10)}>
        {(worktree) => (
          <button
            type="button"
            role="menuitemradio"
            aria-checked={props.currentPath === worktree.path}
            class={`navis-worktree-menu-item ${props.currentPath === worktree.path ? 'is-selected' : ''}`}
            title={worktree.path}
            onClick={() => props.onSelect(worktree.path)}
          >
            <FolderIcon />
            <span class="navis-worktree-menu-name">{worktree.name}</span>
            <span class="navis-worktree-menu-path">{worktree.path}</span>
          </button>
        )}
      </For>
    </Show>
    <div class="navis-worktree-menu-divider" role="separator" />
    <button
      type="button"
      role="menuitem"
      class="navis-worktree-menu-item is-action"
      onClick={props.onChooseNew}
    >
      <FolderPlusIcon />
      <span class="navis-worktree-menu-name">Choose new worktree</span>
    </button>
  </div>
);

export const ComposerProviderMenu: Component<{
  providers: Array<{ id: string; name: string }>;
  currentProviderId: string;
  onSelectProvider: (providerId: string) => void;
}> = (props) => (
  <div class="navis-provider-menu" role="menu" aria-label="Provider">
    <Show when={props.providers.length > 0} fallback={<div class="navis-model-effort-empty">No providers configured</div>}>
      <For each={props.providers}>
        {(provider) => {
          const selected = () => provider.id === props.currentProviderId;
          return (
            <button
              type="button"
              role="menuitemradio"
              aria-checked={selected()}
              class={`navis-provider-menu-item ${selected() ? 'is-selected' : ''}`}
              title={provider.name || provider.id}
              onClick={() => props.onSelectProvider(provider.id)}
            >
              <span>{provider.id}</span>
              <small>{provider.name || provider.id}</small>
              <span class="navis-model-effort-dot" aria-hidden="true" />
            </button>
          );
        }}
      </For>
    </Show>
  </div>
);

export const ComposerModelEffortMenu: Component<{
  models: GatewayModel[];
  currentSelection: GatewayModelSelection | null;
  currentEffort: ReasoningEffort;
  onSelectModel: (selection: GatewayModelSelection) => void;
  onSelectEffort: (effort: ReasoningEffort) => void;
}> = (props) => {
  const modelShortcut = (index: number) => (index < 9 ? String(index + 1) : '');

  return (
    <div class="navis-model-effort-menu" role="menu" aria-label="Model and reasoning effort">
      <div class="navis-model-effort-section-title">
        <span>Models</span>
        <span class="navis-model-effort-shortcut">Shift Ctrl I</span>
      </div>
      <Show
        when={props.models.length > 0}
        fallback={<div class="navis-model-effort-empty">No models configured</div>}
      >
        <For each={props.models}>
          {(model, index) => {
            const selected = () =>
              model.providerId === props.currentSelection?.providerId && model.id === props.currentSelection?.modelId;
            return (
              <button
                type="button"
                role="menuitemradio"
                aria-checked={selected()}
                class={`navis-model-effort-item ${selected() ? 'is-selected' : ''}`}
                title={`${model.providerId} / ${model.id}`}
                onClick={() => props.onSelectModel({ providerId: model.providerId, modelId: model.id })}
              >
                <span class="navis-model-effort-label">{model.name || model.id}</span>
                <span class="navis-model-effort-meta">{modelShortcut(index())}</span>
                <span class="navis-model-effort-dot" aria-hidden="true" />
              </button>
            );
          }}
        </For>
      </Show>
      <div class="navis-model-effort-divider" role="separator" />
      <div class="navis-model-effort-section-title">
        <span>Effort</span>
        <span class="navis-model-effort-shortcut">Shift Ctrl E</span>
      </div>
      <For each={REASONING_EFFORT_OPTIONS}>
        {(effort) => {
          const selected = () => effort.value === props.currentEffort;
          return (
            <button
              type="button"
              role="menuitemradio"
              aria-checked={selected()}
              class={`navis-model-effort-item ${selected() ? 'is-selected' : ''}`}
              onClick={() => props.onSelectEffort(effort.value)}
            >
              <span class="navis-model-effort-label">{effort.label}</span>
              <span class="navis-model-effort-meta">{effort.shortcut}</span>
              <span class="navis-model-effort-dot" aria-hidden="true" />
            </button>
          );
        }}
      </For>
    </div>
  );
};
