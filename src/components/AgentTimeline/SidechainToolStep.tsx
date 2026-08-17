import { Show } from 'solid-js';
import type { AgentTimelinePartRenderer } from '../../lib/agent-timeline';
import { GenericToolStep } from './GenericToolStep';
import {
  recordNumber,
  recordString,
  toolMetadata,
  toolOutput,
  toolProgress,
} from './tool-presentation';

export const SidechainToolStep: AgentTimelinePartRenderer = (props) => {
  const metadata = () => toolMetadata(props.part);
  const progress = () => toolProgress(props.part);
  const output = () => toolOutput(props.part);
  const sidechainSessionId = () =>
    recordString(metadata(), 'sidechainSessionId') ??
    recordString(output(), 'sidechainSessionId') ??
    recordString(progress(), 'sidechainSessionId');
  const agentName = () =>
    recordString(metadata(), 'agentName') ?? recordString(output(), 'agentName') ?? recordString(progress(), 'agentName');
  const toolUses = () =>
    recordNumber(progress(), 'toolUses') ?? recordNumber(metadata(), 'toolUses') ?? recordNumber(output(), 'toolUses');
  const totalTokens = () =>
    recordNumber(progress(), 'totalTokens') ??
    recordNumber(metadata(), 'totalTokens') ??
    recordNumber(output(), 'totalTokens');
  const activity = () =>
    recordString(progress(), 'activity') ?? recordString(progress(), 'description') ?? recordString(output(), 'summary');
  const meta = () => {
    const items = [
      agentName(),
      sidechainSessionId() ? `session ${sidechainSessionId()}` : '',
      toolUses() == null ? '' : `${toolUses()} tools`,
      totalTokens() == null ? '' : `${totalTokens()} tokens`,
      activity(),
    ].filter(Boolean);
    return items.join(' · ');
  };

  return (
    <div class="navis-agent-sidechain-part">
      <GenericToolStep {...props} />
      <Show when={meta()}>
        {(value) => <div class="navis-agent-sidechain-meta">{value()}</div>}
      </Show>
    </div>
  );
};
