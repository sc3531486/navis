import { Channel, invoke } from '@tauri-apps/api/core';
import { createChannelLifecycle } from './channel-lifecycle';
import type { StandardStreamChunk, StreamRunController, StreamRunOptions } from './types';

export function createChannelRun<T, R = unknown>(
  options: StreamRunOptions<T, R>,
): StreamRunController {
  const channel = new Channel<StandardStreamChunk<T>>();
  const lifecycle = createChannelLifecycle<T, R>({
    ...options,
    onCancel: (streamId) => {
      void invoke('ui_cancel_stream', { payload: { streamId } }).catch(() => undefined);
    },
  });

  channel.onmessage = lifecycle.handleEnvelope;

  void invoke<R>(options.command, { ...options.args, channel }).then(
    lifecycle.handleCreated,
    lifecycle.handleInvokeError,
  );

  return {
    stop: lifecycle.stop,
    complete: lifecycle.complete,
    streamId: lifecycle.streamId,
    finished: lifecycle.finished,
    termination: lifecycle.termination,
  };
}
