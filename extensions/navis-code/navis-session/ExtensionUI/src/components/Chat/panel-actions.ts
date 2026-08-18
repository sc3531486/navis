import { openRightWorkspacePanel } from '@/stores/host';
import { activeSessionId } from '@session/stores/session-tree';

export interface BackgroundTasksPanelSelection {
  selectedTaskId?: string | null;
  selectedSidechainSessionId?: string | null;
}

export const openBackgroundTasksPanel = (selection?: BackgroundTasksPanelSelection): void => {
  openRightWorkspacePanel({
    id: 'background-tasks',
    title: 'Background tasks',
    viewId: 'background-tasks',
    sessionId: activeSessionId() ?? undefined,
    config: selection,
  });
};

export const openPlanPanel = (): void => {
  openRightWorkspacePanel({
    id: 'plan',
    title: 'Plan',
    viewId: 'plan',
    sessionId: activeSessionId() ?? undefined,
  });
};
