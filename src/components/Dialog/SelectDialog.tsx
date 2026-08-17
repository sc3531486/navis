/**
 * SelectDialog 选择对话框组件
 *
 * 带选项列表的对话框，支持：
 * - 单选选项列表
 * - 选项描述
 * - 禁用选项
 * - 键盘导航
 *
 * 对应 design/24-dialog.md 中 DialogConfig 的 'select' 类型
 */

import { Component, createSignal, For } from 'solid-js';
import type { DialogConfig, DialogOption } from './store';

/**
 * SelectDialog 组件属性
 */
interface SelectDialogProps {
  /** 对话框配置（包含 options 字段） */
  config: DialogConfig;
  /**
   * 确认回调
   * @param value 用户选择的选项值
   */
  onConfirm: (value: unknown) => void;
  /** 取消回调 */
  onCancel: () => void;
}

/**
 * 选择对话框组件
 *
 * 渲染一个选项列表，用户可以选择其中一个选项。
 * 支持鼠标点击和键盘导航（上下箭头 + Enter 确认）。
 *
 * 设计文档中的接口：
 * dialog.select(title, options): Promise<any | null>
 */
const SelectDialog: Component<SelectDialogProps> = (props) => {
  /** 选项列表 */
  const options = props.config.options ?? [];

  /**
   * 当前选中的选项索引
   * -1 表示未选中任何选项
   */
  const [selectedIndex, setSelectedIndex] = createSignal<number>(-1);

  /**
   * 处理选项点击
   * @param index 被点击的选项索引
   */
  const handleOptionClick = (index: number) => {
    const option = options[index];
    // 跳过禁用的选项
    if (option?.disabled) return;
    setSelectedIndex(index);
  };

  /**
   * 处理选项双击
   * 双击直接确认选择
   * @param index 被双击的选项索引
   */
  const handleOptionDoubleClick = (index: number) => {
    const option = options[index];
    if (option?.disabled) return;
    props.onConfirm(option.value);
  };

  /**
   * 处理确认操作
   * 如果已选中选项，返回其值
   */
  const handleConfirm = () => {
    const idx = selectedIndex();
    if (idx >= 0 && idx < options.length) {
      props.onConfirm(options[idx].value);
    }
  };

  /**
   * 处理键盘事件
   * - 上下箭头：导航选项
   * - Enter：确认选中项
   */
  const handleKeyDown = (e: KeyboardEvent) => {
    switch (e.key) {
      case 'ArrowUp': {
        e.preventDefault();
        const current = selectedIndex();
        // 向上查找上一个非禁用选项
        let prev = current <= 0 ? options.length - 1 : current - 1;
        while (prev >= 0 && options[prev]?.disabled) {
          prev--;
        }
        if (prev >= 0) setSelectedIndex(prev);
        break;
      }
      case 'ArrowDown': {
        e.preventDefault();
        const current = selectedIndex();
        // 向下查找下一个非禁用选项
        let next = current >= options.length - 1 ? 0 : current + 1;
        while (next < options.length && options[next]?.disabled) {
          next++;
        }
        if (next < options.length) setSelectedIndex(next);
        break;
      }
      case 'Enter': {
        e.preventDefault();
        handleConfirm();
        break;
      }
    }
  };

  return (
    // tabindex="-1" 使容器可接收键盘事件（方向键导航 + Enter 确认）
    <div class="navis-dialog-body" tabindex="-1" onKeyDown={handleKeyDown}>
      {/* 提示消息 */}
      {props.config.message && (
        <p class="navis-dialog-message">{props.config.message}</p>
      )}

      {/* 选项列表 */}
      <div class="navis-dialog-option-list">
        <For each={options}>
          {(option: DialogOption, index) => (
            <button
              type="button"
              onClick={() => handleOptionClick(index())}
              onDblClick={() => handleOptionDoubleClick(index())}
              disabled={option.disabled}
              class={`navis-dialog-option ${selectedIndex() === index() ? 'is-selected' : ''} ${option.disabled ? 'is-disabled' : ''}`}
            >
              {/* 选项标签 */}
              <span class="navis-dialog-option-label">{option.label}</span>
              {/* 选项描述（如果有） */}
              {option.description && (
                <span class="navis-dialog-option-description">
                  {option.description}
                </span>
              )}
            </button>
          )}
        </For>
      </div>

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

        {/* 确认按钮（仅当选中选项时可用） */}
        <button
          type="button"
          onClick={handleConfirm}
          disabled={selectedIndex() < 0}
          class="navis-dialog-button is-primary"
        >
          {props.config.confirmText ?? '确认'}
        </button>
      </div>
    </div>
  );
};

export default SelectDialog;


