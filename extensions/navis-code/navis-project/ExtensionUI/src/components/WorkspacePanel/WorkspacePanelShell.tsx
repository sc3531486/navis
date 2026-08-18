import { Component, JSX } from 'solid-js';
import WorkspacePanelFrame from './WorkspacePanelFrame';

interface WorkspacePanelShellProps {
  title: string;
  active?: boolean;
  closeLabel?: string;
  closable?: boolean;
  onFocus?: () => void;
  onClose?: (event: MouseEvent) => void;
  children: JSX.Element;
}

const WorkspacePanelShell: Component<WorkspacePanelShellProps> = (props) => (
  <WorkspacePanelFrame
    title={props.title}
    active={props.active}
    closeLabel={props.closeLabel}
    closable={props.closable}
    onFocus={props.onFocus}
    onClose={props.onClose}
  >
    {props.children}
  </WorkspacePanelFrame>
);

export default WorkspacePanelShell;


