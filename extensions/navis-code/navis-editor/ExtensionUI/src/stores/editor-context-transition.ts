import { activeSession, activeSessionId } from '@session/stores/session-tree'
import { selectSession, setSessionWorktreeRoot } from '@session/stores/session-tree'
import { confirmEditorTabsCanProceed } from '@editor-ext/components/Editor/stores/editor-unsaved-guard'
import { editorState } from '@editor-ext/components/Editor/stores/editor'

function normalizedWorktreeRoot(worktreeRoot: string | null | undefined): string | null {
  const next = worktreeRoot?.trim()
  return next ? next : null
}

function currentEditorContext() {
  return {
    sessionId: activeSessionId(),
    worktreeRoot: normalizedWorktreeRoot(activeSession()?.worktreeRoot),
  }
}

async function confirmEditorContextTransition(
  titleAction: 'switch-session' | 'change-worktree',
): Promise<boolean> {
  const current = currentEditorContext()
  if (!current.sessionId) return true

  return confirmEditorTabsCanProceed(current.sessionId, editorState.tabs, {
    saveSingleLabel: titleAction === 'switch-session' ? 'Save and switch session' : 'Save and change worktree',
    saveMultipleLabel: titleAction === 'switch-session' ? 'Save all and switch session' : 'Save all and change worktree',
    discardSingleLabel: titleAction === 'switch-session' ? 'Discard and switch session' : 'Discard and change worktree',
    discardMultipleLabel: titleAction === 'switch-session' ? 'Discard all and switch session' : 'Discard all and change worktree',
    saveDescription:
      titleAction === 'switch-session'
        ? 'Write the current changes to disk before switching sessions.'
        : 'Write the current changes to disk before changing the worktree.',
    discardDescription:
      titleAction === 'switch-session'
        ? 'Switch sessions without saving the current edits.'
        : 'Change the worktree without saving the current edits.',
    saveFailureTitle: 'Save failed',
    saveFailureMessage: (tab) =>
      titleAction === 'switch-session'
        ? `Failed to save ${tab.fileName}. Session switch was cancelled.`
        : `Failed to save ${tab.fileName}. Worktree change was cancelled.`,
  })
}

export async function selectSessionWithEditorGuard(sessionId: string | null): Promise<boolean> {
  const current = currentEditorContext()
  if (sessionId === current.sessionId) {
    await selectSession(sessionId)
    return true
  }

  const canProceed = await confirmEditorContextTransition('switch-session')
  if (!canProceed) return false

  await selectSession(sessionId)
  return true
}

export async function setSessionWorktreeRootWithEditorGuard(
  sessionId: string,
  worktreeRoot: string | null,
): Promise<boolean> {
  const current = currentEditorContext()
  const nextWorktreeRoot = normalizedWorktreeRoot(worktreeRoot)

  if (sessionId !== current.sessionId) {
    await setSessionWorktreeRoot(sessionId, nextWorktreeRoot)
    return true
  }

  if (nextWorktreeRoot === current.worktreeRoot) return true

  const canProceed = await confirmEditorContextTransition('change-worktree')
  if (!canProceed) return false

  await setSessionWorktreeRoot(sessionId, nextWorktreeRoot)
  return true
}
