/**
 * Dialog 模块公共 API 导出
 *
 * 对外暴露：
 * - dialog: Promise-based 对话框 API 实例
 * - DialogManager: 对话框管理器组件（放置在应用根组件）
 * - 类型定义: 供其他模块使用的 TypeScript 类型
 *
 * 使用方式：
 * ```tsx
 * // 1. 在应用根组件放置 DialogManager
 * import { DialogManager } from '@/components/Dialog';
 *
 * function App() {
 *   return (
 *     <>
 *       <MainLayout />
 *       <DialogManager />
 *     </>
 *   );
 * }
 *
 * // 2. 在业务代码中使用 dialog API
 * import { dialog } from '@/components/Dialog';
 *
 * const confirmed = await dialog.confirm({
 *   title: '确认删除',
 *   message: '确定要删除此文件吗？',
 *   danger: true,
 * });
 *
 * const name = await dialog.input('重命名', '请输入新名称', 'old-name.txt');
 *
 * // Agent 确认（返回 allow_once | allow_session | allow_project | deny_always）
 * const decision = await dialog.agentConfirm({
 *   toolName: 'terminal.exec',
 *   toolArgs: { command: 'npm test' },
 *   riskLevel: 'medium',
 *   message: '命令执行可能产生副作用',
 *   onApprove: () => { /* 执行操作 *\/ },
 *   onDenyAlways: () => { /* 持续拒绝 *\/ },
 *   onAllowProject: () => { /* 当前项目内允许 *\/ },
 * });
 * if (decision === 'allow_once') { /* 执行本次操作 *\/ }
 * ```
 */

// ===== 对话框 API =====

/** Promise-based 对话框操作 API */
export { dialog } from './store';

// ===== 组件 =====

/** 对话框管理器组件 */
export { default as DialogManager } from './DialogManager';

// ===== 类型定义 =====

export type {
  /** 通用对话框配置 */
  DialogConfig,
  /** 对话框输入字段配置 */
  DialogInput,
  /** 对话框选项配置 */
  DialogOption,
  /** Agent 确认框配置 */
  AgentConfirmConfig,
  /** Agent 确认四态决策 */
  AgentConfirmDecision,
  /** Worktree 信任对话框配置 */
  TrustDialogConfig,
  /** 对话框公共 API 接口 */
  DialogAPI,
  /** 活跃对话框联合类型 */
  ActiveDialog,
} from './store';
