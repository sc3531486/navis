import { Show } from 'solid-js';
import ShellOutputWindow from '@terminal-ext/components/ui/ShellOutputWindow';
import type { AgentTimelinePartRenderer } from '@agent-core/lib/agent-timeline';
import { GenericToolStep } from './GenericToolStep';
import {
  formatToolBytes,
  isLiveTimelinePart,
  recordBoolean,
  recordNumber,
  recordString,
  tailLines,
  terminalOutputText,
  timelineDurationLabel,
  timelineTarget,
  toolMetadata,
  toolProgress,
} from './tool-presentation';

export const TerminalToolStep: AgentTimelinePartRenderer = (props) => {
  const progress = () => toolProgress(props.part);
  const progressOutput = () =>
    recordString(progress(), 'output') ??
    tailLines(recordString(progress(), 'fullOutput') ?? terminalOutputText(props.part), 5);
  const totalLines = () =>
    recordNumber(progress(), 'totalLines') ?? recordNumber(toolMetadata(props.part), 'totalLines');
  const totalBytes = () =>
    recordNumber(progress(), 'totalBytes') ?? recordNumber(toolMetadata(props.part), 'totalBytes');
  const elapsedSeconds = () => recordNumber(progress(), 'elapsedTimeSeconds');
  const lineStatus = () => {
    const lines = totalLines();
    if (!lines || lines <= 5) return '';
    if (recordBoolean(progress(), 'fullOutputTruncated')) return `~${lines} lines`;
    return `+${Math.max(0, lines - 5)} lines`;
  };
  const progressMeta = () => {
    const elapsed = elapsedSeconds();
    const bytes = totalBytes();
    const items = [
      lineStatus(),
      elapsed == null ? '' : timelineDurationLabel(elapsed * 1_000, true),
      bytes == null ? '' : formatToolBytes(bytes),
    ].filter(Boolean);
    return items.join(' · ');
  };
  const showProgress = () =>
    isLiveTimelinePart(props.part) && Boolean(progressOutput() || progressMeta());

  return (
    <div class="navis-agent-terminal-part">
      <GenericToolStep {...props} />
      <Show when={showProgress()}>
        <ShellOutputWindow
          title="Shell"
          command={timelineTarget(props.part)}
          class="navis-agent-terminal-progress"
          isTerminal
          ariaLive="polite"
        >
          <Show when={progressOutput()}>
            {(output) => <pre class="navis-agent-terminal-progress-output">{output()}</pre>}
          </Show>
          <Show when={progressMeta()}>
            {(meta) => <div class="navis-agent-terminal-progress-meta">{meta()}</div>}
          </Show>
        </ShellOutputWindow>
      </Show>
    </div>
  );
};
