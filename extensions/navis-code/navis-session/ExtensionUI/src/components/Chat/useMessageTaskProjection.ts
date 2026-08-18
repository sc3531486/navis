import { Accessor, createEffect, createMemo, onCleanup } from 'solid-js';

import type { ChatMessage } from '@session/stores/chat-messages';
import {
  subscribeTaskProjectionPolling,
  taskProjectionState,
  type TaskProjection,
} from '@task-ext/stores/task-projection';
import { openBackgroundTasksPanel } from './panel-actions';

const recordString = (record: Record<string, unknown> | null | undefined, key: string): string | null => {
  const value = record?.[key];
  return typeof value === 'string' && value.trim() ? value.trim() : null;
};

const messageHasWorkTimeline = (message: ChatMessage): boolean =>
  message.role === 'assistant' && message.agentTimelineParts.some((part) => part.kind !== 'text');

const messageTaskLinkIds = (message: ChatMessage): Set<string> => {
  const ids = new Set<string>();
  for (const part of message.agentTimelineParts) {
    for (const record of [part.metadata, part.output, part.progress]) {
      const taskId = recordString(record, 'taskId');
      const sidechainSessionId = recordString(record, 'sidechainSessionId');
      if (taskId) ids.add(taskId);
      if (sidechainSessionId) ids.add(sidechainSessionId);
    }
  }
  return ids;
};

const appendTask = (map: Map<string, TaskProjection[]>, messageId: string, task: TaskProjection): void => {
  const list = map.get(messageId) ?? [];
  if (!list.some((item) => item.id === task.id)) {
    list.push(task);
  }
  map.set(messageId, list);
};

interface UseMessageTaskProjectionOptions {
  activeSessionId: Accessor<string | null>;
  messages: Accessor<ChatMessage[]>;
}

export function useMessageTaskProjection(options: UseMessageTaskProjectionOptions) {
  let previousTaskProjectionSessionId: string | null = null;
  let previousProjectedTaskCount = 0;

  createEffect(() => {
    const release = subscribeTaskProjectionPolling(options.activeSessionId());
    onCleanup(release);
  });

  const projectedSubagentTasks = () =>
    taskProjectionState.sessionId === options.activeSessionId() ? taskProjectionState.tasks : [];

  const tasksByMessageId = createMemo(() => {
    const map = new Map<string, TaskProjection[]>();
    const tasks = projectedSubagentTasks();
    const workMessages = options.messages()
      .filter(messageHasWorkTimeline)
      .slice()
      .sort((left, right) => Date.parse(left.createdAt) - Date.parse(right.createdAt));

    if (tasks.length === 0 || workMessages.length === 0) return map;

    const messageByLinkedId = new Map<string, string>();
    for (const message of workMessages) {
      for (const id of messageTaskLinkIds(message)) {
        messageByLinkedId.set(id, message.id);
      }
    }

    const unresolved: TaskProjection[] = [];
    for (const task of tasks) {
      const directMessageId =
        messageByLinkedId.get(task.id) ||
        (task.sidechainSessionId ? messageByLinkedId.get(task.sidechainSessionId) : undefined) ||
        (task.parentTaskId ? messageByLinkedId.get(task.parentTaskId) : undefined);

      if (directMessageId) {
        appendTask(map, directMessageId, task);
      } else {
        unresolved.push(task);
      }
    }

    for (const task of unresolved) {
      const taskCreatedAtMs = Date.parse(task.createdAt);
      const message =
        workMessages
          .filter((item) => {
            const messageCreatedAtMs = Date.parse(item.createdAt);
            return !Number.isNaN(taskCreatedAtMs) &&
              !Number.isNaN(messageCreatedAtMs) &&
              messageCreatedAtMs <= taskCreatedAtMs;
          })
          .at(-1) ??
        workMessages[0];

      if (message) appendTask(map, message.id, task);
    }

    return map;
  });

  createEffect(() => {
    const sessionId = options.activeSessionId();
    const count = projectedSubagentTasks().length;
    if (sessionId !== previousTaskProjectionSessionId) {
      previousTaskProjectionSessionId = sessionId;
      previousProjectedTaskCount = count;
      return;
    }
    if (previousProjectedTaskCount === 0 && count > 0) {
      openBackgroundTasksPanel();
    }
    previousProjectedTaskCount = count;
  });

  return {
    projectedSubagentTasks,
    tasksByMessageId,
  };
}
