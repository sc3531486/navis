import {
  registerToolRenderer,
  type AgentTimelinePartRenderer,
  type ToolRendererMatch,
} from '@/lib/agent-timeline';

export const BUILTIN_TOOL_DISPLAY_KINDS = [
  'read',
  'list',
  'glob',
  'grep',
  'search',
  'inspect',
  'edit',
  'write-as-edit',
  'bash',
  'git',
  'lsp',
  'todo',
  'task',
  'task_output',
  'task_stop',
  'skill',
  'webfetch',
  'websearch',
  'mcp_resource',
  'browser',
  'permission',
  'error',
] as const;

export type BuiltinToolDisplayKind = typeof BUILTIN_TOOL_DISPLAY_KINDS[number];

export type BuiltinToolRendererKey =
  | 'generic'
  | 'read'
  | 'list'
  | 'search'
  | 'inspect'
  | 'edit'
  | 'terminal'
  | 'sidechain';

export interface BuiltinToolRendererSpec {
  displayKind: BuiltinToolDisplayKind | 'terminal' | 'sidechain' | 'other';
  id: string;
  priority: number;
  rendererKey: BuiltinToolRendererKey;
  match?: Omit<ToolRendererMatch, 'displayKind'>;
}

export type BuiltinToolRendererMap = Record<BuiltinToolRendererKey, AgentTimelinePartRenderer>;

export const BUILTIN_TOOL_RENDERER_SPECS: readonly BuiltinToolRendererSpec[] = [
  { id: 'navis.toolRenderer.read', displayKind: 'read', priority: 10, rendererKey: 'read' },
  { id: 'navis.toolRenderer.list', displayKind: 'list', priority: 10, rendererKey: 'list' },
  { id: 'navis.toolRenderer.glob', displayKind: 'glob', priority: 10, rendererKey: 'list' },
  { id: 'navis.toolRenderer.grep', displayKind: 'grep', priority: 10, rendererKey: 'search' },
  { id: 'navis.toolRenderer.search', displayKind: 'search', priority: 10, rendererKey: 'search' },
  { id: 'navis.toolRenderer.inspect', displayKind: 'inspect', priority: 10, rendererKey: 'inspect' },
  { id: 'navis.toolRenderer.edit', displayKind: 'edit', priority: 10, rendererKey: 'edit' },
  { id: 'navis.toolRenderer.writeAsEdit', displayKind: 'write-as-edit', priority: 10, rendererKey: 'edit' },
  { id: 'navis.toolRenderer.bash', displayKind: 'bash', priority: 10, rendererKey: 'terminal' },
  { id: 'navis.toolRenderer.terminal', displayKind: 'terminal', priority: 10, rendererKey: 'terminal' },
  { id: 'navis.toolRenderer.git', displayKind: 'git', priority: 10, rendererKey: 'terminal' },
  { id: 'navis.toolRenderer.lsp', displayKind: 'lsp', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.todo', displayKind: 'todo', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.task', displayKind: 'task', priority: 10, rendererKey: 'sidechain' },
  { id: 'navis.toolRenderer.taskOutput', displayKind: 'task_output', priority: 10, rendererKey: 'sidechain' },
  { id: 'navis.toolRenderer.taskStop', displayKind: 'task_stop', priority: 10, rendererKey: 'sidechain' },
  { id: 'navis.toolRenderer.skill', displayKind: 'skill', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.webfetch', displayKind: 'webfetch', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.websearch', displayKind: 'websearch', priority: 10, rendererKey: 'search' },
  { id: 'navis.toolRenderer.mcpResource', displayKind: 'mcp_resource', priority: 10, rendererKey: 'inspect' },
  { id: 'navis.toolRenderer.browser', displayKind: 'browser', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.sidechain', displayKind: 'sidechain', priority: 10, rendererKey: 'sidechain' },
  { id: 'navis.toolRenderer.permission', displayKind: 'permission', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.error', displayKind: 'error', priority: 10, rendererKey: 'generic' },
  { id: 'navis.toolRenderer.other', displayKind: 'other', priority: 1_000, rendererKey: 'generic' },
] as const;

export function registerBuiltinToolRenderers(
  extensionName: string,
  renderers: BuiltinToolRendererMap,
): string[] {
  return BUILTIN_TOOL_RENDERER_SPECS.map((spec) =>
    registerToolRenderer(
      extensionName,
      {
        id: spec.id,
        priority: spec.priority,
        match: { ...spec.match, displayKind: spec.displayKind },
      },
      renderers[spec.rendererKey],
    ),
  );
}
