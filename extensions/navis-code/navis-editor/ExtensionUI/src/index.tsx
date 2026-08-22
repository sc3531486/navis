// Navis Editor 扩展入口：提供代码编辑、源码浏览与 Diff 视图
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { EditorView } from './components/EditorView';
import { DiffViewer, type DiffFilePayload } from './components/DiffViewer';

export { DiffViewer, type DiffFilePayload };

export const NavisEditorExtension: NavisPlugin = {
  name: 'navis-editor',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-editor] Initializing Editor & Diff extension...');

    componentRegistry.bind('navis-editor', {
      Editor: () => <EditorView ctx={ctx} />,
      EditorView: () => <EditorView ctx={ctx} />,
      DiffViewer: (props: any) => <DiffViewer ctx={ctx} file={props.file} onClose={props.onClose} />,
    });

    ctx.views.register('navis-code.viewport.editor', {
      id: 'navis-editor.view',
      pluginId: 'navis-editor',
      priority: 15,
      component: () => <EditorView ctx={ctx} />,
    });

    // 注册 editor 服务，供其他扩展（如 agent-core）通过 DI 调用
    ctx.services.provide('editor', {
      openFile: (file: DiffFilePayload) => {
        ctx.events.emit('diff:open', file);
      },
      openDiff: (path: string, name?: string) => {
        ctx.events.emit('diff:open', {
          name: name || path.split('/').pop() || path,
          path,
          type: 'diff',
        });
      },
    });

    ctx.commands.register('editor:save', () => {
      ctx.events.emit('editor:file-saved', {});
    });
  },
};

export default NavisEditorExtension;
