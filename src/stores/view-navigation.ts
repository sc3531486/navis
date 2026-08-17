import type { Navigator } from '@solidjs/router'
import { appState, setActiveView } from './app'

export type BuiltinAppView = 'chat' | 'editor' | 'settings'

let appNavigator: Navigator | null = null

function viewPath(view: BuiltinAppView): string {
  switch (view) {
    case 'editor':
      return '/editor'
    case 'settings':
      return '/settings'
    case 'chat':
    default:
      return '/chat'
  }
}

export function builtinViewFromPath(pathname: string): BuiltinAppView {
  if (pathname === '/editor' || pathname.startsWith('/editor/')) return 'editor'
  if (pathname === '/settings' || pathname.startsWith('/settings/')) return 'settings'
  return 'chat'
}

export function syncActiveViewFromPath(pathname: string): void {
  const nextView = builtinViewFromPath(pathname)
  if (appState.activeView !== nextView) {
    setActiveView(nextView)
  }
}

export function registerAppNavigator(navigate: Navigator): () => void {
  appNavigator = navigate
  return () => {
    if (appNavigator === navigate) {
      appNavigator = null
    }
  }
}

export function navigateToBuiltinView(
  view: BuiltinAppView,
  options?: { replace?: boolean },
): boolean {
  if (!appNavigator) return false
  appNavigator(viewPath(view), options)
  return true
}

export function openChatView(options?: { replace?: boolean }): boolean {
  return navigateToBuiltinView('chat', options)
}

export function openEditorView(options?: { replace?: boolean }): boolean {
  return navigateToBuiltinView('editor', options)
}

export function openSettingsView(options?: { replace?: boolean }): boolean {
  return navigateToBuiltinView('settings', options)
}
