import type { Component } from 'solid-js';

import { AgentIcon, ConnectorIcon, TerminalIcon } from '../Icon';

interface AgentTraceGlyphProps {
  kind: string;
}

const CONNECTOR_KINDS = new Set([
  'integration',
  'filesystem',
  'read',
  'list',
  'glob',
  'todo',
  'search',
  'grep',
  'websearch',
  'inspect',
  'lsp',
  'mcp_resource',
  'webfetch',
  'browser',
  'edit',
  'skill',
]);

const TERMINAL_KINDS = new Set(['terminal', 'shell', 'bash', 'powershell', 'command']);
const AGENT_KINDS = new Set(['agent', 'assistant']);

const normalizedKind = (kind: string): string =>
  kind.toLowerCase().replace(/[^a-z0-9_:-]+/g, '_');

export const AgentTraceGlyph: Component<AgentTraceGlyphProps> = (props) => {
  const kind = () => normalizedKind(props.kind);
  const className = () => `navis-agent-trace-glyph is-${kind()} is-svg-icon`;

  return (
    <>
      {CONNECTOR_KINDS.has(kind()) ? (
        <ConnectorIcon class={className()} />
      ) : TERMINAL_KINDS.has(kind()) ? (
        <TerminalIcon class={className()} />
      ) : AGENT_KINDS.has(kind()) ? (
        <AgentIcon class={className()} />
      ) : (
        <span class={`navis-agent-trace-glyph is-${kind()}`} aria-hidden="true" />
      )}
    </>
  );
};
