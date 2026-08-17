/**
 * Navis 路由配置。
 *
 * 当前 ChatView 对齐 design/22-ui-framework.md 和 design/ui设计 下的区域图：
 * Chat Header / Chat Messages / Composer A / Composer B。
 */
import { Component, Show, createEffect, lazy } from 'solid-js';
import { Router, Route, Navigate } from '@solidjs/router';
import MainLayout from '../layouts/MainLayout';
import { StartWorkspace } from '../components/StartWorkspace';
import { Composer } from '../components/Composer';
import ChatHeader from '../components/Chat/ChatHeader';
import ChatMessages from '../components/Chat/ChatMessages';
import { appState } from '../stores/app';
import SettingsDialogContent from '../components/Settings/SettingsDialogContent';
import {
  chatMessageState,
  loadChatMessages,
} from '../stores/chat-messages';
import {
  activeSessionId,
} from '../stores/session-tree';

const WorktreeEditor = lazy(() => import('../components/Editor/WorktreeEditor'));

// ── 基础图标已迁移至 src/components/Icon/index.ts ──────────

// ── ChatView 组件 ───────────────────────────────────────

const ChatView: Component = () => {
  createEffect(() => {
    void loadChatMessages(activeSessionId());
  });

  const showStartWorkspace = () =>
    Boolean(appState.pendingStartKind) ||
    (Boolean(activeSessionId()) &&
      !chatMessageState.loading &&
      !chatMessageState.error &&
      chatMessageState.messages.length === 0);

  return (
    <div class="flex h-full flex-col overflow-hidden bg-white text-[#242424]">
      <Show
        when={!showStartWorkspace()}
        fallback={<StartWorkspace composer={(variant) => <Composer variant={variant} />} />}
      >
        <ChatHeader />
        <ChatMessages />
        <Composer />
      </Show>
    </div>
  );
};

// ── 其他路由视图 ────────────────────────────────────────

const EditorRouteView: Component = () => <WorktreeEditor />;
const SettingsView: Component = () => (
  <div class="navis-settings-route h-full overflow-hidden">
    <SettingsDialogContent />
  </div>
);

// ── 路由配置 ────────────────────────────────────────────

const AppRoutes: Component = () => {
  return (
    <Router root={MainLayout}>
      <Route path="/" component={() => <Navigate href="/chat" />} />
      <Route path="/chat" component={ChatView} />
      <Route path="/chat/:id" component={ChatView} />
      <Route path="/editor" component={EditorRouteView} />
      <Route path="/editor/*" component={EditorRouteView} />
      <Route path="/settings" component={SettingsView} />
    </Router>
  );
};

export default AppRoutes;
