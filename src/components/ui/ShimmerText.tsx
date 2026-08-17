import type { Component, JSX } from 'solid-js';

export interface ShimmerTextProps {
  active?: boolean;
  baseColor?: string;
  class?: string;
  durationMs?: number;
  peakColor?: string;
  children: JSX.Element;
}

const ShimmerText: Component<ShimmerTextProps> = (props) => {
  const style = (): JSX.CSSProperties => ({
    '--navis-shimmer-base': props.baseColor,
    '--navis-shimmer-duration': props.durationMs ? `${props.durationMs}ms` : undefined,
    '--navis-shimmer-peak': props.peakColor,
  } as JSX.CSSProperties);

  return (
    <span
      class={`${props.active ? 'navis-shimmer-text' : ''} ${props.class ?? ''}`.trim()}
      style={style()}
    >
      {props.children}
    </span>
  );
};

export default ShimmerText;
