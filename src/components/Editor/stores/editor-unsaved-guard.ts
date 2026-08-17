import { dialog } from '../../Dialog'
import type { EditorTab } from '../types'
import { editorState, editorTabsAPI } from './editor'
import { saveEditorWorktreeDocument } from './editor-worktree'

export interface EditorUnsavedGuardLabels {
  saveSingleLabel: string
  saveMultipleLabel: string
  discardSingleLabel: string
  discardMultipleLabel: string
  saveDescription: string
  discardDescription: string
  unavailableSaveDescription?: string
  saveFailureTitle: string
  saveFailureMessage: (tab: EditorTab) => string
}

type UnsavedDecision = 'save' | 'discard'

export function getDirtyEditorTabs(tabs: EditorTab[] = editorState.tabs): EditorTab[] {
  return tabs.filter((tab) => tab.isDirty)
}

async function chooseUnsavedDecision(
  dirtyTabs: EditorTab[],
  canSave: boolean,
  labels: EditorUnsavedGuardLabels,
): Promise<UnsavedDecision | null> {
  const isSingle = dirtyTabs.length === 1

  return dialog.select<UnsavedDecision>(
    isSingle
      ? `${dirtyTabs[0].fileName} has unsaved changes`
      : `${dirtyTabs.length} files have unsaved changes`,
    [
      {
        label: isSingle ? labels.saveSingleLabel : labels.saveMultipleLabel,
        value: 'save',
        description: canSave
          ? labels.saveDescription
          : labels.unavailableSaveDescription ?? 'Save is unavailable because there is no active session.',
        disabled: !canSave,
      },
      {
        label: isSingle ? labels.discardSingleLabel : labels.discardMultipleLabel,
        value: 'discard',
        description: labels.discardDescription,
      },
    ],
  )
}

async function saveDirtyTabs(
  sessionId: string,
  dirtyTabs: EditorTab[],
  labels: EditorUnsavedGuardLabels,
): Promise<boolean> {
  for (const tab of dirtyTabs) {
    const saved = await saveEditorWorktreeDocument(sessionId, tab.filePath)
    if (!saved) {
      await dialog.alert(labels.saveFailureTitle, labels.saveFailureMessage(tab))
      return false
    }
    editorTabsAPI.markSaved(tab.filePath)
  }

  return true
}

export async function confirmEditorTabsCanProceed(
  sessionId: string | null,
  tabs: EditorTab[],
  labels: EditorUnsavedGuardLabels,
): Promise<boolean> {
  const dirtyTabs = getDirtyEditorTabs(tabs)
  if (dirtyTabs.length === 0) return true

  const decision = await chooseUnsavedDecision(dirtyTabs, Boolean(sessionId), labels)
  if (!decision) return false
  if (decision === 'discard') return true
  if (!sessionId) return false

  return saveDirtyTabs(sessionId, dirtyTabs, labels)
}
