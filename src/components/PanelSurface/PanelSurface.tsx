import { Component, JSX } from 'solid-js';

type PanelSurfaceAs = 'div' | 'button' | 'section';

interface PanelSurfaceProps {
  as?: PanelSurfaceAs;
  class?: string;
  children: JSX.Element;
  [key: string]: unknown;
}

const PanelSurface: Component<PanelSurfaceProps> = (props) => {
  const className = () => `navis-panel-surface ${props.class ?? ''}`.trim();
  const rest = () => {
    const { as: _as, class: _class, children: _children, ...attrs } = props;
    return attrs;
  };

  if (props.as === 'button') {
    return (
      <button class={className()} {...rest()}>
        {props.children}
      </button>
    );
  }

  if (props.as === 'section') {
    return (
      <section class={className()} {...rest()}>
        {props.children}
      </section>
    );
  }

  return (
    <div class={className()} {...rest()}>
      {props.children}
    </div>
  );
};

export default PanelSurface;


