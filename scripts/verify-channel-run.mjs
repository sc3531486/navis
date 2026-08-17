import assert from 'node:assert/strict';
import { mkdir, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { build } from 'esbuild';

const outdir = '.tmp/verify-channel-run';
const outfile = `${outdir}/channel-lifecycle.mjs`;

const envelope = (kind, data = null, sequence = 1) => ({
  streamId: 'stream-1',
  sequence,
  kind,
  data,
  isFinal: kind !== 'data',
});

await rm(outdir, { recursive: true, force: true });
await mkdir(outdir, { recursive: true });

try {
  await build({
    entryPoints: ['src/lib/stream/channel-lifecycle.ts'],
    outfile,
    bundle: true,
    format: 'esm',
    platform: 'node',
    logLevel: 'silent',
  });

  const { createChannelLifecycle } = await import(pathToFileURL(outfile).href);

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: (data) => events.push(`chunk:${data}`),
      onCreated: (result) => events.push(`created:${result}`),
      disposeLateResource: (result) => events.push(`late:${result}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
    });

    run.handleEnvelope(envelope('done'));
    run.handleCreated('pty-1');
    run.handleCreated('pty-duplicate');
    run.handleEnvelope(envelope('data', 'stale', 2));

    assert.deepEqual(events, ['termination:completed', 'late:pty-1']);
    assert.equal(run.finished(), true);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: (data) => events.push(`chunk:${data}`),
      disposeLateResource: (result) => events.push(`late:${result}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
      onCancel: (streamId) => events.push(`cancel:${streamId}`),
    });

    run.handleEnvelope(envelope('data', 'ready'));
    run.stop('test stop');
    run.stop();
    run.handleCreated('pty-2');

    assert.deepEqual(events, ['chunk:ready', 'cancel:stream-1', 'termination:stopped', 'late:pty-2']);
    assert.equal(run.termination().reason, 'test stop');
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => undefined,
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
      onCancel: () => {
        events.push('cancel');
        throw new Error('cancel adapter failed');
      },
    });

    run.handleEnvelope(envelope('data', 'ready'));
    assert.throws(() => run.stop(), /cancel adapter failed/);
    run.stop();

    assert.deepEqual(events, ['cancel', 'termination:stopped']);
    assert.equal(run.finished(), true);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => events.push('chunk'),
      onTermination: (reason) => events.push(`termination:${reason.kind}:${reason.error?.message ?? ''}`),
    });

    run.handleInvokeError('invoke failed');
    run.handleInvokeError('duplicate failure');
    run.handleEnvelope(envelope('error', { error: 'stale stream error' }));

    assert.deepEqual(events, ['termination:creation_error:invoke failed']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => events.push('chunk'),
      onTermination: (reason) => events.push(`termination:${reason.kind}:${reason.error?.message ?? ''}`),
    });

    run.handleEnvelope(envelope('error', { error: 'stream failed' }));
    run.handleEnvelope(envelope('done', null, 2));
    run.handleInvokeError('late invoke failure');

    assert.deepEqual(events, ['termination:error:stream failed']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => events.push('chunk'),
      onTermination: (reason) => events.push(`termination:${reason.kind}:${reason.reason ?? ''}`),
    });

    run.handleEnvelope(envelope('cancelled', { reason: 'user cancelled' }));

    assert.deepEqual(events, ['termination:cancelled:user cancelled']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      completion: 'invoke',
      onChunk: () => events.push('chunk'),
      onCreated: (result) => events.push(`created:${result}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
    });

    run.handleCreated('resource-1');
    run.handleEnvelope(envelope('data', 'stale'));

    assert.deepEqual(events, ['created:resource-1', 'termination:completed']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      completion: 'manual',
      onChunk: (data) => events.push(`chunk:${data}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
    });

    run.handleEnvelope(envelope('data', 'first'));
    run.complete();
    run.handleEnvelope(envelope('data', 'stale', 2));

    assert.deepEqual(events, ['chunk:first', 'termination:completed']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => {
        throw new Error('consumer failed');
      },
      onCancel: (streamId) => events.push(`cancel:${streamId}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}:${reason.error?.message ?? ''}`),
    });

    run.handleEnvelope(envelope('data', 'bad'));
    run.handleEnvelope(envelope('data', 'stale', 2));

    assert.deepEqual(events, ['cancel:stream-1', 'termination:error:consumer failed']);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => undefined,
      onCreated: () => {
        throw new Error('ownership transfer failed');
      },
      disposeLateResource: (result) => events.push(`dispose:${result}`),
      onTermination: (reason) => events.push(`termination:${reason.kind}:${reason.error?.message ?? ''}`),
    });

    run.handleCreated('resource-2');

    assert.deepEqual(events, [
      'dispose:resource-2',
      'termination:error:ownership transfer failed',
    ]);
  }

  {
    const events = [];
    const run = createChannelLifecycle({
      onChunk: () => undefined,
      disposeLateResource: () => {
        throw new Error('late cleanup failed');
      },
      onTermination: (reason) => events.push(`termination:${reason.kind}`),
    });

    run.stop('unmount');
    run.handleCreated('resource-3');

    assert.deepEqual(events, ['termination:stopped']);
  }

  console.log('Verified Channel lifecycle ownership and exactly-once cleanup.');
} finally {
  await rm(outdir, { recursive: true, force: true });
}
