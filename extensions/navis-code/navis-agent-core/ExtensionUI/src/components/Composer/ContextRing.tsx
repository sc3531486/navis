import { Show, createEffect, createResource, createSignal, onCleanup, type Component, type JSX } from 'solid-js';

import { chatMessageState } from '@session/stores/chat-messages';
import { contextUsageDisplaySnapshot, loadSessionContextUsage } from '@session/stores/context-usage';
import { activeSessionId } from '@session/stores/session-tree';

const ContextRing: Component = () => {
  const [tooltipOpen, setTooltipOpen] = createSignal(false);
  const [tooltipStyle, setTooltipStyle] = createSignal<JSX.CSSProperties>({});
  let ringRef: HTMLDivElement | undefined;
  let tooltipRef: HTMLDivElement | undefined;
  const [contextUsage] = createResource(
    () => {
      const sessionId = activeSessionId();
      if (!sessionId) return null;
      return {
        sessionId,
        revision: `${chatMessageState.total}:${chatMessageState.messages.length}:${chatMessageState.loading}`,
      };
    },
    ({ sessionId }) => loadSessionContextUsage(sessionId),
  );
  const usageSnapshot = () => contextUsageDisplaySnapshot(contextUsage());
  const usedPercent = () => usageSnapshot()?.usedPercent ?? 0;
  const thresholdPercent = () => usageSnapshot()?.compressionThresholdPercent ?? null;
  const strokeOffset = () => 53.4 * (1 - Math.min(usedPercent(), 100) / 100);

  const updateTooltipPosition = () => {
    if (!ringRef || !tooltipOpen()) return;

    const anchorRect = ringRef.getBoundingClientRect();
    const tooltipWidth = tooltipRef?.offsetWidth ?? 236;
    const tooltipHeight = tooltipRef?.offsetHeight ?? 58;
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const gap = 10;
    const margin = 12;

    let left = anchorRect.left + anchorRect.width / 2 - tooltipWidth / 2;
    left = Math.max(margin, Math.min(left, viewportWidth - tooltipWidth - margin));

    let top = anchorRect.top - tooltipHeight - gap;
    if (top < margin) {
      top = Math.min(anchorRect.bottom + gap, viewportHeight - tooltipHeight - margin);
    }

    setTooltipStyle({
      left: `${Math.round(left)}px`,
      top: `${Math.round(top)}px`,
    });
  };

  createEffect(() => {
    if (!tooltipOpen()) return;

    const frame = requestAnimationFrame(updateTooltipPosition);
    const handleViewportChange = () => updateTooltipPosition();

    window.addEventListener('resize', handleViewportChange);
    window.addEventListener('scroll', handleViewportChange, true);

    onCleanup(() => {
      cancelAnimationFrame(frame);
      window.removeEventListener('resize', handleViewportChange);
      window.removeEventListener('scroll', handleViewportChange, true);
    });
  });

  return (
    <div
      ref={ringRef}
      class="navis-context-ring relative h-4 w-4"
      role="img"
      aria-label={
        usageSnapshot()
          ? `Context usage ${usageSnapshot()!.usedPercent}%, used ${usageSnapshot()!.usedTokensLabel} / ${usageSnapshot()!.totalTokensLabel}, compression threshold ${usageSnapshot()!.compressionThresholdPercent}%`
          : 'Context usage unavailable'
      }
      onMouseEnter={() => setTooltipOpen(true)}
      onMouseLeave={() => setTooltipOpen(false)}
    >
      <svg width="16" height="16" viewBox="0 0 22 22" class="-rotate-90">
        <circle cx="11" cy="11" r="8.5" fill="none" stroke="#e0e0e0" stroke-width="3" />
        <circle
          cx="11"
          cy="11"
          r="8.5"
          fill="none"
          stroke="#6aa36f"
          stroke-width="3"
          stroke-dasharray="53.4"
          stroke-dashoffset={strokeOffset()}
          stroke-linecap="round"
        />
      </svg>
      <Show when={tooltipOpen()}>
        <div ref={tooltipRef} class="navis-context-tooltip" style={tooltipStyle()}>
          <Show
            when={usageSnapshot()}
            fallback={<div class="navis-context-tooltip-empty">Loading context...</div>}
          >
            {(snapshot) => (
              <>
                <div class="navis-context-tooltip-row is-summary">
                  <span>Used context</span>
                  <span>{snapshot().usedPercent}%</span>
                </div>
                <div class="navis-context-tooltip-meter">
                  <span style={{ width: `${snapshot().usedPercent}%` }} />
                  <Show when={thresholdPercent() !== null}>
                    <i style={{ left: `${thresholdPercent()}%` }} />
                  </Show>
                </div>
                <div class="navis-context-tooltip-row is-detail">
                  <span>{snapshot().usedRatioLabel}</span>
                  <span>{snapshot().compressionThresholdLabel}</span>
                </div>
              </>
            )}
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default ContextRing;
