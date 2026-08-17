import { Component, Show } from 'solid-js';
import type { ChatAgentTimelinePart } from '../../lib/stream';
import { agentTimelinePartLabel, toolPath } from './tool-presentation';
import { AgentTimelinePartLabel } from './TimelineToolLabel';

export const TimelineToolTarget: Component<{
  part: ChatAgentTimelinePart;
  canOpenFile: boolean;
  onOpenPanel: () => boolean;
}> = (props) => {
  const path = () => toolPath(props.part);
  const title = () => (props.canOpenFile ? path() : undefined);
  const label = () => agentTimelinePartLabel(props.part);

  return (
    <Show
      when={props.canOpenFile}
      fallback={
        <span class="navis-agent-trace-label-text" title={label()} data-full-label={label()}>
          <AgentTimelinePartLabel part={props.part} />
        </span>
      }
    >
      <span
        role="button"
        tabIndex={0}
        class="navis-agent-file-target"
        title={title() ?? label()}
        data-full-label={label()}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          props.onOpenPanel();
        }}
        onKeyDown={(event) => {
          if (event.key !== 'Enter' && event.key !== ' ') return;
          event.preventDefault();
          event.stopPropagation();
          props.onOpenPanel();
        }}
      >
        <AgentTimelinePartLabel part={props.part} />
      </span>
    </Show>
  );
};
