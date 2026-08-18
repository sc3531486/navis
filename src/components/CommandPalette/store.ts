/**
 * Navis 框架通用命令注册协议。
 *
 * 这里只维护扩展命令的注册与注销，不包含任何产品业务或具体面板实现。
 */
export type CommandSource = 'builtin' | 'extension' | 'skill' | 'command' | 'file' | 'symbol';

export interface Command {
  id: string;
  label: string;
  description?: string;
  category: string;
  keybinding?: string;
  icon?: string;
  handler: () => void | Promise<void>;
  isEnabled?: () => boolean;
  source: CommandSource;
  tags?: string[];
}

const commands = new Map<string, Command>();

export const commandPaletteAPI = {
  register(command: Command): void {
    if (!commands.has(command.id)) commands.set(command.id, command);
  },
  unregister(id: string): void {
    commands.delete(id);
  },
  registerBatch(items: Command[]): void {
    for (const item of items) this.register(item);
  },
  getState(): { commands: Command[] } {
    return { commands: [...commands.values()] };
  },
};
