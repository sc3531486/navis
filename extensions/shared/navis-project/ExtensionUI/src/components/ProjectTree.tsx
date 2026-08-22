import { Component, createSignal, For } from 'solid-js';
import type { NavisContext } from '@/core/context';

interface ProjectTreeProps {
  ctx: NavisContext;
}

interface FileNode {
  name: string;
  isDir: boolean;
  path: string;
}

export const ProjectTree: Component<ProjectTreeProps> = (props) => {
  const [files] = createSignal<FileNode[]>([
    { name: 'src', isDir: true, path: 'src' },
    { name: 'src-tauri', isDir: true, path: 'src-tauri' },
    { name: 'extensions', isDir: true, path: 'extensions' },
    { name: 'navis-code.json', isDir: false, path: 'navis-code.json' },
    { name: 'package.json', isDir: false, path: 'package.json' },
    { name: 'AGENTS.md', isDir: false, path: 'AGENTS.md' },
  ]);

  const handleOpenFile = (file: FileNode) => {
    if (!file.isDir) {
      props.ctx.events.emit('editor:open-file', {
        path: file.path,
        name: file.name,
      });
    }
  };

  return (
    <div style="display: flex; flex-direction: column; height: 100%; padding: 12px; gap: 8px; border-bottom: 1px solid var(--navis-border, #2d2d2d);">
      <div style="display: flex; justify-content: space-between; align-items: center;">
        <span style="font-size: 11px; font-weight: 700; color: #888; letter-spacing: 0.5px;">EXPLORER</span>
        <span style="font-size: 11px; color: #666; overflow: hidden; text-overflow: ellipsis; max-width: 120px;">
          Navis Go
        </span>
      </div>
      <div style="display: flex; flex-direction: column; gap: 2px;">
        <For each={files()}>
          {(file) => (
            <div
              onClick={() => handleOpenFile(file)}
              style="padding: 4px 8px; border-radius: 4px; font-size: 12px; color: #ccc; cursor: pointer; display: flex; align-items: center; gap: 6px; hover:background: #2a2a2a;"
            >
              <span style="opacity: 0.7; font-size: 11px;">{file.isDir ? '📁' : '📄'}</span>
              <span>{file.name}</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
