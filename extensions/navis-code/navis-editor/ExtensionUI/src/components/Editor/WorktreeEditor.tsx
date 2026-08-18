import { Component, For, Show, createEffect, createMemo, createSignal } from 'solid-js'
import { useBeforeLeave } from '@solidjs/router'
import { activeSession, activeSessionId } from '@session/stores/session-tree'
import {
  worktreeState,
  type WorktreeFileNode,
} from '@session/stores/worktree'
import { EditorTabs } from './EditorTabs'
import { EditorView, type EditorDocumentViewModel } from './EditorView'
import { editorDiagnosticsAPI, editorState, editorTabsAPI, resetEditorState } from './stores/editor'
import {
  clearPendingEditorWorktreeFileOpen,
  editorWorktreeState,
  openEditorWorktreeFile,
  pruneEditorWorktreeDocuments,
  resetEditorWorktreeState,
  saveEditorWorktreeDocument,
  syncEditorWorktreeSession,
  updateEditorWorktreeDocument,
} from './stores/editor-worktree'
import { confirmEditorTabsCanProceed } from './stores/editor-unsaved-guard'
import { loadEditorSettings, settingsState } from '@settings-ext/stores/settings'

const fileNameFromPath = (path: string): string => path.split(/[\\/]+/).filter(Boolean).at(-1) ?? path

const absolutePathFromWorktree = (worktreeRoot: string, relativePath: string): string => {
  const separator = worktreeRoot.includes('\\') ? '\\' : '/'
  return `${worktreeRoot.replace(/[\\/]+$/, '')}${separator}${relativePath.replace(/^[\\/]+/, '')}`
}

function findWorktreeNodeByRelativePath(
  nodes: WorktreeFileNode[],
  relativePath: string,
): WorktreeFileNode | null {
  for (const node of nodes) {
    if (node.relativePath === relativePath) return node
    const child = node.children ? findWorktreeNodeByRelativePath(node.children, relativePath) : null
    if (child) return child
  }
  return null
}

const WorktreeTreeNode: Component<{
  node: WorktreeFileNode
  depth: number
  expanded: Set<string>
  activePath: string | null
  onToggleDirectory: (path: string) => void
  onOpenFile: (node: WorktreeFileNode) => void
}> = (props) => {
  const isExpanded = () => props.expanded.has(props.node.relativePath)
  const isActive = () => props.activePath === props.node.relativePath

  return (
    <div>
      <button
        type="button"
        class={`flex h-7 w-full items-center gap-2 rounded-md px-2 text-left text-[12px] ${
          isActive() ? 'bg-[#ececec] text-[#242424]' : 'text-[#555555] hover:bg-[#f2f2f2]'
        }`}
        style={{ 'padding-left': `${props.depth * 14 + 8}px` }}
        onClick={() => (props.node.isDirectory ? props.onToggleDirectory(props.node.relativePath) : props.onOpenFile(props.node))}
      >
        <span class="w-3 shrink-0 text-center text-[#8a8a8a]">
          {props.node.isDirectory ? (isExpanded() ? '▾' : '▸') : '·'}
        </span>
        <span class="truncate">{props.node.name}</span>
      </button>
      <Show when={props.node.isDirectory && isExpanded()}>
        <For each={props.node.children}>
          {(child) => (
            <WorktreeTreeNode
              node={child}
              depth={props.depth + 1}
              expanded={props.expanded}
              activePath={props.activePath}
              onToggleDirectory={props.onToggleDirectory}
              onOpenFile={props.onOpenFile}
            />
          )}
        </For>
      </Show>
    </div>
  )
}

interface WorktreeEditorProps {
  mode?: 'Worktree' | 'file-panel'
}

const WorktreeEditor: Component<WorktreeEditorProps> = (props) => {
  const [expandedDirectories, setExpandedDirectories] = createSignal<Set<string>>(new Set())
  const [navigationTarget, setNavigationTarget] = createSignal<{
    filePath: string
    line: number
    column?: number
    nonce: number
  } | null>(null)

  useBeforeLeave((event) => {
    if (event.defaultPrevented || !editorState.tabs.some((tab) => tab.isDirty)) {
      return
    }

    event.preventDefault()
    void (async () => {
      const canLeave = await confirmEditorTabsCanProceed(activeSessionId(), editorState.tabs, {
        saveSingleLabel: 'Save and leave editor',
        saveMultipleLabel: 'Save all and leave editor',
        discardSingleLabel: 'Discard and leave editor',
        discardMultipleLabel: 'Discard all and leave editor',
        saveDescription: 'Write the current changes to disk before leaving the editor.',
        discardDescription: 'Leave the editor without saving the current edits.',
        saveFailureTitle: 'Save failed',
        saveFailureMessage: (tab) => `Failed to save ${tab.fileName}. Leaving the editor was cancelled.`,
      })

      if (canLeave) {
        event.retry(true)
      }
    })()
  })

  const activeTab = createMemo(() => {
    const activeTabId = editorState.activeTabId
    return activeTabId ? editorState.tabs.find((tab) => tab.id === activeTabId) ?? null : null
  })

  const activeDocument = createMemo(() => {
    const tab = activeTab()
    return tab ? editorWorktreeState.documents[tab.filePath] ?? null : null
  })

  const activeDocumentModel = createMemo<EditorDocumentViewModel | null>(() => {
    const tab = activeTab()
    const document = activeDocument()
    if (!tab || !document) return null
    return {
      absolutePath: document.absolutePath,
      fileName: document.fileName,
      language: tab.language,
      content: document.content,
    }
  })

  const activeDiagnostics = createMemo(() => {
    const tab = activeTab()
    return tab ? editorDiagnosticsAPI.get(tab.filePath) : []
  })

  createEffect(() => {
    const sessionId = activeSessionId()
    const worktreeRoot = activeSession()?.worktreeRoot?.trim() || null
    resetEditorState()
    resetEditorWorktreeState({ preservePendingOpenRequest: true })
    setExpandedDirectories(new Set<string>())
    void syncEditorWorktreeSession(sessionId, worktreeRoot)
  })

  createEffect(() => {
    if (!settingsState.loaded && !settingsState.loading) {
      void loadEditorSettings()
    }
  })

  createEffect(() => {
    const rootDirectories = worktreeState.fileTree
      .filter((node) => node.isDirectory)
      .map((node) => node.relativePath)
    setExpandedDirectories(new Set(rootDirectories))
  })

  createEffect(() => {
    pruneEditorWorktreeDocuments(editorState.tabs.map((tab) => tab.filePath))
  })

  const toggleDirectory = (relativePath: string) => {
    setExpandedDirectories((current) => {
      const next = new Set(current)
      if (next.has(relativePath)) {
        next.delete(relativePath)
      } else {
        next.add(relativePath)
      }
      return next
    })
  }

  const openFile = async (node: WorktreeFileNode, location?: { line?: number; column?: number }) => {
    if (node.isDirectory) {
      toggleDirectory(node.relativePath)
      return
    }

    const sessionId = activeSessionId()
    if (!sessionId) {
      return
    }

    const document = await openEditorWorktreeFile(sessionId, node)
    if (document && activeSessionId() === sessionId) {
      editorTabsAPI.open(document.absolutePath, document.fileName)
      if (typeof location?.line === 'number' && Number.isFinite(location.line)) {
        setNavigationTarget({
          filePath: document.absolutePath,
          line: location.line,
          column: location.column,
          nonce: Date.now(),
        })
      }
    }
  }

  createEffect(() => {
    const pendingRequest = editorWorktreeState.pendingOpenRequest
    const pendingPath = pendingRequest?.relativePath
    const sessionId = activeSessionId()
    if (!pendingPath || !sessionId || worktreeState.isLoadingFileTree) return

    const node = findWorktreeNodeByRelativePath(worktreeState.fileTree, pendingPath)
    const worktreeRoot = worktreeState.currentWorktree?.path?.trim()
    const targetNode = node ?? (worktreeRoot
      ? {
          name: fileNameFromPath(pendingPath),
          relativePath: pendingPath,
          absolutePath: absolutePathFromWorktree(worktreeRoot, pendingPath),
          isDirectory: false,
        }
      : null)
    if (!targetNode || targetNode.isDirectory) {
      clearPendingEditorWorktreeFileOpen(pendingPath)
      return
    }

    clearPendingEditorWorktreeFileOpen(pendingPath)
    void openFile(targetNode, { line: pendingRequest?.line, column: pendingRequest?.column })
  })

  const updateActiveDocument = (content: string) => {
    const tab = activeTab()
    const document = activeDocument()
    if (!tab || !document) return

    updateEditorWorktreeDocument(tab.filePath, content)
    editorTabsAPI.setDirty(tab.id, content !== document.savedContent)
  }

  const saveActiveDocument = async () => {
    const sessionId = activeSessionId()
    const tab = activeTab()
    if (!sessionId || !tab || !activeDocument()) return

    const saved = await saveEditorWorktreeDocument(sessionId, tab.filePath)
    if (saved && activeSessionId() === sessionId) {
      editorTabsAPI.markSaved(tab.filePath)
    }
  }

  const activeRelativePath = () => activeDocument()?.relativePath ?? null
  const activeFileLabel = () => activeDocument()?.fileName ?? 'No file selected'
  const activeFilePath = () => activeDocument()?.absolutePath ?? worktreeState.currentWorktree?.path ?? ''
  const activeDirty = () => Boolean(activeTab()?.isDirty)

  const showWorktreeTree = () => props.mode !== 'file-panel'

  return (
    <div class="flex h-full overflow-hidden bg-white text-[#242424]">
      <Show when={showWorktreeTree()}>
        <aside class="flex w-[280px] shrink-0 flex-col border-r border-[#e6e6e6] bg-[#fbfbfb]">
          <div class="border-b border-[#ececec] px-4 py-3">
            <div class="text-[11px] uppercase tracking-[0.08em] text-[#8a8a8a]">Worktree</div>
            <div class="mt-1 truncate text-[13px] font-medium text-[#242424]">
              {worktreeState.currentWorktree?.name ?? 'No Worktree'}
            </div>
            <div class="mt-1 text-[11px] text-[#8a8a8a]">{worktreeState.worktreeFiles.length} files</div>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto px-2 py-2">
            <Show
              when={!worktreeState.isLoading}
              fallback={<div class="px-2 py-3 text-[12px] text-[#8a8a8a]">Loading Worktree...</div>}
            >
              <Show
                when={!worktreeState.error}
                fallback={<div class="px-2 py-3 text-[12px] text-[#b42318]">{worktreeState.error}</div>}
              >
                <Show
                  when={worktreeState.currentWorktree}
                  fallback={<div class="px-2 py-3 text-[12px] text-[#8a8a8a]">Bind a Worktree to the current session to browse files.</div>}
                >
                  <For each={worktreeState.fileTree}>
                    {(node) => (
                      <WorktreeTreeNode
                        node={node}
                        depth={0}
                        expanded={expandedDirectories()}
                        activePath={activeRelativePath()}
                        onToggleDirectory={toggleDirectory}
                        onOpenFile={(fileNode) => void openFile(fileNode)}
                      />
                    )}
                  </For>
                </Show>
              </Show>
            </Show>
          </div>
        </aside>
      </Show>

      <section class="flex min-w-0 flex-1 flex-col overflow-hidden bg-white">
        <header class="navis-file-header">
          <div class="navis-file-title-row">
            <div class="navis-file-title" title={activeFileLabel()}>
              {activeFileLabel()}
              <Show when={activeDirty()}>
                <span class="navis-file-dirty">• Unsaved</span>
              </Show>
            </div>
          </div>
          <div class="navis-file-path-bar">
            <div class="navis-file-path-scroll" title={activeFilePath()}>
              {activeFilePath()}
            </div>
            <div class="navis-file-actions" aria-hidden="true">
              <span>&lt;/&gt;</span>
              <span>⌕</span>
              <span>▱</span>
              <span>⧉</span>
            </div>
          </div>
          <Show when={activeDirty()}>
            <button
              type="button"
              class="navis-file-save is-enabled"
              onClick={() => void saveActiveDocument()}
            >
              Save
            </button>
          </Show>
        </header>

        <Show when={props.mode !== 'file-panel'}>
          <EditorTabs sessionId={activeSessionId()} />
        </Show>

        <div class="min-h-0 flex-1 overflow-hidden">
          <EditorView
            document={activeDocumentModel()}
            settings={settingsState.editor}
            diagnostics={activeDiagnostics()}
            loading={Boolean(editorWorktreeState.loadingFilePath) && !activeDocument()}
            error={editorWorktreeState.error}
            navigationTarget={navigationTarget()}
            onChange={updateActiveDocument}
            onSave={() => void saveActiveDocument()}
          />
        </div>

        <Show when={editorWorktreeState.loadingFilePath}>
          <div class="border-t border-[#ececec] px-4 py-2 text-[12px] text-[#6f6f6f]">
            Loading {fileNameFromPath(editorWorktreeState.loadingFilePath!)}...
          </div>
        </Show>
      </section>
    </div>
  )
}

export default WorktreeEditor
