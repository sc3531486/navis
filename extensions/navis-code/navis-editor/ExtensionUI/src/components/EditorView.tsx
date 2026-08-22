import { Component, createSignal, onCleanup } from 'solid-js';
import type { NavisContext } from '@/core/context';

interface EditorViewProps {
  ctx: NavisContext;
}

export const EditorView: Component<EditorViewProps> = (props) => {
  const [activeFile, setActiveFile] = createSignal('AGENTS.md');
  const [content, setContent] = createSignal(`# AGENTS.md\n\nNavis 是基于 Tauri 2 的通用桌面应用白板与扩展运行时底座。\n万物皆扩展 (Everything is an extension)！`);

  const unsub = props.ctx.events.on('editor:open-file', (file: any) => {
    setActiveFile(file.name ?? 'Untitled');
    setContent(`// 文件内容预览：${file.path}\n\nexport const ready = true;`);
  });

  onCleanup(() => unsub());

  return (
    <div style="display: flex; flex-direction: column; height: 100%; width: 100%; background: #1e1e1e;">
      <div style="display: flex; background: #252526; border-bottom: 1px solid #333; overflow-x: auto;">
        <div style="padding: 8px 16px; background: #1e1e1e; color: #fff; font-size: 12px; border-right: 1px solid #333; display: flex; align-items: center; gap: 8px;">
          <span>📄 {activeFile()}</span>
          <span style="cursor: pointer; opacity: 0.6;">✕</span>
        </div>
      </div>
      <div style="flex: 1; padding: 16px; font-family: monospace; font-size: 13px; color: #d4d4d4; line-height: 1.6; white-space: pre-wrap; overflow-y: auto;">
        {content()}
      </div>
    </div>
  );
};
