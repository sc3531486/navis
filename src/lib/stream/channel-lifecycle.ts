import type {
  StandardStreamChunk,
  StreamCompletion,
  StreamTermination,
} from './types';

export interface ChannelLifecycleOptions<T, R> {
  completion?: StreamCompletion;
  onChunk: (chunk: T, envelope: StandardStreamChunk<T>) => void;
  onCreated?: (result: R) => void;
  disposeLateResource?: (result: R) => void;
  onTermination?: (termination: StreamTermination) => void;
  onCancel?: (streamId: string) => void;
}

export interface ChannelLifecycle<T, R> {
  handleEnvelope: (chunk: StandardStreamChunk<T>) => void;
  handleCreated: (result: R) => void;
  handleInvokeError: (error: unknown) => void;
  stop: (reason?: string) => void;
  complete: () => void;
  streamId: () => string | null;
  finished: () => boolean;
  termination: () => StreamTermination | null;
}

function chunkError<T>(chunk: StandardStreamChunk<T>): Error {
  const key = chunk.kind === 'cancelled' ? 'reason' : 'error';
  const fallback = chunk.kind === 'cancelled' ? 'Stream cancelled' : 'Stream error';
  const message = typeof chunk.data === 'object' && chunk.data && key in chunk.data
    ? String((chunk.data as Record<string, unknown>)[key])
    : fallback;
  return new Error(message);
}

function chunkReason<T>(chunk: StandardStreamChunk<T>): string | undefined {
  if (typeof chunk.data !== 'object' || !chunk.data) return undefined;
  const reason = (chunk.data as Record<string, unknown>).reason;
  return typeof reason === 'string' && reason.trim() ? reason : undefined;
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

export function createChannelLifecycle<T, R>(
  options: ChannelLifecycleOptions<T, R>,
): ChannelLifecycle<T, R> {
  const completion = options.completion ?? 'channel';
  let currentStreamId: string | null = null;
  let accepting = true;
  let completed = false;
  let creationSettled = false;
  let termination: StreamTermination | null = null;

  const finish = (reason: StreamTermination): void => {
    if (completed) return;
    completed = true;
    termination = reason;
    try {
      options.onTermination?.(reason);
    } catch {
      // Termination is already committed; consumer cleanup must not reopen it.
    }
  };

  const disposeLate = (result: R): void => {
    try {
      options.disposeLateResource?.(result);
    } catch {
      // The resource is already outside this run's ownership; cleanup is best effort.
    }
  };

  const terminate = (reason: StreamTermination): void => {
    if (!accepting) return;
    accepting = false;
    finish(reason);
  };

  return {
    handleEnvelope: (chunk) => {
      if (!accepting) return;
      if (chunk.streamId) currentStreamId = chunk.streamId;
      if (chunk.kind === 'data') {
        try {
          options.onChunk(chunk.data, chunk);
        } catch (error) {
          accepting = false;
          try {
            if (currentStreamId) options.onCancel?.(currentStreamId);
          } finally {
            finish({ kind: 'error', error: asError(error), envelope: chunk });
          }
        }
        return;
      }
      if (chunk.kind === 'done') {
        terminate({ kind: 'completed' });
        return;
      }
      if (chunk.kind === 'cancelled') {
        terminate({ kind: 'cancelled', reason: chunkReason(chunk) });
        return;
      }
      terminate({ kind: 'error', error: chunkError(chunk), envelope: chunk });
    },
    handleCreated: (result) => {
      if (creationSettled) return;
      creationSettled = true;
      if (!accepting) {
        disposeLate(result);
        return;
      }
      try {
        options.onCreated?.(result);
      } catch (error) {
        accepting = false;
        try {
          disposeLate(result);
        } finally {
          finish({ kind: 'error', error: asError(error) });
        }
        return;
      }
      if (completion === 'invoke') terminate({ kind: 'completed' });
    },
    handleInvokeError: (error) => {
      if (creationSettled || !accepting) return;
      creationSettled = true;
      terminate({
        kind: 'creation_error',
        error: asError(error),
      });
    },
    stop: (reason) => {
      if (!accepting) return;
      accepting = false;
      try {
        if (currentStreamId) options.onCancel?.(currentStreamId);
      } finally {
        finish({ kind: 'stopped', reason });
      }
    },
    complete: () => terminate({ kind: 'completed' }),
    streamId: () => currentStreamId,
    finished: () => completed,
    termination: () => termination,
  };
}
