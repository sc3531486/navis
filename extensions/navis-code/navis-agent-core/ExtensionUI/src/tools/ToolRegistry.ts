import type { NavisContext } from '@/core/context';
import { callRemote } from '@/core/tauri-bridge';
import { toast } from '@/core/toast/ToastStore';

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: {
    type: 'object';
    properties: Record<string, any>;
    required: string[];
  };
}

export const agentToolDefinitions: ToolDefinition[] = [
  {
    name: 'write_file',
    description:
      'Creates a new file or overwrites an existing file with the provided content. Automatically creates parent directories if they do not exist.',
    parameters: {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: 'The path to the file to create or write (relative to workspace or absolute)',
        },
        content: {
          type: 'string',
          description: 'The complete content to write into the file',
        },
      },
      required: ['filePath', 'content'],
    },
  },
  {
    name: 'read_file',
    description: 'Reads and returns the complete text content of a file in the workspace.',
    parameters: {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: 'The path of the file to read',
        },
      },
      required: ['filePath'],
    },
  },
  {
    name: 'edit_file',
    description:
      'Replaces occurrences of oldString with newString in the specified file. Use to make targeted edits.',
    parameters: {
      type: 'object',
      properties: {
        filePath: {
          type: 'string',
          description: 'The path of the file to edit',
        },
        oldString: {
          type: 'string',
          description: 'The exact string to find and replace',
        },
        newString: {
          type: 'string',
          description: 'The replacement string',
        },
      },
      required: ['filePath', 'oldString', 'newString'],
    },
  },
  {
    name: 'execute_command',
    description:
      'Executes a shell command (e.g. javac, mvn, cargo, npm, mkdir, git, cat) in the workspace directory and returns stdout and stderr.',
    parameters: {
      type: 'object',
      properties: {
        command: {
          type: 'string',
          description: 'The shell command line to execute',
        },
        cwd: {
          type: 'string',
          description: 'Optional working directory (defaults to current project root)',
        },
      },
      required: ['command'],
    },
  },
  {
    name: 'list_dir',
    description: 'Lists all files and subdirectories within a given directory path.',
    parameters: {
      type: 'object',
      properties: {
        path: {
          type: 'string',
          description: 'The directory path to list (defaults to workspace root)',
        },
      },
      required: [],
    },
  },
];

export class ToolExecutor {
  private ctx: NavisContext;

  constructor(ctx: NavisContext) {
    this.ctx = ctx;
  }

  async execute(toolName: string, args: Record<string, any>): Promise<{ success: boolean; output: string }> {
    console.info(`[ToolExecutor] Executing tool: ${toolName}`, args);

    try {
      switch (toolName) {
        case 'write_file': {
          const filePath = args.filePath || args.path || args.file;
          const content = args.content || '';
          if (!filePath) throw new Error("Missing 'filePath' argument");

          const res = await callRemote('core:fs:write', { path: filePath, content });
          this.ctx.events.emit('file:created', { path: filePath });
          this.ctx.events.emit('artifact:created', { name: filePath, size: content.length });
          toast.success(`已生成文件: ${filePath}`);
          return {
            success: true,
            output: `Successfully wrote ${content.length} bytes to ${filePath}`,
          };
        }

        case 'read_file': {
          const filePath = args.filePath || args.path || args.file;
          if (!filePath) throw new Error("Missing 'filePath' argument");

          const res = await callRemote('core:fs:read', { path: filePath });
          return {
            success: true,
            output: res?.content || '',
          };
        }

        case 'edit_file': {
          const filePath = args.filePath || args.path || args.file;
          const oldStr = args.oldString || args.old_str || '';
          const newStr = args.newString || args.new_str || '';
          if (!filePath) throw new Error("Missing 'filePath' argument");

          await callRemote('core:fs:edit', { path: filePath, old_str: oldStr, new_str: newStr });
          this.ctx.events.emit('file:modified', { path: filePath });
          toast.success(`已更新文件: ${filePath}`);
          return {
            success: true,
            output: `Successfully edited ${filePath}`,
          };
        }

        case 'execute_command': {
          const command = args.command || args.cmd;
          const cwd = args.cwd || '.';
          if (!command) throw new Error("Missing 'command' argument");

          const res = await callRemote('core:shell:exec', { command, cwd });
          const stdout = res?.stdout || '';
          const stderr = res?.stderr || '';
          const code = res?.exit_code ?? 0;
          return {
            success: res?.success ?? true,
            output: `[Exit Code: ${code}]\n${stdout}${stderr ? `\n[STDERR]:\n${stderr}` : ''}`,
          };
        }

        case 'list_dir': {
          const dirPath = args.path || args.dir || '.';
          const res = await callRemote('core:fs:list_dir', { path: dirPath });
          const entries = res?.entries || [];
          const formatted = entries
            .map((e: any) => `${e.is_dir ? '📁 [DIR]' : '📄 [FILE]'} ${e.name}`)
            .join('\n');
          return {
            success: true,
            output: formatted || '(Empty directory)',
          };
        }

        default:
          return {
            success: false,
            output: `Unknown tool: ${toolName}`,
          };
      }
    } catch (err: any) {
      console.error(`[ToolExecutor] Failed to execute ${toolName}:`, err);
      return {
        success: false,
        output: `Error executing ${toolName}: ${err?.message || String(err)}`,
      };
    }
  }
}
