/**
 * 统一图标库 — 从 router/index.tsx 和 layouts/Toolbar.tsx 提取的内联 SVG 图标。
 *
 * 所有图标使用 named export，保持原始 SVG 属性不变。
 */
import { Component } from 'solid-js';
import folderAddUrl from '@project-ext/assets/folder-add.svg';
import guideQueuedTaskUrl from '@agent-core/assets/guide-queued-task.svg';

type IconProps = {
  class?: string;
};

// ── Router 图标 ────────────────────────────────────────────

export const ScreenIcon: Component = () => (
  <svg width="16" height="16" viewBox="0 0 18 18" fill="none" stroke="currentColor" stroke-width="1.4">
    <rect x="2.5" y="3" width="13" height="9" rx="1.5" />
    <path d="M7 15h4M9 12v3" stroke-linecap="round" />
  </svg>
);

export const PanelIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 17 17" fill="none" stroke="currentColor" stroke-width="1.4">
    <rect x="2.5" y="2.5" width="12" height="12" rx="1.5" />
    <path d="M10.5 3v11" />
  </svg>
);

export const SendIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M8 12V4M8 4L4.5 7.5M8 4l3.5 3.5" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const StopIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
    <rect x="4.5" y="4.5" width="7" height="7" rx="1.2" />
  </svg>
);

export const PlusIcon: Component = () => (
  <svg width="13" height="13" viewBox="0 0 15 15" fill="none" stroke="currentColor" stroke-width="1.6">
    <path d="M7.5 3v9M3 7.5h9" stroke-linecap="round" />
  </svg>
);

export const ChevronDown: Component = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1.4">
    <path d="M3 4.5 6 7.5l3-3" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const PaperclipIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <path
      d="M5.1 8.4 9.3 4.2a2.3 2.3 0 0 1 3.2 3.2l-5.1 5.1a3.2 3.2 0 0 1-4.5-4.5l5.3-5.3"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
);

export const FolderIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <path d="M2.4 5.2h4l1.1 1.2h6.1v5.4a1.4 1.4 0 0 1-1.4 1.4H3.8a1.4 1.4 0 0 1-1.4-1.4V5.2Z" />
    <path d="M2.4 5.2V4.4A1.4 1.4 0 0 1 3.8 3h2l1.1 1.2h5.3a1.4 1.4 0 0 1 1.4 1.4v.8" />
  </svg>
);

export const FolderPlusIcon: Component = () => (
  <span
    class="navis-folder-add-icon"
    style={{ '--navis-folder-add-url': `url("${folderAddUrl}")` }}
    aria-hidden="true"
  />
);

export const ConnectorIcon: Component<IconProps> = (props) => (
  <svg class={props.class} width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.25" aria-hidden="true">
    <path d="M4.8 4.8 9.3 7.2M4.8 11.2 9.3 8.8" stroke-linecap="round" />
    <circle cx="3.2" cy="4.5" r="1.5" />
    <circle cx="3.2" cy="11.5" r="1.5" />
    <circle cx="11.8" cy="8" r="1.5" />
  </svg>
);

export const TerminalIcon: Component<IconProps> = (props) => (
  <svg class={props.class} width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true">
    <rect x="2.2" y="2.8" width="11.6" height="10.4" rx="2" stroke-width="1.25" />
    <path d="m5 6.5 2 1.5-2 1.5M8.8 9.6h2.2" stroke-width="1.25" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const AgentIcon: Component<IconProps> = (props) => (
  <svg class={props.class} width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" aria-hidden="true">
    <circle cx="8" cy="4.8" r="2" stroke-width="1.2" />
    <path d="M3.8 13.2c.45-2.25 1.9-3.5 4.2-3.5s3.75 1.25 4.2 3.5" stroke-width="1.2" stroke-linecap="round" />
  </svg>
);

export const ChecklistIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <path d="m2.4 4.2.9.9 1.7-2" stroke-linecap="round" stroke-linejoin="round" />
    <path d="M7 4.2h6.2M7 11.5h6.2" stroke-linecap="round" />
    <path d="m2.4 11.2.9.9 1.7-2" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const TargetIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <circle cx="8" cy="8" r="5.5" />
    <circle cx="8" cy="8" r="2.2" />
    <path d="M11.8 4.2 13 3M12.2 3h.8v.8" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const MultiAgentIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <circle cx="4.2" cy="4.5" r="1.8" />
    <circle cx="11.8" cy="4.5" r="1.8" />
    <circle cx="8" cy="11.5" r="2" />
    <path d="M5.2 6.1 7.1 9.7M10.8 6.1 8.9 9.7M5.8 11.5H3.4M10.2 11.5h2.4" stroke-linecap="round" />
  </svg>
);

export const ExtensionIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <path d="M6.1 2.8h3.8v3.3h3.3v3.8H9.9v3.3H6.1V9.9H2.8V6.1h3.3V2.8Z" stroke-linejoin="round" />
  </svg>
);

export const SlashIcon: Component = () => (
  <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.45">
    <path d="M10.5 2.7 5.5 13.3" stroke-linecap="round" />
  </svg>
);

export const EditIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <path d="M3.2 11.8 3 13l1.2-.2 7.5-7.5-1-1-7.5 7.5Z" stroke-linejoin="round" />
    <path d="m10 4 1-1a1.1 1.1 0 0 1 1.6 0l.4.4a1.1 1.1 0 0 1 0 1.6l-1 1" />
  </svg>
);

export const PauseCircleIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <circle cx="8" cy="8" r="5.6" />
    <path d="M6.6 5.8v4.4M9.4 5.8v4.4" stroke-linecap="round" />
  </svg>
);

export const TrashIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <path d="M3 4.5h10M6.4 4.5V3.2h3.2v1.3M5 6.2l.4 6.2h5.2l.4-6.2" stroke-linecap="round" />
  </svg>
);

export const QuoteIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <path d="M3 5.5h4v4H4.8c0 1.2.5 2 1.5 2.6M9 5.5h4v4h-2.2c0 1.2.5 2 1.5 2.6" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const GuideIcon: Component = () => (
  <span
    class="navis-guide-queued-icon"
    style={{ '--navis-guide-queued-url': `url("${guideQueuedTaskUrl}")` }}
    aria-hidden="true"
  />
);

// ── Toolbar 图标 ───────────────────────────────────────────

export const IconHamburger: Component = () => (
  <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
    <rect x="2" y="3" width="12" height="1.5" rx="0.75" />
    <rect x="2" y="7.25" width="12" height="1.5" rx="0.75" />
    <rect x="2" y="11.5" width="12" height="1.5" rx="0.75" />
  </svg>
);

export const IconSidebar: Component = () => (
  <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <rect x="2" y="2.5" width="12" height="11" rx="1.5" />
    <path d="M6 3v10" />
  </svg>
);

export const IconSearch: Component = () => (
  <svg width="15" height="15" viewBox="0 0 15 15" fill="none" stroke="currentColor" stroke-width="1.5">
    <circle cx="6.5" cy="6.5" r="4.2" />
    <path d="M10 10l3 3" stroke-linecap="round" />
  </svg>
);

export const IconArrowLeft: Component = () => (
  <svg width="15" height="15" viewBox="0 0 15 15" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M9 3.5L5 7.5l4 4" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const IconArrowRight: Component = () => (
  <svg width="15" height="15" viewBox="0 0 15 15" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M6 3.5l4 4-4 4" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);

export const IconMinimize: Component = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
    <rect x="1" y="5.5" width="10" height="1" rx="0.5" />
  </svg>
);

export const IconMaximize: Component = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
    <rect x="1.5" y="1.5" width="9" height="9" rx="1" />
  </svg>
);

export const IconRestore: Component = () => (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
    <rect x="3" y="0.5" width="7.5" height="7.5" rx="1" />
    <rect x="1" y="3" width="7.5" height="7.5" rx="1" fill="#ffffff" />
  </svg>
);

// ── RightWorkspaceHeader 紧凑图标 ─────────────────────────

export const PanelIconCompact: Component = () => (
  <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4">
    <rect x="2.5" y="3" width="11" height="10" rx="1.5" />
    <path d="M10.2 3v10" />
  </svg>
);

export const ChevronDownCompact: Component = () => (
  <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7">
    <path d="m4 6 4 4 4-4" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
);
