// Navis Demo 后端进程：标准 stdio JSON-RPC 2.0 服务。
// 由宿主 ProcessSupervisor 拉起，core_route_ipc/core_route_stream 路由调用。
import { createInterface } from 'node:readline';
import process from 'node:process';

const rl = createInterface({ input: process.stdin, crlfDelay: Infinity });

function respond(id, result, error) {
  const msg = { jsonrpc: '2.0', id };
  if (error) {
    msg.error = error;
  } else {
    msg.result = result;
  }
  process.stdout.write(`${JSON.stringify(msg)}\n`);
}

function notify(method, params) {
  process.stdout.write(`${JSON.stringify({ jsonrpc: '2.0', method, params })}\n`);
}

const pluginId = process.env.NAVIS_PLUGIN_ID ?? 'navis-demo';

async function handle(method, params) {
  switch (method) {
    case 'tool.echo':
      return { text: params?.text ?? '', echoed: true, from: pluginId };
    case 'tool.add':
      return { sum: Number(params?.a ?? 0) + Number(params?.b ?? 0) };
    case 'demo.ping':
      return { pong: true, pluginId, now: Date.now() };
    case 'demo.echo':
      return { method, params, pluginId };
    default:
      return { ok: true, method, params };
  }
}

rl.on('line', async (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let msg;
  try {
    msg = JSON.parse(trimmed);
  } catch {
    return;
  }
  const { id, method, params } = msg ?? {};
  if (!method) return;

  // 流式调用：先推送通知，再返回最终应答
  if (method === 'demo.stream') {
    for (let i = 1; i <= 3; i += 1) {
      notify('demo.chunk', { index: i, pluginId });
      // eslint-disable-next-line no-await-in-loop
      await new Promise((r) => setTimeout(r, 200));
    }
    respond(id, { done: true, count: 3 });
    return;
  }

  try {
    const result = await handle(method, params);
    respond(id, result);
  } catch (err) {
    respond(id, undefined, { code: -32000, message: String(err) });
  }
});

rl.on('close', () => process.exit(0));