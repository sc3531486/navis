import { Component, For, Match, Show, Suspense, Switch, lazy } from 'solid-js';
import { EmptyState } from '../ui/EmptyState';
import { HostViewRenderer } from '../HostView';
import type { RightWorkspacePanel } from '../../stores/app';
import {
  WorkspaceSectionList,
  DESIGN_DOCS,
  kernelPrimitiveRows,
} from './shared';

const GitDiffPanel = lazy(() => import('./DiffPanel'));
const ToolDiffPanel = lazy(() => import('./ToolDiffPanel'));
const BackgroundTasksPanel = lazy(() => import('./BackgroundTasksPanel'));
const PlanPanel = lazy(() => import('./PlanPanel'));
const SessionTranscriptPanel = lazy(() => import('./SessionTranscriptPanel'));
const WorktreeEditor = lazy(() => import('../Editor/WorktreeEditor'));

interface BuiltinRightWorkspaceContentProps {
  panel: RightWorkspacePanel;
}

const BUILTIN_VIEW_IDS = new Set([
  'diff',
  'tool-diff',
  'background-tasks',
  'plan',
  'design',
  'session-transcript',
  'editor',
]);

/* ── Design Panel (kept inline — lightweight, no separate state) ───── */

const DesignPanel: Component = () => {
  const architectureNotes = [
    {
      title: 'Kernel boundary',
      body: 'Kernel owns four primitives only. UI, extensions, MCP, Gateway and tasks must enter through those primitives instead of creating parallel systems.',
    },
    {
      title: 'Host UI surface',
      body: 'Complete views open through rightWorkspace, chatAside, bottomDrawer or settingsSection. Menus and toolbars stay on their own contribution paths.',
    },
    {
      title: 'Fact discipline',
      body: 'Storage remains the durable fact source. Event Bus publishes change notifications after successful writes, and Stream handles high-frequency data.',
    },
  ];

  return (
    <div class="navis-workspace-design">
      <section class="navis-workspace-design-hero">
        <div>
          <div class="navis-workspace-design-title">Navis Go design system</div>
          <p>Architecture notes, kernel boundaries, and module contracts for the current Worktree.</p>
        </div>
        <div class="navis-workspace-design-count">{DESIGN_DOCS.length} docs</div>
      </section>

      <section class="navis-workspace-section">
        <div class="navis-workspace-section-title">Kernel primitives</div>
        <div class="navis-workspace-design-primitives">
          <For each={kernelPrimitiveRows}>
            {([name, body]) => (
              <div class="navis-workspace-design-primitive">
                <strong>{name}</strong>
                <span>{body}</span>
              </div>
            )}
          </For>
        </div>
      </section>

      <WorkspaceSectionList sections={architectureNotes} />

      <section class="navis-workspace-section">
        <div class="navis-workspace-section-title">Design documents</div>
        <div class="navis-workspace-design-docs">
          <For each={DESIGN_DOCS}>
            {(doc) => (
              <article class="navis-workspace-design-doc">
                <div>
                  <strong>{doc.title}</strong>
                  <span>{doc.path}</span>
                </div>
                <span>{doc.area}</span>
              </article>
            )}
          </For>
        </div>
      </section>
    </div>
  );
};

/* ── Switch/Match dispatcher ────────────────────────────────────────── */

const BuiltinPanelBody: Component<{ panel: RightWorkspacePanel }> = (props) => (
  <Switch
    fallback={
      <EmptyState
        title={props.panel.title}
        body="This panel is provided by a extension or a later business module. Navis Go has already allocated its right workspace host surface."
      />
    }
  >
    <Match when={props.panel.viewId === 'diff'}>
      <GitDiffPanel />
    </Match>

    <Match when={props.panel.viewId === 'tool-diff'}>
      <ToolDiffPanel panel={props.panel} />
    </Match>

    <Match when={props.panel.viewId === 'background-tasks'}>
      <BackgroundTasksPanel panel={props.panel} />
    </Match>

    <Match when={props.panel.viewId === 'plan'}>
      <PlanPanel />
    </Match>

    <Match when={props.panel.viewId === 'design'}>
      <DesignPanel />
    </Match>

    <Match when={props.panel.viewId === 'session-transcript'}>
      <SessionTranscriptPanel panel={props.panel} />
    </Match>

    <Match when={props.panel.viewId === 'editor'}>
      <Suspense fallback={<EmptyState title="Loading File" body="Preparing the current session file panel." />}>
        <WorktreeEditor mode="file-panel" />
      </Suspense>
    </Match>
  </Switch>
);

/* ── Default export ─────────────────────────────────────────────────── */

const BuiltinRightWorkspaceContent: Component<BuiltinRightWorkspaceContentProps> = (props) => {
  const isBuiltin = () => BUILTIN_VIEW_IDS.has(props.panel.viewId);
  const extensionView = () => props.panel.extensionView;

  return (
    <div class="navis-workspace-content">
      <Show when={!isBuiltin() && extensionView()}>
        {(view) => <HostViewRenderer view={view()} surface="rightWorkspace" />}
      </Show>

      <Show when={isBuiltin()}>
        <BuiltinPanelBody panel={props.panel} />
      </Show>

      <Show when={!isBuiltin() && !props.panel.extensionView}>
        <EmptyState
          title={props.panel.title}
          body="This right panel has no available content renderer. Close it, or install and enable a extension that provides the matching view renderer."
        />
      </Show>
    </div>
  );
};

export default BuiltinRightWorkspaceContent;
