/**
 * InputDialog 输入对话框组件
 *
 * 带输入框的对话框，支持：
 * - 单个或多个输入字段
 * - 默认值、占位符
 * - 必填验证
 * - Enter 键确认
 *
 * 对应 design/24-dialog.md 中 DialogConfig 的 'input' 类型
 */

import { Component, createSignal, For } from 'solid-js';
import type { DialogConfig, DialogInput } from './store';

/**
 * InputDialog 组件属性
 */
interface InputDialogProps {
  /** 对话框配置（包含 inputs 字段） */
  config: DialogConfig;
  /**
   * 确认回调
   * @param value 用户输入的文本值（多个输入时返回第一个值）
   */
  onConfirm: (value: string) => void;
  /** 取消回调 */
  onCancel: () => void;
}

/**
 * 输入对话框组件
 *
 * 渲染一个或多个输入框，收集用户输入。
 * 当只有一个输入框时，直接返回字符串值。
 *
 * 设计文档中的接口：
 * dialog.input(title, message, defaultValue?): Promise<string | null>
 */
const InputDialog: Component<InputDialogProps> = (props) => {
  /** 获取输入字段列表（默认提供一个空输入字段） */
  const inputs = props.config.inputs ?? [{ label: '', type: 'text' as const }];

  /**
   * 输入值状态
   * 使用信号数组管理每个输入字段的值
   * 初始值从 defaultValue 或空字符串获取
   */
  const [values, setValues] = createSignal<string[]>(
    inputs.map((input) => input.defaultValue ?? '')
  );

  /**
   * 更新指定索引的输入值
   * @param index 输入字段索引
   * @param value 新值
   */
  const updateValue = (index: number, value: string) => {
    setValues((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  };

  /**
   * 处理确认操作
   * 返回第一个输入字段的值（简化 API 设计）
   */
  const handleConfirm = () => {
    const firstValue = values()[0] ?? '';
    props.onConfirm(firstValue);
  };

  /**
   * 处理键盘事件
   * Enter 键触发确认
   */
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleConfirm();
    }
  };

  return (
    // tabindex="-1" 使容器可接收键盘事件（Enter 确认）
    <div class="navis-dialog-body" tabindex="-1" onKeyDown={handleKeyDown}>
      {/* 提示消息 */}
      {props.config.message && (
        <p class="navis-dialog-message">{props.config.message}</p>
      )}

      {/* 输入字段列表 */}
      <div class="navis-dialog-field-stack">
        <For each={inputs}>
          {(input: DialogInput, index) => (
            <div>
              {/* 字段标签（如果有） */}
              {input.label && (
                <label class="navis-dialog-label">
                  {input.label}
                  {input.required && <span class="navis-dialog-required">*</span>}
                </label>
              )}

              {/* 输入框 */}
              <input
                type={input.type ?? 'text'}
                value={values()[index()]}
                placeholder={input.placeholder}
                onInput={(e) => updateValue(index(), e.currentTarget.value)}
                autofocus={index() === 0}
                class="navis-dialog-input"
              />
            </div>
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

        {/* 确认按钮 */}
        <button
          type="button"
          onClick={handleConfirm}
          class="navis-dialog-button is-primary"
        >
          {props.config.confirmText ?? '确认'}
        </button>
      </div>
    </div>
  );
};

export default InputDialog;


