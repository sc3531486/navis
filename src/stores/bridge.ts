/**
 * 扩展白名单桥（宿主侧）。
 *
 * 方案：宿主透传垫片（跨平台一致）。
 * iframe 保持严格 sandbox（allow-scripts，无 allow-same-origin），因此 Tauri 不向
 * iframe 注入 IPC（CVE-2024-35222 修复后非 Windows 跨源均不可用）。扩展页面经
 * 宿主读取 entry HTML + 注入垫片脚本（`srcdoc` 渲染），垫片提供与宿主前端同构的
 * `__NAVIS__` API（invoke / listen / getContext / dialog / call / storage / fetch /
 * extensions），底层经 postMessage 与宿主透传。
 *
 * Worker 轨（阶段 5）复用同一白名单桥：worker 内引导脚本建立 `self.__NAVIS__`，
 * 经 `postMessage` 与宿主桥透传，宿主桥转发到 Tauri invoke/listen。
 *
 * 权限全部集中在 Rust 命令内（ui_extension_bridge_invoke 三层校验 +
 * ui_extension_bridge_authorize_event 事件授权），本模块只做来源校验 + 透传。
 */

import { Channel, invoke } from '@tauri-apps/api/core';
import { listen as tauriListen, type UnlistenFn } from '@tauri-apps/api/event';

export interface BridgeContextSnapshot {
  session?: unknown;
  activeProject?: unknown;
}

export interface ExtensionWorkerBridgeOptions {
  extensionId: string;
}

interface BridgeRequest {
  __navis?: boolean;
  id: number;
  type: 'invoke' | 'listen' | 'stream' | 'stream:unsubscribe';
  cmd?: string;
  args?: unknown;
  event?: string;
  filter?: unknown;
  streamId?: string;
}

interface BridgeResponse {
  id: number;
  ok: boolean;
  data?: unknown;
  error?: string;
}

/** 垫片脚本。注入到扩展入口 HTML 的 <head>，定义 window.__NAVIS__。 */
export const NAVIS_SHIM_SOURCE = `
(function () {
  'use strict';
  var pending = new Map();
  var nextId = 0;
  var subscriptions = new Map();

  function post(message) {
    window.parent.postMessage(message, '*');
  }

  window.addEventListener('message', function (event) {
    var msg = event.data;
    if (!msg || msg.__navis !== true) return;
    if (msg.type === 'invoke:result' || msg.type === 'listen:result') {
      var entry = pending.get(msg.id);
      if (!entry) return;
      pending.delete(msg.id);
      if (msg.ok) {
        entry.resolve(msg.data);
      } else {
        entry.reject(new Error(msg.error || 'bridge request failed'));
      }
    } else if (msg.type === 'event') {
      var handlers = subscriptions.get(msg.event);
      if (!handlers) return;
      handlers.forEach(function (handler) {
        try { handler(msg.payload); } catch (error) { console.error('[navis] extension event handler error', error); }
      });
    } else if (msg.type === 'context') {
      window.__NAVIS_CONTEXT__ = msg.payload || {};
    }
  });

  function request(type, body) {
    var id = ++nextId;
    var promise = new Promise(function (resolve, reject) {
      pending.set(id, { resolve: resolve, reject: reject });
    });
    post(Object.assign({ __navis: true, id: id, type: type }, body));
    return promise;
  }

  window.__NAVIS__ = {
    invoke: function (cmd, args) {
      return request('invoke', { cmd: cmd, args: args || {} });
    },
    listen: function (event, handler) {
      var key = String(event);
      if (!subscriptions.has(key)) {
        subscriptions.set(key, new Set());
        request('listen', { event: key }).catch(function (error) {
          subscriptions.delete(key);
          console.error('[navis] extension listen failed', error);
        });
      }
      subscriptions.get(key).add(handler);
      return function () {
        var set = subscriptions.get(key);
        if (set) {
          set.delete(handler);
          if (set.size === 0) subscriptions.delete(key);
        }
      };
    },
    getContext: function () {
      return Promise.resolve(window.__NAVIS_CONTEXT__ || {});
    },
    dialog: {
      open: function (options) {
        return request('invoke', { cmd: 'dialog.open', args: options || {} });
      },
      close: function () {
        return request('invoke', { cmd: 'dialog.close', args: {} });
      },
    },
    call: function (target, action, payload) {
      return request('invoke', { cmd: 'route.call', args: { target: target, action: action, payload: payload } });
    },
    storage: {
      get: function (key, opts) {
        return request('invoke', { cmd: 'storage.get', args: { key: key, scope: opts && opts.scope } });
      },
      set: function (key, value, opts) {
        return request('invoke', { cmd: 'storage.set', args: { key: key, value: value, scope: opts && opts.scope } });
      },
      delete: function (key, opts) {
        return request('invoke', { cmd: 'storage.delete', args: { key: key, scope: opts && opts.scope } });
      },
      clear: function (opts) {
        return request('invoke', { cmd: 'storage.clear', args: { scope: opts && opts.scope } });
      },
    },
    fetch: function (url, init) {
      return request('invoke', { cmd: 'network.fetch', args: { url: url, init: init || {} } });
    },
    extensions: {
      query: function (params) {
        return request('invoke', { cmd: 'extensions.query', args: params || {} });
      },
    },
    stream: {
      subscribeSource: function (filter, onChunk) {
        var streamKey = null;
        request('stream', { filter: filter || {} }).then(function (streamId) {
          streamKey = 'stream:' + streamId;
          if (!subscriptions.has(streamKey)) subscriptions.set(streamKey, new Set());
          subscriptions.get(streamKey).add(onChunk);
        }).catch(function (error) {
          console.error('[navis] extension stream subscribeSource failed', error);
        });
        return function () {
          if (!streamKey) return;
          var set = subscriptions.get(streamKey);
          if (set) {
            set.delete(onChunk);
            if (set.size === 0) subscriptions.delete(streamKey);
          }
          request('stream:unsubscribe', { streamId: streamKey.slice(7) }).catch(function () {});
        };
      },
    },
  };
})();
`;

/**
 * Worker 引导脚本。包装扩展 entry module，建立 `self.__NAVIS__` 白名单桥
 * （postMessage 透传），并响应宿主的 run 消息。
 */
export function extensionWorkerBootstrapScript(moduleUrl: string): string {
  return `
const navisHost = {
  pending: new Map(),
  nextId: 0,
  subscriptions: new Map(),
  send(message) {
    self.postMessage({ source: 'navis-extension', ...message });
  },
  request(type, body) {
    const id = ++this.nextId;
    const promise = new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
    this.send({ __navis: true, id, type, ...body });
    return promise;
  },
};
self.addEventListener('message', (event) => {
  const msg = event.data;
  if (!msg || msg.source !== 'navis-host') return;
  if (msg.type === 'invoke:result' || msg.type === 'listen:result') {
    const entry = navisHost.pending.get(msg.id);
    if (!entry) return;
    navisHost.pending.delete(msg.id);
    if (msg.ok) entry.resolve(msg.data);
    else entry.reject(new Error(msg.error || 'bridge request failed'));
  } else if (msg.type === 'event') {
    const handlers = navisHost.subscriptions.get(msg.event);
    if (handlers) handlers.forEach((handler) => { try { handler(msg.payload); } catch (error) { console.error('[navis] worker event handler error', error); } });
  } else if (msg.type === 'context') {
    self.__NAVIS_CONTEXT__ = msg.payload || {};
  } else if (msg.type === 'run') {
    navisHost.extensionId = msg.extensionId;
    navisHost.scriptId = msg.scriptId;
    navisHost.runArgs = msg.args || {};
    if (navisHost.onRun) navisHost.onRun(navisHost.runArgs);
  }
});
self.__NAVIS__ = {
  invoke: (cmd, args) => navisHost.request('invoke', { cmd, args: args || {} }),
  listen: (event, handler) => {
    const key = String(event);
    if (!navisHost.subscriptions.has(key)) {
      navisHost.subscriptions.set(key, new Set());
      navisHost.request('listen', { event: key }).catch((error) => {
        navisHost.subscriptions.delete(key);
        console.error('[navis] worker listen failed', error);
      });
    }
    navisHost.subscriptions.get(key).add(handler);
    return () => {
      const set = navisHost.subscriptions.get(key);
      if (set) {
        set.delete(handler);
        if (set.size === 0) navisHost.subscriptions.delete(key);
      }
    };
  },
  getContext: () => Promise.resolve(self.__NAVIS_CONTEXT__ || {}),
  notify: (payload) => navisHost.send({ __navis: true, type: 'result', ok: true, data: payload }),
  dialog: {
    open: (options) => navisHost.request('invoke', { cmd: 'dialog.open', args: options || {} }),
    close: () => navisHost.request('invoke', { cmd: 'dialog.close', args: {} }),
  },
  call: (target, action, payload) => navisHost.request('invoke', { cmd: 'route.call', args: { target, action, payload } }),
  storage: {
    get: (key, opts) => navisHost.request('invoke', { cmd: 'storage.get', args: { key, scope: opts && opts.scope } }),
    set: (key, value, opts) => navisHost.request('invoke', { cmd: 'storage.set', args: { key, value, scope: opts && opts.scope } }),
    delete: (key, opts) => navisHost.request('invoke', { cmd: 'storage.delete', args: { key, scope: opts && opts.scope } }),
    clear: (opts) => navisHost.request('invoke', { cmd: 'storage.clear', args: { scope: opts && opts.scope } }),
  },
  fetch: (url, init) => navisHost.request('invoke', { cmd: 'network.fetch', args: { url, init: init || {} } }),
  extensions: {
    query: (params) => navisHost.request('invoke', { cmd: 'extensions.query', args: params || {} }),
  },
  stream: {
    subscribeSource: (filter, onChunk) => {
      let streamKey = null;
      navisHost.request('stream', { filter: filter || {} }).then((streamId) => {
        streamKey = 'stream:' + streamId;
        if (!navisHost.subscriptions.has(streamKey)) navisHost.subscriptions.set(streamKey, new Set());
        navisHost.subscriptions.get(streamKey).add(onChunk);
      }).catch((error) => {
        console.error('[navis] worker stream subscribeSource failed', error);
      });
      return () => {
        if (!streamKey) return;
        const set = navisHost.subscriptions.get(streamKey);
        if (set) {
          set.delete(onChunk);
          if (set.size === 0) navisHost.subscriptions.delete(streamKey);
        }
        navisHost.request('stream:unsubscribe', { streamId: streamKey.slice(7) }).catch(() => {});
      };
    },
  },
};
import('${moduleUrl}').then((module) => {
  if (typeof module.onRun === 'function') navisHost.onRun = module.onRun;
  if (typeof module.onMessage === 'function') {
    self.addEventListener('message', (event) => {
      const msg = event.data;
      if (!msg || msg.source !== 'navis-host' || msg.type === 'run') return;
      if (msg.type === 'message' && typeof module.onMessage === 'function') {
        Promise.resolve(module.onMessage(msg.payload)).then(
          (data) => navisHost.send({ __navis: true, type: 'result', ok: true, data }),
          (error) => navisHost.send({ __navis: true, type: 'result', ok: false, error: String(error) }),
        );
      }
    });
  }
}).catch((error) => {
  navisHost.send({ __navis: true, type: 'result', ok: false, error: 'Failed to load extension script: ' + String(error) });
  self.close();
});
`;
}

interface ActiveSubscription {
  event: string;
  unlisten: UnlistenFn;
}

interface BridgeDispatcher {
  dispatchInvoke: (id: number, cmd: string, args: unknown) => void;
  dispatchListen: (id: number, event: string) => void;
  dispatchStream: (id: number, filter: unknown) => void;
  dispatchStreamUnsubscribe: (id: number, streamId: string) => void;
  sendToTarget: (message: Record<string, unknown>) => void;
}

/** 共享的宿主侧请求处理：校验 → 透传 Tauri invoke/listen → 回传目标。 */
function createBridgeDispatcher(
  extensionId: string,
  options: { dispatchInvoke?: (id: number, cmd: string, args: unknown) => Promise<void> } = {},
): BridgeDispatcher {
  const activeSubscriptions = new Map<string, ActiveSubscription>();
  const activeStreams = new Map<string, Channel<unknown>>();
  const pendingResponses = new Map<number, (response: BridgeResponse) => void>();
  let sendToTarget: (message: Record<string, unknown>) => void = () => {};

  async function handleInvoke(id: number, cmd: string, args: unknown): Promise<void> {
    try {
      const result = (await invoke('ui_extension_bridge_invoke', {
        extension_id: extensionId,
        cmd,
        args,
      })) as { ok: boolean; data?: unknown; error?: string };
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: result.ok, data: result.data, error: result.error });
    } catch (error) {
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: false, error: String(error) });
    }
  }

  async function handleListen(id: number, event: string): Promise<void> {
    try {
      await invoke('ui_extension_bridge_authorize_event', { extension_id: extensionId, pattern: event });
      const unlisten = await tauriListen(event, (tauriEvent) => {
        sendToTarget({ __navis: true, type: 'event', event, payload: tauriEvent.payload });
      });
      activeSubscriptions.set(event, { event, unlisten });
      sendToTarget({ __navis: true, id, type: 'listen:result', ok: true });
    } catch (error) {
      sendToTarget({ __navis: true, id, type: 'listen:result', ok: false, error: String(error) });
    }
  }

  /**
   * `extension.stream.subscribeSource` 后端出口（独立 Tauri 命令，不走桥注册表）。
   *
   * 宿主侧可信，已校验消息来源。创建 Tauri Channel → `ui_extension_stream_subscribe`
   * 返回 streamId → channel.onmessage 以 `stream:<streamId>` 事件转发回目标。
   */
  async function handleStream(id: number, filter: unknown): Promise<void> {
    try {
      const channel = new Channel<unknown>();
      const streamId = (await invoke('ui_extension_stream_subscribe', {
        request: { extensionId, filter },
        channel,
      })) as string;
      channel.onmessage = (payload) => {
        sendToTarget({ __navis: true, type: 'event', event: `stream:${streamId}`, payload });
      };
      activeStreams.set(streamId, channel);
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: true, data: streamId });
    } catch (error) {
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: false, error: String(error) });
    }
  }

  async function handleStreamUnsubscribe(id: number, streamId: string): Promise<void> {
    try {
      activeStreams.delete(streamId);
      await invoke('ui_extension_stream_unsubscribe', { payload: { streamId } });
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: true });
    } catch (error) {
      sendToTarget({ __navis: true, id, type: 'invoke:result', ok: false, error: String(error) });
    }
  }

  return {
    dispatchInvoke: (id, cmd, args) => void handleInvoke(id, cmd, args),
    dispatchListen: (id, event) => void handleListen(id, event),
    dispatchStream: (id, filter) => void handleStream(id, filter),
    dispatchStreamUnsubscribe: (id, streamId) => void handleStreamUnsubscribe(id, streamId),
    sendToTarget,
  };
}

/** 建立宿主侧 iframe 桥。校验消息来源后透传到 Tauri invoke/listen，返回卸载函数。 */
export function mountExtensionBridge(
  iframe: HTMLIFrameElement,
  extensionId: string,
  context: BridgeContextSnapshot,
): () => void {
  const dispatcher = createBridgeDispatcher(extensionId);
  dispatcher.sendToTarget = (message) => {
    iframe.contentWindow?.postMessage(message, '*');
  };

  function onMessage(event: MessageEvent): void {
    if (event.source !== iframe.contentWindow) return;
    if (!event.data || event.data.__navis !== true) return;
    const message = event.data as BridgeRequest;
    if (message.type === 'invoke' && message.cmd) {
      dispatcher.dispatchInvoke(message.id, message.cmd, message.args);
    } else if (message.type === 'listen' && message.event) {
      dispatcher.dispatchListen(message.id, message.event);
    } else if (message.type === 'stream') {
      dispatcher.dispatchStream(message.id, message.filter);
    } else if (message.type === 'stream:unsubscribe' && message.streamId) {
      dispatcher.dispatchStreamUnsubscribe(message.id, message.streamId);
    }
  }

  window.addEventListener('message', onMessage);
  iframe.contentWindow?.postMessage({ __navis: true, type: 'context', payload: context }, '*');

  return () => {
    window.removeEventListener('message', onMessage);
  };
}

/** 建立宿主侧 Worker 桥（阶段 5，脚本轨）。校验来源标记后透传，返回卸载函数。 */
export function bindExtensionWorkerBridge(
  worker: Worker,
  options: ExtensionWorkerBridgeOptions,
): () => void {
  const dispatcher = createBridgeDispatcher(options.extensionId);
  dispatcher.sendToTarget = (message) => {
    worker.postMessage({ source: 'navis-host', ...message });
  };

  function onMessage(event: MessageEvent): void {
    const data = event.data as (BridgeRequest & { source?: string }) | undefined;
    if (!data || data.source !== 'navis-extension' || data.__navis !== true) return;
    if (data.type === 'invoke' && data.cmd) {
      dispatcher.dispatchInvoke(data.id, data.cmd, data.args);
    } else if (data.type === 'listen' && data.event) {
      dispatcher.dispatchListen(data.id, data.event);
    }
  }

  worker.addEventListener('message', onMessage);

  return () => {
    worker.removeEventListener('message', onMessage);
  };
}