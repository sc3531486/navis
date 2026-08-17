import { Component, Show, createEffect, createResource, createSignal, onCleanup, onMount } from 'solid-js';
import { closeMenu, isMenuOpen, toggleMenu, type MenuActionItem } from '../../stores/menu';
import {
  buildPermissionMenuItems,
  executeComposerInputPlusItem,
  executeComposerModelSelection,
  executeComposerPermissionMenuItem,
  executeComposerReasoningEffortSelection,
  modelNameLabel,
  permissionLabel,
  reasoningEffortLabel,
  resolveComposerModelMenuTrigger,
} from '../../stores/composer-menu';
import {
  bindComposerWorktree,
  chooseComposerWorktree,
  rememberRecentWorktree,
} from '../../stores/composer-worktree';
import { insertComposerSlashTrigger } from '../../stores/composer-input';
import { openPlanPanel } from '../Chat/panel-actions';
import { requestEditorWorktreeFileOpen } from '../Editor/stores/editor-worktree';
import { dialog } from '../Dialog';
import type { AgentConfirmConfig } from '../Dialog';
import { openSettingsDialog } from '../Settings/openSettingsDialog';
import { commandPaletteState, type Command } from '../CommandPalette/store';
import UnifiedDiffViewer from '../ui/UnifiedDiffViewer';
import { gatewayState, loadGatewayCatalog, type GatewayModelSelection } from '../../stores/gateway';
import {
  chatMessageState,
  respondToolApproval,
  stopChatMessage,
} from '../../stores/chat-messages';
import type { ToolApprovalDecision } from '../../lib/stream';
import {
  composerRunState,
  disablePlanMode,
  clearRunningComposerRuntimeTask,
  toggleGoalTracking,
  toggleMultiAgent,
  togglePlanMode,
} from '../../stores/composer-run';
import {
  activeSession,
  activeSessionId,
  type ReasoningEffort,
} from '../../stores/session-tree';
import {
  resolvedComposerModelSelection,
  resolvedComposerPermissionPolicy,
  resolvedComposerReasoningEffort,
} from '../../stores/composer-session';
import {
  composerInputFocusToken,
  composerInputValue,
  composerPromptHistory,
  setComposerInputValue,
} from '../../stores/composer-input';
import { loadRecentWorktrees, projectState } from '../../stores/project';
import { sessionTodosState, subscribeSessionTodosPolling } from '../../stores/session-todos';
import { useComposerAttachments, type ComposerInputAttachment } from './useComposerAttachments';
import { useComposerPromptInput } from './useComposerPromptInput';
import ComposerApprovalPanel from './ComposerApprovalPanel';
import ComposerInputShell from './ComposerInputShell';
import ComposerRunStack from './ComposerRunStack';
import ComposerToolbar from './ComposerToolbar';
import ComposerWorktreeSelector from './ComposerWorktreeSelector';
import { useComposerSession } from './useComposerSession';
import { useComposerRunControls } from './useComposerRunControls';
import { useComposerSubmission } from './useComposerSubmission';

const Composer: Component<{ variant?: 'docked' | 'start-session' | 'start-task' }> = (props) => {
  const variant = () => props.variant ?? 'docked';
  const isStartVariant = () => variant() === 'start-session' || variant() === 'start-task';
  const inputValue = composerInputValue;
  const setInputValue = setComposerInputValue;
  const {
    attachments,
    addClipboardFiles,
    clearAttachments,
    inputAttachments,
    removeAttachment,
  } = useComposerAttachments();
  const [isRespondingApproval, setIsRespondingApproval] = createSignal(false);
  const [goalStripNow, setGoalStripNow] = createSignal(Date.now());
  const {
    focusInput,
    handlePromptHistoryKey,
    resetHistoryNavigation,
    resizeInput,
    setTextareaRef,
  } = useComposerPromptInput({
    inputValue,
    setInputValue,
    promptHistory: composerPromptHistory,
    focusToken: composerInputFocusToken,
  });
  const planModeEnabled = () => composerRunState.planModeEnabled;
  const planExecutionStarted = () => composerRunState.planExecutionStarted;
  const multiAgentEnabled = () => composerRunState.multiAgentEnabled;
  const goalTrackingEnabled = () => composerRunState.goalTrackingEnabled;
  const goalPaused = () => composerRunState.goalPaused;
  const activeGoalText = () => composerRunState.activeGoalText;
  const pendingPlanReview = () => composerRunState.pendingPlanReview;
  const activeGoalStartedAt = () => composerRunState.activeGoalStartedAt;
  const runningTask = () => composerRunState.runningTask;
  const queuedTasks = () => composerRunState.queuedTasks;
  const currentPermissionPolicy = resolvedComposerPermissionPolicy;
  const pendingToolApproval = () => {
    const approval = chatMessageState.pendingApproval;
    if (!approval || approval.sessionId !== activeSessionId()) return null;
    return approval;
  };
  const currentModelSelection = resolvedComposerModelSelection;
  const currentReasoningEffort = resolvedComposerReasoningEffort;
  const planPhases = () =>
    planModeEnabled() && planExecutionStarted() && sessionTodosState.sessionId === activeSessionId()
      ? sessionTodosState.todos
      : [];
  const currentWorktreeRoot = () => activeSession()?.worktreeRoot?.trim() || null;
  const recentWorktrees = () => projectState.recentWorktrees.slice(0, 10);
  const currentPermissionLabel = () => permissionLabel(currentPermissionPolicy());
  // Slash 命令下拉状态
  const [showSlashDropdown, setShowSlashDropdown] = createSignal(false);
  const [slashQuery, setSlashQuery] = createSignal('');
  const slashCommands = () => {
    const items = commandPaletteState.filteredCommands;
    return items.filter((cmd) => cmd.source === 'skill' || cmd.source === 'command');
  };
  const permissionMenuItems = (): MenuActionItem[] => buildPermissionMenuItems();
  const providerMenuItems = () =>
    gatewayState.providers.map((provider) => ({
      id: provider.id,
      name: provider.name,
    }));
  const currentProviderId = () => {
    const selectedProviderId = currentModelSelection()?.providerId?.trim();
    if (selectedProviderId && gatewayState.providers.some((provider) => provider.id === selectedProviderId)) {
      return selectedProviderId;
    }
    const defaultProviderId = gatewayState.config?.defaultProvider?.trim();
    if (defaultProviderId && gatewayState.providers.some((provider) => provider.id === defaultProviderId)) {
      return defaultProviderId;
    }
    return gatewayState.providers[0]?.id ?? '';
  };
  const currentProviderLabel = () => currentProviderId() || 'Provider';
  const currentProviderModels = () => gatewayState.models.filter((model) => model.providerId === currentProviderId());
  const currentModelEffortLabel = () =>
    `${modelNameLabel(currentProviderModels(), currentModelSelection())} · ${reasoningEffortLabel(currentReasoningEffort())}`;
  const { ensureComposerSession } = useComposerSession();

  function escapeStructuredReferenceValue(value: string): string {
    return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  }

  function structuredReferenceToken(kind: 'file' | 'folder', path: string): string {
    return `@${kind} "${escapeStructuredReferenceValue(path)}"`;
  }

  function insertPathReferences(kind: 'file' | 'folder', paths: string[]): void {
    if (paths.length === 0) return;
    const nextReferences = paths
      .map((path) => structuredReferenceToken(kind, path))
      .join('\n');
    setInputValue((current) => {
      const trimmed = current.trimEnd();
      if (!trimmed) return nextReferences;
      return `${trimmed}\n${nextReferences}`;
    });
  }

  function handleComposerPaste(event: ClipboardEvent): void {
    const files = Array.from(event.clipboardData?.files ?? []);
    if (files.length === 0) return;
    event.preventDefault();
    void addClipboardFiles(files).catch(() => undefined);
  }

  function handleComposerDrop(event: DragEvent): void {
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length === 0) return;
    event.preventDefault();
    void addClipboardFiles(files).catch(() => undefined);
  }

  function handleComposerInput(newValue: string, textarea: HTMLTextAreaElement): void {
    setInputValue(newValue);
    resetHistoryNavigation();
    resizeInput(textarea);
    const slashMatch = newValue.match(/(?:^|\s)(\/\S*)$/);
    if (slashMatch) {
      setSlashQuery(slashMatch[1]);
      setShowSlashDropdown(true);
    } else {
      setShowSlashDropdown(false);
    }
  }

  function handleSlashCommandSelect(command: Command): void {
    const text = inputValue();
    const match = text.match(/(?:^|\s)(\/\S*)$/);
    if (match) {
      const before = text.slice(0, text.length - match[0].length);
      const suffix = match[0].startsWith(' ') ? ' ' : '';
      setInputValue(before + suffix + command.label + ' ');
    } else {
      insertComposerSlashTrigger(command.label);
    }
    setShowSlashDropdown(false);
    focusInput();
  }

  function stopCurrentResponse(): void {
    stopChatMessage();
    const sessionId = activeSessionId();
    if (sessionId) void clearRunningComposerRuntimeTask(sessionId).catch(() => undefined);
  }

  function composerPlaceholder(): string {
    if (variant() === 'start-task') return 'How can I help you today?';
    if (variant() === 'start-session') return 'Describe a task or ask a question';
    if (goalTrackingEnabled() && !activeGoalText()) return 'What goal should Navis Go pursue?';
    if (activeGoalText() && goalPaused()) return 'Goal paused';
    if (activeGoalText()) return 'Add guidance for this goal';
    return 'Request the next change';
  }

  function approvalDialogConfig(): AgentConfirmConfig | null {
    const approval = pendingToolApproval();
    if (!approval) return null;
    return {
      id: approval.requestId,
      toolName: approval.title || approval.gatewayTool || approval.tool,
      toolArgs: approval.args,
      riskLevel: ['low', 'medium', 'high'].includes(approval.riskLevel)
        ? (approval.riskLevel as AgentConfirmConfig['riskLevel'])
        : 'medium',
      message: approval.summary ? `${approval.message} ${approval.summary}` : approval.message,
      onApprove: () => undefined,
      onDenyAlways: () => undefined,
      onTrustThisSession: () => undefined,
      onAllowProject: () => undefined,
    };
  }

  async function handleToolApproval(decision: ToolApprovalDecision): Promise<void> {
    const approval = pendingToolApproval();
    if (!approval || isRespondingApproval()) return;
    setIsRespondingApproval(true);
    try {
      await respondToolApproval(approval.requestId, decision);
    } catch {
      // The approval request may have expired; the timeline refresh is the visible source of truth.
    } finally {
      setIsRespondingApproval(false);
    }
  }

  onMount(() => {
    void loadRecentWorktrees();
    if (!gatewayState.loaded) void loadGatewayCatalog();
    const timer = window.setInterval(() => setGoalStripNow(Date.now()), 1000);
    onCleanup(() => window.clearInterval(timer));
  });

  onCleanup(() => {
    clearAttachments();
  });

  createEffect(() => {
    const root = currentWorktreeRoot();
    if (root) void rememberRecentWorktree(root);
  });

  createEffect(() => {
    if (planModeEnabled()) {
      const release = subscribeSessionTodosPolling(activeSessionId());
      onCleanup(release);
    }
  });

  const instructionFlags = () => ({
    planModeEnabled: planModeEnabled(),
    multiAgentEnabled: multiAgentEnabled(),
  });

  const submission = useComposerSubmission({
    inputValue,
    setInputValue,
    inputAttachments,
    clearAttachments,
    resetHistoryNavigation,
    ensureSession: ensureComposerSession,
    planModeEnabled,
    pendingPlanReview,
    goalTrackingEnabled,
    activeGoalText,
    instructionFlags,
  });
  const runControls = useComposerRunControls({
    activeSessionId,
    activeGoalText,
    goalPaused,
    runningTask,
    queuedTasks,
    instructionFlags,
    setInputValue,
    runComposerTask: submission.runComposerTask,
    submitComposerTask: submission.submitComposerTask,
  });

  async function selectWorktreeRootForComposer(): Promise<void> {
    closeMenu();
    await chooseComposerWorktree({
      ensureSessionId: ensureComposerSession,
    });
  }

  async function handleWorktreeMenuSelect(worktreeRoot: string | null): Promise<void> {
    closeMenu();
    await bindComposerWorktree(worktreeRoot, {
      ensureSessionId: ensureComposerSession,
    });
  }

  async function handleInputPlusSelect(item: MenuActionItem): Promise<void> {
    await executeComposerInputPlusItem(item, {
      onInsertReferences: insertPathReferences,
      onTogglePlanMode: () => {
        const shouldOpenPlan = !planModeEnabled();
        togglePlanMode();
        if (shouldOpenPlan) openPlanPanel();
      },
      onToggleMultiAgent: toggleMultiAgent,
      onToggleGoalTracking: toggleGoalTracking,
    });
    closeMenu();
  }

  async function handlePermissionMenuSelect(item: MenuActionItem): Promise<void> {
    closeMenu();
    await executeComposerPermissionMenuItem(item, activeSessionId());
  }

  async function handleModelTriggerClick(): Promise<void> {
    switch (await resolveComposerModelMenuTrigger(activeSessionId(), isMenuOpen('composer-model'))) {
      case 'close':
        closeMenu();
        return;
      case 'gateway-settings':
        closeMenu();
        await openSettingsDialog('gateway', 'Gateway has no connection or model configured. Add a model first.');
        return;
      case 'open':
        toggleMenu('composer-model');
        return;
      case 'noop':
      default:
        return;
    }
  }

  function defaultModelForProvider(providerId: string): GatewayModelSelection | null {
    const provider = gatewayState.config?.providers.find((item) => item.id === providerId);
    const configuredDefault = provider?.defaultModel?.trim();
    const providerModels = gatewayState.models.filter((item) => item.providerId === providerId);
    const defaultModel = configuredDefault
      ? providerModels.find((model) => model.id === configuredDefault)
      : undefined;
    const model = defaultModel ?? providerModels[0];
    return model ? { providerId, modelId: model.id } : null;
  }

  async function handleProviderSelect(providerId: string): Promise<void> {
    closeMenu();
    const selection = defaultModelForProvider(providerId);
    if (!selection) return;
    await executeComposerModelSelection(activeSessionId(), currentModelSelection(), selection);
  }

  async function handleModelSelect(selection: GatewayModelSelection): Promise<void> {
    closeMenu();
    await executeComposerModelSelection(activeSessionId(), currentModelSelection(), selection);
  }

  async function handleReasoningEffortSelect(reasoningEffort: ReasoningEffort): Promise<void> {
    closeMenu();
    await executeComposerReasoningEffortSelection(activeSessionId(), currentReasoningEffort(), reasoningEffort);
  }

  const worktreeSelector = () => (
    <ComposerWorktreeSelector
      currentWorktreeRoot={currentWorktreeRoot}
      recentWorktrees={recentWorktrees}
      onSelect={(worktreeRoot) => void handleWorktreeMenuSelect(worktreeRoot)}
      onChooseNew={() => void selectWorktreeRootForComposer()}
    />
  );

  return (
    <footer class={`navis-composer is-${variant()} ${isStartVariant() ? 'is-start' : 'is-docked'} flex-shrink-0`}>
      <div class="navis-composer-inner w-full">
        {/* A 区：对话输入框 */}
        <ComposerRunStack
          planPhases={planPhases}
          queuedTasks={queuedTasks}
          activeGoalText={activeGoalText}
          activeGoalStartedAt={activeGoalStartedAt}
          runningTask={runningTask}
          guidedQueuedTaskId={runControls.guidedQueuedTaskId}
          goalPaused={goalPaused}
          goalExpanded={runControls.goalExpanded}
          now={goalStripNow}
          onGuideQueuedTask={runControls.guideQueuedTask}
          onRemoveQueuedTask={runControls.removeQueuedTask}
          onEditQueuedTask={runControls.editQueuedTask}
          onEditGoal={runControls.editActiveGoal}
          onToggleGoalPaused={runControls.toggleGoalPaused}
          onToggleGoalExpanded={runControls.toggleGoalExpanded}
          onClearGoal={runControls.clearActiveGoal}
        />
        <Show when={variant() === 'start-session'}>
          <section class="navis-composer-worktree-row" aria-label="Session worktree">
            {worktreeSelector()}
          </section>
        </Show>
        <section
          class={`navis-composer-input-shell rounded-md ${pendingToolApproval() ? 'is-approval' : ''}`}
          onDrop={handleComposerDrop}
          onDragOver={(event) => event.preventDefault()}
        >
          <Show
            when={approvalDialogConfig() ?? pendingPlanReview()}
            fallback={
              <ComposerInputShell
                attachments={attachments}
                inputValue={inputValue}
                placeholder={composerPlaceholder}
                showSlashDropdown={showSlashDropdown}
                slashQuery={slashQuery}
                slashCommands={slashCommands}
                sending={() => chatMessageState.sending}
                loading={() => chatMessageState.loading}
                setTextareaRef={setTextareaRef}
                onInput={handleComposerInput}
                onPaste={handleComposerPaste}
                onPromptHistoryKey={handlePromptHistoryKey}
                onSubmit={() => void submission.enqueueComposerTask()}
                onStop={stopCurrentResponse}
                onRemoveAttachment={removeAttachment}
                onSlashSelect={handleSlashCommandSelect}
                onSlashDismiss={() => setShowSlashDropdown(false)}
              />
            }
          >
            <ComposerApprovalPanel
              approvalConfig={approvalDialogConfig()}
              pendingPlanReview={pendingPlanReview()}
              planReviewInput={submission.planReviewInput()}
              isRespondingApproval={isRespondingApproval()}
              isStartingPlanExecution={submission.isStartingPlanExecution()}
              multiAgentEnabled={multiAgentEnabled()}
              onToolApproval={(decision) => void handleToolApproval(decision)}
              onCancelPlanReview={submission.cancelPlanReview}
              onStartPlanExecution={() => void submission.startPlanExecution()}
              onPlanReviewInput={submission.setPlanReviewInput}
            />
          </Show>
        </section>

        <ComposerToolbar
          variant={variant}
          worktreeSelector={worktreeSelector}
          permissionMenuItems={permissionMenuItems}
          currentPermissionLabel={currentPermissionLabel}
          currentPermissionPolicy={currentPermissionPolicy}
          planModeEnabled={planModeEnabled}
          multiAgentEnabled={multiAgentEnabled}
          goalTrackingEnabled={goalTrackingEnabled}
          goalRunning={() => Boolean(runningTask()) && !goalPaused()}
          currentProviderLabel={currentProviderLabel}
          providerMenuItems={providerMenuItems}
          currentProviderId={currentProviderId}
          currentModelEffortLabel={currentModelEffortLabel}
          currentProviderModels={currentProviderModels}
          currentModelSelection={currentModelSelection}
          currentReasoningEffort={currentReasoningEffort}
          onPermissionSelect={(item) => void handlePermissionMenuSelect(item)}
          onInputPlusSelect={(item) => void handleInputPlusSelect(item)}
          onDisablePlanMode={disablePlanMode}
          onToggleMultiAgent={toggleMultiAgent}
          onClearGoal={runControls.clearActiveGoal}
          onProviderSelect={(providerId) => void handleProviderSelect(providerId)}
          onModelTriggerClick={() => void handleModelTriggerClick()}
          onModelSelect={(selection) => void handleModelSelect(selection)}
          onReasoningEffortSelect={(effort) => void handleReasoningEffortSelect(effort)}
        />

      </div>
    </footer>
  );
};

export default Composer;
