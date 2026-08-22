# 24 - Dialog 对话框系统 详细设计

> 模块编号：24 | 层级：UI 层
> 依赖：22-UI-Framework
> 被依赖：06-Sandbox, 16-Agent

---

## 一、模块概述

### 1.1 定位

Dialog 管理应用内所有模态对话框，包括确认框、输入框、选择框和项目信任确认。Agent 工具确认复用 `AgentConfirmDialog` 内容组件，但在会话运行期不作为全局模态弹框出现，而是由 Chat Composer 在 A 区以内联覆盖层承载。

所有模态对话框必须复用公共弹框外壳：居中白色面板、轻遮罩、紧凑内容、统一按钮样式，并通过主题变量适配亮色/暗色主题。菜单浮层、右键菜单、下拉菜单不属于 Dialog，由 Menu/FloatingMenu 负责。

### 1.2 职责边界

```
负责：
├── 模态框管理（打开/关闭/层级）
├── 对话框类型（Confirm/Input/Select/Custom）
├── 公共弹框外壳（遮罩、面板、标题、按钮、主题变量）
├── Agent 确认内容组件（供 Composer 工具授权覆盖层复用）
├── 项目信任确认
├── 对话框队列（多个对话框排队显示）
└── 键盘交互（ESC 关闭、Enter 确认）

不负责：
├── 具体业务逻辑 → 各模块
├── 菜单/右键菜单/下拉浮层 → Menu/FloatingMenu
└── 通知提示 → Notification
```

---

## 二、架构设计

```
src/components/Dialog/
├── DialogManager.tsx       # 对话框管理器
├── ConfirmDialog.tsx       # 确认框
├── AlertDialog.tsx         # 提示框
├── InputDialog.tsx         # 输入框
├── SelectDialog.tsx        # 选择框
├── AgentConfirmDialog.tsx  # Agent 工具调用确认内容组件（可被 Dialog 或 Composer 复用）
├── TrustDialog.tsx         # 项目信任确认
└── store.ts                # 对话框状态
```

### 2.1 公共弹框外壳

`DialogManager` 是唯一模态容器，负责 `Dialog.Root`、遮罩、居中面板、标题、无障碍描述、关闭行为和队列渲染。各具体弹框只负责内容区和操作按钮，不允许单独定义一套弹框外观。

公共样式使用语义 class：

```text
navis-dialog-overlay
navis-dialog-content
navis-dialog-title
navis-dialog-body
navis-dialog-message
navis-dialog-actions
navis-dialog-button(.is-secondary/.is-primary/.is-danger)
navis-dialog-input
navis-dialog-option
navis-dialog-code-block
navis-dialog-risk
```

颜色、阴影、边框、危险按钮等通过 `--navis-dialog-*` 主题变量控制。默认亮色主题是白色简洁风格；暗色主题只覆盖变量，不改组件结构。

---

## 三、数据模型

```typescript
interface DialogConfig {
  id: string
  tyee: 'confirm' | 'alert' | 'input' | 'select' | 'custom'
  title: string
  message?: string
  content?: Component  // 自定义内容
  confirmText?: string
  cancelText?: string
  danger?: boolean     // 危险操作样式
  inputs?: DialogInput[]
  options?: DialogOetion[]
  onConfirm?: (result?: any) => void
  onCancel?: () => void
}

interface AgentConfirmConfig {
  id: string
  toolName: string
  toolArgs: Record<string, unknown>
  riskLevel: 'low' | 'medium' | 'high'
  message: string
}
```

---

## 四、接口定义

```typescript
// 通用对话框
dialog.confirm(config: DialogConfig): Promise<boolean>
dialog.alert(title: string, message: string): Promise<void>
dialog.input(title: string, message: string, defaultValue?: string): Promise<string | null>
dialog.select(title: string, options: DialogOetion[]): Promise<any | null>

// Agent 确认
dialog.agentConfirm(config: AgentConfirmConfig): Promise<'allow_once' | 'allow_session' | 'allow_project' | 'deny_always'>
// 返回 Promise，resolve 时携带用户的四态工具审批决策。
// Agent 运行期审批优先使用 Composer 内联确认面板；全局 Dialog API 只作为非会话场景的复用入口。

// 项目信任
dialog.trustProject(path: string): Promise<'trusted' | 'untrusted' | 'ask'>

// 管理
dialog.close(id: string): void
dialog.closeAll(): void
```

---

## 五、公共视觉规范

通用确认框示例：

```
┌──────────────────────────────────────┐
│ Delete session?                      │
│ "General coding session" will be     │
│ permanently deleted. This can't be   │
│ undone.                              │
│                                      │
│                  Cancel    Delete    │
└──────────────────────────────────────┘
```

视觉要求：

- 弹框为应用内紧凑尺寸，默认宽度约 420px，内容超长时允许在面板内滚动。
- 标题、正文、按钮均使用公共 class，不在业务弹框中写独立样式。
- 危险操作使用 `.is-danger` 红色主按钮；普通确认使用 `.is-primary` 中性主按钮；取消使用 `.is-secondary`。
- 遮罩点击和 Esc 均关闭弹框，并以取消默认值解析 Promise。
- 菜单浮层不使用公共弹框外壳，不进入 Dialog 队列。

---

## 六、Agent 工具确认内容设计

```
┌─────────────────────────────────────────────────┐
│  Confirm action                                  │
├─────────────────────────────────────────────────┤
│                                                 │
│  Navis Go wants to run this action.                 │
│                                                 │
│  Tool        terminal.exec                       │
│  command     npm test                            │
│  cwd         /home/user/project                  │
│                                                 │
│  ┌─────────────────────────────────────────┐   │
│  │ Risk level: Medium                       │   │
│  │ This command may change project state.   │   │
│  └─────────────────────────────────────────┘   │
│                                                 │
│  ┌─────────────┐ ┌───────────────┐ ┌───────────────┐ ┌────────────┐ │
│  │ Deny always │ │ Allow project │ │ Allow session │ │ Allow once │ │
│  └─────────────┘ └───────────────┘ └───────────────┘ └────────────┘ │
└─────────────────────────────────────────────────┘
```

---

## 七、Sandbox 审批模式与工具确认映射

工具确认策略由当前 Session 的权限策略 / Sandbox 审批模式（ApprovalMode）共同决定：

| 审批模式 | 确认策略 | 说明 |
|----------|-----------|------|
| Suggest | 修改类工具每次覆盖 Composer 输入框请求批准 | 写文件、编辑文件、删除、命令等修改类工具必须逐次等待用户批准；只读工具不打断工作流 |
| AutoEdit | 只弹高风险操作确认框 | 文件写入可自动执行；文件删除、命令执行（写入类）、网络请求仍弹确认框 |
| FullAuto | 普通操作不弹确认框 | Agent 自动执行已授权操作；越权路径、未信任 Project、危险 denylist 操作仍由 Sandbox 拒绝或强制确认 |

### 7.1 实现逻辑

```
Agent 发起操作
     │
     ▼
Permission/Sandbox check 返回需要确认
     │
     ├── 写入 permission AgentTimelinePart(status = waiting_permission, source = permission_runtime)
     ├── ui_stream_session_message 发送 toolApproval chunk
     ├── Composer 内联覆盖 AgentConfirmDialog
     └── ui_respond_tool_approval 回传 allow_once | allow_session | allow_project | deny_always
```

Agent 运行期工具审批不进入全局 Dialog 队列，避免中央弹窗打断输入上下文。`AgentConfirmDialog` 是可复用内容组件；在会话流中由 Composer 内联承载，在非会话场景才允许由 DialogManager 承载。

### 7.2 FullAuto 模式的审计替代

FullAuto 模式下虽然不弹确认框，但所有操作仍通过 `sandbox.check.allowed` / `sandbox.check.denied` 事件记录到审计日志，用户可通过审计面板查看 Agent 的完整操作历史。

---

## 八、"Allow this session" 范围定义（SessionScoped）

用户在 Agent 确认框中点击 `Allow this session` 时，信任范围定义如下：

### 8.1 信任规则

- **生效范围**：仅对当前会话（Session）生效，会话结束或应用重启后自动清除
- **匹配条件**：同一 `sessionId + permission pattern`
- **不持久化**：SessionScoped 信任只存在 `ToolApprovalStore` 内存缓存中，不写入用户配置、Project 配置或扩展配置
- **审计记录**：首次点击和后续自动放行都必须写入 `permission` AgentTimelinePart，`decision = allow_session`

### 8.2 示例

用户在会话 A 中对 `terminal:npm test` pattern 点击 `Allow this session`：
- 会话 A 内后续所有 `npm test` 命令自动放行
- 会话 A 内其他命令（如 `rm -rf`）不受影响
- 新会话 B 中 `npm test` 仍需确认

### 8.3 与 ProjectTrust 的关系

```
ProjectTrust 枚举变体：
├── Trusted        → 完全信任（持久化）
├── Untrusted      → 不信任（持久化）
├── AskEachTime    → 每次询问（持久化）
└── SessionScoped  → 仅本次会话信任（内存态，会话结束清除）
```

`Allow this session` 不是 ProjectTrust 的持久变体，也不调用项目级 trust 写入。当前实现通过 `ui_respond_tool_approval({ requestId, decision: "allow_session" })` 唤醒后端等待中的 tool call，并在 `ToolApprovalStore` 中记录 `sessionId + permission pattern`。

---

## 九、任务取消时清理排队对话框

当用户取消正在执行的 Agent 任务时，需要同步清理排队中的对话框，避免残留的确认框阻塞 UI。

### 9.1 清理流程

```
用户取消 Agent 任务
     │
     ▼
Agent 发出任务取消事件
     │
     ▼
DialogManager 收到事件
     │
     ├── 关闭当前正在显示的 AgentConfirmDialog（触发 onCancel）
     ├── 清空 Dialog 队列中所有该会话的待显示对话框
     └── 调用 dialog.closeAll(sessionId) 批量清理
```

### 9.2 接口补充

```typescript
// 按会话关闭对话框
dialog.closeBySession(sessionId: string): void

// 清空队列中指定会话的待显示对话框
dialog.clearQueue(sessionId: string): void
```

### 9.3 注意事项

- 清理时需触发每个对话框的 `onCancel` 回调，确保相关资源释放
- 仅清理属于被取消会话的对话框，不影响其他会话的确认框
- 清理完成后发出 `dialog.queue.cleared` 事件，供审计模块记录

---

## 十、测试策略

```
单元测试：对话框打开/关闭、层级管理、队列排序
集成测试：Agent 确认流程、项目信任确认、键盘交互
扩展测试：审批模式与确认框映射、SessionScoped 信任范围、任务取消清理队列
```
