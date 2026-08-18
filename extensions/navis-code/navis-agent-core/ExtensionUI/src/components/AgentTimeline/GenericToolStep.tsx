import { Show, createEffect, createSignal, onCleanup, onMount } from 'solid-js';
import ShellOutputWindow from '@terminal-ext/components/ui/ShellOutputWindow';
import UnifiedDiffViewer from '@editor-ext/components/ui/UnifiedDiffViewer';
import { openBackgroundTasksPanel } from '@session/components/Chat/panel-actions';
import type { AgentTimelinePartRenderer } from '@agent-core/lib/agent-timeline';
import type { ChatAgentTimelinePart } from '@/lib/stream';
import { statusClass } from '@/lib/status';
import { openTimelineDiffPanel } from './timeline-panel-actions';
import { currentWorktreeRelativePath, openToolPathInFilePanel } from './timeline-file-target';
import { TimelineToolTarget } from './TimelineToolTarget';
import { AgentTraceGlyph } from './TraceIcon';
import {
  agentTimelinePartLabel,
  isExceptionalTimelineStatus,
  isLiveTimelinePart,
  recordString,
  structuredToolDetail,
  terminalDetailStatusLabel,
  terminalOutputText,
  timelineDurationLabel,
  timelineDurationMs,
  timelineStatusLabel,
  timelineTarget,
  timelineToolKind,
  toolHasUsefulPreview,
  toolMetadata,
  toolOutput,
  toolPath,
  toolProgress,
  toolResultSummary,
} from './tool-presentation';

const partDetailText = (part: ChatAgentTimelinePart): string => {
  const structuredDetail = structuredToolDetail(part);
  if (structuredDetail) return structuredDetail;
  if (timelineToolKind(part) === 'edit') {
    const diff = recordString(toolMetadata(part), 'diff') ?? part.detail?.trim() ?? '';
    if (diff) return diff;
  }
  if (timelineToolKind(part) === 'terminal') {
    const output = terminalOutputText(part);
    const detail = part.detail?.trim() ?? '';
    if (detail) return detail;
    if (output) return output;
  }
  const detail = part.detail?.trim() ?? '';
  const label = agentTimelinePartLabel(part).trim();
  if (detail && detail !== label) return detail;
  if (isExceptionalTimelineStatus(part.status)) {
    const summary = part.summary?.trim();
    const status = timelineStatusLabel(part);
    return [status, summary && summary !== label ? summary : '', `Target: ${timelineTarget(part)}`]
      .filter(Boolean)
      .join('\n');
  }
  return '';
};

const isDiffDetail = (part: ChatAgentTimelinePart): boolean =>
  timelineToolKind(part) === 'edit' && Boolean(partDetailText(part));

const isTerminalDetail = (part: ChatAgentTimelinePart): boolean =>
  timelineToolKind(part) === 'terminal' && Boolean(partDetailText(part));

const isErrorDetail = (part: ChatAgentTimelinePart): boolean =>
  part.statusPresentation.outcome === 'failed' && Boolean(partDetailText(part));

export const GenericToolStep: AgentTimelinePartRenderer = (props) => {
  let labelRef: HTMLSpanElement | undefined;
  const [localExpanded, setLocalExpanded] = createSignal(false);
  const [labelOverflows, setLabelOverflows] = createSignal(false);
  const [copyState, setCopyState] = createSignal<'idle' | 'copied' | 'failed'>('idle');
  const expanded = () => props.expanded ?? localExpanded();
  const setExpanded = (value: boolean | ((current: boolean) => boolean)) => {
    const next = typeof value === 'function' ? value(expanded()) : value;
    if (props.onExpandedChange) {
      props.onExpandedChange(next);
    } else {
      setLocalExpanded(next);
    }
  };
  const detail = () => partDetailText(props.part);
  const hasPreviewDetail = () => toolHasUsefulPreview(props.part, detail());
  const durationLabel = () => {
    const durationMs = timelineDurationMs(props.part, props.nowMs);
    return durationMs == null ? '' : timelineDurationLabel(durationMs, isLiveTimelinePart(props.part));
  };
  const resultSummary = () => (
    isLiveTimelinePart(props.part) || props.part.statusPresentation?.terminal
      ? toolResultSummary(props.part)
      : ''
  );
  const canOpenFile = () => {
    const kind = timelineToolKind(props.part);
    return (kind === 'read' || kind === 'edit' || kind === 'inspect') && Boolean(currentWorktreeRelativePath(toolPath(props.part)));
  };
  const canExpand = () =>
    Boolean(detail() && (
      isDiffDetail(props.part) ||
      hasPreviewDetail() ||
      isErrorDetail(props.part) ||
      timelineToolKind(props.part) !== 'edit' ||
      labelOverflows()
    ));
  const taskSelection = () => {
    const metadata = toolMetadata(props.part);
    const output = toolOutput(props.part);
    const progress = toolProgress(props.part);
    const selectedTaskId =
      recordString(metadata, 'taskId') ??
      recordString(output, 'taskId') ??
      recordString(progress, 'taskId');
    const selectedSidechainSessionId =
      recordString(metadata, 'sidechainSessionId') ??
      recordString(output, 'sidechainSessionId') ??
      recordString(progress, 'sidechainSessionId');
    return { selectedTaskId, selectedSidechainSessionId };
  };
  const canOpenBackgroundTask = () => {
    const selection = taskSelection();
    return timelineToolKind(props.part) === 'sidechain' && Boolean(selection.selectedTaskId || selection.selectedSidechainSessionId);
  };
  const copyDetail = async () => {
    try {
      await navigator.clipboard.writeText(detail());
      setCopyState('copied');
    } catch {
      setCopyState('failed');
    }
    window.setTimeout(() => setCopyState('idle'), 1_400);
  };
  const openPrimaryPanel = (): boolean => {
    if (timelineToolKind(props.part) === 'edit' && isDiffDetail(props.part)) {
      return openTimelineDiffPanel(agentTimelinePartLabel(props.part), detail());
    }
    return openToolPathInFilePanel(props.part);
  };
  const updateOverflow = () => {
    const label = labelRef;
    if (!label) return;
    setLabelOverflows(label.scrollWidth > label.clientWidth + 1);
  };

  createEffect(() => {
    agentTimelinePartLabel(props.part);
    detail();
    queueMicrotask(updateOverflow);
  });

  onMount(() => {
    updateOverflow();
    const resizeObserver = new ResizeObserver(updateOverflow);
    if (labelRef) resizeObserver.observe(labelRef);
    window.addEventListener('resize', updateOverflow);
    onCleanup(() => {
      resizeObserver.disconnect();
      window.removeEventListener('resize', updateOverflow);
    });
  });

  return (
    <div class={`navis-agent-trace-row-wrap ${expanded() && canExpand() ? 'is-expanded' : ''}`}>
      <div
        role="button"
        tabIndex={0}
        class={`navis-agent-trace-row ${statusClass(props.part.statusPresentation)} ${canExpand() ? 'is-expandable' : ''}`}
        title={agentTimelinePartLabel(props.part)}
        aria-expanded={canExpand() ? expanded() : undefined}
        onClick={(event) => {
          if (canOpenBackgroundTask()) {
            event.preventDefault();
            event.stopPropagation();
            openBackgroundTasksPanel(taskSelection());
            return;
          }
          if (canExpand()) setExpanded((value) => !value);
        }}
        onKeyDown={(event) => {
          if (event.key !== 'Enter' && event.key !== ' ') return;
          event.preventDefault();
          if (canExpand()) setExpanded((value) => !value);
        }}
      >
        <AgentTraceGlyph kind={timelineToolKind(props.part)} />
        <span
          ref={labelRef}
          class="navis-agent-trace-label"
        >
          <TimelineToolTarget
            part={props.part}
            canOpenFile={canOpenFile()}
            onOpenPanel={openPrimaryPanel}
          />
        </span>
        <span class={`navis-agent-trace-meta ${resultSummary() ? '' : 'is-empty'}`}>
          {resultSummary()}
        </span>
        <span
          class={`navis-agent-trace-more ${expanded() ? 'is-open' : ''} ${canExpand() ? '' : 'is-hidden'}`}
          aria-label={expanded() ? 'Collapse details' : 'Expand details'}
          aria-hidden="true"
        />
        <span class={`navis-agent-trace-duration ${durationLabel() ? '' : 'is-empty'}`}>
          {durationLabel()}
        </span>
      </div>
      <Show when={expanded() && canExpand()}>
        <Show
          when={isDiffDetail(props.part)}
          fallback={
            <ShellOutputWindow
              title={timelineToolKind(props.part) === 'terminal' ? 'Shell' : 'Output'}
              command={timelineToolKind(props.part) === 'terminal' ? timelineTarget(props.part) : undefined}
              class="navis-agent-trace-detail-surface"
              isTerminal={isTerminalDetail(props.part)}
              copyLabel={
                isTerminalDetail(props.part)
                  ? copyState() === 'copied'
                    ? 'Copied'
                    : copyState() === 'failed'
                      ? 'Copy failed'
                      : 'Copy output'
                  : undefined
              }
              onCopy={
                isTerminalDetail(props.part)
                  ? (event) => {
                    event.stopPropagation();
                    void copyDetail();
                  }
                  : undefined
              }
              statusPresentation={props.part.statusPresentation}
              status={isTerminalDetail(props.part) ? terminalDetailStatusLabel(props.part) : ''}
            >
              <pre class={`navis-agent-trace-detail-content ${isTerminalDetail(props.part) ? 'is-terminal' : ''}`}>{detail()}</pre>
            </ShellOutputWindow>
          }
        >
          <UnifiedDiffViewer diff={detail()} class="navis-agent-trace-detail is-diff" ariaLabel="File diff" />
        </Show>
      </Show>
    </div>
  );
};
