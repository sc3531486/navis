import { Component } from 'solid-js';
import closeIconUrl from '../../assets/close.svg';

const CloseIcon: Component<{ class?: string }> = (props) => (
  <span
    class={`navis-close-icon ${props.class ?? ''}`}
    style={{ '--navis-close-icon-url': `url("${closeIconUrl}")` }}
    aria-hidden="true"
  />
);

export default CloseIcon;
