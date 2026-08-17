import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import { createPollingCoordinator } from '../lib/status';
import type { StatusPresentation } from '../lib/status';

export interface SessionTodoItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed' | string;
  statusPresentation: StatusPresentation;
  priority?: string | null;
}

interface SessionTodosState {
  sessionId: string | null;
  loading: boolean;
  error: string | null;
  todos: SessionTodoItem[];
}

export const [sessionTodosState, setSessionTodosState] = createStore<SessionTodosState>({
  sessionId: null,
  loading: false,
  error: null,
  todos: [],
});

export async function refreshSessionTodos(sessionId: string | null): Promise<void> {
  if (!sessionId) {
    setSessionTodosState({
      sessionId: null,
      loading: false,
      error: null,
      todos: [],
    });
    return;
  }

  setSessionTodosState({
    sessionId,
    loading: sessionTodosState.sessionId !== sessionId,
    error: null,
  });

  try {
    const todos = await invoke<SessionTodoItem[]>('ui_list_session_todos', {
      payload: { sessionId },
    });
    if (sessionTodosState.sessionId !== sessionId) return;
    setSessionTodosState({
      loading: false,
      todos,
      error: null,
    });
  } catch (error) {
    if (sessionTodosState.sessionId !== sessionId) return;
    setSessionTodosState({
      loading: false,
      error: error instanceof Error ? error.message : String(error),
    });
  }
}

const sessionTodosPolling = createPollingCoordinator(refreshSessionTodos);

export const subscribeSessionTodosPolling = (sessionId: string | null) =>
  sessionTodosPolling.subscribe(sessionId);
