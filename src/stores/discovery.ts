import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import type { UiExtensionDiscoveryResult } from '@/lib/extension-ui';

export interface ExtensionDiscoveryQuery {
  capability?: string;
  provides?: string;
}

interface DiscoveryState {
  loading: boolean;
  lastQuery: ExtensionDiscoveryQuery | null;
  results: UiExtensionDiscoveryResult[];
  error: string | null;
}

export const [extensionDiscoveryState, setExtensionDiscoveryState] = createStore<DiscoveryState>({
  loading: false,
  lastQuery: null,
  results: [],
  error: null,
});

const cache = new Map<string, UiExtensionDiscoveryResult[]>();
let invalidationInstall: Promise<void> | null = null;
let unlisteners: UnlistenFn[] = [];

function cacheKey(query: ExtensionDiscoveryQuery): string {
  return JSON.stringify({
    capability: query.capability ?? null,
    provides: query.provides ?? null,
  });
}

export async function queryExtensionDiscovery(query: ExtensionDiscoveryQuery = {}): Promise<UiExtensionDiscoveryResult[]> {
  const key = cacheKey(query);
  const cached = cache.get(key);
  if (cached) {
    setExtensionDiscoveryState({ lastQuery: query, results: cached, error: null });
    return cached;
  }

  setExtensionDiscoveryState({ loading: true, lastQuery: query, error: null });
  try {
    const results = await invoke<UiExtensionDiscoveryResult[]>('ui_extension_discovery_query', { query });
    cache.set(key, results);
    setExtensionDiscoveryState({ loading: false, results, error: null });
    return results;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    setExtensionDiscoveryState({ loading: false, results: [], error: message });
    throw error;
  }
}

export function invalidateExtensionDiscoveryCache(): void {
  cache.clear();
  setExtensionDiscoveryState({ results: [], error: null });
}

export function installExtensionDiscoveryInvalidation(): () => void {
  let disposed = false;
  if (!invalidationInstall) {
    // 订阅后端真实内核事件（经 tauri_events.rs 原样透传）：
    // enabled/disabled/installed/uninstalled/updated 表示扩展集合已变化，失效查询缓存。
    // enabling/error 是瞬时态/失败态，不改变可发现性，无需失效。
    invalidationInstall = Promise.all(['extension.enabled', 'extension.disabled', 'extension.installed', 'extension.uninstalled', 'extension.updated'].map(async (eventName) => {
      const unlisten = await listen(eventName, () => invalidateExtensionDiscoveryCache());
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    })).then(() => undefined);
  }
  return () => {
    disposed = true;
    for (const unlisten of unlisteners.splice(0)) unlisten();
    invalidationInstall = null;
  };
}
