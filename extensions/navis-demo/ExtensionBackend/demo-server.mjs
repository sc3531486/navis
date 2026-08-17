#!/usr/bin/env node
/**
 * navis-demo 后端扩展（演示）：最小 JSON-RPC over stdio 服务。
 *
 * 形态：独立进程，容器 spawn，经 stdio 通信（复用 transport_adapters 契约）。
 * 真实后端可用 Rust/Java/Node 实现；此处用 Node 演示协议形态。
 *
 * 协议：每行一个 JSON-RPC 2.0 请求/响应：
 *   {"jsonrpc":"2.0","id":1,"method":"echo","params":{"message":"hi"}}
 *   → {"jsonrpc":"2.0","id":1,"result":{"echo":"hi"}}
 */

const readline = require('node:readline');

const methods = {
  echo(params) {
    return { echo: params?.message ?? 'hello from ExtensionBackend' };
  },
  ping() {
    return { pong: true };
  },
};

const rl = readline.createInterface({ input: process.stdin, terminal: false });

rl.on('line', (line) => {
  let request;
  try {
    request = JSON.parse(line);
  } catch {
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'parse error' } }) + '\n');
    return;
  }

  const { id, method, params } = request;
  if (typeof methods[method] !== 'function') {
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code: -32601, message: `method not found: ${method}` } }) + '\n');
    return;
  }

  try {
    const result = methods[method](params);
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n');
  } catch (error) {
    process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code: -32603, message: String(error) } }) + '\n');
  }
});
