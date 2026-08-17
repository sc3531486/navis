import type { StatusPresentation, StatusPhase } from './types';
import { unknownStatusPresentation } from './types';

export type PresentedStatus = { statusPresentation: StatusPresentation };
export type StatusSemantic =
  | 'queued'
  | 'active'
  | 'waiting'
  | 'succeeded'
  | 'failed'
  | 'cancelled'
  | 'skipped'
  | 'inactive'
  | 'unknown';

export const statusPhase = (presentation: StatusPresentation): StatusPhase => presentation.phase;

export const isStatusLive = (presentation: StatusPresentation | null | undefined): boolean =>
  presentation?.live === true;

export const isStatusTerminal = (presentation: StatusPresentation | null | undefined): boolean =>
  presentation?.terminal === true;

export const statusSemantic = (presentation: StatusPresentation | null | undefined): StatusSemantic => {
  if (!presentation) return 'unknown';
  if (presentation.outcome && presentation.outcome !== 'unknown') return presentation.outcome;
  return presentation.phase;
};

export const statusClass = (presentation: StatusPresentation | null | undefined): string =>
  `is-status-${statusSemantic(presentation)}`;

export const statusPresentationFor = (
  value: PresentedStatus | null | undefined,
): StatusPresentation => value?.statusPresentation ?? unknownStatusPresentation;

export const statusLabel = (presentation: StatusPresentation | null | undefined): string => {
  switch (statusSemantic(presentation)) {
    case 'queued': return 'Queued';
    case 'active': return 'Running';
    case 'waiting': return 'Waiting';
    case 'succeeded': return 'Completed';
    case 'failed': return 'Failed';
    case 'cancelled': return 'Cancelled';
    case 'skipped': return 'Skipped';
    case 'inactive': return 'Inactive';
    default: return 'Unknown';
  }
};

export const statusOutcomeLabel = (presentation: StatusPresentation): string => {
  switch (presentation.outcome) {
    case 'succeeded': return 'Completed';
    case 'failed': return 'Failed';
    case 'cancelled': return 'Cancelled';
    case 'skipped': return 'Skipped';
    case 'unknown': return 'Unknown';
    default: return '';
  }
};