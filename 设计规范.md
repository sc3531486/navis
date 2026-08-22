---
version: alpha
name: Navi-design-analysis
description: Navi 是一个 AI 编码助手桌面应用，视觉风格对标 Claude Desktop 产品内 UI。深色主题为主，暖色调深灰背景（#1a1a1a）搭配蓝色强调色（#2563eb），无衬线字体（Inter），开发者工具风格。布局为左侧边栏 + 中央内容区 + 底部/右侧面板，状态栏固定底部。整体视觉基调：专业、克制、信息密度高、减少视觉噪音。

colors:
  primary: "#2563eb"
  primary-hover: "#3b82f6"
  primary-active: "#1d4ed8"
  primary-disabled: "#1e3a5f"
  ink: "#e5e5e5"
  body: "#cccccc"
  body-strong: "#ffffff"
  muted: "#888888"
  muted-soft: "#666666"
  hairline: "#333333"
  hairline-soft: "#2a2a2a"
  canvas: "#1a1a1a"
  canvas-soft: "#222222"
  surface-card: "#2a2a2a"
  surface-elevated: "#333333"
  surface-input: "#252525"
  on-primary: "#ffffff"
  on-dark: "#e5e5e5"
  on-dark-soft: "#999999"
  accent-teal: "#2dd4bf"
  accent-amber: "#f59e0b"
  accent-purple: "#a78bfa"
  success: "#22c55e"
  warning: "#f59e0b"
  error: "#ef4444"
  info: "#3b82f6"

typography:
  display-lg:
    fontFamily: "Inter, -apple-system, BlinkMacSystemFont, sans-serif"
    fontSize: 24px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.3px
  display-md:
    fontFamily: "Inter, sans-serif"
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: -0.2px
  title-lg:
    fontFamily: "Inter, sans-serif"
    fontSize: 18px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0
  title-md:
    fontFamily: "Inter, sans-serif"
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0
  title-sm:
    fontFamily: "Inter, sans-serif"
    fontSize: 14px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0
  body-md:
    fontFamily: "Inter, sans-serif"
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: 0
  body-sm:
    fontFamily: "Inter, sans-serif"
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
  caption:
    fontFamily: "Inter, sans-serif"
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
  caption-uppercase:
    fontFamily: "Inter, sans-serif"
    fontSize: 11px
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: 0.8px
    textTransform: uppercase
  code:
    fontFamily: "JetBrains Mono, ui-monospace, monospace"
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: 0
  button:
    fontFamily: "Inter, sans-serif"
    fontSize: 13px
    fontWeight: 500
    lineHeight: 1.0
    letterSpacing: 0
  nav-link:
    fontFamily: "Inter, sans-serif"
    fontSize: 13px
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: 0

rounded:
  xs: 4px
  sm: 6px
  md: 8px
  lg: 12px
  xl: 16px
  pill: 9999px
  full: 9999px

spacing:
  xxs: 2px
  xs: 4px
  sm: 8px
  md: 12px
  base: 16px
  lg: 24px
  xl: 32px
  xxl: 48px

components:
  top-nav:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    typography: "{typography.nav-link}"
    height: 48px

  sidebar:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    width: 260px
    borderRight: "1px {colors.hairline}"

  sidebar-item:
    backgroundColor: transparent
    textColor: "{colors.body}"
    typography: "{typography.body-sm}"
    height: 36px
    padding: "0 12px"
    rounded: "{rounded.md}"

  sidebar-item-active:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
    borderLeft: "2px {colors.primary}"

  sidebar-item-hover:
    backgroundColor: "{colors.surface-card}"

  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "6px 14px"
    height: 32px

  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"

  button-primary-active:
    backgroundColor: "{colors.primary-active}"

  button-secondary:
    backgroundColor: transparent
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "6px 14px"
    height: 32px
    border: "1px {colors.hairline}"

  button-secondary-hover:
    backgroundColor: "{colors.surface-card}"

  button-ghost:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    height: 32px

  button-ghost-hover:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"

  button-icon:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    rounded: "{rounded.md}"
    size: 32px

  button-icon-hover:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"

  button-danger:
    backgroundColor: "{colors.error}"
    textColor: "{colors.on-primary}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "6px 14px"
    height: 32px

  text-input:
    backgroundColor: "{colors.surface-input}"
    textColor: "{colors.ink}"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
    height: 36px
    border: "1px {colors.hairline}"

  text-input-focus:
    border: "1px {colors.primary}"

  text-input-placeholder:
    textColor: "{colors.muted-soft}"

  card:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "{spacing.base}"
    border: "1px {colors.hairline}"

  card-hover:
    border: "1px {colors.muted-soft}"

  panel:
    backgroundColor: "{colors.canvas}"
    border: "1px {colors.hairline}"
    padding: "{spacing.base}"

  panel-header:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.ink}"
    typography: "{typography.title-sm}"
    height: 40px
    borderBottom: "1px {colors.hairline}"

  statusbar:
    backgroundColor: "{colors.canvas}"
    textColor: "{colors.on-dark-soft}"
    typography: "{typography.caption}"
    height: 24px
    borderTop: "1px {colors.hairline}"

  statusbar-item:
    padding: "0 8px"
    height: 24px

  chat-message-user:
    backgroundColor: "{colors.primary}"
    backgroundOpacity: 0.1
    textColor: "{colors.ink}"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: "12px 16px"

  chat-message-assistant:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: "12px 16px"

  code-block:
    backgroundColor: "#0d1117"
    textColor: "#e6edf3"
    typography: "{typography.code}"
    rounded: "{rounded.md}"
    padding: "{spacing.base}"
    lineNumbers: true
    syntaxHighlight: true

  code-inline:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.accent-purple}"
    typography: "{typography.code}"
    rounded: "{rounded.xs}"
    padding: "2px 6px"

  tool-call-indicator:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.muted}"
    typography: "{typography.caption}"
    rounded: "{rounded.sm}"
    padding: "4px 10px"
    icon: "terminal"
    border: "1px {colors.hairline}"

  tool-call-running:
    backgroundColor: "{colors.primary}"
    backgroundOpacity: 0.1
    textColor: "{colors.primary}"
    border: "1px {colors.primary}"
    borderOpacity: 0.3

  agent-status-idle:
    textColor: "{colors.muted-soft}"
    icon: "circle"

  agent-status-running:
    textColor: "{colors.primary}"
    icon: "loader"
    animation: "spin 1s linear infinite"

  agent-status-error:
    textColor: "{colors.error}"
    icon: "alert-circle"

  agent-status-done:
    textColor: "{colors.success}"
    icon: "check-circle"

  badge:
    backgroundColor: "{colors.surface-elevated}"
    textColor: "{colors.muted}"
    typography: "{typography.caption-uppercase}"
    rounded: "{rounded.pill}"
    padding: "2px 8px"

  badge-primary:
    backgroundColor: "{colors.primary}"
    backgroundOpacity: 0.15
    textColor: "{colors.primary}"

  badge-success:
    backgroundColor: "{colors.success}"
    backgroundOpacity: 0.15
    textColor: "{colors.success}"

  badge-error:
    backgroundColor: "{colors.error}"
    backgroundOpacity: 0.15
    textColor: "{colors.error}"

  toast:
    backgroundColor: "{colors.surface-elevated}"
    textColor: "{colors.ink}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
    border: "1px {colors.hairline}"
    shadow: "0 4px 12px rgba(0,0,0,0.3)"

  modal-overlay:
    backgroundColor: "rgba(0,0,0,0.6)"

  modal:
    backgroundColor: "{colors.surface-card}"
    textColor: "{colors.ink}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
    border: "1px {colors.hairline}"
    shadow: "0 8px 24px rgba(0,0,0,0.4)"

  divider:
    backgroundColor: "{colors.hairline}"
    height: 1px

  tab:
    backgroundColor: transparent
    textColor: "{colors.muted}"
    typography: "{typography.body-sm}"
    height: 36px
    padding: "0 {spacing.base}"
    borderBottom: "2px transparent"

  tab-active:
    textColor: "{colors.ink}"
    borderBottom: "2px {colors.primary}"

  tab-hover:
    textColor: "{colors.body}"
---

## Overview

Navi 是一个 AI 编码助手桌面应用，视觉风格对标 **Claude Desktop 产品内 UI**。深色主题为主，暖色调深灰背景搭配蓝色强调色，无衬线字体，开发者工具风格。

整体设计哲学：**功能优先、信息密度高、减少视觉噪音**。不做营销页面风格，不做装饰性设计。每一个像素都应该为开发者的工作流服务。

**Key Characteristics:**
- 深色主题为默认（`{colors.canvas}` — #1a1a1a），暖色调深灰，不是纯黑
- 蓝色强调色（`{colors.primary}` — #2563eb），用于按钮、链接、选中状态、进度指示
- Inter 无衬线字体，JetBrains Mono 代码字体，无衬线标题
- 信息密度高：紧凑间距、小字号、高对比度
- 无阴影或极轻阴影，用边框和背景色区分层级
- 侧边栏 + 内容区 + 面板的经典 IDE 布局

## Colors

### Brand & Accent
- **Primary Blue** (`{colors.primary}` — #2563eb): 按钮、链接、选中状态、进度条、Agent 运行指示
- **Primary Hover** (`{colors.primary-hover}` — #3b82f6): 悬停状态
- **Primary Active** (`{colors.primary-active}` — #1d4ed8): 按下状态
- **Accent Teal** (`{colors.accent-teal}` — #2dd4bf): 成功连接、在线状态
- **Accent Amber** (`{colors.accent-amber}` — #f59e0b): 警告、配额提醒
- **Accent Purple** (`{colors.accent-purple}` — #a78bfa): 内联代码、特殊标记

### Surface
- **Canvas** (`{colors.canvas}` — #1a1a1a): 应用主背景、侧边栏背景
- **Canvas Soft** (`{colors.canvas-soft}` — #222222): 次级区域背景
- **Surface Card** (`{colors.surface-card}` — #2a2a2a): 卡片、面板、消息气泡、输入框
- **Surface Elevated** (`{colors.surface-elevated}` — #333333): 悬浮元素、Toast、下拉菜单
- **Surface Input** (`{colors.surface-input}` — #252525): 输入框背景（比 card 略深）
- **Hairline** (`{colors.hairline}` — #333333): 1px 边框、分隔线

### Text
- **Ink** (`{colors.ink}` — #e5e5e5): 主文字、标题
- **Body** (`{colors.body}` — #cccccc): 正文
- **Body Strong** (`{colors.body-strong}` — #ffffff): 强调文字（极少使用）
- **Muted** (`{colors.muted}` — #888888): 次要文字、描述、时间戳
- **Muted Soft** (`{colors.muted-soft}` — #666666): 占位符、禁用文字
- **On Primary** (`{colors.on-primary}` — #ffffff): 按钮上文字
- **On Dark Soft** (`{colors.on-dark-soft}` — #999999): 状态栏文字

### Semantic
- **Success** (`{colors.success}` — #22c55e): 成功状态、完成指示、Git 已提交
- **Warning** (`{colors.warning}` — #f59e0b): 警告、配额提醒
- **Error** (`{colors.error}` — #ef4444): 错误、危险操作
- **Info** (`{colors.info}` — #3b82f6): 信息提示

## Typography

### Font Family
**Inter** 是主字体。**JetBrains Mono** 处理所有代码相关文本。不使用衬线字体。

### Hierarchy

| Token | Size | Weight | Line Height | Letter Spacing | Use |
|---|---|---|---|---|---|
| `{typography.display-lg}` | 24px | 600 | 1.3 | -0.3px | 页面标题（极少使用） |
| `{typography.display-md}` | 20px | 600 | 1.3 | -0.2px | 面板标题 |
| `{typography.title-lg}` | 18px | 600 | 1.4 | 0 | 章节标题 |
| `{typography.title-md}` | 16px | 600 | 1.4 | 0 | 子标题、卡片标题 |
| `{typography.title-sm}` | 14px | 600 | 1.4 | 0 | 列表标签、面板头 |
| `{typography.body-md}` | 14px | 400 | 1.6 | 0 | 正文（基础字号） |
| `{typography.body-sm}` | 13px | 400 | 1.5 | 0 | 次要文字、辅助信息 |
| `{typography.caption}` | 12px | 400 | 1.4 | 0 | 标签、时间戳、状态栏 |
| `{typography.caption-uppercase}` | 11px | 600 | 1.4 | 0.8px | 分类标签、徽章 |
| `{typography.code}` | 13px | 400 | 1.6 | 0 | 代码块、终端、路径 |
| `{typography.button}` | 13px | 500 | 1.0 | 0 | 按钮文字 |
| `{typography.nav-link}` | 13px | 500 | 1.4 | 0 | 导航菜单 |

### Principles
- 无衬线字体贯穿全部 UI，不使用衬线标题
- 代码路径和终端输出始终使用 JetBrains Mono
- 基础字号 14px，比营销页面小，信息密度更高
- 字重以 400（正文）和 600（标题/强调）为主，500 仅用于按钮和导航

## Layout

### Application Shell

```
┌──────────────────────────────────────────────────────────┐
│  Toolbar (48px)                                          │
│  ┌──┐┌──┐┌────────────────────────────────┐┌──────────┐ │
│  │☰ ││🔍 ││          搜索/命令             ││  用户头像 │ │
│  └──┘└──┘└────────────────────────────────┘└──────────┘ │
├────────┬─────────────────────────────────────────────────┤
│        │                                                 │
│ Side   │         Content Area                            │
│ bar    │  ┌───────────────────────────────────────────┐ │
│ 260px  │  │                                           │ │
│        │  │  Chat / Editor / Terminal / Settings       │ │
│  会话  │  │                                           │ │
│  列表  │  │                                           │ │
│        │  └───────────────────────────────────────────┘ │
│  文件  │                                                 │
│  树    │  ┌───────────────────────────────────────────┐ │
│        │  │  Panel (Bottom or Right)                   │ │
│        │  │  Diff / Terminal / Output / Logs           │ │
│        │  └───────────────────────────────────────────┘ │
├────────┴─────────────────────────────────────────────────┤
│  StatusBar (24px)                                        │
│  🟢 Idle │ main.ts:42 │ 🔧 3 tools │ ⏱ 12:34 │ 🌐 OK  │
└──────────────────────────────────────────────────────────┘
```

### Spacing System
- **Base unit:** 4px
- **Tokens:** `{spacing.xxs}` 2px · `{spacing.xs}` 4px · `{spacing.sm}` 8px · `{spacing.md}` 12px · `{spacing.base}` 16px · `{spacing.lg}` 24px · `{spacing.xl}` 32px · `{spacing.xxl}` 48px
- **Panel padding:** `{spacing.base}` (16px)
- **Card padding:** `{spacing.base}` (16px)
- **列表项间距：** `{spacing.sm}` (8px)
- **按钮间距：** `{spacing.sm}` (8px)

### Grid & Container
- 内容区自适应，不需要 max-width 约束
- 面板大小可拖拽调整
- 侧边栏宽度 260px（可拖拽，范围 200px ~ 400px）

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| Flat (canvas) | `{colors.canvas}` 无边框 | 主背景、侧边栏 |
| Hairline | 1px `{colors.hairline}` 边框 | 卡片、面板、输入框 |
| Elevated | `{colors.surface-elevated}` 背景 | Toast、下拉菜单、悬浮面板 |
| Modal | `{colors.surface-card}` + 轻阴影 | 模态框、对话框 |

**深度哲学：边框优先，阴影极少。** 大部分层级通过背景色差异和 1px 边框区分。阴影仅用于模态框和 Toast。

## Components

### Sidebar
**`sidebar`** — `{colors.canvas}` 背景，260px 宽，右侧 1px `{colors.hairline}` 边框。

**`sidebar-item`** — 36px 高，`{colors.body}` 文字，`{typography.body-sm}`。Hover 时背景变为 `{colors.surface-card}`。Active 时背景 `{colors.surface-card}` + 左边框 2px `{colors.primary}`。

### Buttons

**`button-primary`** — `{colors.primary}` 背景，白色文字，`{rounded.md}` (8px)，32px 高，13px 字号。Hover `{colors.primary-hover}`，Active `{colors.primary-active}`。

**`button-secondary`** — 透明背景，`{colors.ink}` 文字，1px `{colors.hairline}` 边框。Hover 背景 `{colors.surface-card}`。

**`button-ghost`** — 透明背景，`{colors.muted}` 文字。Hover 背景 `{colors.surface-card}` + 文字变 `{colors.ink}`。

**`button-icon`** — 32px 方形，透明背景，`{colors.muted}` 图标。Hover 背景 `{colors.surface-card}`。

**`button-danger`** — `{colors.error}` 背景，白色文字。用于删除、危险操作。

### Inputs

**`text-input`** — `{colors.surface-input}` 背景，`{colors.ink}` 文字，`{rounded.md}`，36px 高，1px `{colors.hairline}` 边框。Focus 时边框变为 `{colors.primary}`。Placeholder 颜色 `{colors.muted-soft}`。

### Cards & Panels

**`card`** — `{colors.surface-card}` 背景，`{rounded.md}`，`{spacing.base}` padding，1px `{colors.hairline}` 边框。Hover 时边框变亮。

**`panel`** — `{colors.canvas}` 背景，可拖拽分隔条（4px 宽，hover 时变为 `{colors.primary}`）。

### Chat Messages

**`chat-message-user`** — `{colors.primary}` 背景 10% 透明度，`{colors.ink}` 文字，`{rounded.md}`，右对齐。

**`chat-message-assistant`** — `{colors.surface-card}` 背景，`{colors.ink}` 文字，`{rounded.md}`，左对齐。

### Code

**`code-block`** — `#0d1117` 深色背景，`#e6edf3` 文字，JetBrains Mono，`{rounded.md}`，带行号和语法高亮。

**`code-inline`** — `{colors.surface-card}` 背景，`{colors.accent-purple}` 文字，`{rounded.xs}`，2px 6px padding。

### Agent Status

**`agent-status-idle`** — `{colors.muted-soft}` 文字 + 空心圆图标
**`agent-status-running`** — `{colors.primary}` 文字 + 旋转 loader 图标
**`agent-status-error`** — `{colors.error}` 文字 + alert-circle 图标
**`agent-status-done`** — `{colors.success}` 文字 + check-circle 图标

**`tool-call-indicator`** — `{colors.surface-card}` 背景，`{colors.muted}` 文字，终端图标，1px 边框。运行中时背景变为 `{colors.primary}` 10%，边框变为 `{colors.primary}` 30%。

### Badges

**`badge`** — `{colors.surface-elevated}` 背景，`{colors.muted}` 文字，pill 圆角，caption-uppercase 字号。
**`badge-primary`** — `{colors.primary}` 15% 透明度背景 + 文字。
**`badge-success`** — `{colors.success}` 15% 透明度背景 + 文字。
**`badge-error`** — `{colors.error}` 15% 透明度背景 + 文字。

### Status Bar

**`statusbar`** — 24px 高，`{colors.canvas}` 背景，上边框 1px `{colors.hairline}`，`{typography.caption}` 字号，`{colors.on-dark-soft}` 文字。

### Toast

**`toast`** — `{colors.surface-elevated}` 背景，`{rounded.md}`，轻阴影 `0 4px 12px rgba(0,0,0,0.3)`，1px 边框。

### Modal

**`modal`** — `{colors.surface-card}` 背景，`{rounded.lg}`，阴影 `0 8px 24px rgba(0,0,0,0.4)`。遮罩层 `rgba(0,0,0,0.6)`。

### Tabs

**`tab`** — 透明背景，`{colors.muted}` 文字，36px 高。Active 时文字变 `{colors.ink}` + 底部 2px `{colors.primary}` 边框。Hover 时文字变 `{colors.body}`。

## Do's and Don'ts

### Do
- 使用 CSS 变量（`var(--color-xxx)`），不要硬编码颜色值
- 使用 Kobalte 组件库，不要自己造基础组件
- 保持暗色主题为默认
- 代码/路径/终端输出始终使用 JetBrains Mono
- 交互元素必须有 hover 和 active 状态
- 输入框 focus 时必须有明确的视觉反馈（边框变蓝）
- 使用 Tailwind 工具类布局

### Don't
- 不要使用纯黑 `#000000` 作为背景（使用 `#1a1a1a`）
- 不要在暗色主题上使用饱和度过高的大面积色块
- 不要让圆角不一致（统一使用 `--radius-*` token）
- 不要忽略可访问性（文字与背景对比度 >= 4.5:1）
- 不要在状态栏/工具栏中放过多信息
- 不要使用衬线字体
- 不要添加不必要的阴影（边框优先）
- 不要使用动画/过渡来装饰（仅用于功能反馈）

## Responsive Behavior

Navi 是桌面应用（Tauri），最小窗口 800px × 600px。

| Window Width | Changes |
|---|---|
| < 1000px | 侧边栏折叠为图标模式（48px 宽） |
| < 900px | 面板自动收到底部（不占右侧空间） |
| >= 1200px | 正常三栏布局 |

面板尺寸：
- 侧边栏：260px（范围 200px ~ 400px）
- 底部面板：300px 高（范围 100px ~ 70%）
- 右侧面板：360px 宽（范围 200px ~ 50%）

## Agent Prompt Guide

> 生成 Navi UI 代码时的快速参考：

```
Navi UI 规范速查：
- 框架: Solid.js + Kobalte + Tailwind
- 主题: 暗色优先，CSS 变量: var(--color-xxx)
- 主背景: #1a1a1a, 次背景: #2a2a2a
- 强调色: #2563eb (蓝)
- 文字: #e5e5e5 (主) / #cccccc (正文) / #888888 (次)
- 字体: Inter (正文) / JetBrains Mono (代码)
- 圆角: 8px (默认), 4px (小), 12px (大)
- 间距: 2/4/8/12/16/24/32px
- 按钮: 32px 高, 8px 圆角, 13px 字号
- 输入框: 36px 高, #252525 背景, focus 蓝边框
- 边框: 1px solid #333333
- 阴影: 仅模态框和 Toast 使用
- 布局: 侧边栏 260px + 内容区 + 面板(可选)
- 状态栏: 24px 高, 底部固定

代码生成时：
1. 使用 var(--color-xxx) 而非硬编码颜色
2. 使用 Kobalte 组件 (Button, Input, Select, Modal, Tabs)
3. 使用 Tailwind 工具类布局
4. 交互元素必须有 hover/active 状态
5. 代码块使用 JetBrains Mono + #0d1117 背景
6. Agent 状态用颜色区分（蓝=运行, 绿=完成, 红=错误）
```

## Known Gaps

- 实际产品 UI（聊天界面、编辑器、终端）的完整组件需要在开发过程中逐步补充
- 动画/过渡时序（消息流式显示、代码打字效果）不在本文档范围内
- 高对比度/无障碍主题变体待后续设计
- 扩展 UI 组件需要参考本文档保持风格一致
