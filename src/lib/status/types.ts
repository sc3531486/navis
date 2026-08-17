export type StatusPhase = 'queued' | 'active' | 'waiting' | 'inactive' | 'unknown';

export type StatusOutcome =
  | 'succeeded'
  | 'failed'
  | 'cancelled'
  | 'skipped'
  | 'unknown';

export type StatusAttention = 'normal' | 'needs_action' | 'warning';

export interface StatusPresentation {
  phase: StatusPhase;
  outcome: StatusOutcome | null;
  attention: StatusAttention;
  live: boolean;
  terminal: boolean;
}

export const unknownStatusPresentation: StatusPresentation = {
  phase: 'unknown',
  outcome: 'unknown',
  attention: 'warning',
  live: false,
  terminal: true,
};

export const failedStatusPresentation: StatusPresentation = {
  phase: 'inactive',
  outcome: 'failed',
  attention: 'warning',
  live: false,
  terminal: true,
};

export const cancelledStatusPresentation: StatusPresentation = {
  phase: 'inactive',
  outcome: 'cancelled',
  attention: 'normal',
  live: false,
  terminal: true,
};
