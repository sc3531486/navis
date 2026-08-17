import { Show } from 'solid-js';
import type { Component, JSX } from 'solid-js';
import type { StatusPresentation } from '../../lib/status';
import { statusClass } from '../../lib/status';

export interface ShellOutputWindowProps {
  title?: string;
  command?: string;
  status?: string;
  statusPresentation?: StatusPresentation;
  class?: string;
  bodyClass?: string;
  isTerminal?: boolean;
  ariaLive?: 'off' | 'polite' | 'assertive';
  copyLabel?: string;
  onCopy?: (event: MouseEvent) => void;
  children: JSX.Element;
}

const ShellOutputWindow: Component<ShellOutputWindowProps> = (props) => (
  <section
    class={`navis-shell-window ${props.isTerminal ? 'is-terminal' : ''} ${props.class ?? ''}`.trim()}
    aria-live={props.ariaLive}
  >
    <div class="navis-shell-window-header">
      <span class="navis-shell-window-title">{props.title ?? 'Shell'}</span>
      <Show when={props.copyLabel && props.onCopy}>
        <button
          type="button"
          class="navis-shell-window-copy"
          onClick={(event) => props.onCopy?.(event)}
        >
          {props.copyLabel}
        </button>
      </Show>
    </div>
    <Show when={props.command}>
      {(command) => (
        <div class="navis-shell-window-command">
          <span class="navis-shell-window-prompt">$</span>
          <code>{command()}</code>
        </div>
      )}
    </Show>
    <div class={`navis-shell-window-body ${props.bodyClass ?? ''}`.trim()}>
      {props.children}
    </div>
    <Show when={props.status}>
      {(status) => (
        <div class={`navis-shell-window-status ${statusClass(props.statusPresentation)}`}>
          {status()}
        </div>
      )}
    </Show>
  </section>
);

export default ShellOutputWindow;
