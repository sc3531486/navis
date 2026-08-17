import { mkdir, readFile, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const outdir = '.tmp/verify-tool-renderers';
const outfile = `${outdir}/verify-tool-renderers.mjs`;

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

try {

await build({
  stdin: {
    sourcefile: 'verify-tool-renderers-entry.mjs',
    resolveDir: process.cwd(),
    contents: `
      import assert from 'node:assert/strict';
      import {
        registerToolRenderer,
        resolveToolRenderer,
      } from './src/lib/agent-timeline/tool-renderer-catalog.ts';
      import {
        BUILTIN_TOOL_RENDERER_SPECS,
        registerBuiltinToolRenderers,
      } from './src/components/AgentTimeline/builtin-tool-renderers.ts';

      const requiredDisplayKinds = [
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
      ];

      const registeredKinds = new Set(BUILTIN_TOOL_RENDERER_SPECS.map((spec) => spec.displayKind));
      for (const kind of requiredDisplayKinds) {
        assert.equal(registeredKinds.has(kind), true, \`missing renderer spec for \${kind}\`);
      }

      assert.equal(
        BUILTIN_TOOL_RENDERER_SPECS.find((spec) => spec.displayKind === 'write-as-edit')?.rendererKey,
        'edit',
        'write-as-edit must use the edit renderer',
      );
      assert.equal(
        BUILTIN_TOOL_RENDERER_SPECS.some((spec) => /wrote/i.test(\`\${spec.id} \${spec.displayKind} \${spec.rendererKey}\`)),
        false,
        'renderer specs must not expose a Wrote label',
      );

      const rendererNames = [
        'generic',
        'read',
        'list',
        'search',
        'inspect',
        'edit',
        'terminal',
        'sidechain',
      ];
      const renderers = Object.fromEntries(rendererNames.map((name) => [name, () => name]));

      registerBuiltinToolRenderers('verify', renderers);

      const baseStep = {
        stepId: 'step-1',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: 1,
        kind: 'tool',
        status: 'completed',
        createdAt: '2026-01-01T00:00:00Z',
      };

      for (const spec of BUILTIN_TOOL_RENDERER_SPECS) {
        const resolved = resolveToolRenderer(
          {
            ...baseStep,
            metadata: { displayKind: spec.displayKind },
          },
          'other',
          renderers.generic,
        );
        assert.equal(resolved, renderers[spec.rendererKey], \`\${spec.displayKind} did not resolve to \${spec.rendererKey}\`);
      }

      const hintRenderer = () => 'hint';
      registerToolRenderer(
        'verify',
        {
          id: 'verify.rendererHint',
          priority: 1,
          match: { renderer: 'browser.table', detailView: 'events' },
        },
        hintRenderer,
      );
      assert.equal(
        resolveToolRenderer(
          {
            ...baseStep,
            metadata: {
              displayKind: 'browser',
              rendererHint: { renderer: 'browser.table', detailView: 'events' },
            },
          },
          'other',
          renderers.generic,
        ),
        hintRenderer,
        'rendererHint should take priority over displayKind fallback',
      );

      console.log(\`Verified \${requiredDisplayKinds.length} built-in tool renderer displayKinds.\`);
    `,
  },
  outfile,
  bundle: true,
  format: 'esm',
  platform: 'node',
  logLevel: 'silent',
});

await import(pathToFileURL(outfile).href);

const uiSources = await Promise.all([
  readFile('src/router/index.tsx', 'utf8'),
  readFile('src/lib/agent-timeline/tool-renderer-catalog.ts', 'utf8'),
]);

for (const source of uiSources) {
  if (/\bWrote\b/.test(source)) {
    throw new Error('Tool UI source must not expose Wrote as a user-facing label.');
  }
}
} finally {
  await rm(outdir, { recursive: true, force: true });
}
