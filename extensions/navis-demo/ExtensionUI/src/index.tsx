// Navis Demo 扩展前端：清单驱动的具名组件 + 命令 + 管线钩子绑定。
import { createSignal } from 'solid-js';
import type { NavisContext, NavisPlugin } from '../../../../src/core/context';
import { DynamicSlot } from '../../../../src/core/slots/DynamicSlot';
import { componentRegistry } from '../../../../src/core/components/ComponentRegistry';
import { coreRouteIpc, coreRouteStream } from '../../../../src/core/tauri-bridge';

const DemoBanner = () => {
  const [status, setStatus] = createSignal('idle');
  const [logs, setLogs] = createSignal<string[]>([]);

  const pushLog = (line: string) => {
    setLogs((prev) => [...prev.slice(-6), line]);
  };

  const callPing = async () => {
    setStatus('calling backend via core_route_ipc...');
    try {
      const r = await coreRouteIpc('navis-demo', 'demo.ping', {});
      pushLog(`demo.ping -> ${JSON.stringify(r)}`);
      setStatus('ok');
    } catch (err) {
      setStatus(`error: ${String(err)}`);
    }
  };

  const callStream = async () => {
    setStatus('streaming via core_route_stream...');
    try {
      await coreRouteStream('navis-demo', 'demo.stream', {}, (ev) => {
        pushLog(`stream chunk -> ${JSON.stringify(ev)}`);
      });
      setStatus('stream done');
    } catch (err) {
      setStatus(`error: ${String(err)}`);
    }
  };

  const callTool = async () => {
    setStatus('invoking tool through tool registry...');
    try {
      const r = await coreRouteIpc('navis-demo', 'tool.add', { a: 2, b: 3 });
      pushLog(`tool.add(2,3) -> ${JSON.stringify(r)}`);
      setStatus('ok');
    } catch (err) {
      setStatus(`error: ${String(err)}`);
    }
  };

  return (
    <div class="navis-demo-banner">
      <div class="navis-demo-title">
        <span class="navis-demo-dot" />
        <strong>Navis Demo Extension</strong>
        <span class="navis-demo-badge">Generic Runtime Shell</span>
      </div>
      <p class="navis-demo-desc">
        清单驱动插槽 + 统一工具网关 + stdio JSON-RPC 后端进程
      </p>
      <div class="navis-demo-actions">
        <button onClick={callPing}>IPC Ping</button>
        <button onClick={callStream}>Stream</button>
        <button onClick={callTool}>Tool add(2,3)</button>
      </div>
      <div class="navis-demo-status">{status()}</div>
      <div class="navis-demo-logs">
        {logs().map((l) => <div>{l}</div>)}
      </div>
      <DynamicSlot name="navis-demo.controls" />
    </div>
  );
};

const Controls = () => (
  <div class="navis-demo-controls">
    <span>子插槽 navis-demo.controls 已挂载（Slot-in-Slot）</span>
  </div>
);

export const NavisDemoExtension: NavisPlugin = {
  name: 'navis-demo',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-demo] Binding components & hooks...');

    // 绑定清单 slots 引用的具名组件
    componentRegistry.bind('navis-demo', {
      DemoBanner: () => <DemoBanner />,
      // 管线钩子 handler：按名字从组件注册中心解析
      toolGuard: (toolCall: any) => {
        console.info('[navis-demo] pipeline beforeToolExecute guard:', toolCall);
        return undefined; // 不拦截
      },
    });

    // 注册扩展自身提供的子插槽内容
    ctx.views.register('navis-demo.controls', {
      id: 'navis-demo.controls',
      pluginId: 'navis-demo',
      priority: 10,
      component: () => <Controls />,
    });
  },
};

export default NavisDemoExtension;