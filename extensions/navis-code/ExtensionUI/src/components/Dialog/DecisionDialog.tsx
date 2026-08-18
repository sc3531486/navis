import { Component, For, Show } from 'solid-js';
import type { JSX } from 'solid-js';

export interface DecisionDialogDetail {
  key: string;
  value: string;
}

export interface DecisionDialogNotice {
  title: string;
  message: string;
  tone?: 'low' | 'medium' | 'high' | 'info';
}

export interface DecisionDialogAction {
  label: string;
  variant?: 'primary' | 'secondary' | 'danger';
  autofocus?: boolean;
  disabled?: boolean;
  onClick: () => void;
}

interface DecisionDialogProps {
  message: string;
  details?: DecisionDialogDetail[];
  notice?: DecisionDialogNotice;
  actions: DecisionDialogAction[];
  children?: JSX.Element;
}

const toneClass = (tone?: DecisionDialogNotice['tone']): string => {
  switch (tone) {
    case 'low':
      return 'is-low';
    case 'high':
      return 'is-high';
    case 'medium':
    case 'info':
    default:
      return 'is-medium';
  }
};

const buttonClass = (variant?: DecisionDialogAction['variant']): string => {
  switch (variant) {
    case 'primary':
      return 'is-primary';
    case 'danger':
      return 'is-danger';
    case 'secondary':
    default:
      return 'is-secondary';
  }
};

const DecisionDialog: Component<DecisionDialogProps> = (props) => (
  <div class="navis-dialog-body">
    <p class="navis-dialog-message">{props.message}</p>

    <Show when={(props.details?.length ?? 0) > 0}>
      <div class="navis-dialog-code-block">
        <For each={props.details}>
          {(detail) => (
            <div class="navis-dialog-code-row">
              <span class="navis-dialog-code-key">{detail.key}</span>
              <span class="navis-dialog-code-value">{detail.value}</span>
            </div>
          )}
        </For>
      </div>
    </Show>

    <Show when={props.notice}>
      {(notice) => (
        <div class={`navis-dialog-risk ${toneClass(notice().tone)}`}>
          <div class="navis-dialog-risk-title">{notice().title}</div>
          <p class="navis-dialog-risk-message">{notice().message}</p>
        </div>
      )}
    </Show>

    {props.children}

    <div class="navis-dialog-actions">
      <For each={props.actions}>
        {(action) => (
          <button
            type="button"
            onClick={action.onClick}
            autofocus={action.autofocus}
            disabled={action.disabled}
            class={`navis-dialog-button ${buttonClass(action.variant)}`}
          >
            {action.label}
          </button>
        )}
      </For>
    </div>
  </div>
);

export default DecisionDialog;
