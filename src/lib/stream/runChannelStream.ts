import { createChannelRun } from './channel-run';
import type { StreamRunController, StreamRunOptions } from './types';

export function runChannelStream<T, R = unknown>(
  options: StreamRunOptions<T, R>,
): StreamRunController {
  return createChannelRun(options);
}
