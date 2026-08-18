import { Component, Show } from 'solid-js';

import type { AgentConfirmConfig } from '@navis-code/components/Dialog';
import AgentConfirmDialog from '@agent-core/components/Dialog/AgentConfirmDialog';
import DecisionDialog from '@navis-code/components/Dialog/DecisionDialog';
import type { ToolApprovalDecision } from '@/lib/stream';
import type { PendingPlanReview } from '@agent-core/stores/composer-run';

interface ComposerApprovalPanelProps {
  approvalConfig: AgentConfirmConfig | null;
  pendingPlanReview: PendingPlanReview | null;
  planReviewInput: string;
  isRespondingApproval: boolean;
  isStartingPlanExecution: boolean;
  multiAgentEnabled: boolean;
  onToolApproval: (decision: ToolApprovalDecision) => void;
  onCancelPlanReview: () => void;
  onStartPlanExecution: () => void;
  onPlanReviewInput: (value: string) => void;
}

const ComposerApprovalPanel: Component<ComposerApprovalPanelProps> = (props) => (
  <Show
    when={props.approvalConfig}
    fallback={
      <div class={`navis-composer-approval ${props.isStartingPlanExecution ? 'is-busy' : ''}`}>
        <DecisionDialog
          message="Review the plan before execution."
          details={[
            { key: 'Request', value: props.pendingPlanReview?.requestText ?? '' },
            { key: 'Mode', value: props.multiAgentEnabled ? 'Plan + multi-agent' : 'Plan' },
          ]}
          notice={{
            title: 'Plan gate',
            message: 'Start execution to let Navis Go run the approved phases. Add optional notes if the plan needs adjustment.',
            tone: 'info',
          }}
          actions={[
            { label: 'Cancel', variant: 'secondary', onClick: props.onCancelPlanReview },
            {
              label: 'Start execution',
              variant: 'primary',
              autofocus: true,
              disabled: props.isStartingPlanExecution,
              onClick: props.onStartPlanExecution,
            },
          ]}
        >
          <textarea
            class="navis-dialog-input navis-plan-review-input"
            rows={3}
            placeholder="Optional custom instruction or clarification"
            value={props.planReviewInput}
            onInput={(event) => props.onPlanReviewInput(event.currentTarget.value)}
          />
        </DecisionDialog>
      </div>
    }
  >
    {(config) => (
      <div class={`navis-composer-approval ${props.isRespondingApproval ? 'is-busy' : ''}`}>
        <AgentConfirmDialog
          config={config()}
          onApprove={() => props.onToolApproval('allow_once')}
          onDenyAlways={() => props.onToolApproval('deny_always')}
          onTrustThisSession={() => props.onToolApproval('allow_session')}
          onAllowProject={() => props.onToolApproval('allow_project')}
        />
      </div>
    )}
  </Show>
);

export default ComposerApprovalPanel;
