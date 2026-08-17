/**
 * ConfirmDialog 确认对话框组件
 *
 * 通用确认框，支持：
 * - 自定义标题和消息
 * - 确认/取消按钮
 * - 危险操作样式（红色警告）
 * - 自定义内容组件
 * - Enter 键确认
 *
 * 对应 design/24-dialog.md 中 DialogConfig 的 'confirm' 类型
 */

import { Component } from 'solid-js';
import type { DialogConfig } from './store';

/**
 * ConfirmDialog 组件属性
 */
interface ConfirmDialogProps {
  /** 对话框配置 */
  config: DialogConfig;
  /** 确认回调（解析 Promise 为 true） */
  onConfirm: () => void;
  /** 取消回调（解析 Promise 为 false） */
  onCancel: () => void;
}

/**
 * 确认对话框组件
 *
 * 渲染一个标准的确认/取消对话框。
 * 支持 Enter 键快速确认，ESC 由 Kobalte Dialog 自动处理。
 *
 * 设计文档中的通用对话框接口：
 * dialog.confirm(config): Promise<boolean>
 */
const ConfirmDialog: Component<ConfirmDialogProps> = (props) => {
  /**
   * 处理键盘事件
   * Enter 键触发确认操作
   */
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      props.onConfirm();
    }
  };

  return (
    // tabindex="-1" 使容器可接收键盘事件（Enter 确认）
    <div class="navis-dialog-body" tabindex="-1" onKeyDown={handleKeyDown}>
      {/* 对话框消息内容 */}
      {props.config.message && (
        <p class="navis-dialog-message">{props.config.message}</p>
      )}

      {/* 自定义内容（如果提供） */}
      {props.config.content && <props.config.content />}

      {/* 操作按钮区域 */}
      <div class="navis-dialog-actions">
        {/* 取消按钮 */}
        {props.config.cancelText && (
          <button
            type="button"
            onClick={props.onCancel}
            class="navis-dialog-button is-secondary"
          >
            {props.config.cancelText}
          </button>
        )}

        {/* 确认按钮（危险操作使用红色样式） */}
        <button
          type="button"
          onClick={props.onConfirm}
          autofocus
          class={`navis-dialog-button ${props.config.danger ? 'is-danger' : 'is-primary'}`}
        >
          {props.config.confirmText ?? '确认'}
        </button>
      </div>
    </div>
  );
};

export default ConfirmDialog;


