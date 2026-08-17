/**
 * AlertDialog 提示对话框组件
 *
 * 简单的消息提示框，仅有确认按钮，无取消操作。
 * 用于通知用户某些信息，不涉及决策。
 *
 * 对应 design/24-dialog.md 中 DialogConfig 的 'alert' 类型
 */

import { Component } from 'solid-js';
import type { DialogConfig } from './store';

/**
 * AlertDialog 组件属性
 */
interface AlertDialogProps {
  /** 对话框配置 */
  config: DialogConfig;
  /** 确认回调（关闭对话框） */
  onConfirm: () => void;
}

/**
 * 提示对话框组件
 *
 * 渲染一个仅有确认按钮的消息提示框。
 * Enter 键和 ESC 均触发关闭。
 *
 * 设计文档中的接口：
 * dialog.alert(title, message): Promise<void>
 */
const AlertDialog: Component<AlertDialogProps> = (props) => {
  /**
   * 处理键盘事件
   * Enter 键触发确认（关闭）
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
      {/* 提示消息 */}
      {props.config.message && (
        <p class="navis-dialog-message">{props.config.message}</p>
      )}

      {/* 自定义内容（如果提供） */}
      {props.config.content && <props.config.content />}

      {/* 仅有确认按钮 */}
      <div class="navis-dialog-actions">
        <button
          type="button"
          onClick={props.onConfirm}
          autofocus
          class="navis-dialog-button is-primary"
        >
          {props.config.confirmText ?? '确认'}
        </button>
      </div>
    </div>
  );
};

export default AlertDialog;


