/**
 * navis-demo 最小扩展：worker 轨演示"注册受控操作 → 执行 → 存储结果"。
 *
 * 遵循扩展开发手册 design/36-extension-development.md：
 * - 桥只暴露领域无关原语（runtime.operation.* / storage.* / extensions.query）
 * - 受控操作机制在容器，操作实现由扩展 worker 承载
 */

export function onRun(args) {
  return runDemo(args);
}

export async function onMessage(payload) {
  return runDemo(payload || {});
}

async function runDemo(args) {
  const operationId = 'navis-demo.echo';

  // 1. 注册一个受控操作（Extension 实现，容器只做门禁）
  try {
    await self.__NAVIS__.invoke('runtime.operation.register', {
      extensionId: 'navis-demo',
      id: operationId,
      label: 'Demo Echo',
      permissionLevel: 'Unrestricted',
      operationType: 'CommandExecute',
      handlerKind: 'Extension',
      paramsSchema: {
        type: 'object',
        properties: { message: { type: 'string' } },
      },
    });
  } catch (error) {
    // 已注册时幂等容忍
    console.warn('[navis-demo] register skipped', error);
  }

  // 2. 执行该操作（过容器 Sandbox 门禁）
  const result = await self.__NAVIS__.invoke('runtime.operation.execute', {
    extensionId: 'navis-demo',
    operationId,
    params: { message: args?.message || 'hello from navis-demo' },
  });

  // 3. Extension handler：容器返回信号，扩展 worker 自己实现并回存
  if (result && result.status === 'extension_handler') {
    const reply = { ok: true, echo: result.params?.message, ts: Date.now() };
    const clicks = await self.__NAVIS__.storage.get('demo.clicks', { scope: 'global' });
    await self.__NAVIS__.storage.set('demo.clicks', (clicks?.value ?? 0) + 1, { scope: 'global' });
    return reply;
  }

  return { ok: false, error: 'unexpected execute result', result };
}
