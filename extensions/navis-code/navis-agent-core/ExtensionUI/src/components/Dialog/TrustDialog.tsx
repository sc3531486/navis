/**
 * TrustDialog 项目信任确认对话框组件
 *
 * 当首次打开项目时弹出，询问用户对该项目的信任级别。
 *
 * ProjectTrust 枚举变体（design/24-dialog.md 第七章）：
 * - Trusted: 完全信任（持久化）—— Agent 操作无需确认
 * - Untrusted: 不信任（持久化）—— Agent 操作全部需要确认
 * - AskEachTime: 每次询问（持久化）—— 每次操作都弹确认框
 *
 * 与 SessionScoped 的区别：
 * - TrustDialog 的选择是持久化的，重启后仍然有效
 * - SessionScoped 仅当前会话有效，重启后不保留
 */

import { Component } from 'solid-js';

/**
 * TrustDialog 组件属性
 */
interface TrustDialogProps {
  /** Worktree 路径 */
  path: string;
  /** 提示消息 */
  message: string;
  /**
   * 确认信任回调（信任此项目）
   * 返回 'trusted'（完全信任）
   */
  onTrust: () => void;
  /**
   * 不信任回调（不信任此项目）
   * 返回 'untrusted'（不信任）
   */
  onUntrust: () => void;
  /**
   * 每次询问回调（每次操作前询问）
   * 返回 'ask'（AskEachTime）
   */
  onAsk: () => void;
}

/**
 * 项目信任确认对话框组件
 *
 * 设计文档第四章接口：
 * dialog.trustProject(path): Promise<'trusted' | 'untrusted' | 'ask'>
 *
 * 渲染一个三选一对话框，让用户决定对项目的信任级别。
 * 每个选择都有详细说明，帮助用户理解后果。
 */
const TrustDialog: Component<TrustDialogProps> = (props) => {
  return (
    <div class="navis-dialog-body">
      <p class="navis-dialog-message">{props.message}</p>

      {/* Worktree / Project 路径展示 */}
      <div class="navis-dialog-code-block">
        <div class="navis-dialog-code-row">
          <span class="navis-dialog-code-key">路径</span>
          <span class="navis-dialog-code-value">{props.path}</span>
        </div>
      </div>

      {/* 信任选项列表 */}
      <div class="navis-dialog-option-list">
        {/* 选项 1：完全信任 */}
        <button
          type="button"
          onClick={props.onTrust}
          class="navis-dialog-option"
        >
          <span class="navis-dialog-option-label">信任此项目</span>
          <span class="navis-dialog-option-description">
            Agent 可以自由操作此项目中的文件和命令，无需逐一确认。
            适合你信任的项目。
          </span>
        </button>

        {/* 选项 2：每次询问 */}
        <button
          type="button"
          onClick={props.onAsk}
          class="navis-dialog-option"
        >
          <span class="navis-dialog-option-label">每次询问</span>
          <span class="navis-dialog-option-description">
            每次 Agent 操作都需要你确认。适合不确定安全性的项目。
          </span>
        </button>

        {/* 选项 3：不信任 */}
        <button
          type="button"
          onClick={props.onUntrust}
          class="navis-dialog-option"
        >
          <span class="navis-dialog-option-label">不信任此项目</span>
          <span class="navis-dialog-option-description">
            拒绝 Agent 对此项目的任何操作。适合不受信任的代码。
          </span>
        </button>
      </div>
    </div>
  );
};

export default TrustDialog;


