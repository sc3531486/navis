import { Component, For, Show } from 'solid-js';
import type { JSX } from 'solid-js';

import type { GatewayModel, GatewayModelSelection } from '../../stores/gateway';
import type { MenuActionItem } from '../../stores/menu';
import { getMenuItems, isMenuOpen, toggleMenu } from '../../stores/menu';
import type { ReasoningEffort } from '../../stores/session-tree';
import { FloatingMenu } from '../Menu';
import { ChecklistIcon, ChevronDown, MultiAgentIcon, PlusIcon, TargetIcon } from '../Icon';
import {
  ComposerModelEffortMenu,
  ComposerProviderMenu,
  ComposerQuickMenu,
  ComposerRunningChip,
} from './ComposerMenus';
import ContextRing from './ContextRing';
import { composerInlineExtensionPoints } from '../../stores/extension-points';
import InlineExtensionPoint from '../ExtensionInline/InlineExtensionPoint';

interface ComposerToolbarProps {
  variant: () => 'docked' | 'start-session' | 'start-task';
  worktreeSelector: () => JSX.Element;
  permissionMenuItems: () => MenuActionItem[];
  currentPermissionLabel: () => string;
  currentPermissionPolicy: () => string;
  planModeEnabled: () => boolean;
  multiAgentEnabled: () => boolean;
  goalTrackingEnabled: () => boolean;
  goalRunning: () => boolean;
  currentProviderLabel: () => string;
  providerMenuItems: () => Array<{ id: string; name: string }>;
  currentProviderId: () => string;
  currentModelEffortLabel: () => string;
  currentProviderModels: () => GatewayModel[];
  currentModelSelection: () => GatewayModelSelection | null;
  currentReasoningEffort: () => ReasoningEffort;
  onPermissionSelect: (item: MenuActionItem) => void;
  onInputPlusSelect: (item: MenuActionItem) => void;
  onDisablePlanMode: () => void;
  onToggleMultiAgent: () => void;
  onClearGoal: () => void;
  onProviderSelect: (providerId: string) => void;
  onModelTriggerClick: () => void;
  onModelSelect: (selection: GatewayModelSelection) => void;
  onReasoningEffortSelect: (effort: ReasoningEffort) => void;
}

const ComposerToolbar: Component<ComposerToolbarProps> = (props) => (
  <section class="navis-composer-toolbar flex min-h-[26px] items-center justify-between">
    <div class="navis-composer-toolbar-left flex min-w-0 items-center">
      <Show when={props.variant() === 'start-task'}>
        <div class="navis-composer-task-worktree" aria-label="Task worktree">
          {props.worktreeSelector()}
        </div>
      </Show>
<For each={composerInlineExtensionPoints()}>
        {(point) => (
          <InlineExtensionPoint
            point={point}
            class="navis-composer-extension-action h-[22px] rounded-md px-1.5 text-[11px] hover:bg-[#f0f0f0]"
          />
        )}
      </For>      <div class="navis-permission-menu-anchor" data-menu-anchor="composer-permission">
        <button
          type="button"
          class="navis-permission-trigger h-[22px] rounded-md text-[11px] outline-none"
          aria-label={`Permission policy: ${props.currentPermissionLabel()}`}
          title="Permission policy"
          aria-expanded={isMenuOpen('composer-permission')}
          onClick={() => toggleMenu('composer-permission')}
        >
          <span>{props.currentPermissionLabel()}</span>
          <ChevronDown />
        </button>
        <Show when={isMenuOpen('composer-permission')}>
          <FloatingMenu
            items={props.permissionMenuItems()}
            triggerLabel="Permission policy"
            placement="above"
            width={210}
            selectedCommands={[`composer.permission:${props.currentPermissionPolicy()}`]}
            onSelect={props.onPermissionSelect}
          />
        </Show>
      </div>
      <div class="relative" data-menu-anchor="input-plus">
        <button
          type="button"
          class="navis-composer-plus-trigger flex h-[22px] w-[22px] items-center justify-center rounded-md"
          aria-label="Add menu"
          title="Add menu"
          aria-expanded={isMenuOpen('input-plus')}
          onClick={() => toggleMenu('input-plus')}
        >
          <PlusIcon />
        </button>
        <Show when={isMenuOpen('input-plus')}>
          <ComposerQuickMenu
            items={getMenuItems('InputPlus')}
            selectedCommands={[
              ...(props.planModeEnabled() ? ['composer.togglePlanMode'] : []),
              ...(props.multiAgentEnabled() ? ['composer.toggleMultiAgent'] : []),
              ...(props.goalTrackingEnabled() ? ['composer.toggleGoalTracking'] : []),
            ]}
            planModeEnabled={props.planModeEnabled()}
            multiAgentEnabled={props.multiAgentEnabled()}
            goalTrackingEnabled={props.goalTrackingEnabled()}
            onSelect={props.onInputPlusSelect}
          />
        </Show>
      </div>
      <Show when={props.planModeEnabled()}>
        <ComposerRunningChip
          label="Plan"
          ariaLabel="Close plan mode"
          onClose={props.onDisablePlanMode}
          running={props.planModeEnabled()}
        >
          <ChecklistIcon />
        </ComposerRunningChip>
      </Show>
      <Show when={props.multiAgentEnabled()}>
        <ComposerRunningChip
          label="Multi-agent"
          ariaLabel="Disable multi-agent"
          onClose={props.onToggleMultiAgent}
          running={props.multiAgentEnabled()}
        >
          <MultiAgentIcon />
        </ComposerRunningChip>
      </Show>
      <Show when={props.goalTrackingEnabled()}>
        <ComposerRunningChip
          label="Goal"
          ariaLabel="Close goal tracking"
          onClose={props.onClearGoal}
          running={props.goalRunning()}
        >
          <TargetIcon />
        </ComposerRunningChip>
      </Show>
    </div>
    <div class="navis-composer-toolbar-right flex flex-shrink-0 items-center">
      <div class="relative" data-menu-anchor="composer-provider">
        <button
          type="button"
          class="navis-provider-button h-[22px] rounded-md"
          aria-label={`Provider selector: ${props.currentProviderLabel()}`}
          aria-expanded={isMenuOpen('composer-provider')}
          title="Provider selector"
          onClick={() => toggleMenu('composer-provider')}
        >
          {props.currentProviderLabel()}
        </button>
        <Show when={isMenuOpen('composer-provider')}>
          <ComposerProviderMenu
            providers={props.providerMenuItems()}
            currentProviderId={props.currentProviderId()}
            onSelectProvider={props.onProviderSelect}
          />
        </Show>
      </div>
      <div class="relative" data-menu-anchor="composer-model">
        <button
          type="button"
          class="navis-model-button h-[22px] rounded-md"
          aria-label={`Model and effort selector: ${props.currentModelEffortLabel()}`}
          aria-expanded={isMenuOpen('composer-model')}
          title="Model and effort selector"
          onClick={props.onModelTriggerClick}
        >
          {props.currentModelEffortLabel()}
        </button>
        <Show when={isMenuOpen('composer-model')}>
          <ComposerModelEffortMenu
            models={props.currentProviderModels()}
            currentSelection={props.currentModelSelection()}
            currentEffort={props.currentReasoningEffort()}
            onSelectModel={props.onModelSelect}
            onSelectEffort={props.onReasoningEffortSelect}
          />
        </Show>
      </div>
      <ContextRing />
    </div>
  </section>
);

export default ComposerToolbar;

