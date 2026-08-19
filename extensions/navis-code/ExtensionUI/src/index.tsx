import { NavisContext, NavisPlugin } from '../../../../src/core/context';
import { SlotRenderer } from '../../../../src/core/SlotRenderer';

export const NavisCodeExtension: NavisPlugin = {
  name: 'navis-code',
  apply: async (ctx: NavisContext) => {
    console.info('[navis-code] Registering Studio layout into root slot...');

    // 1. 在 root 插槽挂载 navis-code 专属布局树
    ctx.registerSlot('root', {
      id: 'navis-code.layout.root',
      priority: 10,
      component: () => (
        <div class="navis-code-studio-root">
          <div class="navis-code-body-grid">
            <SlotRenderer
              ctx={ctx}
              target="navis-code.sidebar.left"
              class="navis-code-sidebar-container"
            />
            <SlotRenderer
              ctx={ctx}
              target="navis-code.viewport.main"
              class="navis-code-main-container"
            />
          </div>
          <SlotRenderer
            ctx={ctx}
            target="navis-code.statusbar"
            class="navis-code-statusbar-container"
          />
        </div>
      )
    });

    // 2. 注册全局命令
    ctx.registerCommand('navis-code.new-session', () => {
      ctx.emit('session:create', { timestamp: Date.now() });
    });
  }
};

export default NavisCodeExtension;
