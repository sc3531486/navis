import { Component } from 'solid-js';
import { EmptyState } from '@/components/ui/EmptyState';
import UnifiedDiffViewer from '@editor-ext/components/ui/UnifiedDiffViewer';
import type { RightWorkspacePanel } from '@/stores/host';

interface ToolDiffPanelConfig {
  title?: string;
  diff?: string;
}

const toolDiffPanelConfig = (config: unknown): ToolDiffPanelConfig => {
  if (!config || typeof config !== 'object') return {};
  const record = config as Record<string, unknown>;
  return {
    title: typeof record.title === 'string' ? record.title : undefined,
    diff: typeof record.diff === 'string' ? record.diff : undefined,
  };
};

const ToolDiffPanel: Component<{ panel: RightWorkspacePanel }> = (props) => {
  const config = () => toolDiffPanelConfig(props.panel.config);
  const diff = () => config().diff?.trim() ?? '';

  return (
    <div class="navis-workspace-tool-diff">
      {diff()
        ? <UnifiedDiffViewer diff={diff()} class="navis-workspace-diff-code" ariaLabel={config().title ?? 'Tool diff'} />
        : <EmptyState title="No diff selected" body="Select an edit step with a diff to inspect the changes." />}
    </div>
  );
};

export default ToolDiffPanel;
