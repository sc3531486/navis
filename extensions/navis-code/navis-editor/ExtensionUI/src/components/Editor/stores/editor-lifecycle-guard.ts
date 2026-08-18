import { getDirtyEditorTabs } from './editor-unsaved-guard'

export function installEditorBeforeUnloadGuard(): () => void {
  const handleBeforeUnload = (event: BeforeUnloadEvent) => {
    if (getDirtyEditorTabs().length === 0) return
    event.preventDefault()
    event.returnValue = ''
  }

  window.addEventListener('beforeunload', handleBeforeUnload)
  return () => {
    window.removeEventListener('beforeunload', handleBeforeUnload)
  }
}
