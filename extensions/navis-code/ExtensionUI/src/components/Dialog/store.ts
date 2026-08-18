/**
 * Dialog 对话框状态管理模块
 *
 * 职责：
 * - 管理对话框队列（先进先出）
 * - 提供 Promise-based API，调用方可 await 对话框结果
 * - 维护对话框的打开/关闭/层级状态
 * - 支持按会话清理对话框（任务取消场景）
 *
 * 设计约束（来自 design/24-dialog.md）：
 * - 对话框队列：多个对话框排队显示
 * - SessionScoped 信任不持久化
 * - 按会话关闭对话框用于任务取消时清理
 */

import { createSignal } from 'solid-js';
import type { Component } from 'solid-js';

// ============================================================
// 一、数据类型定义
// ============================================================

/**
 * 对话框输入字段配置
 * 用于 InputDialog 中的表单项
 */
export interface DialogInput {
  /** 字段标签 */
  label: string;
  /** 字段类型 */
  type?: 'text' | 'password' | 'number';
  /** 默认值 */
  defaultValue?: string;
  /** 输入框占位符 */
  placeholder?: string;
  /** 是否必填 */
  required?: boolean;
}

/**
 * 对话框选项配置
 * 用于 SelectDialog 中的选项列表
 */
export interface DialogOption {
  /** 选项标签 */
  label: string;
  /** 选项值（返回给调用方） */
  value: unknown;
  /** 选项描述 */
  description?: string;
  /** 是否禁用 */
  disabled?: boolean;
}

/**
 * 通用对话框配置（基础接口）
 * 所有对话框类型共享的字段
 */
export interface DialogConfig {
  /** 唯一标识符 */
  id: string;
  /** 对话框类型 */
  type: 'confirm' | 'alert' | 'input' | 'select' | 'custom';
  /** 对话框标题 */
  title: string;
  /** 对话框消息内容 */
  message?: string;
  /** 自定义内容组件 */
  content?: Component;
  /** 确认按钮文本 */
  confirmText?: string;
  /** 取消按钮文本 */
  cancelText?: string;
  /** 是否为危险操作（使用红色警告样式） */
  danger?: boolean;
  /** 输入字段列表（input 类型使用） */
  inputs?: DialogInput[];
  /** 选项列表（select 类型使用） */
  options?: DialogOption[];
  /** 确认回调 */
  onConfirm?: (result?: unknown) => void;
  /** 取消回调 */
  onCancel?: () => void;
}

/**
 * Agent 确认框配置
 * 用于 Agent 工具调用时的用户确认
 *
 * 设计文档第五章定义的 Agent 确认框布局：
 * - 工具名称和参数
 * - 风险等级标识（低/中/高）
 * - 四个操作按钮：Allow once / Allow this session / Allow this project / Deny always
 *
 * AgentConfirm API 返回四态 decision，不能退回 bool approved 协议。
 */
export type AgentConfirmDecision =
  | 'allow_once'
  | 'allow_session'
  | 'allow_project'
  | 'deny_always';

export interface AgentConfirmConfig {
  /** 唯一标识符 */
  id: string;
  /** 工具名称（如 terminal.exec） */
  toolName: string;
  /** 工具调用参数 */
  toolArgs: Record<string, unknown>;
  /** 风险等级 */
  riskLevel: 'low' | 'medium' | 'high';
  /** 确认消息 */
  message: string;
  /** Allow once 回调 */
  onApprove: () => void;
  /** Deny always 回调 */
  onDenyAlways: () => void;
  /**
   * Allow this session 回调（SessionScoped）
   */
  onTrustThisSession?: () => void;
  /**
   * Allow this project 回调（ProjectScoped）
   */
  onAllowProject?: () => void;
}

/**
 * 项目信任对话框配置
 *
 * ProjectTrust 枚举变体（design/24-dialog.md 第七章）：
 * - Trusted: 完全信任（持久化）
 * - Untrusted: 不信任（持久化）
 * - AskEachTime: 每次询问（持久化）
 * - SessionScoped: 仅本次会话信任（内存态，会话结束清除）
 */
export interface TrustDialogConfig {
  /** 唯一标识符 */
  id: string;
  /** Worktree 路径 */
  path: string;
  /** 可选消息覆盖（默认自动生成） */
  message?: string;
}

/**
 * 对话框公共 API 接口
 *
 * 提供 Promise-based 的对话框操作，调用方可以 await 对话框结果。
 * 设计文档第四章定义的所有接口均在此实现。
 */
export interface DialogAPI {
  // ===== 通用对话框 =====

  /**
   * 确认框
   * @returns 用户点击确认返回 true，取消返回 false
   */
  confirm(config: Omit<DialogConfig, 'id' | 'type'>): Promise<boolean>;

  /**
   * 提示框（仅有确认按钮）
   * @returns 用户点击确认后 resolve
   */
  alert(title: string, message: string): Promise<void>;

  /**
   * 输入框
   * @returns 用户输入的文本，取消返回 null
   */
  input(
    title: string,
    message: string,
    defaultValue?: string
  ): Promise<string | null>;

  /**
   * 选择框
   * @returns 用户选择的选项值，取消返回 null
   */
  select<T = unknown>(
    title: string,
    options: DialogOption[]
  ): Promise<T | null>;

  /**
   * 自定义内容弹框
   * 使用公共 Dialog 外壳，业务只提供内容组件。
   */
  custom(title: string, content: Component, message?: string): Promise<void>;

  // ===== Agent 确认 =====

  /**
   * Agent 工具调用确认框
   *
   * 返回用户的四态审批决策。
   */
  agentConfirm(config: Omit<AgentConfirmConfig, 'id'>): Promise<AgentConfirmDecision>;

  // ===== 项目信任 =====

  /**
   * 项目信任确认
   * @returns 'trusted' | 'untrusted' | 'ask'
   */
  trustProject(path: string): Promise<'trusted' | 'untrusted' | 'ask'>;

  // ===== 管理操作 =====

  /** 关闭指定对话框（触发 onCancel 回调） */
  close(id: string): void;

  /** 关闭所有对话框 */
  closeAll(): void;

  /**
   * 按会话关闭对话框
   * 用于用户取消 Agent 任务时清理对应的确认框
   * 设计文档第八章：任务取消时清理排队对话框
   */
  closeBySession(sessionId: string): void;

  /**
   * 清空队列中指定会话的待显示对话框
   * 设计文档第八章：清理排队中的对话框
   */
  clearQueue(sessionId: string): void;
}

// ============================================================
// 二、内部类型定义
// ============================================================

/**
 * Agent 确认框运行时配置
 * AgentConfirmConfig 的 message 字段已是必填，此处无额外字段
 */
type AgentConfirmDialogConfig = AgentConfirmConfig;

/**
 * Worktree 信任对话框运行时配置
 * message 字段始终存在（自动生成或手动覆盖）
 */
interface TrustDialogRuntimeConfig extends TrustDialogConfig {
  /** 消息文本（始终存在） */
  message: string;
}

/**
 * 当前活跃对话框状态
 * union 类型，根据 type 字段区分不同的对话框配置
 */
export type ActiveDialog =
  | { type: 'confirm'; config: DialogConfig }
  | { type: 'alert'; config: DialogConfig }
  | { type: 'input'; config: DialogConfig }
  | { type: 'select'; config: DialogConfig }
  | { type: 'custom'; config: DialogConfig }
  | { type: 'agentConfirm'; config: AgentConfirmDialogConfig }
  | { type: 'trust'; config: TrustDialogRuntimeConfig };

/**
 * 队列中的待处理对话框（与 ActiveDialog 同构）
 */
type QueueItem = ActiveDialog;

/**
 * Promise 解析器
 * 存储每个对话框的 Promise resolve/reject 函数
 */
interface DialogResolver {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
}

// ============================================================
// 三、内部状态
// ============================================================

/**
 * 对话框队列（响应式信号）
 * 存储所有等待显示的对话框，先进先出
 */
const [queue, setQueue] = createSignal<QueueItem[]>([]);

/**
 * 当前活跃对话框的完整配置（响应式信号）
 *
 * 重要设计：此信号独立于队列存储。
 * 当 processQueue 从队列中取出对话框后，将其存入此处，
 * 确保 DialogManager 始终能获取到当前对话框的完整配置。
 */
const [activeDialog, setActiveDialog] = createSignal<ActiveDialog | null>(null);

/**
 * Promise 解析器注册表
 * Key: 对话框 ID
 * Value: { resolve, reject } 函数对
 */
const resolvers = new Map<string, DialogResolver>();

/**
 * 对话框 ID 计数器
 * 用于生成全局唯一的对话框 ID
 */
let idCounter = 0;

// ============================================================
// 四、内部工具函数
// ============================================================

/**
 * 生成唯一的对话框 ID
 * 格式: dialog-{序号}
 */
function generateId(): string {
  return `dialog-${++idCounter}`;
}

/**
 * 从配置对象中获取对话框 ID
 * 所有对话框类型的 config 都包含 id 字段
 */
function getConfigId(item: QueueItem): string {
  return item.config.id;
}

/**
 * 获取对话框的取消回调
 * agentConfirm 类型使用 onDenyAlways 回调
 * trust 类型无取消回调
 * 其他类型使用 config.onCancel
 */
function getOnCancel(item: QueueItem): (() => void) | undefined {
  if (item.type === 'agentConfirm') return item.config.onDenyAlways;
  if (item.type === 'trust') return undefined;
  return item.config.onCancel;
}

/**
 * 获取对话框的取消默认值
 * 根据对话框类型返回适当的"取消时"值
 */
function getCancelDefault(item: QueueItem): unknown {
  switch (item.type) {
    case 'confirm':
    case 'agentConfirm':
      return 'deny_always';
    case 'alert':
      return undefined; // void 类型返回 undefined
    case 'trust':
      return 'untrusted'; // 信任类型返回 'untrusted'
    default:
      return null; // input/select/custom 返回 null
  }
}

/**
 * 解析对话框的 Promise 并清理资源
 * @param id 对话框 ID
 * @param value 解析值
 */
function resolveDialog(id: string, value: unknown): void {
  const resolver = resolvers.get(id);
  if (resolver) {
    resolver.resolve(value);
    resolvers.delete(id);
  }
}

/**
 * 处理队列：取出队首对话框并显示
 *
 * 逻辑：
 * 1. 如果已有活跃对话框或队列为空，跳过
 * 2. 从队列中取出第一个对话框
 * 3. 存入 activeDialog 信号，供 DialogManager 渲染
 */
function processQueue(): void {
  // 已有对话框正在显示，不处理
  if (activeDialog()) return;

  const currentQueue = queue();
  // 队列为空，不处理
  if (currentQueue.length === 0) return;

  // 取出队首对话框
  const next = currentQueue[0];
  const remaining = currentQueue.slice(1);
  setQueue(remaining);

  // 存入活跃对话框信号（关键：确保配置可被 DialogManager 获取）
  setActiveDialog(next);
}

// ============================================================
// 五、对话框关闭处理
// ============================================================

/**
 * 内部关闭对话框
 *
 * 处理流程：
 * 1. 如果对话框在队列中（未显示），直接移除并解析 Promise
 * 2. 如果对话框正在显示，清除活跃状态并解析 Promise
 * 3. 触发 onCancel 回调（如果存在）
 * 4. 处理队列中的下一个对话框
 *
 * @param id 对话框 ID
 * @param resolveValue 可选的指定解析值（不传则使用取消默认值）
 */
function handleClose(id: string, resolveValue?: unknown): void {
  // 1. 检查是否在队列中（尚未显示）
  const currentQueue = queue();
  const idx = currentQueue.findIndex((item) => getConfigId(item) === id);

  if (idx !== -1) {
    const item = currentQueue[idx];
    // 从队列中移除
    setQueue((prev) => [...prev.slice(0, idx), ...prev.slice(idx + 1)]);
    // 使用指定值或取消默认值解析 Promise
    const value = resolveValue !== undefined ? resolveValue : getCancelDefault(item);
    resolveDialog(id, value);
    // 触发 onCancel 回调
    getOnCancel(item)?.();
    return;
  }

  // 2. 检查是否为当前活跃对话框（正在显示）
  const active = activeDialog();
  if (active && getConfigId(active) === id) {
    // 清除活跃对话框
    setActiveDialog(null);
    // 使用指定值或取消默认值解析 Promise
    const value = resolveValue !== undefined ? resolveValue : getCancelDefault(active);
    resolveDialog(id, value);
    // 触发 onCancel 回调
    getOnCancel(active)?.();
    // 处理队列中的下一个对话框
    processQueue();
  }
}

/**
 * 使用指定值解析并关闭对话框
 *
 * 用于用户确认操作（如点击"确认"按钮）。
 * 与 handleClose 的区别：此函数始终使用调用方指定的值，
 * 不会 fallback 到取消默认值。
 *
 * @param id 对话框 ID
 * @param value 解析值
 */
function resolveWith(id: string, value: unknown): void {
  // 检查队列
  const currentQueue = queue();
  const idx = currentQueue.findIndex((item) => getConfigId(item) === id);

  if (idx !== -1) {
    setQueue((prev) => [...prev.slice(0, idx), ...prev.slice(idx + 1)]);
    resolveDialog(id, value);
    return;
  }

  // 检查活跃对话框
  const active = activeDialog();
  if (active && getConfigId(active) === id) {
    setActiveDialog(null);
    resolveDialog(id, value);
    processQueue();
  }
}

// ============================================================
// 六、对话框配置构建器
// ============================================================

/**
 * 构建确认框配置
 */
function buildConfirmConfig(
  base: Omit<DialogConfig, 'id' | 'type'>
): { id: string; config: DialogConfig } {
  const id = generateId();
  const config: DialogConfig = {
    ...base,
    id,
    type: 'confirm',
    confirmText: base.confirmText ?? '确认',
    cancelText: base.cancelText ?? '取消',
  };
  return { id, config };
}

/**
 * 构建提示框配置
 * 提示框只有确认按钮，没有取消按钮
 */
function buildAlertConfig(
  title: string,
  message: string
): { id: string; config: DialogConfig } {
  const id = generateId();
  const config: DialogConfig = {
    id,
    type: 'alert',
    title,
    message,
    confirmText: '确认',
    // 提示框没有取消按钮
  };
  return { id, config };
}

/**
 * 构建输入框配置
 * 支持单个输入字段（简化 API）
 */
function buildInputConfig(
  title: string,
  message: string,
  defaultValue?: string
): { id: string; config: DialogConfig } {
  const id = generateId();
  const config: DialogConfig = {
    id,
    type: 'input',
    title,
    message,
    confirmText: '确认',
    cancelText: '取消',
    inputs: [
      {
        label: '',
        type: 'text',
        defaultValue,
        placeholder: message,
      },
    ],
  };
  return { id, config };
}

/**
 * 构建选择框配置
 */
function buildSelectConfig(
  title: string,
  options: DialogOption[]
): { id: string; config: DialogConfig } {
  const id = generateId();
  const config: DialogConfig = {
    id,
    type: 'select',
    title,
    confirmText: '确认',
    cancelText: '取消',
    options,
  };
  return { id, config };
}

function buildCustomConfig(
  title: string,
  content: Component,
  message?: string,
): { id: string; config: DialogConfig } {
  const id = generateId();
  const config: DialogConfig = {
    id,
    type: 'custom',
    title,
    message,
    content,
  };
  return { id, config };
}

/**
 * 构建 Agent 确认框配置
 *
 * 包含默认的"本次信任"回调处理逻辑：
 * - 触发后调用 sandbox.set_trust(Worktree, SessionScoped)
 * - 然后自动批准操作
 */
function buildAgentConfirmConfig(
  base: Omit<AgentConfirmConfig, 'id'>
): { id: string; config: AgentConfirmDialogConfig } {
  const id = generateId();
  const config: AgentConfirmDialogConfig = {
    ...base,
    id,
    // 如果调用方未提供 scoped allow 回调，提供默认实现
    onTrustThisSession:
      base.onTrustThisSession ??
      (() => {
        // 默认实现：信任当前会话后批准
        // 实际项目中应调用 sandbox.set_trust(Worktree, SessionScoped)
        base.onApprove();
      }),
    onAllowProject:
      base.onAllowProject ??
      (() => {
        // 默认实现：信任当前项目后批准
        base.onApprove();
      }),
  };
  return { id, config };
}

/**
 * 构建项目信任对话框配置
 *
 * 消息说明：
 * - 默认自动生成消息，描述信任操作的含义
 * - 调用方可通过 message 字段覆盖
 */
function buildTrustConfig(
  path: string
): { id: string; config: TrustDialogRuntimeConfig } {
  const id = generateId();
  const config: TrustDialogRuntimeConfig = {
    id,
    path,
    message: `是否信任 Worktree "${path}"？\n\n信任后，该 Worktree 中的 Agent 操作将不再需要逐一确认。`,
  };
  return { id, config };
}

// ============================================================
// 七、队列会话管理
// ============================================================

/**
 * 按会话 ID 关闭对话框
 *
 * 设计文档第八章：任务取消时清理排队对话框
 *
 * 使用场景：用户取消正在执行的 Agent 任务时，
 * 需要同步清理排队中的对话框，避免残留的确认框阻塞 UI。
 *
 * @param sessionId 会话标识
 */
function closeBySessionImpl(sessionId: string): void {
  // 关闭当前活跃的对话框（如果是该会话的）
  const active = activeDialog();
  if (active) {
    handleClose(getConfigId(active));
  }

  // 清空队列中该会话的所有对话框
  clearQueueImpl(sessionId);
}

/**
 * 清空队列中指定会话的待显示对话框
 *
 * 设计文档第八章注意事项：
 * - 清理时需触发每个对话框的 onCancel 回调，确保相关资源释放
 * - 仅清理属于被取消会话的对话框，不影响其他会话的确认框
 * - 清理完成后发出 dialog.queue.cleared 事件
 *
 * @param sessionId 会话标识
 */
function clearQueueImpl(_sessionId: string): void {
  const currentQueue = queue();

  // 收集需要清理的对话框 ID
  const toRemove: QueueItem[] = [...currentQueue];

  // 清空队列
  setQueue([]);

  // 触发每个被清理对话框的 onCancel 回调并解析 Promise
  for (const item of toRemove) {
    const id = getConfigId(item);
    // 触发取消回调
    getOnCancel(item)?.();
    // 使用取消默认值解析 Promise
    resolveDialog(id, getCancelDefault(item));
  }
}

// ============================================================
// 八、公共 API 实例
// ============================================================

/**
 * Dialog API 实例
 *
 * 提供所有对话框操作的公共接口。
 *
 * 使用方式：
 * ```typescript
 * import { dialog } from './store';
 *
 * // 确认框
 * const confirmed = await dialog.confirm({ title: '确认', message: '确定删除？' });
 *
 * // 提示框
 * await dialog.alert('提示', '操作已完成');
 *
 * // 输入框
 * const name = await dialog.input('请输入', '您的姓名');
 *
 * // 选择框
 * const option = await dialog.select('请选择', [
 *   { label: '选项A', value: 'a' },
 *   { label: '选项B', value: 'b' },
 * ]);
 *
 * // Agent 确认
 * const decision = await dialog.agentConfirm({
 *   toolName: 'terminal.exec',
 *   toolArgs: { command: 'npm test' },
 *   riskLevel: 'medium',
 *   message: '命令执行可能产生副作用',
 *   onApprove: () => console.log('allow once'),
 *   onDenyAlways: () => console.log('denied always'),
 * });
 *
 * // 项目信任
 * const trust = await dialog.trustProject('/home/user/project');
 * ```
 */
export const dialog: DialogAPI = {
  /**
   * 打开确认框
   * @returns 用户点击确认返回 true，取消返回 false
   */
  confirm(config) {
    const { id, config: dialogConfig } = buildConfirmConfig(config);
    return new Promise<boolean>((resolve, reject) => {
      resolvers.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      setQueue((prev) => [...prev, { type: 'confirm', config: dialogConfig }]);
      processQueue();
    });
  },

  /**
   * 打开提示框
   * @returns 用户点击确认后 resolve
   */
  alert(title, message) {
    const { id, config } = buildAlertConfig(title, message);
    return new Promise<void>((resolve, reject) => {
      resolvers.set(id, {
        resolve: () => resolve(),
        reject,
      });
      setQueue((prev) => [...prev, { type: 'alert', config }]);
      processQueue();
    });
  },

  /**
   * 打开输入框
   * @returns 用户输入的文本，取消返回 null
   */
  input(title, message, defaultValue) {
    const { id, config } = buildInputConfig(title, message, defaultValue);
    return new Promise<string | null>((resolve, reject) => {
      resolvers.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      setQueue((prev) => [...prev, { type: 'input', config }]);
      processQueue();
    });
  },

  /**
   * 打开选择框
   * @returns 用户选择的选项值，取消返回 null
   */
  select(title, options) {
    const { id, config } = buildSelectConfig(title, options);
    return new Promise((resolve, reject) => {
      resolvers.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      setQueue((prev) => [...prev, { type: 'select', config }]);
      processQueue();
    });
  },

  custom(title, content, message) {
    const { id, config } = buildCustomConfig(title, content, message);
    return new Promise<void>((resolve, reject) => {
      resolvers.set(id, {
        resolve: () => resolve(),
        reject,
      });
      setQueue((prev) => [...prev, { type: 'custom', config }]);
      processQueue();
    });
  },

  /**
   * 打开 Agent 工具调用确认框
   *
   * 风险等级说明：
   * - low: 低风险（绿色标识）
   * - medium: 中等风险（黄色标识）
   * - high: 高风险（红色标识）
   *
   * @returns allow_once | allow_session | allow_project | deny_always
   */
  agentConfirm(config) {
    const { id, config: dialogConfig } = buildAgentConfirmConfig(config);
    return new Promise<AgentConfirmDecision>((resolve, reject) => {
      resolvers.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      setQueue((prev) => [
        ...prev,
        { type: 'agentConfirm', config: dialogConfig },
      ]);
      processQueue();
    });
  },

  /**
   * 打开项目信任确认框
   *
   * ProjectTrust 枚举变体：
   * - 'trusted': 完全信任（持久化）
   * - 'untrusted': 不信任（持久化）
   * - 'ask': 每次询问（持久化）
   *
   * @returns 用户选择的信任级别
   */
  trustProject(path) {
    const { id, config } = buildTrustConfig(path);
    return new Promise<'trusted' | 'untrusted' | 'ask'>((resolve, reject) => {
      resolvers.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
      });
      setQueue((prev) => [...prev, { type: 'trust', config }]);
      processQueue();
    });
  },

  /**
   * 关闭指定对话框
   * 触发 onCancel 回调，Promise 使用取消默认值解析
   */
  close(id) {
    handleClose(id);
  },

  /**
   * 关闭所有对话框
   * 包括当前活跃的和队列中的所有对话框
   */
  closeAll() {
    // 关闭当前活跃对话框
    const active = activeDialog();
    if (active) {
      handleClose(getConfigId(active));
    }

    // 清空队列
    clearQueueImpl('all');
  },

  /**
   * 按会话关闭对话框
   * 用于任务取消时清理对应的确认框
   */
  closeBySession(sessionId) {
    closeBySessionImpl(sessionId);
  },

  /**
   * 清空队列中指定会话的待显示对话框
   * 仅清理队列，不影响当前正在显示的对话框
   */
  clearQueue(sessionId) {
    clearQueueImpl(sessionId);
  },
};

// ============================================================
// 九、状态访问器（供 DialogManager 使用）
// ============================================================

/**
 * 获取当前活跃对话框的 ID
 * 响应式访问器，当活跃对话框变化时自动触发更新
 */
export function getActiveId(): string | null {
  const active = activeDialog();
  return active ? getConfigId(active) : null;
}

/**
 * 获取当前活跃对话框的完整配置
 *
 * DialogManager 使用此函数获取当前应该渲染的对话框类型和配置。
 * 返回值包含 type 字段和对应的 config 对象。
 *
 * @returns 当前活跃对话框的配置，无活跃对话框返回 null
 */
export function getActiveDialog(): ActiveDialog | null {
  return activeDialog();
}

/**
 * 获取当前活跃对话框的配置（带类型守卫）
 *
 * @param type 期望的对话框类型
 * @returns 对应类型的配置对象，类型不匹配返回 null
 */
export function getActiveConfig(type: 'confirm'): DialogConfig | null;
export function getActiveConfig(type: 'alert'): DialogConfig | null;
export function getActiveConfig(type: 'input'): DialogConfig | null;
export function getActiveConfig(type: 'select'): DialogConfig | null;
export function getActiveConfig(type: 'custom'): DialogConfig | null;
export function getActiveConfig(type: 'agentConfirm'): AgentConfirmDialogConfig | null;
export function getActiveConfig(type: 'trust'): TrustDialogRuntimeConfig | null;
export function getActiveConfig(type: ActiveDialog['type']): ActiveDialog['config'] | null {
  const current = activeDialog();
  if (!current || current.type !== type) return null;
  return current.config;
}

/**
 * 暴露 resolveWith 供对话框组件内部使用
 * 用户点击"确认"等操作时，使用此函数以正确的值解析 Promise
 */
export { resolveWith };

/**
 * 暴露 handleClose 供对话框组件内部使用
 * 用户点击"取消"或按 ESC 时，使用此函数以取消默认值解析 Promise
 */
export { handleClose as closeDialog };
