import { invoke } from '@tauri-apps/api/core';
import { createStore } from 'solid-js/store';
import { navisCodeProductState, setActiveSession as setProductActiveSession, setPendingStartKind } from '@navis-code/stores/product-app';
import { gatewayState, loadGatewayCatalog, preferredGatewayDefaultModelSelection, type GatewayModelSelection } from '@project-ext/stores/gateway';
import type { TranscriptView } from '@/lib/transcript-view';

export type { TranscriptView } from '@/lib/transcript-view';

export interface SidebarSession {
  id: string;
  name: string;
  createdAt?: string;
  pinned?: boolean;
  unread?: boolean;
  hasRunningTask?: boolean;
  hasCompletedTask?: boolean;
  model?: string | null;
  providerId?: string | null;
  modelId?: string | null;
  mode?: string | null;
  worktreeRoot?: string | null;
  permissionPolicy?: string | null;
  transcriptView: TranscriptView;
  reasoningEffort: ReasoningEffort;
}

export type ReasoningEffort = 'low' | 'medium' | 'high' | 'extra-high' | 'max';
type PermissionPolicy = 'suggest' | 'auto-edit' | 'full-auto';

const permissionPolicies = new Set<string>(['suggest', 'auto-edit', 'full-auto']);

function normalizeSessionPermissionPolicy(permissionPolicy: string | null | undefined): PermissionPolicy {
  return permissionPolicies.has(permissionPolicy ?? '') ? (permissionPolicy as PermissionPolicy) : 'suggest';
}

export interface SessionWorktree {
  name: string;
  sessions: SidebarSession[];
  collapsed: boolean;
}

interface UiSessionTree {
  worktrees: SessionWorktree[];
  activeSessionId: string | null;
}

export const [sessionTreeState, setSessionTreeState] = createStore<{
  worktrees: SessionWorktree[];
  loaded: boolean;
}>({
  worktrees: [],
  loaded: false,
});

function applySessionTree(tree: UiSessionTree): void {
  const collapsedByName = new Map(sessionTreeState.worktrees.map((worktree) => [worktree.name, worktree.collapsed]));
  setSessionTreeState({
    worktrees: tree.worktrees.map((worktree) => ({
      ...worktree,
      collapsed: collapsedByName.get(worktree.name) ?? worktree.collapsed,
    })),
    loaded: true,
  });
  setProductActiveSession(tree.activeSessionId);
}

async function refreshSessionTree(): Promise<void> {
  const tree = await invoke<UiSessionTree>('ui_list_session_tree');
  applySessionTree(tree);
}

async function applySessionTreeCommand(command: string, payload?: unknown): Promise<void> {
  const tree = await invoke<UiSessionTree>(command, payload === undefined ? undefined : { payload });
  applySessionTree(tree);
}

export async function loadSessionTree(): Promise<void> {
  await refreshSessionTree();
}

export function activeSessionId(): string | null {
  return navisCodeProductState.activeSessionId;
}

export function allSessions(): SidebarSession[] {
  return sessionTreeState.worktrees.flatMap((worktree) => worktree.sessions);
}

export function getSession(sessionId: string | null): SidebarSession | undefined {
  if (!sessionId) return undefined;
  return allSessions().find((session) => session.id === sessionId);
}

export function activeSession(): SidebarSession | undefined {
  return getSession(activeSessionId());
}

export function findSessionWorktreeIndex(sessionId: string): number {
  return sessionTreeState.worktrees.findIndex((worktree) =>
    worktree.sessions.some((session) => session.id === sessionId),
  );
}

export async function createSession(
  mode?: string,
  name = 'New session',
  selection?: GatewayModelSelection | null,
): Promise<string | null> {
  if ((!selection?.providerId?.trim() || !selection?.modelId?.trim()) && !gatewayState.loaded) {
    await loadGatewayCatalog();
  }
  const resolvedSelection =
    selection?.providerId?.trim() && selection?.modelId?.trim()
      ? selection
      : preferredGatewayDefaultModelSelection();
  await applySessionTreeCommand('ui_create_session', {
    name,
    worktreeName: null,
    mode: mode ?? null,
    providerId: resolvedSelection?.providerId ?? null,
    modelId: resolvedSelection?.modelId ?? null,
  });
  return activeSessionId();
}

export async function selectSession(sessionId: string | null): Promise<void> {
  if (!sessionId) {
    setProductActiveSession(null);
    return;
  }
  await applySessionTreeCommand('ui_set_active_session', { sessionId });
  setPendingStartKind(null);
}

export async function activateSession(sessionId: string): Promise<void> {
  await selectSession(sessionId);
}

export function toggleWorktree(index: number): void {
  setSessionTreeState('worktrees', index, 'collapsed', (collapsed) => !collapsed);
}

export async function renameWorktree(index: number, name: string, mode?: string | null): Promise<void> {
  const oldName = sessionTreeState.worktrees[index]?.name;
  if (!oldName) return;
  await applySessionTreeCommand('ui_rename_worktree', { oldName, newName: name, mode: mode ?? null });
}

export async function deleteWorktree(index: number, mode?: string | null): Promise<void> {
  const worktreeName = sessionTreeState.worktrees[index]?.name;
  if (!worktreeName) return;
  await applySessionTreeCommand('ui_delete_worktree', { worktreeName, mode: mode ?? null });
}

export async function renameSession(sessionId: string, name: string): Promise<void> {
  await applySessionTreeCommand('ui_rename_session', { sessionId, name });
}

export async function setSessionModelSelection(
  sessionId: string,
  providerId: string,
  modelId: string,
): Promise<void> {
  await applySessionTreeCommand('ui_set_session_model', {
    sessionId,
    providerId: providerId.trim(),
    modelId: modelId.trim(),
  });
}

export async function setSessionPermissionPolicy(sessionId: string, permissionPolicy: string): Promise<void> {
  await applySessionTreeCommand('ui_set_session_permission_policy', {
    sessionId,
    permissionPolicy: normalizeSessionPermissionPolicy(permissionPolicy),
  });
}

export async function setSessionTranscriptView(sessionId: string, transcriptView: TranscriptView): Promise<void> {
  await applySessionTreeCommand('ui_set_session_transcript_view', { sessionId, transcriptView });
}

export async function setSessionReasoningEffort(sessionId: string, reasoningEffort: ReasoningEffort): Promise<void> {
  await applySessionTreeCommand('ui_set_session_reasoning_effort', { sessionId, reasoningEffort });
}

export async function setSessionWorktreeRoot(sessionId: string, worktreeRoot: string | null): Promise<void> {
  await applySessionTreeCommand('ui_set_session_worktree_root', { sessionId, worktreeRoot });
}

export async function toggleSessionPin(sessionId: string): Promise<void> {
  await applySessionTreeCommand('ui_set_session_pinned', { sessionId, value: null });
}

export async function setSessionUnread(sessionId: string, unread: boolean): Promise<void> {
  await applySessionTreeCommand('ui_set_session_unread', { sessionId, value: unread });
}

export async function markSessionUnread(sessionId: string): Promise<void> {
  await setSessionUnread(sessionId, true);
}

export async function forkSession(sessionId: string): Promise<string | null> {
  const previousIds = new Set(allSessions().map((session) => session.id));
  await applySessionTreeCommand('ui_fork_session', { sessionId });
  return allSessions().find((session) => !previousIds.has(session.id))?.id ?? null;
}

export async function moveSessionToWorktree(sessionId: string, targetWorktreeIndex: number): Promise<void> {
  const worktreeName = sessionTreeState.worktrees[targetWorktreeIndex]?.name;
  if (!worktreeName) return;
  await applySessionTreeCommand('ui_move_session_to_worktree', { sessionId, worktreeName });
}

export async function moveSessionToWorktreeName(sessionId: string, worktreeName: string): Promise<void> {
  const nextWorktreeName = worktreeName.trim();
  if (!nextWorktreeName) return;
  await applySessionTreeCommand('ui_move_session_to_worktree', { sessionId, worktreeName: nextWorktreeName });
}

export async function archiveSession(sessionId: string): Promise<void> {
  await applySessionTreeCommand('ui_archive_session', { sessionId });
}

export async function removeSession(sessionId: string): Promise<void> {
  await applySessionTreeCommand('ui_delete_session', { sessionId });
}
