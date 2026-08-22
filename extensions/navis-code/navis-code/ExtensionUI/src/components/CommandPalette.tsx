import { Component, createSignal, onCleanup, onMount, For, Show } from 'solid-js';
import type { NavisContext } from '@/core/context';
import { toast } from '@/core/toast/ToastStore';
import { IconSearch } from '@/components/icons';

export interface CommandPaletteProps {
  ctx: NavisContext;
}

export const CommandPalette: Component<CommandPaletteProps> = (props) => {
  const [open, setOpen] = createSignal(false);
  const [query, setQuery] = createSignal('');
  const [selectedIndex, setSelectedIndex] = createSignal(0);

  const commands = [
    { id: 'session:new', title: '✨ 新建会话 (New Session)', category: '会话' },
    { id: 'settings:open', title: '⚙️ 打开全局设置 (Open Settings)', category: '设置' },
    { id: 'agent:status', title: '📊 查看 Agent 网关状态 (Gateway Status)', category: 'Agent' },
    { id: 'theme:toggle', title: '🌓 切换主题模式 (Toggle Theme Light/Dark)', category: '外观' },
    { id: 'editor:save', title: '💾 保存当前文件 (Save File)', category: '编辑器' },
    { id: 'project:open-folder', title: '📁 打开项目目录 (Open Folder)', category: '工作区' },
  ];

  const filteredCommands = () => {
    const q = query().toLowerCase().trim();
    if (!q) return commands;
    return commands.filter((c) => c.title.toLowerCase().includes(q) || c.id.toLowerCase().includes(q));
  };

  const handleOpen = () => {
    setOpen(true);
    setQuery('');
    setSelectedIndex(0);
  };

  const handleClose = () => {
    setOpen(false);
  };

  const handleExecute = (cmd: (typeof commands)[0]) => {
    handleClose();
    if (cmd.id === 'theme:toggle') {
      const cur = document.documentElement.getAttribute('data-theme') || 'light';
      const next = cur === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', next);
      toast.success(`已切换主题至 ${next}`);
    } else {
      props.ctx.commands.execute(cmd.id);
      toast.info(`执行命令: ${cmd.title}`);
    }
  };

  onMount(() => {
    const unsubCmd = props.ctx.commands.register('command:palette', () => {
      handleOpen();
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'p' || e.key === 'k')) {
        e.preventDefault();
        if (open()) {
          handleClose();
        } else {
          handleOpen();
        }
      } else if (e.key === 'Escape' && open()) {
        handleClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    onCleanup(() => {
      unsubCmd();
      window.removeEventListener('keydown', handleKeyDown);
    });
  });

  return (
    <Show when={open()}>
      <div
        onClick={handleClose}
        style="position: fixed; inset: 0; background: rgba(0,0,0,0.35); backdrop-filter: blur(2px); z-index: 9999; display: flex; align-items: flex-start; justify-content: center; padding-top: 100px; pointer-events: auto;"
      >
        <div
          onClick={(e) => e.stopPropagation()}
          style="width: 540px; max-width: 90vw; background: #ffffff; border: 1px solid #e7e4dc; border-radius: 12px; box-shadow: 0 12px 36px rgba(0,0,0,0.18); overflow: hidden; display: flex; flex-direction: column; animation: navis-pop 0.15s ease-out; pointer-events: auto;"
        >
          {/* 搜索输入框 */}
          <div style="display: flex; align-items: center; gap: 10px; padding: 12px 16px; border-bottom: 1px solid #eae7e1;">
            <span style="color: #8e8b83; display: flex; align-items: center;">
              <IconSearch size={15} />
            </span>
            <input
              type="text"
              autofocus
              placeholder="输入命令、搜索文件或会话 (Ctrl+P)..."
              value={query()}
              onInput={(e) => {
                setQuery(e.currentTarget.value);
                setSelectedIndex(0);
              }}
              onKeyDown={(e) => {
                if (e.key === 'ArrowDown') {
                  e.preventDefault();
                  setSelectedIndex((i) => Math.min(i + 1, filteredCommands().length - 1));
                } else if (e.key === 'ArrowUp') {
                  e.preventDefault();
                  setSelectedIndex((i) => Math.max(i - 1, 0));
                } else if (e.key === 'Enter') {
                  const list = filteredCommands();
                  if (list[selectedIndex()]) {
                    handleExecute(list[selectedIndex()]);
                  }
                }
              }}
              style="flex: 1; border: none; outline: none; font-size: 14px; color: #2d2b28; background: transparent;"
            />
            <span style="font-size: 11px; color: #a8a49c; background: #f0eee8; padding: 2px 6px; border-radius: 4px;">ESC 退出</span>
          </div>

          {/* 命令列表 */}
          <div style="max-height: 320px; overflow-y: auto; padding: 6px; overscroll-behavior: contain;">
            <For
              each={filteredCommands()}
              fallback={<div style="padding: 20px; text-align: center; color: #8e8b83; font-size: 13px;">未找到匹配命令</div>}
            >
              {(cmd, index) => (
                <div
                  onClick={() => handleExecute(cmd)}
                  onMouseEnter={() => setSelectedIndex(index())}
                  style={`display: flex; align-items: center; justify-content: space-between; padding: 9px 12px; border-radius: 6px; cursor: pointer; font-size: 13px; transition: background 0.1s; ${
                    selectedIndex() === index() ? 'background: #f0eee8; color: #1e1d1b; font-weight: 500;' : 'color: #4b4843;'
                  }`}
                >
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span>{cmd.title}</span>
                  </div>
                  <span style="font-size: 11px; color: #8e8b83; background: #faf9f6; border: 1px solid #eae7e1; padding: 2px 6px; border-radius: 4px;">
                    {cmd.category}
                  </span>
                </div>
              )}
            </For>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default CommandPalette;
