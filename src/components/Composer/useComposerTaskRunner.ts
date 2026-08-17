import {
  chatMessageState,
  sendChatMessage,
} from '../../stores/chat-messages';
import {
  clearRunningComposerRuntimeTask,
  finishComposerRuntimeTask,
  submitComposerRuntimeTask,
  type ComposerTask,
} from '../../stores/composer-run';
import type { ChatAgentTimelinePart } from '../../lib/stream';

interface ComposerTaskRunnerOptions {
  ensureSession: () => Promise<string | null>;
  onPlanDraftComplete: (task: ComposerTask, sessionId: string) => Promise<void>;
}

export function useComposerTaskRunner(options: ComposerTaskRunnerOptions): {
  runComposerTask: (task: ComposerTask) => Promise<void>;
  submitComposerTask: (task: ComposerTask) => Promise<void>;
} {
  async function runComposerTask(task: ComposerTask): Promise<void> {
    const sessionId = await options.ensureSession();
    if (!sessionId) {
      return;
    }

    const finishTask = async () => {
      const finishResult = await finishComposerRuntimeTask(sessionId, task.id);
      if (finishResult.nextTask) {
        await runComposerTask(finishResult.nextTask);
      }
    };

    try {
      await sendChatMessage(sessionId, task.text, {
        displayContent: task.displayText ?? task.text,
        attachments: task.attachments,
        onTermination: (termination) => {
          if (termination.kind === 'completed') {
            void options.onPlanDraftComplete(task, sessionId).finally(finishTask);
            return;
          }
          void clearRunningComposerRuntimeTask(sessionId).catch(() => undefined);
        },
      });
    } catch (error) {
      void error;
      await clearRunningComposerRuntimeTask(sessionId).catch(() => undefined);
    }
  }

  async function submitComposerTask(task: ComposerTask): Promise<void> {
    const sessionId = await options.ensureSession();
    if (!sessionId) {
      return;
    }
    const result = await submitComposerRuntimeTask(sessionId, task);
    if (result.disposition === 'queued') {
      return;
    }
    await runComposerTask(result.task);
  }

  return {
    runComposerTask,
    submitComposerTask,
  };
}

export function latestAssistantPlanContent(): string | undefined {
  return latestExitPlanModeContent() ?? latestAssistantTextContent();
}

function latestAssistantTextContent(): string | undefined {
  return chatMessageState.messages
    .slice()
    .reverse()
    .find((message) => message.role === 'assistant' && message.content.trim().length > 0)
    ?.content
    .trim();
}

function latestExitPlanModeContent(): string | undefined {
  for (const message of chatMessageState.messages.slice().reverse()) {
    if (message.role !== 'assistant') continue;
    for (const part of message.agentTimelineParts.slice().reverse()) {
      const plan = exitPlanModeOutputPlan(part);
      if (plan) return plan;
    }
  }
  return undefined;
}

function exitPlanModeOutputPlan(part: ChatAgentTimelinePart): string | undefined {
  const toolName = (part.gatewayTool || part.tool || '').trim();
  if (toolName !== 'exit_plan_mode' && toolName !== 'navis.exit_plan_mode') return undefined;
  const output = part.output;
  if (!output || typeof output !== 'object') return undefined;
  const plan = output.plan;
  return typeof plan === 'string' && plan.trim() ? plan.trim() : undefined;
}
