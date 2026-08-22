import { Component, createSignal, For } from 'solid-js';
import type { NavisContext } from '@/core/context';

interface TerminalPanelProps {
  ctx: NavisContext;
}

export const TerminalPanel: Component<TerminalPanelProps> = (props) => {
  const [logs, setLogs] = createSignal<string[]>([
    'Navis Integrated Terminal v1.0.0',
    'powershell.exe -NoLogo',
    'PS D:\\myworkspace\\Navis Go> ',
  ]);
  const [cmd, setCmd] = createSignal('');

  const handleRun = () => {
    const text = cmd().trim();
    if (!text) return;
    setLogs((prev) => [...prev, `PS D:\\myworkspace\\Navis Go> ${text}`, `[Process stdout] Command '${text}' executed successfully.`]);
    setCmd('');
    props.ctx.events.emit('terminal:command:executed', { command: text });
  };

  return (
    <div style="height: 180px; background: #181818; border-top: 1px solid #333; display: flex; flex-direction: column; font-family: monospace; font-size: 12px;">
      <div style="display: flex; justify-content: space-between; align-items: center; padding: 4px 12px; background: #222; border-bottom: 1px solid #333;">
        <span style="color: #888; font-weight: 600; font-size: 11px;">TERMINAL</span>
        <button
          onClick={() => setLogs(['Navis Integrated Terminal v1.0.0'])}
          style="background: transparent; border: none; color: #888; cursor: pointer; font-size: 11px;"
        >
          Clear
        </button>
      </div>
      <div style="flex: 1; padding: 8px 12px; overflow-y: auto; color: #a9b7c6; line-height: 1.4;">
        <For each={logs()}>
          {(line) => <div>{line}</div>}
        </For>
      </div>
      <div style="display: flex; padding: 4px 12px; background: #1e1e1e; border-top: 1px solid #2a2a2a; align-items: center;">
        <span style="color: #2563eb; margin-right: 6px;">❯</span>
        <input
          value={cmd()}
          onInput={(e) => setCmd(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleRun();
          }}
          placeholder="输入命令..."
          style="flex: 1; background: transparent; border: none; color: #fff; font-family: inherit; font-size: 12px; outline: none;"
        />
      </div>
    </div>
  );
};
