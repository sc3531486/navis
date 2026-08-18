/**
 * Navis 交互式终端面板。
 *
 * xterm.js 只负责显示和输入，PTY 与 Tauri Channel 的生命周期分别由
 * TerminalManager 和公共 useChannel 管理。
 */

import { Component, createEffect, createSignal, on, onCleanup, onMount, Show, splitProps } from 'solid-js';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { invoke } from '@tauri-apps/api/core';
import { activeSessionId } from '@session/stores/session-tree';
import { useChannel } from '@/lib/stream';
import { IconRestore } from '@navis-code/components/Icon';
import CloseIcon from '@/components/Icon/CloseIcon';

export interface TerminalPanelProps {
  visible: boolean;
  onCreated?: (ptyId: string) => void;
  onClose?: () => void;
}

interface CreatedPty {
  ptyId: string;
  sessionId: string;
}

const TERMINAL_THEME = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  cursor: '#d4d4d4',
  selectionBackground: '#264f78',
};

const TERMINAL_OPTIONS = {
  cursorBlink: true,
  fontSize: 14,
  fontFamily: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
  theme: TERMINAL_THEME,
  allowProposedApi: true,
};

const TerminalPanel: Component<TerminalPanelProps> = (props) => {
  const [local] = splitProps(props, ['visible', 'onCreated', 'onClose']);

  let containerRef!: HTMLDivElement;
  let terminal!: Terminal;
  let fitAddon!: FitAddon;
  let resizeObserver: ResizeObserver | null = null;
  let inputDisposable: { dispose: () => void } | null = null;
  let ptyId: string | null = null;
  let ptySessionId: string | null = null;

  const [isReady, setIsReady] = createSignal(false);
  const [isConnecting, setIsConnecting] = createSignal(false);

  const disconnectBackendPty = (created: CreatedPty): void => {
    void invoke('ui_terminal_close_pty', {
      payload: { sessionId: created.sessionId, ptyId: created.ptyId },
    }).catch((error) => {
      console.warn('Failed to close terminal PTY', error);
    });
  };

  const channel = useChannel<string, CreatedPty>({
    command: 'ui_terminal_create_pty',
    args: () => ({
      payload: {
        sessionId: activeSessionId(),
        shell: null,
        cwd: null,
      },
    }),
    mode: 'callback',
    onChunk: (chunk) => terminal?.write(chunk),
    onCreated: (created) => {
      ptyId = created.ptyId;
      ptySessionId = created.sessionId;
      local.onCreated?.(ptyId);
      setIsReady(true);
      setIsConnecting(false);
      registerInputHandler();
      terminal.focus();
      sendResize();
    },
    disposeLateResource: disconnectBackendPty,
    onTermination: (termination) => {
      inputDisposable?.dispose();
      inputDisposable = null;
      ptyId = null;
      ptySessionId = null;
      setIsConnecting(false);
      setIsReady(false);
      if (termination.kind === 'error' || termination.kind === 'creation_error') {
        terminal?.writeln(`\r\n\x1b[31m连接失败: ${termination.error.message}\x1b[0m`);
      }
    },
  });

  function sendResize(): void {
    if (!ptyId || !terminal) return;
    void invoke('ui_terminal_resize_pty', {
      payload: { ptyId, cols: terminal.cols, rows: terminal.rows },
    }).catch((error) => {
      console.warn('Failed to resize terminal PTY', error);
    });
  }

  function registerInputHandler(): void {
    inputDisposable?.dispose();
    inputDisposable = terminal.onData((data) => {
      if (!ptyId) return;
      void invoke('ui_terminal_write_pty', {
        payload: { ptyId, data },
      }).catch((error) => {
        console.warn('Failed to write terminal PTY', error);
      });
    });
  }

  function disconnectPty(): void {
    inputDisposable?.dispose();
    inputDisposable = null;
    const closingPtyId = ptyId;
    const closingSessionId = ptySessionId;
    ptyId = null;
    ptySessionId = null;
    channel.stop();
    if (closingPtyId && closingSessionId) {
      disconnectBackendPty({ ptyId: closingPtyId, sessionId: closingSessionId });
    }
    setIsReady(false);
    terminal?.clear();
  }

  function connectPty(): void {
    if (isConnecting() || channel.isActive()) return;
    const sessionId = activeSessionId();
    if (!sessionId) {
      terminal?.writeln('\r\n\x1b[31m连接失败: no active session\x1b[0m');
      return;
    }
    setIsConnecting(true);
    void channel.start();
  }

  onMount(() => {
    terminal = new Terminal(TERMINAL_OPTIONS);
    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(containerRef);
    fitAddon.fit();

    resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      sendResize();
    });
    resizeObserver.observe(containerRef);
  });

  createEffect(on(
    () => [local.visible, activeSessionId()] as const,
    ([visible, sessionId], previous) => {
      const [wasVisible, previousSessionId] = previous ?? [false, null];
      if (visible && wasVisible && sessionId !== previousSessionId) disconnectPty();
      if (visible && !wasVisible) connectPty();
      if (visible && wasVisible && sessionId !== previousSessionId) connectPty();
      if (!visible && wasVisible) disconnectPty();
    },
    { defer: false },
  ));

  onCleanup(() => {
    disconnectPty();
    resizeObserver?.disconnect();
    inputDisposable?.dispose();
    terminal?.dispose();
  });

  return (
    <div class="h-full flex flex-col" style={{ display: local.visible ? 'flex' : 'none' }}>
      <div
        class="flex items-center justify-between px-3 py-1"
        style={{ 'border-bottom': '1px solid var(--color-border, #333)' }}
      >
        <span class="text-xs text-[var(--color-text-secondary, #999)]">终端</span>
        <div class="flex gap-2">
          <button
            type="button"
            aria-label="重新连接"
            title="重新连接"
            onClick={() => {
              disconnectPty();
              connectPty();
            }}
            class="text-xs bg-transparent border-none text-[var(--color-text-secondary, #999)] cursor-pointer hover:text-[var(--color-text-primary, #d4d4d4)]"
          >
            <IconRestore />
          </button>
          <button
            type="button"
            aria-label="关闭终端"
            title="关闭终端"
            onClick={() => {
              disconnectPty();
              local.onClose?.();
            }}
            class="text-xs bg-transparent border-none text-[var(--color-text-secondary, #999)] cursor-pointer hover:text-[var(--color-text-primary, #d4d4d4)]"
          >
            <CloseIcon />
          </button>
        </div>
      </div>
      <div ref={containerRef} class="flex-1 p-1 overflow-y-auto" style={{ background: '#1e1e1e' }} />
      <Show when={isConnecting() && !isReady()}>
        <span class="sr-only">正在连接终端</span>
      </Show>
    </div>
  );
};

export default TerminalPanel;
