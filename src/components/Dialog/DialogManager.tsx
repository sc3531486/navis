/**
 * DialogManager 对话框管理器组件
 *
 * 核心职责（design/24-dialog.md 第二章）：
 * - 渲染当前活跃的对话框
 * - 管理对话框的打开/关闭状态
 * - 处理对话框队列（先进先出）
 * - 键盘交互（ESC 关闭、Enter 确认）
 *
 * 架构设计：
 * - 使用 Kobalte Dialog 作为底层模态容器
 * - 根据对话框类型（type）渲染对应的子组件
 * - 从 store 获取活跃对话框配置和队列状态
 *
 * 使用方式：
 * ```tsx
 * // 在应用根组件中放置
 * <DialogManager />
 * ```
 */

import { Component, Switch, Match, Show } from 'solid-js';
import { Dynamic } from 'solid-js/web';
import * as Dialog from '@kobalte/core/dialog';
import {
  getActiveDialog,
  resolveWith,
  closeDialog,
} from './store';
import ConfirmDialog from './ConfirmDialog';
import AlertDialog from './AlertDialog';
import InputDialog from './InputDialog';
import SelectDialog from './SelectDialog';
import AgentConfirmDialog from './AgentConfirmDialog';
import TrustDialog from './TrustDialog';
import CloseIcon from '../Icon/CloseIcon';

/**
 * DialogManager 组件
 *
 * 渲染逻辑：
 * 1. 从 store 获取当前活跃对话框
 * 2. 如果无活跃对话框，不渲染任何内容
 * 3. 根据 type 字段渲染对应的子对话框组件
 * 4. 所有对话框共享同一个模态层（overlay + portal）
 *
 * 关闭行为：
 * - ESC 键：Kobalte Dialog 自动处理，触发 onOpenChange(false)
 * - 点击遮罩层：Kobalte Dialog 自动处理
 * - 关闭时使用 cancelDefault 值解析 Promise
 */
const DialogManager: Component = () => {
  /**
   * 处理对话框打开/关闭状态变化
   *
   * Kobalte Dialog 的 onOpenChange 回调：
   * - open=true: 对话框打开（由 store 自动处理）
   * - open=false: 用户通过 ESC 或点击遮罩关闭
   *
   * 当用户通过 ESC 或点击遮罩关闭时，需要：
   * 1. 使用 cancelDefault 值解析 Promise
   * 2. 触发 onCancel 回调
   * 3. 处理队列中的下一个对话框
   */
  const handleOpenChange = (isOpen: boolean) => {
    if (!isOpen) {
      const active = getActiveDialog();
      if (active) {
        // 使用 closeDialog 以取消默认值关闭
        closeDialog(active.config.id);
      }
    }
  };

  /**
   * 获取当前活跃对话框的 ID
   * 用于 Dialog.Root 的 open 状态控制
   */
  const isOpen = () => getActiveDialog() !== null;

  /**
   * 获取当前对话框的标题
   */
  const getTitle = (): string => {
    const active = getActiveDialog();
    if (!active) return '';
    // trust 类型使用固定标题
    if (active.type === 'trust') return '项目信任确认';
    if (active.type === 'agentConfirm') return 'Confirm action';
    return active.config.title;
  };

  const getDescription = (): string => {
    const active = getActiveDialog();
    if (!active) return '';
    if (active.type === 'trust' || active.type === 'agentConfirm') {
      return active.config.message;
    }
    return active.config.message ?? '';
  };

  const getCustomContent = (): Component | null => {
    const active = getActiveDialog();
    if (!active || active.type !== 'custom') return null;
    return active.config.content ?? null;
  };

  return (
    /**
     * Kobalte Dialog.Root
     *
     * 属性说明：
     * - open: 控制对话框是否显示（响应式）
     * - onOpenChange: 打开/关闭状态变化回调
     *
     * Kobalte Dialog 自动处理：
     * - ESC 键关闭
     * - 焦点陷阱（focus trap）
     * - 焦点恢复（关闭后焦点回到触发元素）
     * - aria 属性（role="dialog", aria-modal 等）
     * - 遮罩层点击关闭
     */
    <Dialog.Root open={isOpen()} onOpenChange={handleOpenChange}>
      {/**
       * Dialog.Portal
       * 将对话框内容渲染到 document.body，避免 z-index 和 overflow 问题
       */}
      <Dialog.Portal>
        {/**
         * Dialog.Overlay
         * 半透明遮罩层，覆盖整个视口
         * 点击遮罩层会触发关闭
         */}
        <Dialog.Overlay class="navis-dialog-overlay" />

        {/**
         * Dialog.Content
         * 对话框内容容器
         *
         * 样式说明：
         * - fixed + inset-0 + m-auto: 居中定位
         * - max-h-[85vh]: 最大高度 85% 视口高度
         * - overflow-y-auto: 内容超出时滚动
         * - z-50: 确保在遮罩层之上
         */}
        <Dialog.Content class="navis-dialog-content">
          {/**
           * 对话框标题区域
           * 使用 Kobalte 的 Dialog.Title 确保正确的 aria 标记
           */}
          <Dialog.Title class="navis-dialog-title">
            {getTitle()}
          </Dialog.Title>

          {/**
           * 无障碍描述
           * 使用 Kobalte 的 Dialog.Description 关联 aria-describedby
           */}
          <Dialog.Description class="sr-only">
            {/* 使用消息内容作为无障碍描述 */}
            {getDescription()}
          </Dialog.Description>

          {/**
           * 根据对话框类型渲染对应的子组件
           *
           * 使用 Switch/Match 确保同一时间只渲染一种类型的对话框。
           * 当活跃对话框类型变化时，Solid.js 会销毁旧组件并创建新组件。
           */}
          <Show when={getActiveDialog()}>
            {(active) => (
              <Switch>
                {/* ===== 确认框 ===== */}
                <Match when={active().type === 'confirm'}>
                  <ConfirmDialog
                    config={active().config as any}
                    onConfirm={() =>
                      resolveWith(active().config.id, true)
                    }
                    onCancel={() =>
                      closeDialog(active().config.id)
                    }
                  />
                </Match>

                {/* ===== 提示框 ===== */}
                <Match when={active().type === 'alert'}>
                  <AlertDialog
                    config={active().config as any}
                    onConfirm={() =>
                      resolveWith(active().config.id, undefined)
                    }
                  />
                </Match>

                {/* ===== 输入框 ===== */}
                <Match when={active().type === 'input'}>
                  <InputDialog
                    config={active().config as any}
                    onConfirm={(value) =>
                      resolveWith(active().config.id, value)
                    }
                    onCancel={() =>
                      closeDialog(active().config.id)
                    }
                  />
                </Match>

                {/* ===== 选择框 ===== */}
                <Match when={active().type === 'select'}>
                  <SelectDialog
                    config={active().config as any}
                    onConfirm={(value) =>
                      resolveWith(active().config.id, value)
                    }
                    onCancel={() =>
                      closeDialog(active().config.id)
                    }
                  />
                </Match>

                {/* ===== 自定义内容弹框 ===== */}
                <Match when={active().type === 'custom'}>
                  <div class="navis-dialog-body">
                    <Show when={active().config.message}>
                      {(message) => <p class="navis-dialog-message">{message()}</p>}
                    </Show>
                    <Show when={getCustomContent()}>
                      {(Content) => <Dynamic component={Content()} />}
                    </Show>
                  </div>
                </Match>

                {/* ===== Agent 工具调用确认框 ===== */}
                <Match when={active().type === 'agentConfirm'}>
                  {(() => {
                    // 获取 Agent 确认框配置
                    const cfg = active().config as import('./store').AgentConfirmConfig;
                    return (
                      <AgentConfirmDialog
                        config={cfg}
                        onApprove={() => {
                          // Allow once：触发 onApprove 回调并返回四态决策
                          cfg.onApprove();
                          resolveWith(cfg.id, 'allow_once');
                        }}
                        onDenyAlways={() => {
                          // Deny always：触发 onDenyAlways 回调并返回四态决策
                          cfg.onDenyAlways();
                          resolveWith(cfg.id, 'deny_always');
                        }}
                        onTrustThisSession={() => {
                          // Allow this session：触发 onTrustThisSession 回调并返回四态决策
                          cfg.onTrustThisSession?.();
                          resolveWith(cfg.id, 'allow_session');
                        }}
                        onAllowProject={() => {
                          // Allow this project：触发 onAllowProject 回调并返回四态决策
                          cfg.onAllowProject?.();
                          resolveWith(cfg.id, 'allow_project');
                        }}
                      />
                    );
                  })()}
                </Match>

                {/* ===== Worktree 信任确认框 ===== */}
                <Match when={active().type === 'trust'}>
                  {(() => {
                    // 获取信任对话框配置
                    const cfg = active().config as {
                      id: string;
                      path: string;
                      message: string;
                    };
                    return (
                      <TrustDialog
                        path={cfg.path}
                        message={cfg.message}
                        onTrust={() =>
                          resolveWith(cfg.id, 'trusted')
                        }
                        onUntrust={() =>
                          resolveWith(cfg.id, 'untrusted')
                        }
                        onAsk={() =>
                          resolveWith(cfg.id, 'ask')
                        }
                      />
                    );
                  })()}
                </Match>
              </Switch>
            )}
          </Show>

          {/**
           * 关闭按钮（右上角 X）
           * 使用 Kobalte 的 Dialog.CloseButton
           * 点击后触发 onOpenChange(false)，走 ESC 关闭逻辑
           */}
          <Dialog.CloseButton class="navis-dialog-close" aria-label="Close dialog">
            <CloseIcon />
          </Dialog.CloseButton>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
};

export default DialogManager;


