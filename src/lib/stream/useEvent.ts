/**
 * Tauri 事件监听 Hook — 封装 Tauri app.listen()
 * 来源：02b-stream.md §5.3
 *
 * 在组件生命周期内监听 Tauri 投影事件，并在 onCleanup 时自动清理。
 *
 * @example
 * useEvent('terminal.command.completed', (e) => {
 *   console.log('命令完成:', e.payload);
 * });
 */
import { onCleanup } from 'solid-js';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export function useEvent<T = unknown>(
  eventName: string,
  handler: (event: { payload: T }) => void,
): void {
  let unlisten: UnlistenFn | null = null;

  listen<T>(eventName, handler).then(fn => {
    unlisten = fn;
  });

  onCleanup(() => {
    unlisten?.();
  });
}
