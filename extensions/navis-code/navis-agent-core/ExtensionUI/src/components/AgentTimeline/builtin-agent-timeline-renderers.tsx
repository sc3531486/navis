import type { AgentTimelinePartRenderer } from '@agent-core/lib/agent-timeline';
import { registerBuiltinToolRenderers } from './builtin-tool-renderers';
import { GenericToolStep } from './GenericToolStep';
import { SidechainToolStep } from './SidechainToolStep';
import { TerminalToolStep } from './TerminalToolStep';

const ReadToolStep: AgentTimelinePartRenderer = (props) => <GenericToolStep {...props} />;
const ListToolStep: AgentTimelinePartRenderer = (props) => <GenericToolStep {...props} />;
const SearchToolStep: AgentTimelinePartRenderer = (props) => <GenericToolStep {...props} />;
const InspectToolStep: AgentTimelinePartRenderer = (props) => <GenericToolStep {...props} />;
const EditToolStep: AgentTimelinePartRenderer = (props) => <GenericToolStep {...props} />;

let builtinAgentTimelineRenderersRegistered = false;

export function registerBuiltinAgentTimelineRenderers(): void {
  if (builtinAgentTimelineRenderersRegistered) return;
  builtinAgentTimelineRenderersRegistered = true;
  registerBuiltinToolRenderers('navis', {
    generic: GenericToolStep,
    read: ReadToolStep,
    list: ListToolStep,
    search: SearchToolStep,
    inspect: InspectToolStep,
    edit: EditToolStep,
    terminal: TerminalToolStep,
    sidechain: SidechainToolStep,
  });
}
