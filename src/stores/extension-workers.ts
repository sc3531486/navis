/**
 * 扩展 Worker 轨生命周期注册表（阶段 5，design/34 §阶段5）。
 *
 * 职责：
 * - 按 extensionId 管理活跃 Worker 列表，统一 spawn / terminate。
 * - 扩展禁用/卸载/更新时回收已 spawn 的 worker（terminateExtensionWorkers）。
 * - 消费 UiExtensionScript.runOn："activation" 脚本在应用启动/扩展启用时自动 spawn。
 *
 * 竞态说明（task 4）：bridge.ts 的 `extensionWorkerBootstrapScript` 中 run 消息处理器先于
 * import() resolve 注册，若 run 消息先到则 onRun 为 undefined。本模块通过 ready 信号
 * 包装模块（wrapper 在真实 entry import 完成后向宿主 postMessage ready），宿主收到
 * ready 后才下发 run 消息；并保留超时兜底，保证无 ready 时也不会永久挂起。
 */

import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bindExtensionWorkerBridge, extensionWorkerBootstrapScript } from './bridge';
import type { UiExtensionScript } from '@/lib/extension-ui';

export interface ExtensionScriptWorkerHandle {
  extensionId: string;
  scriptId: string;
  worker: Worker;
  terminate: () => void;
}

export interface SpawnExtensionWorkerOptions {
  /** 传递给扩展 onRun 的 payload。 */
  args?: unknown;
  /** 常驻模式：不因单次 result 消息自动回收（activation 类后台脚本）。 */
  keepAlive?: boolean;
}

interface ActiveWorkerEntry {
  extensionId: string;
  scriptId: string;
  worker: Worker;
  terminate: () => void;
}

/** extensionId → 活跃 Worker 列表。 */
const activeWorkers = new Map<string, ActiveWorkerEntry[]>();

/** 已自动 spawn 的 activation 脚本（去重，禁用后清除以支持重新启用）。 */
const activatedScripts = new Set<string>();

let lifecycleInstalled = false;
let lifecycleUnlisteners: UnlistenFn[] = [];

/** ready 信号超时后仍强制下发 run 的兜底窗口。 */
const READY_TIMEOUT_MS = 3000;

function workerKey(extensionId: string, scriptId: string): string {
  return `${extensionId}:${scriptId}`;
}

/** 从后端 kernel 事件 payload 中提取 extensionId（信封透传 { extensionId } 或 { payload: { extensionId } }）。 */
function payloadExtensionId(payload: unknown): string | null {
  if (typeof payload === 'string') return payload.length > 0 ? payload : null;
  if (!payload || typeof payload !== 'object') return null;
  const record = payload as Record<string, unknown>;
  if (typeof record.extensionId === 'string' && record.extensionId) return record.extensionId;
  const inner = record.payload;
  if (inner && typeof inner === 'object') {
    const id = (inner as Record<string, unknown>).extensionId;
    if (typeof id === 'string' && id) return id;
  }
  return null;
}

/**
 * 构造 ready 信号包装模块：等待真实 entry import 完成后向宿主发 ready，
 * 再重导出 onRun/onMessage，规避 bootstrap 中 run 消息先于模块加载的竞态。
 */
function createReadySignalingWrapper(realModuleUrl: string): string {
  return [
    `import * as real from ${JSON.stringify(realModuleUrl)};`,
    `self.postMessage({ source: 'navis-extension', __navis: true, type: 'ready' });`,
    `export const onRun = real.onRun;`,
    `export const onMessage = real.onMessage;`,
  ].join('\n');
}

function removeEntry(extensionId: string, scriptId: string, worker: Worker): void {
  const entries = activeWorkers.get(extensionId);
  if (!entries) return;
  const index = entries.findIndex((entry) => entry.worker === worker);
  if (index >= 0) entries.splice(index, 1);
  if (entries.length === 0) activeWorkers.delete(extensionId);
  activatedScripts.delete(workerKey(extensionId, scriptId));
}

/**
 * 启动扩展脚本 Worker（白名单桥 + ready 门控 run 派发），并登记到生命周期注册表。
 * fail-closed：脚本无 resourcePath 时返回 null 不报错。
 */
export function spawnExtensionScriptWorker(
  script: UiExtensionScript,
  options: SpawnExtensionWorkerOptions = {},
): ExtensionScriptWorkerHandle | null {
  if (!script.resourcePath) return null;
  const { extensionId, scriptId } = script;

  const realModuleUrl = convertFileSrc(script.resourcePath);
  const wrapperUrl = URL.createObjectURL(
    new Blob([createReadySignalingWrapper(realModuleUrl)], { type: 'text/javascript' }),
  );
  const bootstrapUrl = URL.createObjectURL(
    new Blob([extensionWorkerBootstrapScript(wrapperUrl)], { type: 'text/javascript' }),
  );
  const worker = new Worker(bootstrapUrl, { type: 'module', name: `${extensionId}:${scriptId}` });
  const cleanupBridge = bindExtensionWorkerBridge(worker, { extensionId });

  let disposed = false;
  let runPosted = false;
  let readyTimer = 0;

  const postRun = () => {
    if (disposed || runPosted) return;
    runPosted = true;
    worker.postMessage({ source: 'navis-host', type: 'run', extensionId, scriptId, args: options.args ?? {} });
  };

  const onReady = (event: MessageEvent<{ source?: string; type?: string }>) => {
    if (event.data?.source !== 'navis-extension' || event.data.type !== 'ready') return;
    postRun();
  };
  const onError = (event: ErrorEvent) => {
    console.warn('[ExtensionWorker] worker error', extensionId, scriptId, event.message);
    cleanup();
  };
  const onResult = (event: MessageEvent<{ source?: string; type?: string; ok?: boolean; error?: string }>) => {
    if (event.data?.source !== 'navis-extension' || event.data.type !== 'result') return;
    if (!event.data.ok) console.warn('[ExtensionWorker] worker failed', extensionId, scriptId, event.data.error);
    if (!options.keepAlive) cleanup();
  };

  const cleanup = () => {
    if (disposed) return;
    disposed = true;
    window.clearTimeout(readyTimer);
    worker.removeEventListener('message', onReady);
    worker.removeEventListener('message', onResult);
    worker.removeEventListener('error', onError);
    cleanupBridge();
    worker.terminate();
    URL.revokeObjectURL(bootstrapUrl);
    URL.revokeObjectURL(wrapperUrl);
    removeEntry(extensionId, scriptId, worker);
  };

  readyTimer = window.setTimeout(postRun, READY_TIMEOUT_MS);
  worker.addEventListener('message', onReady);
  worker.addEventListener('message', onResult);
  worker.addEventListener('error', onError);

  activeWorkers.set(extensionId, [
    ...(activeWorkers.get(extensionId) ?? []),
    { extensionId, scriptId, worker, terminate: cleanup },
  ]);

  return { extensionId, scriptId, worker, terminate: cleanup };
}

/**
 * 自动触发 run_on 含 "activation" 的脚本。幂等：同一 (extensionId, scriptId) 仅 spawn 一次。
 * 可传入已拉取的 scripts（应用启动路径），缺省时重新 invoke 拉取。
 */
export async function runActivationScripts(scripts?: UiExtensionScript[]): Promise<number> {
  const list = scripts ?? (await invoke<UiExtensionScript[]>('ui_list_extension_scripts').catch(() => []));
  let spawned = 0;
  for (const script of list) {
    if (!script.runOn.some((trigger) => trigger === 'activation')) continue;
    if (!script.resourcePath) continue;
    const key = workerKey(script.extensionId, script.scriptId);
    if (activatedScripts.has(key)) continue;
    const handle = spawnExtensionScriptWorker(script, { keepAlive: true });
    if (!handle) continue;
    activatedScripts.add(key);
    spawned++;
  }
  return spawned;
}

/** 回收指定扩展的全部 Worker（禁用/卸载/更新时调用）。 */
export function terminateExtensionWorkers(extensionId: string): void {
  const entries = activeWorkers.get(extensionId);
  if (!entries) return;
  for (const entry of entries.slice()) entry.terminate();
  activeWorkers.delete(extensionId);
  for (const key of [...activatedScripts]) {
    if (key.startsWith(`${extensionId}:`)) activatedScripts.delete(key);
  }
}

/** 回收所有扩展 Worker。 */
export function terminateAllExtensionWorkers(): void {
  for (const extensionId of [...activeWorkers.keys()]) terminateExtensionWorkers(extensionId);
}

/**
 * 订阅后端扩展生命周期事件，维护 worker 生命周期：
 * - disabled/uninstalled/updated → 回收该扩展的 worker；
 * - enabled → 重新拉取脚本并触发 activation 自动 spawn（幂等）。
 */
export function installExtensionWorkerLifecycle(): () => void {
  if (lifecycleInstalled) return () => {};
  lifecycleInstalled = true;
  let disposed = false;
  void Promise.all([
    listen('extension.disabled', (event) => {
      const extensionId = payloadExtensionId(event.payload);
      if (extensionId) terminateExtensionWorkers(extensionId);
    }),
    listen('extension.uninstalled', (event) => {
      const extensionId = payloadExtensionId(event.payload);
      if (extensionId) terminateExtensionWorkers(extensionId);
    }),
    listen('extension.updated', (event) => {
      const extensionId = payloadExtensionId(event.payload);
      if (extensionId) terminateExtensionWorkers(extensionId);
    }),
    listen('extension.enabled', () => {
      void runActivationScripts();
    }),
  ]).then((unlisteners) => {
    if (disposed) {
      for (const unlisten of unlisteners) unlisten();
    } else {
      lifecycleUnlisteners.push(...unlisteners);
    }
  });
  return () => {
    disposed = true;
    for (const unlisten of lifecycleUnlisteners.splice(0)) unlisten();
    lifecycleInstalled = false;
  };
}