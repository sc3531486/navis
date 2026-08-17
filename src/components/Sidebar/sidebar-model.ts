import type { WorkMode } from '../../stores/agent';

export type BuiltinMode = 'cowork' | 'code';
export type ModeTab = BuiltinMode | 'custom';

export interface SidebarMenuItem {
  id: string;
  label: string;
  marker: string;
}

export const SESSION_TREE_REFRESH_EVENTS = [
  'session.created',
  'session.updated',
  'session.metadata.updated',
  'session.deleted',
  'session.archived',
  'session.restored',
  'session.switched',
  'session.message.added',
  'session.message.updated',
  'session.agent_timeline_part.updated',
  'session.change.recorded',
  'session.change.reverted',
  'action.started',
  'action.completed',
];

export const MODE_MENU: Record<BuiltinMode, SidebarMenuItem[]> = {
  cowork: [
    { id: 'new-task', label: 'New task', marker: '+' },
    { id: 'customize', label: 'Customize', marker: '▭' },
  ],
  code: [
    { id: 'new-session', label: 'New session', marker: '+' },
    { id: 'customize', label: 'Customize', marker: '▭' },
  ],
};

export function workModeFromSessionMode(mode: string | null | undefined): WorkMode | null {
  if (mode === 'cowork' || mode === 'code') return { type: mode };
  if (!mode?.startsWith('custom:')) return null;

  const runtimeId = mode.slice('custom:'.length);
  const [extensionId, modeId] = runtimeId.split('/');
  if (!extensionId || !modeId) return null;

  return {
    type: 'custom',
    extensionId,
    modeId,
    runtimeId,
  };
}
