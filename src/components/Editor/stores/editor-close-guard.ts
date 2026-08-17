import type { EditorTab } from '../types'
import { editorState, editorTabsAPI } from './editor'
import { confirmEditorTabsCanProceed } from './editor-unsaved-guard'

type CloseAction = 'single' | 'others' | 'all'
function getSingleCloseTargets(tabId: string): EditorTab[] {
  const tab = editorState.tabs.find((item) => item.id === tabId)
  return tab && !tab.isPinned ? [tab] : []
}

function getCloseOthersTargets(tabId: string): EditorTab[] {
  return editorState.tabs.filter((tab) => tab.id !== tabId && !tab.isPinned)
}

function getCloseAllTargets(): EditorTab[] {
  return editorState.tabs.filter((tab) => !tab.isPinned)
}

function getCloseTargets(action: CloseAction, tabId?: string): EditorTab[] {
  switch (action) {
    case 'single':
      return tabId ? getSingleCloseTargets(tabId) : []
    case 'others':
      return tabId ? getCloseOthersTargets(tabId) : []
    case 'all':
      return getCloseAllTargets()
  }
}

async function ensureTabsCanClose(
  sessionId: string | null,
  tabsToClose: EditorTab[],
): Promise<boolean> {
  return confirmEditorTabsCanProceed(sessionId, tabsToClose, {
    saveSingleLabel: 'Save and close',
    saveMultipleLabel: 'Save all and close',
    discardSingleLabel: 'Discard changes',
    discardMultipleLabel: 'Discard all changes',
    saveDescription: 'Write the current changes to disk before closing.',
    discardDescription: 'Close without saving the current edits.',
    saveFailureTitle: 'Save failed',
    saveFailureMessage: (tab) => `Failed to save ${tab.fileName}. Close was cancelled.`,
  })
}

async function runCloseAction(
  sessionId: string | null,
  action: CloseAction,
  tabId?: string,
): Promise<void> {
  const tabsToClose = getCloseTargets(action, tabId)
  if (tabsToClose.length === 0) return

  const canClose = await ensureTabsCanClose(sessionId, tabsToClose)
  if (!canClose) return

  switch (action) {
    case 'single':
      if (tabId) {
        editorTabsAPI.close(tabId)
      }
      return
    case 'others':
      if (tabId) {
        editorTabsAPI.closeOthers(tabId)
      }
      return
    case 'all':
      editorTabsAPI.closeAll()
      return
  }
}

export async function closeEditorTabWithGuard(
  sessionId: string | null,
  tabId: string,
): Promise<void> {
  await runCloseAction(sessionId, 'single', tabId)
}

export async function closeOtherEditorTabsWithGuard(
  sessionId: string | null,
  tabId: string,
): Promise<void> {
  await runCloseAction(sessionId, 'others', tabId)
}

export async function closeAllEditorTabsWithGuard(sessionId: string | null): Promise<void> {
  await runCloseAction(sessionId, 'all')
}
