import type { Navigator } from '@solidjs/router'
import { hostState, setActiveView } from './host'

/**
 * 通用视图导航系统。
 *
 * 视图由扩展通过 registerView() 注册，不再硬编码。
 * 框架只提供导航基础设施。
 */
export type AppView = string

interface ViewRegistration {
  path: string
  label: string
}

const viewRegistry = new Map<string, ViewRegistration>()
let appNavigator: Navigator | null = null

/** 注册一个视图（由扩展调用） */
export function registerView(id: string, path: string, label: string): void {
  viewRegistry.set(id, { path, label })
}

/** 获取视图路径 */
function viewPath(view: AppView): string {
  return viewRegistry.get(view)?.path ?? '/' + view
}

export function builtinViewFromPath(pathname: string): AppView {
  for (const [id, reg] of viewRegistry) {
    if (pathname === reg.path || pathname.startsWith(reg.path + '/')) return id
  }
  return viewRegistry.keys().next().value ?? 'home'
}

export function syncActiveViewFromPath(pathname: string): void {
  const nextView = builtinViewFromPath(pathname)
  if (hostState.activeView !== nextView) {
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

export function navigateToView(view: AppView, options?: { replace?: boolean }): boolean {
  if (!appNavigator) return false
  appNavigator(viewPath(view), options)
  return true
}
