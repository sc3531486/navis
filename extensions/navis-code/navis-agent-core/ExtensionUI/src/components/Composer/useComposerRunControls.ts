import { createSignal, onCleanup } from 'solid-js';
import {
  clearActiveGoalState,
  clearRunningComposerRuntimeTask,
  createComposerTask,
  type ComposerTask,
  pauseGoalRunner,
  promoteQueuedComposerTask,
  removeQueuedComposerTask,
  resumeGoalRunner,
  stopGoalRunner,
} from '@agent-core/stores/composer-run';
import { stopChatMessage } from '@session/stores/chat-messages';
import { goalTaskMessage, type ComposerInstructionFlags } from './composer-instructions';

interface ComposerRunControlsOptions {
  activeSessionId: () => string | null;
  activeGoalText: () => string | null;
  goalPaused: () => boolean;
  runningTask: () => ComposerTask | null;
  queuedTasks: () => ComposerTask[];
  instructionFlags: () => ComposerInstructionFlags;
  setInputValue: (value: string) => void;
  runComposerTask: (task: ComposerTask) => Promise<void>;
  submitComposerTask: (task: ComposerTask) => Promise<void>;
}

export function useComposerRunControls(options: ComposerRunControlsOptions) {
  const [goalExpanded, setGoalExpanded] = createSignal(false);
  const [guidedQueuedTaskId, setGuidedQueuedTaskId] = createSignal<string | null>(null);
  let guidedQueuedTaskTimer: number | undefined;

  onCleanup(() => {
    if (guidedQueuedTaskTimer) window.clearTimeout(guidedQueuedTaskTimer);
  });

  function removeQueuedTask(taskId: string): void {
    const sessionId = options.activeSessionId();
    if (!sessionId) return;
    void removeQueuedComposerTask(sessionId, taskId).catch(() => undefined);
  }

  function guideQueuedTask(taskId: string): void {
    const sessionId = options.activeSessionId();
    if (!sessionId) return;

    setGuidedQueuedTaskId(taskId);
    if (guidedQueuedTaskTimer) window.clearTimeout(guidedQueuedTaskTimer);
    guidedQueuedTaskTimer = window.setTimeout(() => setGuidedQueuedTaskId(null), 1000);

    void promoteQueuedComposerTask(sessionId, taskId)
      .then((result) => {
        if (result.disposition === 'runNow') {
          void options.runComposerTask(result.task);
        }
      })
      .catch(() => undefined);
  }

  function editQueuedTask(taskId: string): void {
    const task = options.queuedTasks().find((item) => item.id === taskId);
    if (!task) return;
    options.setInputValue(task.text);
    removeQueuedTask(taskId);
  }

  function clearActiveGoal(): void {
    const sessionId = options.activeSessionId();
    if (sessionId) {
      void stopGoalRunner(sessionId).catch(() => undefined);
    }
    clearActiveGoalState();
    setGoalExpanded(false);
  }

  function editActiveGoal(): void {
    const goal = options.activeGoalText();
    if (!goal) return;
    options.setInputValue(goal);
    const sessionId = options.activeSessionId();
    if (sessionId) {
      void stopGoalRunner(sessionId).catch(() => undefined);
    } else {
      clearActiveGoalState();
    }
    setGoalExpanded(false);
  }

  function toggleGoalExpanded(): void {
    setGoalExpanded((expanded) => !expanded);
  }

  function toggleGoalPaused(): void {
    const goal = options.activeGoalText();
    if (!goal) return;
    const sessionId = options.activeSessionId();
    if (!sessionId) return;

    if (options.goalPaused()) {
      void resumeGoalRunner(sessionId).catch(() => undefined);
      void options.submitComposerTask(createComposerTask(
        `goal-resume-${Date.now()}`,
        goalTaskMessage(options.activeGoalText(), goal, options.instructionFlags()),
        {
          kind: 'goal',
          sourceText: goal,
          displayText: goal,
        },
      ));
      return;
    }

    void pauseGoalRunner(sessionId).catch(() => undefined);
    if (options.runningTask()) {
      const taskId = options.runningTask()?.id;
      stopChatMessage();
      if (taskId) void clearRunningComposerRuntimeTask(sessionId).catch(() => undefined);
    }
  }

  return {
    guidedQueuedTaskId,
    goalExpanded,
    clearActiveGoal,
    editActiveGoal,
    editQueuedTask,
    guideQueuedTask,
    removeQueuedTask,
    toggleGoalExpanded,
    toggleGoalPaused,
  };
}
