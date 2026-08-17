/**
 * ============================================================
 * Navis Stores 统一导出 - stores/index.ts
 * ============================================================
 *
 * 集中导出所有 store 模块，方便外部一次性引入。
 *
 * 四层架构：
 *   第 1 层：AppState（顶层全局 Store）— 跨模块共享状态
 *   第 2 层：模块 Store（agent/project/session-tree）— 独立业务状态
 *   第 3 层：IPC 事件同步层 — useEvent/useStream 钩子自动同步
 *   第 4 层：持久化层 — Config 模块持久化偏好设置
 *
 * 归属扩展标注：
 *   🏠 = 框架层保留（桌面应用白板底座）
 *   🔌 = 归属扩展（业务领域插件）
 *
 * @example
 * ```tsx
 * import { appState, sessionTreeState, projectState } from '@/stores';
 * ```
 * ============================================================

// ══════════════════════════════════════════════════════════
// 第 1 层：AppState（顶层全局 Store）
 */
// ══════════════════════════════════════════════════════════

// ── 🏠 框架层：app ──────────────────────────────────────
// 跨模块共享状态，不归属任何扩展，始终保留在框架层。
export {
  appState,
  setAppState,
  restoreAppState,
  init,
  setActiveSession as setAppActiveSession,
  setActiveProject,
  setSidebarWidth,
  setRightWorkspaceWidth,
  setOffline,
  setError,
  setLoading,
  toggleSidebar,
  toggleRightWorkspace,
  setActiveView,
  openRightWorkspacePanel,
  closeRightWorkspacePanel,
} from './app';
export type {
  AppState,
  WindowState,
  RightWorkspacePanel,
  RightWorkspaceColumn,
} from './app';

// ══════════════════════════════════════════════════════════
// 第 2 层：模块 Store
// ══════════════════════════════════════════════════════════

// ── 🔌 归属扩展：navis-agent-core ────────────────────────
// Agent 决策状态、运行时状态
export {
  agentState,
  setAgentState,
  setWorkMode,
} from './agent';
export type {
  AgentState,
  WorkMode,
} from './agent';

export {
  agentRuntimeStatus,
} from './agent-runtime';
export type {
  AgentRuntimeStatus,
} from './agent-runtime';

// ── 🏠 框架层：extension ─────────────────────────────────
// 扩展管理本体，归属框架层
export {
  extensionState,
  setExtensionState,
  loadExtensions,
  setExtensionEnabled,
  installExtension,
  uninstallExtension,
  setWorkModes,
  getWorkModeDisplayName,
} from './extension';
export type {
  WorkModeRegistration,
  WorkModeModelPreferences,
  RegisteredWorkMode,
  ExtensionRuntimeState,
  ExtensionStatus,
} from './extension';

// ── 🏠 框架层：extension-commands ────────────────────────
// 扩展命令注册（只有函数，无 state）
// 调用方直接 import { loadExtensionCommands } from '@/stores/extension-commands'

// ── 🏠 框架层：extension-keybindings ─────────────────────
// 扩展快捷键注册（只有函数，无 state）

// ── 🏠 框架层：extension-points ─────────────────────────
// 扩展点投影状态
export {
  extensionProjectionState,
} from './extension-points';
export type {
  ExtensionProjectionState,
  InlineHostTarget,
} from './extension-points';
export {
  INLINE_HOST_TARGETS,
} from './extension-points';

// ── 🏠 框架层：extension-workers ─────────────────────────
// 扩展 Worker 生命周期（只有函数 + 类型，无 state）

// ── 🔌 归属扩展：navis-ai-platform ──────────────────────
// 网关配置、Provider / Model 目录
export {
  gatewayState,
  setGatewayState,
  loadGatewayCatalog,
} from './gateway';
export type {
  GatewayProvider,
  GatewayModel,
} from './gateway';

// ── 🏠 框架层：language ──────────────────────────────────
// 语言切换，归属框架层
export {
  languageState,
  setLanguageState,
  loadLanguage,
  setAppLanguage,
} from './language';
export type {
  LanguageState,
} from './language';

// ── 🏠 框架层：session-tree ──────────────────────────────
// 会话树导航，归属框架层（跨扩展共享的会话选择状态）
export {
  sessionTreeState,
  setSessionTreeState,
  activeSessionId,
  activeSession,
  allSessions,
  getSession,
  findSessionWorktreeIndex,
  loadSessionTree,
  createSession as createTreeSession,
  selectSession as selectTreeSession,
  activateSession as activateTreeSession,
  toggleWorktree,
  renameWorktree,
  deleteWorktree,
  renameSession as renameTreeSession,
  setSessionModelSelection,
  setSessionPermissionPolicy,
  toggleSessionPin,
  setSessionUnread,
  markSessionUnread,
  forkSession,
  moveSessionToWorktree,
  moveSessionToWorktreeName,
  archiveSession,
  removeSession,
} from './session-tree';
export type {
  SidebarSession,
  SessionWorktree,
} from './session-tree';

// ── 🔌 归属扩展：navis-settings ─────────────────────────
// 设置状态，归属设置扩展
export {
  settingsState,
  setSettingsState,
  loadEditorSettings,
  updateEditorSettings,
  resetEditorSettingsDraft,
  saveEditorSettings,
  resetSettingsState,
} from './settings';

// ── 🔌 归属扩展：navis-project ──────────────────────────
// 项目状态，归属项目扩展
export {
  projectState,
  setProjectState,
  setCurrentProject,
  loadRecentWorktrees,
  addRecentWorktree,
  removeRecentBoundWorktree,
  resetProjectState,
} from './project';
export type {
  RecentWorktree,
  ProjectState,
} from './project';

// ── 🔌 归属扩展：navis-project ──────────────────────────
// Composer worktree 选择（归属项目扩展）
export {
  bindComposerWorktree,
  chooseComposerWorktree,
  worktreeLabel,
  pathNameFromPath,
  rememberRecentWorktree,
} from './composer-worktree';

// ── 🏠 框架层：right-workspace-menu ─────────────────────
// 右侧工作区菜单操作，归属框架层
export {
  executeRightWorkspaceMenuItem,
  getOpenRightWorkspaceCommands,
} from './right-workspace-menu';

// ── 🏠 框架层：worktree-menu ────────────────────────────
// 工作树菜单操作，归属框架层
export {
  executeWorktreeMenuItem,
} from './worktree-menu';

// ── 🏠 框架层：worktree ─────────────────────────────────
// 工作树状态，归属框架层
export {
  worktreeState,
  setWorktreeState,
  setCurrentWorktree,
  setWorktreeFiles,
  setFileTree,
  setWorktreeLoading,
  setFileTreeLoading,
  setWorktreeError,
  loadSessionWorktree,
  readSessionWorktreeFile,
  writeSessionWorktreeFile,
  removeRecentWorktree,
  toggleWorktreeStar,
  resetWorktreeState,
} from './worktree';
export type {
  Worktree,
  WorktreeFileNode,
  WorktreeFileDocument,
  WorktreeState,
} from './worktree';

// ══════════════════════════════════════════════════════════
// 第 3 层：IPC 事件同步 + UI 交互 Store
// ══════════════════════════════════════════════════════════

// ── 🔌 归属扩展：navis-session ──────────────────────────
// 会话消息状态（Chat 面板核心状态）
export {
  chatMessageState,
  setChatMessageState,
} from './chat-message-state';
export {
  CHAT_MESSAGES_PAGE_SIZE,
} from './chat-message-state';

// ── 🔌 归属扩展：navis-session ──────────────────────────
// 消息类型定义（只有类型，无 state）
// 调用方直接 import { ChatMessage } from '@/stores/chat-message-types'

// ── 🔌 归属扩展：navis-session ──────────────────────────
// 消息快照操作（只有函数，无 state）
// 调用方直接 import { mergeSnapshotMessages } from '@/stores/chat-message-reducer'

// ── 🔌 归属扩展：navis-session ──────────────────────────
// 消息列表视图（只有类型，无 state）
// 调用方直接 import { UiSessionMessages } from '@/stores/chat-messages'

// ── 🔌 归属扩展：navis-session ──────────────────────────
// 流式传输控制（只有函数，无 state）
// 调用方直接 import { stopActiveChatStream } from '@/stores/chat-turn-stream'

// ── 🔌 归属扩展：navis-agent-core ────────────────────────
// Composer 输入状态（signal state）
export {
  composerInputValue,
  setComposerInputValue,
  composerInputFocusToken,
  setComposerInputFocusToken,
} from './composer-input';

// ── 🔌 归属扩展：navis-agent-core ────────────────────────
// Composer 菜单（只有 const + 函数，无 state）
// 调用方直接 import { buildPermissionMenuItems } from '@/stores/composer-menu'

// ── 🔌 归属扩展：navis-agent-core ────────────────────────
// Composer 运行状态（task 队列、plan mode、goal tracking）
export {
  composerRunState,
} from './composer-run';
export type {
  ComposerTask,
  CreateComposerTaskOptions,
  PendingPlanReview,
  ComposerTaskKind,
} from './composer-run';

// ── 🔌 归属扩展：navis-agent-core ────────────────────────
// Composer session 解析（只有函数，无 state）
// 调用方直接 import { resolvedComposerModelSelection } from '@/stores/composer-session'

// ── 🔌 归属扩展：navis-task ─────────────────────────────
// 任务投影状态（子 agent 运行态、时间线）
export {
  taskProjectionState,
  setTaskProjectionState,
} from './task-projection';
export type {
  TaskProjection,
} from './task-projection';

// ── 🔌 归属扩展：navis-settings ─────────────────────────
// Session 待办事项状态
export {
  sessionTodosState,
  setSessionTodosState,
} from './session-todos';
export type {
  SessionTodoItem,
} from './session-todos';

// ── 🔌 归属扩展：navis-ai-platform ──────────────────────
// 网关菜单（只有函数，无 state）
// 调用方直接 import { gatewayMenuSelectedCommands } from '@/stores/gateway-menu'

// ── 🏠 框架层：discovery ────────────────────────────────
// 扩展发现状态
export {
  extensionDiscoveryState,
  setExtensionDiscoveryState,
} from './discovery';
export type {
  ExtensionDiscoveryQuery,
} from './discovery';

// ── 🏠 框架层：menu ─────────────────────────────────────
// 菜单系统状态
export {
  menuState,
  setMenuState,
  getMenuItems,
  isMenuOpen,
  openMenu,
  closeMenu,
  toggleMenu,
  loadMenus,
  installMenuDismissHandlers,
  registerMenuItems,
  unregisterMenuItems,
} from './menu';
export type {
  MenuActionItem,
  MenuTarget,
  MenuRisk,
  MenuHostViewTarget,
  MenuBuiltinAction,
} from './menu';

// ── 🏠 框架层：menu-actions ─────────────────────────────
// 声明式菜单动作分发（只有函数，无 state）
// 调用方直接 import { executeDeclarativeMenuAction } from '@/stores/menu-actions'

// ── 🏠 框架层：menu-command-coverage ─────────────────────
// 内置菜单命令覆盖检查（只有 const + 函数，无 state）
export {
  BUILTIN_MENU_COMMAND_COVERAGE,
} from './menu-command-coverage';
export type {
  BuiltinMenuCommandCoverage,
} from './menu-command-coverage';

// ── 🏠 框架层：slash-commands ───────────────────────────
// 斜杠命令加载（只有函数，无 state）
// 调用方直接 import { loadSlashCommands } from '@/stores/slash-commands'

// ── 🏠 框架层：view-navigation ──────────────────────────
// 视图导航（只有函数，无 state）
// 调用方直接 import { navigateToBuiltinView } from '@/stores/view-navigation'

// ── 🏠 框架层：bridge ───────────────────────────────────
// 扩展 Worker 桥接（只有 const + 函数 + 类型，无 state）
export {
  NAVIS_SHIM_SOURCE,
} from './bridge';
export type {
  BridgeContextSnapshot,
  ExtensionWorkerBridgeOptions,
} from './bridge';

// ── 🏠 框架层：editor-context-transition ────────────────
// 编辑器上下文切换守卫（只有函数，无 state）
// 调用方直接 import { selectSessionWithEditorGuard } from '@/stores/editor-context-transition'

// ── 🏠 框架层：tools-menu ───────────────────────────────
// 工具菜单执行（只有函数，无 state）
// 调用方直接 import { executeToolsMenuItem } from '@/stores/tools-menu'

// ── 🏠 框架层：session-menu ─────────────────────────────
// 会话菜单操作（只有函数 + 类型，无 state）
// 调用方直接 import { getSessionMenuItems } from '@/stores/session-menu'

// ── 🏠 框架层：context-usage ────────────────────────────
// 上下文用量显示（只有函数 + 类型，无 state）
// 调用方直接 import { contextUsageDisplaySnapshot } from '@/stores/context-usage'
