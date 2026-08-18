/**
 * AgentConfirmDialog Agent 工具调用确认对话框组件
 *
 * 设计文档第五章定义的 Agent 确认框布局：
 * - 工具名称和参数展示
 * - 风险等级标识（低/中/高，带颜色编码）
 * - 四个操作按钮：Allow once / Allow this session / Allow this project / Deny always
 *
 * 审批模式映射（design/24-dialog.md 第六章）：
 * - Suggest: 所有操作弹确认框
 * - AutoEdit: 只弹高风险操作确认框
 * - FullAuto: 不弹确认框（由 Sandbox 控制）
 *
 * "本次信任"范围（design/24-dialog.md 第七章）：
 * - 仅当前会话生效
 * - 同类操作自动放行
 * - 不持久化
 */

import { Component } from 'solid-js';
import DecisionDialog from './DecisionDialog';
import type { AgentConfirmConfig } from './store';

/**
 * AgentConfirmDialog 组件属性
 *
 * 回调只负责处理按钮事件；Promise 结果由 Dialog store 的 agentConfirm API 解析。
 */
interface AgentConfirmDialogProps {
  /** Agent 确认框配置 */
  config: AgentConfirmConfig;
  /** 批准执行回调 */
  onApprove: () => void;
  /** 永久拒绝回调 */
  onDenyAlways: () => void;
  /** 本次信任回调 */
  onTrustThisSession: () => void;
  /** 当前项目信任回调 */
  onAllowProject: () => void;
}

/**
 * 风险等级配置
 * 定义每个风险等级的颜色和显示标签
 */
const RISK_LEVEL_CONFIG: Record<
  AgentConfirmConfig['riskLevel'],
  {
    /** 显示标签 */
    label: string;
    /** 语义样式 */
    className: string;
  }
> = {
  low: {
    label: 'Low',
    className: 'is-low',
  },
  medium: {
    label: 'Medium',
    className: 'is-medium',
  },
  high: {
    label: 'High',
    className: 'is-high',
  },
};

/**
 * 格式化工具参数为可读列表
 *
 * 将 Record<string, unknown> 转换为 key: value 形式的数组
 * 用于在确认框中展示工具调用的具体参数
 *
 * @param args 工具调用参数
 * @returns 格式化后的参数列表
 */
function formatToolArgs(args: Record<string, unknown>): Array<{ key: string; value: string }> {
  return Object.entries(args).map(([key, value]) => ({
    key,
    value: typeof value === 'string' ? value : JSON.stringify(value),
  }));
}

/**
 * Agent 工具调用确认对话框组件
 *
 * 当 Agent 请求执行工具调用时弹出，展示：
 * 1. 工具名称（如 terminal.exec）
 * 2. 工具参数（如命令、路径等）
 * 3. 风险等级和原因
 * 4. 操作按钮（Allow once / Allow this session / Allow this project / Deny always）
 *
 * 用户可选择：
 * - Allow once：批准本次工具调用
 * - Deny always：持续拒绝同类操作
 * - Allow this session：当前会话内同类操作自动放行
 * - Allow this project：当前项目内同类操作自动放行
 */
const AgentConfirmDialog: Component<AgentConfirmDialogProps> = (props) => {
  /** 获取风险等级的样式配置 */
  const riskConfig = () => RISK_LEVEL_CONFIG[props.config.riskLevel];

  /** 格式化工具参数 */
  const formattedArgs = () => formatToolArgs(props.config.toolArgs);

  return (
    <DecisionDialog
      message="Navis Go wants to run this action."
      details={[{ key: 'Tool', value: props.config.toolName }, ...formattedArgs()]}
      notice={{
        title: `Risk level: ${riskConfig().label}`,
        message: props.config.message,
        tone: props.config.riskLevel,
      }}
      actions={[
        { label: 'Deny always', variant: 'secondary', onClick: props.onDenyAlways },
        ...(props.config.onAllowProject
          ? [{ label: 'Allow this project', variant: 'secondary' as const, onClick: props.onAllowProject }]
          : []),
        ...(props.config.onTrustThisSession
          ? [{ label: 'Allow this session', variant: 'secondary' as const, onClick: props.onTrustThisSession }]
          : []),
        { label: 'Allow once', variant: 'primary', autofocus: true, onClick: props.onApprove },
      ]}
    />
  );
};

export default AgentConfirmDialog;


