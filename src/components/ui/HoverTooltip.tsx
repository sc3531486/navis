import { Component, JSX, Show, createSignal } from 'solid-js';

interface HoverTooltipProps {
  label: string;
  detail?: string;
  children: JSX.Element;
}

const HoverTooltip: Component<HoverTooltipProps> = (props) => {
  const [open, setOpen] = createSignal(false);

  return (
    <span
      class="navis-hover-tooltip-anchor"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocusIn={() => setOpen(true)}
      onFocusOut={() => setOpen(false)}
    >
      {props.children}
      <Show when={open()}>
        <span class="navis-hover-tooltip" role="tooltip">
          <span class="navis-hover-tooltip-label">{props.label}</span>
          <Show when={props.detail}>
            {(detail) => <span class="navis-hover-tooltip-detail">{detail()}</span>}
          </Show>
        </span>
      </Show>
    </span>
  );
};

export default HoverTooltip;
