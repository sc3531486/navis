import { createSignal } from 'solid-js';
import { openPlanPanel } from '../Chat/panel-actions';
import { setPendingStartKind } from '../../stores/app';
import {
  createComposerTask,
  createPendingPlanReview,
  setPendingPlanReview,
  setPlanExecutionStarted,
  startGoalRunner,
  type ComposerTask,
  type PendingPlanReview,
} from '../../stores/composer-run';
import { rememberComposerPrompt } from '../../stores/composer-input';
import { loadSessionTree } from '../../stores/session-tree';
import { refreshSessionTodos, sessionTodosState } from '../../stores/session-todos';
import type { ComposerInputAttachment } from './useComposerAttachments';
import {
  goalTaskMessage,
  goalTaskMessageForGoal,
  modeInstructionMessage,
  planExecutionInstruction,
  type ComposerInstructionFlags,
} from './composer-instructions';
import { latestAssistantPlanContent, useComposerTaskRunner } from './useComposerTaskRunner';

interface ComposerSubmissionOptions {
  inputValue: () => string;
  setInputValue: (value: string | ((current: string) => string)) => void;
  inputAttachments: () => ComposerInputAttachment[];
  clearAttachments: () => void;
  resetHistoryNavigation: () => void;
  ensureSession: () => Promise<string | null>;
  planModeEnabled: () => boolean;
  pendingPlanReview: () => PendingPlanReview | null;
  goalTrackingEnabled: () => boolean;
  activeGoalText: () => string | null;
  instructionFlags: () => ComposerInstructionFlags;
}

export function useComposerSubmission(options: ComposerSubmissionOptions) {
  const [planReviewInput, setPlanReviewInput] = createSignal('');
  const [isStartingPlanExecution, setIsStartingPlanExecution] = createSignal(false);

  async function finishCompletedComposerTask(task: ComposerTask, sessionId: string): Promise<void> {
    await loadSessionTree();
    if (task.kind !== 'planDraft') return;

    await refreshSessionTodos(sessionId);
    const planContent = latestAssistantPlanContent();

    if (sessionTodosState.todos.length > 0) {
      setPendingPlanReview(createPendingPlanReview(task.sourceText ?? task.displayText ?? task.text, planContent));
      setPlanExecutionStarted(false);
      openPlanPanel();
    }
  }

  const { runComposerTask, submitComposerTask } = useComposerTaskRunner({
    ensureSession: options.ensureSession,
    onPlanDraftComplete: finishCompletedComposerTask,
  });

  async function startPlanExecution(): Promise<void> {
    const review = options.pendingPlanReview();
    if (!review || isStartingPlanExecution()) return;

    setIsStartingPlanExecution(true);
    try {
      const customText = planReviewInput();
      setPendingPlanReview(null);
      setPlanReviewInput('');
      setPlanExecutionStarted(true);
      await submitComposerTask(createComposerTask(
        `plan-execution-${Date.now()}`,
        planExecutionInstruction(review.requestText, customText),
        {
          kind: 'planExecution',
          sourceText: review.requestText,
          displayText: customText.trim() ? `Start plan execution\n\n${customText.trim()}` : 'Start plan execution',
        },
      ));
    } catch {
      setPendingPlanReview(review);
      setPlanExecutionStarted(false);
    } finally {
      setIsStartingPlanExecution(false);
    }
  }

  function cancelPlanReview(): void {
    setPendingPlanReview(null);
    setPlanReviewInput('');
    setPlanExecutionStarted(false);
  }

  async function enqueueComposerTask(): Promise<void> {
    const text = options.inputValue().trim();
    const messageText = text;
    const messageAttachments = options.inputAttachments();
    if (!messageText && messageAttachments.length === 0) return;
    rememberComposerPrompt(messageText || messageAttachments.map((attachment) => attachment.name).join(', '));
    options.resetHistoryNavigation();

    if (options.pendingPlanReview()) {
      setPlanReviewInput((current) => {
        const trimmed = current.trim();
        return trimmed ? `${trimmed}\n${messageText}` : messageText;
      });
      options.setInputValue('');
      options.clearAttachments();
      return;
    }

    if (options.planModeEnabled() && !options.pendingPlanReview()) {
      options.setInputValue('');
      options.clearAttachments();
      try {
        if (!await options.ensureSession()) {
          options.setInputValue(text);
          return;
        }
        setPendingStartKind(null);
        await submitComposerTask(createComposerTask(`plan-${Date.now()}`, modeInstructionMessage(messageText, options.instructionFlags()), {
          kind: 'planDraft',
          sourceText: messageText,
          displayText: messageText,
          attachments: messageAttachments,
        }));
      } catch {
        options.setInputValue(text);
      }
      return;
    }

    if (!options.goalTrackingEnabled() && !options.planModeEnabled()) {
      options.setInputValue('');
      options.clearAttachments();
      try {
        await submitComposerTask(createComposerTask(`prompt-${Date.now()}`, modeInstructionMessage(messageText, options.instructionFlags()), {
          kind: 'prompt',
          sourceText: messageText,
          displayText: messageText,
          attachments: messageAttachments,
        }));
      } catch {
        options.setInputValue(text);
      }
      return;
    }

    if (options.goalTrackingEnabled() && !options.activeGoalText()) {
      options.setInputValue('');
      options.clearAttachments();
      try {
        const sessionId = await options.ensureSession();
        if (!sessionId) {
          options.setInputValue(text);
          return;
        }
        const goalPrompt = goalTaskMessageForGoal(messageText, messageText, options.instructionFlags());
        await startGoalRunner(sessionId, messageText, goalPrompt);
        await submitComposerTask(createComposerTask(`goal-${Date.now()}`, goalPrompt, {
          kind: 'goal',
          sourceText: messageText,
          displayText: messageText,
          attachments: messageAttachments,
        }));
      } catch {
        options.setInputValue(text);
        return;
      }
      return;
    }

    if (options.activeGoalText()) {
      try {
        await submitComposerTask(createComposerTask(`running-${Date.now()}`, goalTaskMessage(options.activeGoalText(), messageText, options.instructionFlags()), {
          kind: 'goal',
          sourceText: messageText,
          displayText: messageText,
          attachments: messageAttachments,
        }));
      } catch {
        options.setInputValue(text);
        return;
      }
      options.setInputValue('');
      options.clearAttachments();
      return;
    }

    if (options.planModeEnabled()) {
      options.setInputValue('');
      options.clearAttachments();
      try {
        await submitComposerTask(createComposerTask(`queue-${Date.now()}`, modeInstructionMessage(messageText, options.instructionFlags()), {
          kind: 'prompt',
          sourceText: messageText,
          displayText: messageText,
          attachments: messageAttachments,
        }));
      } catch {
        options.setInputValue(text);
      }
    }
  }

  return {
    cancelPlanReview,
    enqueueComposerTask,
    isStartingPlanExecution,
    planReviewInput,
    runComposerTask,
    setPlanReviewInput,
    startPlanExecution,
    submitComposerTask,
  };
}
