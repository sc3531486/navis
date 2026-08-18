import { createStore } from 'solid-js/store';
import { invoke } from '@tauri-apps/api/core';
import type { ComposerInputAttachment } from '@agent-core/components/Composer/useComposerAttachments';

export type ComposerTaskKind = 'prompt' | 'planDraft' | 'planExecution' | 'goal';

export interface ComposerTask {
  id: string;
  kind: ComposerTaskKind;
  text: string;
  sourceText?: string;
  displayText?: string;
  attachments?: ComposerInputAttachment[];
  createdAt: string;
}

export interface CreateComposerTaskOptions {
  kind?: ComposerTaskKind;
  createdAt?: string;
  sourceText?: string;
  displayText?: string;
  attachments?: ComposerInputAttachment[];
}

export interface PendingPlanReview {
  id: string;
  requestText: string;
  planContent?: string | null;
  createdAt: string;
}

interface ComposerRunState {
  sessionId: string | null;
  loaded: boolean;
  planModeEnabled: boolean;
  planExecutionStarted: boolean;
  multiAgentEnabled: boolean;
  pendingPlanReview: PendingPlanReview | null;
  goalTrackingEnabled: boolean;
  goalPaused: boolean;
  activeGoalText: string | null;
  activeGoalStartedAt: string | null;
  runningTask: ComposerTask | null;
  queuedTasks: ComposerTask[];
}

const defaultComposerRunState: ComposerRunState = {
  sessionId: null,
  loaded: false,
  planModeEnabled: false,
  planExecutionStarted: false,
  multiAgentEnabled: false,
  pendingPlanReview: null,
  goalTrackingEnabled: false,
  goalPaused: false,
  activeGoalText: null,
  activeGoalStartedAt: null,
  runningTask: null,
  queuedTasks: [],
};

const [composerRunStateStore, setComposerRunState] = createStore<ComposerRunState>({
  ...defaultComposerRunState,
});

export const composerRunState = composerRunStateStore;

interface UiComposerRunState {
  sessionId: string;
  planModeEnabled: boolean;
  planExecutionStarted?: boolean;
  multiAgentEnabled?: boolean;
  pendingPlanReview?: PendingPlanReview | null;
  goalTrackingEnabled: boolean;
  goalPaused: boolean;
  activeGoalText: string | null;
  activeGoalStartedAt: string | null;
  runningTask?: ComposerTask | null;
  queuedTasks: ComposerTask[];
}

interface ComposerTaskSubmitResult {
  state: UiComposerRunState;
  disposition: 'runNow' | 'queued';
  task: ComposerTask;
}

interface ComposerTaskFinishResult {
  state: UiComposerRunState;
  nextTask?: ComposerTask | null;
}

interface ComposerTaskClearResult {
  state: UiComposerRunState;
}

let loadingSession = false;
let saveQueued = false;
let loadToken = 0;

function applyComposerRunState(state: UiComposerRunState): void {
  const sessionChanged = composerRunState.sessionId !== state.sessionId;
  setComposerRunState({
    sessionId: state.sessionId,
    loaded: true,
    planModeEnabled: state.planModeEnabled,
    planExecutionStarted: Boolean(state.planExecutionStarted),
    multiAgentEnabled: Boolean(state.multiAgentEnabled),
    pendingPlanReview: state.pendingPlanReview ?? null,
    goalTrackingEnabled: state.goalTrackingEnabled,
    goalPaused: state.goalPaused,
    activeGoalText: state.activeGoalText,
    activeGoalStartedAt: state.activeGoalStartedAt,
    runningTask: state.runningTask ?? (sessionChanged ? null : composerRunState.runningTask),
    queuedTasks: state.queuedTasks,
  });
}

function resetComposerRunState(): void {
  setComposerRunState({ ...defaultComposerRunState });
}

function currentPayload(): UiComposerRunState | null {
  if (!composerRunState.sessionId) return null;

  return {
    sessionId: composerRunState.sessionId,
    planModeEnabled: composerRunState.planModeEnabled,
    planExecutionStarted: composerRunState.planExecutionStarted,
    multiAgentEnabled: composerRunState.multiAgentEnabled,
    pendingPlanReview: composerRunState.pendingPlanReview,
    goalTrackingEnabled: composerRunState.goalTrackingEnabled,
    goalPaused: composerRunState.goalPaused,
    activeGoalText: composerRunState.activeGoalText,
    activeGoalStartedAt: composerRunState.activeGoalStartedAt,
    queuedTasks: [],
  };
}

export function createComposerTask(
  id: string,
  text: string,
  options: CreateComposerTaskOptions = {},
): ComposerTask {
  return {
    id,
    kind: options.kind ?? 'prompt',
    text,
    sourceText: options.sourceText,
    displayText: options.displayText,
    attachments: options.attachments,
    createdAt: options.createdAt ?? new Date().toISOString(),
  };
}

export function elapsedTimeLabel(startedAt: string | null | undefined, nowMs = Date.now()): string {
  if (!startedAt) return '0s';
  const startedAtMs = Date.parse(startedAt);
  if (Number.isNaN(startedAtMs)) return '0s';

  const elapsedSeconds = Math.max(0, Math.floor((nowMs - startedAtMs) / 1000));
  const hours = Math.floor(elapsedSeconds / 3600);
  const minutes = Math.floor((elapsedSeconds % 3600) / 60);
  const seconds = elapsedSeconds % 60;

  if (hours > 0) return `${hours}h ${minutes}m ${seconds}s`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

async function persistComposerRunState(): Promise<void> {
  const payload = currentPayload();
  if (!payload) return;

  const saved = await invoke<UiComposerRunState>('ui_set_session_composer_run_state', {
    payload,
  });

  if (saved.sessionId === composerRunState.sessionId) {
    applyComposerRunState(saved);
  }
}

function scheduleComposerRunPersist(): void {
  if (loadingSession || !composerRunState.sessionId || saveQueued) return;
  saveQueued = true;
  queueMicrotask(() => {
    saveQueued = false;
    void persistComposerRunState();
  });
}

export async function loadComposerRunState(sessionId: string | null): Promise<void> {
  const token = ++loadToken;
  loadingSession = true;
  try {
    if (!sessionId) {
      if (token === loadToken) resetComposerRunState();
      return;
    }

    const state = await invoke<UiComposerRunState>('ui_get_session_composer_run_state', {
      payload: { sessionId },
    });
    if (token === loadToken) applyComposerRunState(state);
  } finally {
    if (token === loadToken) loadingSession = false;
  }
}

export function togglePlanMode(): void {
  setComposerRunState('planModeEnabled', (enabled) => {
    const next = !enabled;
    if (next) {
      setComposerRunState('planExecutionStarted', false);
    } else {
      setComposerRunState({
        planExecutionStarted: false,
        pendingPlanReview: null,
      });
    }
    return next;
  });
  scheduleComposerRunPersist();
}

export function setPlanExecutionStarted(started: boolean): void {
  setComposerRunState('planExecutionStarted', started);
  scheduleComposerRunPersist();
}

export function setPendingPlanReview(review: PendingPlanReview | null): void {
  setComposerRunState({
    pendingPlanReview: review,
    planExecutionStarted: review ? false : composerRunState.planExecutionStarted,
  });
  scheduleComposerRunPersist();
}

export function createPendingPlanReview(
  requestText: string,
  planContent?: string | null,
  createdAt = new Date().toISOString(),
): PendingPlanReview {
  return {
    id: `plan-${Date.now()}`,
    requestText,
    planContent: planContent?.trim() || null,
    createdAt,
  };
}

export function toggleMultiAgent(): void {
  setComposerRunState('multiAgentEnabled', (enabled) => !enabled);
  scheduleComposerRunPersist();
}

export function disablePlanMode(): void {
  setComposerRunState({
    planModeEnabled: false,
    planExecutionStarted: false,
    pendingPlanReview: null,
  });
  scheduleComposerRunPersist();
}

export function toggleGoalTracking(): void {
  setComposerRunState('goalTrackingEnabled', (enabled) => {
    const next = !enabled;
    if (!next) {
      setComposerRunState({
        activeGoalText: null,
        activeGoalStartedAt: null,
        goalPaused: false,
      });
    }
    return next;
  });
  scheduleComposerRunPersist();
}

export function clearActiveGoalState(): void {
  setComposerRunState({
    activeGoalText: null,
    activeGoalStartedAt: null,
    goalTrackingEnabled: false,
    goalPaused: false,
  });
  scheduleComposerRunPersist();
}

export async function startGoalRunner(sessionId: string, goal: string, prompt?: string): Promise<void> {
  const saved = await invoke<UiComposerRunState>('ui_start_goal_runner', {
    payload: { sessionId, goal, prompt },
  });
  applyComposerRunState(saved);
}

export async function pauseGoalRunner(sessionId: string): Promise<void> {
  const saved = await invoke<UiComposerRunState>('ui_pause_goal_runner', {
    payload: { sessionId },
  });
  applyComposerRunState(saved);
}

export async function resumeGoalRunner(sessionId: string): Promise<void> {
  const saved = await invoke<UiComposerRunState>('ui_resume_goal_runner', {
    payload: { sessionId },
  });
  applyComposerRunState(saved);
}

export async function stopGoalRunner(sessionId: string): Promise<void> {
  const saved = await invoke<UiComposerRunState>('ui_stop_goal_runner', {
    payload: { sessionId },
  });
  applyComposerRunState(saved);
}

export async function submitComposerRuntimeTask(
  sessionId: string,
  task: ComposerTask,
): Promise<ComposerTaskSubmitResult> {
  const result = await invoke<ComposerTaskSubmitResult>('ui_submit_composer_task', {
    payload: { sessionId, task },
  });
  applyComposerRunState(result.state);
  return result;
}

export async function finishComposerRuntimeTask(
  sessionId: string,
  taskId: string,
): Promise<ComposerTaskFinishResult> {
  const result = await invoke<ComposerTaskFinishResult>('ui_finish_composer_task', {
    payload: { sessionId, taskId },
  });
  applyComposerRunState(result.state);
  return result;
}

export async function clearRunningComposerRuntimeTask(sessionId: string): Promise<void> {
  const result = await invoke<ComposerTaskClearResult>('ui_clear_running_composer_task', {
    payload: { sessionId },
  });
  applyComposerRunState(result.state);
}

export async function removeQueuedComposerTask(sessionId: string, taskId: string): Promise<void> {
  const state = await invoke<UiComposerRunState>('ui_remove_queued_composer_task', {
    payload: { sessionId, taskId },
  });
  applyComposerRunState(state);
}

export async function promoteQueuedComposerTask(
  sessionId: string,
  taskId: string,
): Promise<ComposerTaskSubmitResult> {
  const result = await invoke<ComposerTaskSubmitResult>('ui_promote_queued_composer_task', {
    payload: { sessionId, taskId },
  });
  applyComposerRunState(result.state);
  return result;
}
