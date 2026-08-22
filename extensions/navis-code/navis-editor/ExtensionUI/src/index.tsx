// Navis Editor 扩展入口：提供代码编辑与 Diff 视图
import type { NavisContext, NavisPlugin } from '@/core/context';
import { componentRegistry } from '@/core/components/ComponentRegistry';
import { EditorView } from './components/EditorView';

export const NavisEditorExtension: NavisPlugin = {
  name: 'navis-editor',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-editor] Initializing Editor extension...');

    componentRegistry.bind('navis-editor', {
      Editor: () => <EditorView ctx={ctx} />,
      EditorView: () => <EditorView ctx={ctx} />,
    });

    ctx.views.register('navis-code.viewport.editor', {
      id: 'navis-editor.view',
      pluginId: 'navis-editor',
      priority: 15,
      component: () => <EditorView ctx={ctx} />,
    });

    ctx.commands.register('editor:save', () => {
      ctx.events.emit('editor:file-saved', {});
    });
  },
};

export default NavisEditorExtension;
