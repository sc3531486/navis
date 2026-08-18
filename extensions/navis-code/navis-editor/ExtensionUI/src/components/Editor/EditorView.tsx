import { Component, Show, createEffect, createSignal, onCleanup } from 'solid-js'
import { basicSetup } from 'codemirror'
import { Compartment, EditorState as CodeMirrorState } from '@codemirror/state'
import { css } from '@codemirror/lang-css'
import { html } from '@codemirror/lang-html'
import { javascript } from '@codemirror/lang-javascript'
import { json } from '@codemirror/lang-json'
import { indentUnit } from '@codemirror/language'
import { python } from '@codemirror/lang-python'
import { rust } from '@codemirror/lang-rust'
import { EditorView as CodeMirrorView, keymap } from '@codemirror/view'
import type { EditorSettings } from '@settings-ext/stores/settings'
import { Minimap } from './components/Minimap'
import type { Diagnostic } from './types'

export interface EditorDocumentViewModel {
  absolutePath: string
  fileName: string
  language: string
  content: string
}

export interface EditorViewProps {
  document: EditorDocumentViewModel | null
  settings: EditorSettings
  diagnostics?: Diagnostic[]
  loading?: boolean
  error?: string | null
  navigationTarget?: { filePath: string; line: number; column?: number; nonce: number } | null
  onChange: (content: string) => void
  onSave: () => void
}

interface EditorViewportState {
  visibleStartLine: number
  visibleEndLine: number
  totalLines: number
}

function languageExtension(language: string) {
  switch (language) {
    case 'typescript':
      return javascript({ typescript: true, jsx: true })
    case 'javascript':
      return javascript({ jsx: true })
    case 'json':
      return json()
    case 'python':
      return python()
    case 'rust':
      return rust()
    case 'html':
      return html()
    case 'css':
    case 'scss':
    case 'less':
      return css()
    default:
      return []
  }
}

function createEditorTheme(fontSize: number) {
  return CodeMirrorView.theme({
    '&': {
      height: '100%',
      backgroundColor: '#ffffff',
      color: '#242424',
    },
    '.cm-scroller': {
      overflow: 'auto',
      fontFamily: '"SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace',
      fontSize: `${fontSize}px`,
      lineHeight: '1.6',
    },
    '.cm-content': {
      minHeight: '100%',
      padding: '14px 0',
    },
    '.cm-line': {
      padding: '0 16px',
    },
    '.cm-gutters': {
      backgroundColor: '#fafafa',
      color: '#8a8a8a',
      borderRight: '1px solid #ececec',
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#f0f0f0',
    },
    '.cm-activeLine': {
      backgroundColor: '#f7f7f7',
    },
    '.cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#dbeafe',
    },
    '.cm-cursor': {
      borderLeftColor: '#242424',
    },
  })
}

function tabExtensions(tabSize: number) {
  return [
    CodeMirrorState.tabSize.of(tabSize),
    indentUnit.of(' '.repeat(tabSize)),
  ]
}

function wrapExtensions(wordWrap: EditorSettings['wordWrap']) {
  return wordWrap === 'on' ? [CodeMirrorView.lineWrapping] : []
}

export const EditorView: Component<EditorViewProps> = (props) => {
  let containerRef: HTMLDivElement | undefined
  let editor: CodeMirrorView | null = null
  let currentPath: string | null = null
  let syncingFromProps = false

  const [viewportState, setViewportState] = createSignal<EditorViewportState>({
    visibleStartLine: 0,
    visibleEndLine: 1,
    totalLines: 1,
  })

  const languageCompartment = new Compartment()
  const appearanceCompartment = new Compartment()
  const tabCompartment = new Compartment()
  const wrapCompartment = new Compartment()

  const saveKeymap = keymap.of([
    {
      key: 'Mod-s',
      run: () => {
        props.onSave()
        return true
      },
    },
  ])

  const syncViewportState = (view: CodeMirrorView | null) => {
    if (!view) {
      setViewportState({
        visibleStartLine: 0,
        visibleEndLine: 1,
        totalLines: 1,
      })
      return
    }

    const { doc } = view.state
    const totalLines = doc.lines
    const visibleStartLine = Math.max(0, doc.lineAt(view.viewport.from).number - 1)
    const visibleEndLine = Math.max(
      visibleStartLine + 1,
      doc.lineAt(Math.max(view.viewport.to - 1, 0)).number,
    )

    setViewportState({
      visibleStartLine,
      visibleEndLine,
      totalLines,
    })
  }

  const applyEditorSettings = (
    view: CodeMirrorView,
    language: string,
    settings: EditorSettings,
  ) => {
    view.dispatch({
      effects: [
        languageCompartment.reconfigure(languageExtension(language)),
        appearanceCompartment.reconfigure(createEditorTheme(settings.fontSize)),
        tabCompartment.reconfigure(tabExtensions(settings.tabSize)),
        wrapCompartment.reconfigure(wrapExtensions(settings.wordWrap)),
      ],
    })
    syncViewportState(view)
  }

  const destroyEditor = () => {
    editor?.destroy()
    editor = null
    currentPath = null
    syncViewportState(null)
  }

  const createEditor = (document: EditorDocumentViewModel, settings: EditorSettings) => {
    if (!containerRef) return

    destroyEditor()
    currentPath = document.absolutePath
    editor = new CodeMirrorView({
      state: CodeMirrorState.create({
        doc: document.content,
        extensions: [
          basicSetup,
          saveKeymap,
          languageCompartment.of(languageExtension(document.language)),
          appearanceCompartment.of(createEditorTheme(settings.fontSize)),
          tabCompartment.of(tabExtensions(settings.tabSize)),
          wrapCompartment.of(wrapExtensions(settings.wordWrap)),
          CodeMirrorView.updateListener.of((update) => {
            if (!update.docChanged && !update.viewportChanged && !update.geometryChanged) {
              return
            }

            syncViewportState(update.view)
            if (!update.docChanged || syncingFromProps) return
            props.onChange(update.state.doc.toString())
          }),
        ],
      }),
      parent: containerRef,
    })

    syncViewportState(editor)
  }

  createEffect(() => {
    const loading = props.loading ?? false
    const documentPath = props.document?.absolutePath ?? null

    if (!containerRef || loading || !documentPath) {
      destroyEditor()
      return
    }

    if (!editor || currentPath !== documentPath) {
      createEditor(props.document!, props.settings)
      return
    }
  })

  createEffect(() => {
    const documentPath = props.document?.absolutePath ?? null
    const language = props.document?.language ?? ''
    const { fontSize, tabSize, wordWrap } = props.settings

    if (!editor || !documentPath || currentPath !== documentPath) return

    applyEditorSettings(editor, language, {
      ...props.settings,
      fontSize,
      tabSize,
      wordWrap,
    })
  })

  createEffect(() => {
    const documentPath = props.document?.absolutePath ?? null
    const nextContent = props.document?.content ?? null

    if (!editor || !documentPath || currentPath !== documentPath || nextContent === null) return

    const currentContent = editor.state.doc.toString()
    if (nextContent !== currentContent) {
      syncingFromProps = true
      editor.dispatch({
        changes: {
          from: 0,
          to: editor.state.doc.length,
          insert: nextContent,
        },
      })
      syncingFromProps = false
      syncViewportState(editor)
    }
  })

  onCleanup(() => destroyEditor())

  const navigateToLine = (line: number, column = 0) => {
    if (!editor) return
    const targetLine = editor.state.doc.line(Math.min(editor.state.doc.lines, Math.max(line + 1, 1)))
    const anchor = Math.min(targetLine.to, Math.max(targetLine.from, targetLine.from + Math.max(0, column)))
    editor.dispatch({
      selection: { anchor },
      scrollIntoView: true,
    })
    editor.focus()
    syncViewportState(editor)
  }

  createEffect(() => {
    const target = props.navigationTarget
    const documentPath = props.document?.absolutePath ?? null
    if (!target || !documentPath || target.filePath !== documentPath) return
    void target.nonce
    navigateToLine(target.line, target.column ?? 0)
  })

  return (
    <div class="flex h-full flex-col bg-white">
      <Show
        when={!props.loading}
        fallback={
          <div class="flex flex-1 items-center justify-center text-[12px] text-[#6f6f6f]">
            Loading file...
          </div>
        }
      >
        <Show
          when={!props.error}
          fallback={
            <div class="flex flex-1 items-center justify-center px-6 text-center text-[12px] text-[#b42318]">
              {props.error}
            </div>
          }
        >
          <Show
            when={props.document}
            fallback={
              <div class="flex flex-1 items-center justify-center text-[#8a8a8a]">
                <div class="text-center">
                  <div class="text-[13px] font-medium text-[#4a4a4a]">Open a file to start editing</div>
                  <div class="mt-1 text-[12px]">The editor reads files from the current session Worktree.</div>
                </div>
              </div>
            }
          >
            <div class="flex min-h-0 flex-1 overflow-hidden">
              <div ref={containerRef} class="min-h-0 min-w-0 flex-1 overflow-hidden" />
              <Show when={props.document && props.settings.minimap}>
                <div class="w-[72px] shrink-0 bg-[#fafafa]">
                  <Minimap
                    content={props.document?.content ?? ''}
                    visibleStartLine={viewportState().visibleStartLine}
                    visibleEndLine={viewportState().visibleEndLine}
                    totalLines={viewportState().totalLines}
                    diagnostics={props.diagnostics}
                    onNavigate={navigateToLine}
                    width={72}
                  />
                </div>
              </Show>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  )
}
