import type { MenuTarget } from './menu';

export interface BuiltinMenuCommandCoverage {
  directCommands: readonly string[];
  submenuParentCommands: readonly string[];
  generatedCommands?: readonly string[];
  generatedCommandPrefixes?: readonly string[];
}

export const BUILTIN_MENU_COMMAND_COVERAGE: Record<MenuTarget, BuiltinMenuCommandCoverage> = {
  Tools: {
    directCommands: [
      'tools.commandPalette',
      'tools.settings',
      'tools.gateway',
      'tools.codingEditor',
      'tools.extensions',
    ],
    submenuParentCommands: [],
  },
  InputPlus: {
    directCommands: [
      'composer.addFiles',
      'composer.addFolder',
      'composer.insertSlashCommand',
      'composer.addConnectors',
      'composer.addExtensions',
      'composer.togglePlanMode',
      'composer.toggleMultiAgent',
      'composer.toggleGoalTracking',
    ],
    submenuParentCommands: [],
  },
  ChatTitle: {
    directCommands: [
      'session.rename',
      'session.fork',
      'session.archive',
      'session.delete',
    ],
    submenuParentCommands: [
      'session.openIn',
      'session.transcriptView',
    ],
    generatedCommands: [
      'session.openIn.current',
      'session.openIn.right',
      'session.openIn.configureExternalEditors',
    ],
    generatedCommandPrefixes: [
      'session.openIn.externalEditor:',
      'session.transcriptView.',
    ],
  },
  RightPanel: {
    directCommands: [
      'rightWorkspace.open.diff',
      'rightWorkspace.open.backgroundTasks',
      'rightWorkspace.open.plan',
      'rightWorkspace.open.design',
    ],
    submenuParentCommands: [],
  },
  Gateway: {
    directCommands: [
      'gateway.settings',
    ],
    submenuParentCommands: [
      'gateway.language',
    ],
    generatedCommandPrefixes: [
      'gateway.language:',
    ],
  },
  WorktreeContext: {
    directCommands: [
      'worktree.rename',
      'worktree.delete',
    ],
    submenuParentCommands: [],
  },
  SessionContext: {
    directCommands: [
      'session.pin',
      'session.markUnread',
      'session.markRead',
      'session.rename',
      'session.fork',
      'session.moveToWorktree.new',
      'session.archive',
      'session.delete',
    ],
    submenuParentCommands: [
      'session.moveToWorktree',
    ],
    generatedCommandPrefixes: [
      'session.moveToWorktree:',
    ],
  },
};

function includesCommand(commands: readonly string[] | undefined, command: string): boolean {
  return commands?.includes(command) ?? false;
}

export function isBuiltinMenuCommandCovered(target: MenuTarget, command: string): boolean {
  const coverage = BUILTIN_MENU_COMMAND_COVERAGE[target];
  return (
    includesCommand(coverage.directCommands, command) ||
    includesCommand(coverage.submenuParentCommands, command) ||
    includesCommand(coverage.generatedCommands, command) ||
    (coverage.generatedCommandPrefixes?.some((prefix) => command.startsWith(prefix)) ?? false)
  );
}
