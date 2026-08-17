import assert from 'node:assert/strict';
import { mkdir, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const outdir = '.tmp/verify-chat-message-reducer';
const outfile = `${outdir}/chat-message-reducer.mjs`;
const timelineOutfile = `${outdir}/timeline-order.mjs`;
const mergeOutfile = `${outdir}/agent-timeline-merge.mjs`;

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });
await build({
  entryPoints: ['src/stores/chat-message-reducer.ts'],
  outfile,
  bundle: true,
  format: 'esm',
  platform: 'node',
  logLevel: 'silent',
});
await build({
  entryPoints: ['src/lib/agent-timeline/timeline-order.ts'],
  outfile: timelineOutfile,
  bundle: true,
  format: 'esm',
  platform: 'node',
  logLevel: 'silent',
});
await build({
  entryPoints: ['src/lib/agent-timeline/merge.ts'],
  outfile: mergeOutfile,
  bundle: true,
  format: 'esm',
  platform: 'node',
  logLevel: 'silent',
});

const {
  appendAgentTimelinePartDeltaToSnapshot,
} = await import(pathToFileURL(outfile).href);
const { buildAgentTimelineItems, visibleTurnPreludePart } = await import(pathToFileURL(timelineOutfile).href);
const { mergeAgentTimelinePart } = await import(pathToFileURL(mergeOutfile).href);

const activeStatusPresentation = {
  phase: 'active',
  outcome: 'unknown',
  attention: 'normal',
  live: true,
  terminal: false,
};
const waitingStatusPresentation = {
  phase: 'waiting',
  outcome: 'unknown',
  attention: 'needs_action',
  live: true,
  terminal: false,
};
const succeededStatusPresentation = {
  phase: 'inactive',
  outcome: 'succeeded',
  attention: 'normal',
  live: false,
  terminal: true,
};

const baseStep = {
  partId: 'text:turn-1',
  turnId: 'turn-1',
  messageId: 'assistant-1',
  sequence: 10000,
  kind: 'text',
  status: 'running',
  statusPresentation: activeStatusPresentation,
  text: 'Hel',
  createdAt: '2026-01-01T00:00:00Z',
};

const baseMessages = [
  {
    id: 'assistant-1',
    sessionId: 'session-1',
    role: 'assistant',
    content: 'Hel',
    createdAt: '2026-01-01T00:00:00Z',
    agentTimelineParts: [baseStep],
  },
];

const applied = appendAgentTimelinePartDeltaToSnapshot(baseMessages, {
  type: 'agentTimelinePartDelta',
  messageId: 'assistant-1',
  turnId: 'turn-1',
  partId: 'text:turn-1',
  field: 'text',
  delta: 'lo',
});

assert.equal(applied.applied, true);
assert.equal(applied.messages[0].content, 'Hello');
assert.equal(applied.messages[0].agentTimelineParts[0].text, 'Hello');
assert.equal(baseMessages[0].agentTimelineParts[0].text, 'Hel');

const missing = appendAgentTimelinePartDeltaToSnapshot(baseMessages, {
  type: 'agentTimelinePartDelta',
  messageId: 'assistant-1',
  turnId: 'turn-1',
  partId: 'text:missing',
  field: 'text',
  delta: 'lo',
});

assert.equal(missing.applied, false);
assert.equal(missing.messages[0].content, 'Hel');

const mergedByStepId = mergeAgentTimelinePart([
  {
    partId: 'tool:bash:1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 1,
    kind: 'tool',
    status: 'running',
    statusPresentation: activeStatusPresentation,
    callId: 'call-1',
    tool: 'bash',
    summary: 'npm run build',
    createdAt: '2026-01-01T00:00:01Z',
  },
], {
  partId: 'tool:bash:1',
  turnId: 'turn-1',
  messageId: 'assistant-1',
  sequence: 1,
  kind: 'tool',
  status: 'completed',
  statusPresentation: succeededStatusPresentation,
  callId: 'call-1',
  tool: 'bash',
  summary: 'npm run build',
  createdAt: '2026-01-01T00:00:01Z',
  completedAt: '2026-01-01T00:00:03Z',
});

assert.equal(mergedByStepId.length, 1);
assert.equal(mergedByStepId[0].status, 'completed');
assert.equal(mergedByStepId[0].completedAt, '2026-01-01T00:00:03Z');

const notMergedByPartId = mergeAgentTimelinePart(mergedByStepId, {
  partId: 'tool:bash:different',
  turnId: 'turn-1',
  messageId: 'assistant-1',
  sequence: 2,
  kind: 'tool',
  status: 'completed',
  statusPresentation: succeededStatusPresentation,
  callId: 'call-1',
  tool: 'bash',
  summary: 'npm run build',
  createdAt: '2026-01-01T00:00:04Z',
});

assert.equal(notMergedByPartId.length, 2);

const orderedItemsWhileToolRuns = buildAgentTimelineItems([
  {
    partId: 'prelude:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: -100,
    kind: 'reasoning',
    status: 'running',
    statusPresentation: activeStatusPresentation,
    source: 'turn_prelude',
    summary: 'Thinking...',
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    partId: 'tool:bash:1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 1,
    kind: 'tool',
    status: 'running',
    statusPresentation: activeStatusPresentation,
    tool: 'terminal.run_command',
    summary: 'cd demo && mvn package',
    createdAt: '2026-01-01T00:00:01Z',
  },
  {
    partId: 'text:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10000,
    kind: 'text',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    text: 'Build succeeded.',
    source: 'gateway',
    createdAt: '2026-01-01T00:00:02Z',
  },
  {
    partId: 'finalizer:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10002,
    kind: 'summary',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    source: 'turn_finalizer',
    summary: 'Finished response',
    createdAt: '2026-01-01T00:00:03Z',
  },
]);

assert.deepEqual(
  orderedItemsWhileToolRuns.map((item) => item.part.partId),
  ['tool:bash:1', 'text:turn-1', 'finalizer:turn-1'],
);
assert.equal(
  visibleTurnPreludePart(
    [
      {
        partId: 'prelude:turn-1',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: -100,
        kind: 'reasoning',
        status: 'running',
        statusPresentation: activeStatusPresentation,
        source: 'turn_prelude',
        summary: 'Thinking',
        createdAt: '2026-01-01T00:00:00Z',
      },
      {
        partId: 'tool:bash:1',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: 1,
        kind: 'tool',
        status: 'running',
        statusPresentation: activeStatusPresentation,
        tool: 'terminal.run_command',
        summary: 'cd demo && mvn package',
        createdAt: '2026-01-01T00:00:01Z',
      },
    ],
    orderedItemsWhileToolRuns,
  )?.partId,
  undefined,
);

const orderedItemsForEmptyRunningText = buildAgentTimelineItems([
  {
    partId: 'prelude:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: -100,
    kind: 'reasoning',
    status: 'running',
    statusPresentation: activeStatusPresentation,
    source: 'turn_prelude',
    summary: 'Thinking...',
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    partId: 'text:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10000,
    kind: 'text',
    status: 'running',
    statusPresentation: activeStatusPresentation,
    text: '',
    summary: 'Waiting for response',
    source: 'gateway',
    createdAt: '2026-01-01T00:00:01Z',
  },
]);

assert.deepEqual(
  orderedItemsForEmptyRunningText.map((item) => item.part.partId),
  [],
);

const orderedItemsWithoutFinalizer = buildAgentTimelineItems([
  {
    partId: 'tool:bash:1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 1,
    kind: 'tool',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    tool: 'terminal.run_command',
    summary: 'cd demo && mvn package',
    createdAt: '2026-01-01T00:00:01Z',
  },
  {
    partId: 'text:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10000,
    kind: 'text',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    text: 'Build succeeded.',
    source: 'gateway',
    createdAt: '2026-01-01T00:00:02Z',
  },
]);

assert.deepEqual(
  orderedItemsWithoutFinalizer.map((item) => item.part.partId),
  ['tool:bash:1', 'text:turn-1'],
);

const orderedItemsWhilePermissionWaits = buildAgentTimelineItems([
  {
    partId: 'tool:bash:1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 1,
    kind: 'tool',
    status: 'waiting_permission',
    statusPresentation: waitingStatusPresentation,
    tool: 'terminal.run_command',
    summary: 'mvn package',
    createdAt: '2026-01-01T00:00:01Z',
  },
  {
    partId: 'text:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10000,
    kind: 'text',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    text: 'Build succeeded.',
    source: 'gateway',
    createdAt: '2026-01-01T00:00:02Z',
  },
  {
    partId: 'finalizer:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10002,
    kind: 'summary',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    source: 'turn_finalizer',
    summary: 'Finished response',
    createdAt: '2026-01-01T00:00:03Z',
  },
]);

assert.deepEqual(
  orderedItemsWhilePermissionWaits.map((item) => item.part.partId),
  ['tool:bash:1', 'text:turn-1', 'finalizer:turn-1'],
);

const orderedItemsAfterToolCompletes = buildAgentTimelineItems([
  {
    partId: 'text:turn-1:tool-prelude:0',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 0,
    kind: 'text',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    text: 'I will run the build.',
    source: 'gateway_tool_prelude',
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    partId: 'text:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10000,
    kind: 'text',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    text: 'Build succeeded.',
    source: 'gateway',
    createdAt: '2026-01-01T00:00:02Z',
  },
  {
    partId: 'tool:bash:1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 1,
    kind: 'tool',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    tool: 'terminal.run_command',
    summary: 'cd demo && mvn package',
    createdAt: '2026-01-01T00:00:01Z',
  },
  {
    partId: 'finalizer:turn-1',
    turnId: 'turn-1',
    messageId: 'assistant-1',
    sequence: 10002,
    kind: 'summary',
    status: 'completed',
    statusPresentation: succeededStatusPresentation,
    source: 'turn_finalizer',
    summary: 'Finished response',
    createdAt: '2026-01-01T00:00:03Z',
  },
]);

assert.deepEqual(
  orderedItemsAfterToolCompletes.map((item) => item.part.partId),
  ['text:turn-1:tool-prelude:0', 'tool:bash:1', 'text:turn-1', 'finalizer:turn-1'],
);
assert.equal(
  visibleTurnPreludePart(
    [
      {
        partId: 'prelude:turn-1',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: -100,
        kind: 'reasoning',
        status: 'running',
        statusPresentation: activeStatusPresentation,
        source: 'turn_prelude',
        summary: 'Thinking',
        createdAt: '2026-01-01T00:00:00Z',
      },
      {
        partId: 'text:turn-1:tool-prelude:0',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: 0,
        kind: 'text',
        status: 'completed',
        statusPresentation: succeededStatusPresentation,
        text: 'I will run the build.',
        source: 'gateway_tool_prelude',
        createdAt: '2026-01-01T00:00:00Z',
      },
    ],
    orderedItemsAfterToolCompletes,
  ),
  undefined,
);
assert.equal(
  visibleTurnPreludePart(
    [
      {
        partId: 'prelude:turn-1',
        turnId: 'turn-1',
        messageId: 'assistant-1',
        sequence: -100,
        kind: 'reasoning',
        status: 'completed',
        statusPresentation: succeededStatusPresentation,
        source: 'turn_prelude',
        summary: 'Thinking complete',
        createdAt: '2026-01-01T00:00:00Z',
      },
    ],
    [],
  ),
  undefined,
);

await rm(outdir, { recursive: true, force: true });
