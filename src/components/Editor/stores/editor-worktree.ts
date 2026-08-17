import { createStore } from 'solid-js/store'
import {
  loadSessionWorktree,
  readSessionWorktreeFile,
  writeSessionWorktreeFile,
  type WorktreeFileDocument,
  type WorktreeFileNode,
} from '../../../stores/worktree'

export interface EditorWorktreeDocument extends WorktreeFileDocument {
  savedContent: string
}

export interface PendingEditorWorktreeOpen {
  relativePath: string
  line?: number
  column?: number
}

interface EditorWorktreeState {
  sessionId: string | null
  worktreeRoot: string | null
  documents: Record<string, EditorWorktreeDocument>
  loadingFilePath: string | null
  pendingOpenRequest: PendingEditorWorktreeOpen | null
  error: string | null
}

const defaultEditorWorktreeState: EditorWorktreeState = {
  sessionId: null,
  worktreeRoot: null,
  documents: {},
  loadingFilePath: null,
  pendingOpenRequest: null,
  error: null,
}

export const [editorWorktreeState, setEditorWorktreeState] = createStore<EditorWorktreeState>({
  ...defaultEditorWorktreeState,
})

let syncToken = 0

export function resetEditorWorktreeState(options: { preservePendingOpenRequest?: boolean } = {}): void {
  setEditorWorktreeState({
    ...defaultEditorWorktreeState,
    pendingOpenRequest: options.preservePendingOpenRequest
      ? editorWorktreeState.pendingOpenRequest
      : null,
  })
}

export async function syncEditorWorktreeSession(
  sessionId: string | null,
  worktreeRoot: string | null,
): Promise<void> {
  const token = ++syncToken
  setEditorWorktreeState({
    sessionId,
    worktreeRoot,
    documents: {},
    loadingFilePath: null,
    pendingOpenRequest: editorWorktreeState.pendingOpenRequest,
    error: null,
  })

  await loadSessionWorktree(sessionId)
  if (token !== syncToken) return
  setEditorWorktreeState({
    sessionId,
    worktreeRoot,
    loadingFilePath: null,
    pendingOpenRequest: editorWorktreeState.pendingOpenRequest,
    error: null,
  })
}

export function requestEditorWorktreeFileOpen(relativePath: string, location?: { line?: number; column?: number }): void {
  const nextPath = relativePath.trim()
  if (!nextPath) return
  setEditorWorktreeState('pendingOpenRequest', {
    relativePath: nextPath,
    line: location?.line,
    column: location?.column,
  })
}

export function clearPendingEditorWorktreeFileOpen(relativePath?: string): void {
  if (relativePath && editorWorktreeState.pendingOpenRequest?.relativePath !== relativePath) return
  setEditorWorktreeState('pendingOpenRequest', null)
}

export async function openEditorWorktreeFile(
  sessionId: string,
  node: WorktreeFileNode,
): Promise<EditorWorktreeDocument | null> {
  if (node.isDirectory) return null

  const existing = editorWorktreeState.documents[node.absolutePath]
  if (existing) {
    setEditorWorktreeState('error', null)
    return existing
  }

  setEditorWorktreeState({
    loadingFilePath: node.relativePath,
    error: null,
  })

  try {
    const document = await readSessionWorktreeFile(sessionId, node.relativePath)
    if (editorWorktreeState.sessionId !== sessionId) return null

    const nextDocument: EditorWorktreeDocument = {
      ...document,
      savedContent: document.content,
    }
    setEditorWorktreeState('documents', document.absolutePath, nextDocument)
    setEditorWorktreeState({
      loadingFilePath: null,
      error: null,
    })
    return nextDocument
  } catch (error) {
    if (editorWorktreeState.sessionId === sessionId) {
      setEditorWorktreeState({
        loadingFilePath: null,
        error: error instanceof Error ? error.message : String(error),
      })
    }
    return null
  }
}

export function updateEditorWorktreeDocument(filePath: string, content: string): void {
  if (!editorWorktreeState.documents[filePath]) return
  setEditorWorktreeState('documents', filePath, 'content', content)
}

export async function saveEditorWorktreeDocument(
  sessionId: string,
  filePath: string,
): Promise<EditorWorktreeDocument | null> {
  const document = editorWorktreeState.documents[filePath]
  if (!document) return null

  setEditorWorktreeState('error', null)
  try {
    const saved = await writeSessionWorktreeFile(sessionId, document.relativePath, document.content)
    if (editorWorktreeState.sessionId !== sessionId) return null

    const nextDocument: EditorWorktreeDocument = {
      ...document,
      ...saved,
      savedContent: saved.content,
    }
    setEditorWorktreeState('documents', filePath, nextDocument)
    return nextDocument
  } catch (error) {
    if (editorWorktreeState.sessionId === sessionId) {
      setEditorWorktreeState('error', error instanceof Error ? error.message : String(error))
    }
    return null
  }
}

export function pruneEditorWorktreeDocuments(openFilePaths: string[]): void {
  const openSet = new Set(openFilePaths)
  setEditorWorktreeState('documents', (current) =>
    Object.fromEntries(Object.entries(current).filter(([filePath]) => openSet.has(filePath))),
  )
}
