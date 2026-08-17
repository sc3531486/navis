/**
 * 通用 Channel Hook — 封装 Tauri Channel<T> 生命周期
 * 来源：02b-stream.md §5.2
 *
 * 两种消费模式：
 * - 'callback'：每次 chunk 直接回调（terminal、高频流，不经 Signal）
 * - 'signal'：累积到 Solid Signal 数组（LLM 文本累积）
 */
import { createSignal, onCleanup } from 'solid-js';
import { createChannelRun } from './channel-run';
import type { StreamCompletion, StreamRunController, StreamTermination } from './types';

/** useChannel 配置项 */
export interface UseChannelOptions<T, R = unknown> {
  /** 要调用的 Tauri Command 名称 */
  command: string;
  /** 命令参数（不含 channel，hook 自动注入） */
  args?: Record<string, unknown> | (() => Record<string, unknown>);
  /**
   * 数据消费模式：
   * - 'callback'：每次 chunk 直接调用 onChunk（适合 terminal、高频流）
   * - 'signal'：累积到 data() 数组（适合 LLM 文本累积）
   */
  mode: 'callback' | 'signal';
  /** 流的完成来源：Channel 终态、命令返回或业务手动完成。 */
  completion?: StreamCompletion;
  /** mode='callback' 时必传：每个 chunk 的处理函数 */
  onChunk?: (chunk: T) => void;
  /**
   * Tauri Command 返回值回调（流启动成功后触发）。
   * 用于获取 Command 返回的数据（如 ptyId、streamId 等）。
   * @example terminal.createPty 返回 { ptyId, channel }，通过此回调获取 ptyId
   */
  onCreated?: (result: R) => void;
  /** stop/unmount 后 Command 才返回时释放其创建的后端资源。 */
  disposeLateResource?: (result: R) => void;
  /** 流结束时触发，业务只需处理一个明确的终止结果。 */
  onTermination?: (termination: StreamTermination) => void;
}

/** useChannel 返回值 */
export interface UseChannelReturn<T> {
  /** mode='signal' 时：累积的数据数组；mode='callback' 时：空数组 */
  data: () => T[];
  /** 流是否正在进行 */
  isActive: () => boolean;
  /** 最后一次终止结果（null 表示尚未结束）。 */
  termination: () => StreamTermination | null;
  /** 手动启动流 */
  start: () => Promise<void>;
  /** 停止当前流并请求后端取消已知的 stream。 */
  stop: (reason?: string) => void;
  /** completion='manual' 时由业务显式结束当前流。 */
  complete: () => void;
}

export function useChannel<T, R = unknown>(options: UseChannelOptions<T, R>): UseChannelReturn<T> {
  const [data, setData] = createSignal<T[]>([]);
  const [isActive, setIsActive] = createSignal(false);
  const [termination, setTermination] = createSignal<StreamTermination | null>(null);

  type ChannelRun = {
    id: number;
    controller: StreamRunController;
  };

  let nextRunId = 0;
  let activeRun: ChannelRun | null = null;

  function isCurrentRun(run: ChannelRun): boolean {
    return activeRun?.id === run.id;
  }

  function finish(run: ChannelRun, reason: StreamTermination): void {
    if (!isCurrentRun(run)) return;
    activeRun = null;
    setIsActive(false);
    setTermination(reason);
    try {
      options.onTermination?.(reason);
    } catch {
      // The lifecycle is already committed; a UI callback must not reopen it.
    }
  }

  function handleChunk(run: ChannelRun, chunk: T): void {
    if (!isCurrentRun(run)) return;

    if (options.mode === 'callback' && options.onChunk) {
      options.onChunk(chunk);
    } else {
      setData(prev => [...prev, chunk]);
    }
  }

  const start = async (): Promise<void> => {
    if (activeRun) return;

    let run!: ChannelRun;
    run = {
      id: ++nextRunId,
      controller: createChannelRun<T, R>({
        command: options.command,
        args: typeof options.args === 'function' ? options.args() : options.args,
        completion: options.completion,
        onChunk: (chunk) => handleChunk(run, chunk),
        onCreated: (result) => {
          if (isCurrentRun(run)) options.onCreated?.(result);
        },
        disposeLateResource: options.disposeLateResource,
        onTermination: (reason) => {
          finish(run, reason);
        },
      }),
    };
    activeRun = run;
    setIsActive(true);
    setTermination(null);
  };

  const stop = (reason?: string) => {
    const run = activeRun;
    if (!run) return;
    run.controller.stop(reason);
  };

  const complete = () => {
    const run = activeRun;
    if (!run) return;
    run.controller.complete();
  };

  onCleanup(stop);

  return { data, isActive, termination, start, stop, complete };
}
