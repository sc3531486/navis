/**
 * ============================================================
 * Navis UI 组件统一导出 - components/ui/index.ts
 * ============================================================
 *
 * 集中导出所有基础 UI 组件，方便外部一次性引入。
 *
 * @example
 * ```tsx
 * import { Button, Input, Select, Tooltip, Tabs } from '@/components/ui';
 * ```
 * ============================================================
 */

export { default as Button } from './Button';
export type { ButtonProps, ButtonVariant, ButtonSize } from './Button';

export { default as Input } from './Input';
export type { InputProps, InputSize } from './Input';

export { default as Select } from './Select';
export type { SelectProps, SelectOption, SelectSize } from './Select';

export { default as Tooltip } from './Tooltip';
export type { TooltipProps, TooltipPlacement } from './Tooltip';

export { default as Tabs } from './Tabs';
export type { TabsProps, TabItem, TabsSize } from './Tabs';

export { default as ShimmerText } from './ShimmerText';
export type { ShimmerTextProps } from './ShimmerText';

export { default as ShellOutputWindow } from './ShellOutputWindow';
export type { ShellOutputWindowProps } from './ShellOutputWindow';

export { default as MessageContentRenderer } from './MessageContentRenderer';

export { default as UnifiedDiffViewer, parseUnifiedDiff } from './UnifiedDiffViewer';
export type { UnifiedDiffLine, UnifiedDiffLineKind, UnifiedDiffViewerProps } from './UnifiedDiffViewer';
