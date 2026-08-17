import { convertFileSrc } from '@tauri-apps/api/core';
import { invoke } from '@tauri-apps/api/core';
import { Component, createMemo, createResource, onCleanup, Show } from 'solid-js';
import { appState } from '../../stores/app';
import { mountExtensionBridge, NAVIS_SHIM_SOURCE, type BridgeContextSnapshot } from '../../stores/bridge';
import type { HostViewRendererProps } from './types';

function resolveResourceSrc(resourcePath: string | null): string | null {
  const value = resourcePath?.trim();
  return value ? convertFileSrc(value) : null;
}

function assetBaseHref(resourcePath: string | null): string | null {
  if (!resourcePath) return null;
  const normalized = resourcePath.replace(/\\/g, '/');
  const lastSlash = normalized.lastIndexOf('/');
  if (lastSlash <= 0) return null;
  const dir = normalized.slice(0, lastSlash);
  return `${convertFileSrc(dir)}/`;
}

/** 在扩展入口 HTML 的 <head> 中注入 <base> + 垫片脚本。 */
function injectShimIntoHtml(html: string, baseHref: string | null): string {
  const shimTag = `<script>${NAVIS_SHIM_SOURCE}<\/script>`;
  const baseTag = baseHref ? `<base href="${baseHref}">` : '';
  const injection = `${baseTag}${shimTag}`;

  const headClose = html.toLowerCase().indexOf('</head>');
  if (headClose !== -1) {
    return `${html.slice(0, headClose)}${injection}${html.slice(headClose)}`;
  }
  return `<!doctype html><html><head>${injection}</head><body>${html}</body></html>`;
}

const HtmlSandboxRenderer: Component<HostViewRendererProps> = (props) => {
  const resourcePath = createMemo(() => props.view.resourcePath);
  const baseHref = createMemo(() => assetBaseHref(resourcePath()));

  const [htmlResource] = createResource(
    () => [props.view.extensionId, props.view.viewId] as const,
    async ([extensionId, viewId]) => {
      const rawHtml = await invoke<string>('ui_read_extension_entry_html', {
        extensionId,
        viewId,
      });
      return injectShimIntoHtml(rawHtml, baseHref());
    },
  );

  let iframeRef: HTMLIFrameElement | null = null;
  let unmountBridge: (() => void) | null = null;

  const attachBridge = () => {
    if (!iframeRef?.contentWindow) return;
    const snapshot: BridgeContextSnapshot = {
      session: { sessionId: appState.activeSessionId },
      activeProject: { projectId: appState.activeProjectId },
    };
    unmountBridge = mountExtensionBridge(iframeRef, props.view.extensionId, snapshot);
  };

  onCleanup(() => {
    unmountBridge?.();
  });

  const resourceSrc = createMemo(() => resolveResourceSrc(resourcePath()));

  return (
    <Show
      when={htmlResource()}
      fallback={
        <div class="navis-host-view-empty">
          <strong>HTML view unavailable</strong>
          <span>The Extension resource is unavailable or could not be read.</span>
        </div>
      }
    >
      {(html) => (
        <div class="navis-html-sandbox-renderer">
          <Show when={resourceSrc()} fallback={<div class="navis-host-view-empty"><span>Resource path missing</span></div>}>
            {(url) => (
              <iframe
                ref={(element) => {
                  iframeRef = element;
                }}
                class="navis-html-sandbox-frame"
                srcdoc={html()}
                title={props.view.name}
                loading="lazy"
                referrerPolicy="no-referrer"
                sandbox="allow-scripts"
                onLoad={attachBridge}
                data-asset-base={url()}
              />
            )}
          </Show>
        </div>
      )}
    </Show>
  );
};

export default HtmlSandboxRenderer;