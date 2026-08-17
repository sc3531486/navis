# 22 - UI Framework 界面框架层 详细设计

> 模块编号：22 | 层级：UI 层
> 依赖：02-Event+IPC, 03-Config
> 被依赖：23-Command-Palette, 24-Dialog, 25-Notification

---

## 一、模块概述

### 1.1 定位

UI Framework 是前端基础框架层，基于 Solid.js + Kobalte + Tailwind，提供组件库、布局系统、主题、路由、状态管理、HostView surface 与扩展视图承接。

### 1.2 职责边界

```
负责：
├── 基础组件库（Button/Input/Select/Modal/Tooltip 等）
├── 布局系统（顶部栏/左侧栏/对话工作台/右侧动态面板区）
├── 主题系统（明暗模式/自定义主题/CSS 变量）
├── 路由管理（页面导航）
├── 全局状态管理（Solid Store）
├── HostView surface（扩展完整视图承接）
├── 错误边界（全局错误兜底）
└── 响应式设计

不负责：
├── 业务视图 → 各 View 组件
├── 命令面板 → Command Palette
├── 对话框管理 → Dialog
└── 通知展示 → Notification
```

---

## 二、架构设计

```
src/
├── App.tsx                 # 应用挂载入口，渲染 AppRoutes
├── index.tsx               # 渲染入口
├── layouts/
│   ├── MainLayout.tsx      # 主布局（顶部栏 + 左侧栏 + 对话工作台 + 右侧动态面板区）
│   ├── Sidebar.tsx         # 侧边栏
│   ├── StatusBar.tsx       # 可选状态栏（当前主工作台不挂载）
│   └── Toolbar.tsx         # 工具栏
├── components/
│   ├── Icon/                 # 统一图标库
│   │   ├── index.tsx         # 29 个命名导出 SVG 图标（Router / Toolbar / RightWorkspaceHeader）
│   │   └── CloseIcon.tsx     # CSS mask 方式关闭图标
│   ├── ui/                   # 基础组件（Kobalte 封装）
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   ├── Select.tsx
│   │   ├── Modal.tsx
│   │   ├── Tooltip.tsx
│   │   ├── Tabs.tsx
│   │   ├── Tree.tsx
│   │   ├── List.tsx
│   │   ├── LoadingSpinner.tsx    # 公共加载旋转指示器
│   │   ├── EmptyState.tsx        # 公共空状态占位组件
│   │   ├── HoverTooltip.tsx
│   │   ├── ShimmerText.tsx
│   │   ├── ShellOutputWindow.tsx
│   │   ├── MessageContentRenderer.tsx
│   │   ├── UnifiedDiffViewer.tsx
│   │   └── index.ts
│   ├── SlashCommandDropdown.tsx  # 输入框 "/" 斜杠命令下拉列表
│   ├── RightWorkspaceHeader.tsx  # 右侧面板顶部工具栏（下拉菜单 + 面板选择器）
│   ├── HostView/               # 宿主扩展视图 surface 与 renderer
│   │   ├── HostViewSurface.tsx
│   │   ├── HostViewRenderer.tsx
│   │   └── index.ts
│   ├── WorkspacePanel/          # 右侧动态面板区核心模块
│   │   ├── BuiltinRightWorkspaceContent.tsx  # 面板分发调度器（内建 view / HostViewRenderer）
│   │   ├── DiffPanel.tsx                       # Git Diff 面板
│   │   ├── BackgroundTasksPanel.tsx            # 后台任务面板
│   │   ├── PlanPanel.tsx                       # 计划面板
│   │   ├── SessionTranscriptPanel.tsx          # 会话记录面板
│   │   ├── ToolDiffPanel.tsx                   # 工具 Diff 面板
│   │   ├── WorkspacePanelShell.tsx             # 面板外壳（标题栏 + 关闭按钮）
│   │   ├── WorkspacePanelFrame.tsx             # 面板框架（PanelSurface + 滚动区 + 区段标题 + 卡片）
│   │   ├── shared.tsx                          # 公共类型、常量、辅助函数和共享面板组件
│   │   └── index.ts
├── stores/
│   ├── app.ts              # 全局应用状态
│   ├── agent.ts            # Agent 状态
│   ├── session.ts          # 会话状态
│   ├── settings.ts         # 设置状态
│   └── project.ts        # 项目状态
├── router/
│   └── index.tsx           # 单一 Router 与路由视图配置
├── theme/
│   ├── variables.css       # CSS 变量定义
│   ├── light.css           # 亮色主题
│   ├── dark.css            # 暗色主题
│   └── index.ts            # 主题切换逻辑
├── styles/
│   ├── index.css           # 样式唯一入口，只 import 区域级 index
│   ├── global.css          # 全局收敛区；不得继续新增区域样式
│   ├── shared/             # 公共 icon、animation、基础可复用语义
│   │   └── index.css
│   ├── overlays/           # 公共浮层：Dialog、Command Palette 等
│   │   └── index.css
│   ├── settings/           # Settings dialog 管理面
│   │   └── index.css
│   ├── chatMessages/       # drawio 03 对话消息区
│   │   ├── index.css
│   │   ├── header.css
│   │   └── message-list.css
│   └── composer/           # drawio 04 输入框与多功能栏
│       ├── index.css
│       ├── base.css
│       ├── worktree-row.css
│       ├── input.css
│       ├── attachments.css
│       ├── approval.css
│       ├── toolbar.css
│       ├── menus.css
│       ├── run-status.css
│       └── slash-dropdown.css   # Slash 命令下拉列表样式
└── views/
    ├── Chat/               # Chat 视图
    ├── Editor/             # Editor 视图
    ├── Git/                # Git 视图
    ├── Settings/           # 设置视图
    ├── Search/             # 搜索视图
    └── KnowledgeBase/      # 知识库视图
```

前端运行态按职责拆分，不再把主对话流和 Composer 行为堆在单个文件：

- `stores/chat-message-types.ts` 定义 `ChatMessage`、附件、运行状态和历史查询 DTO。
- `stores/chat-message-state.ts` 持有 Solid store 和分页常量。
- `stores/chat-message-reducer.ts` 只放纯合并逻辑：历史 messages、`AgentTimelinePart` 完整快照、`AgentTimelinePartDelta` 和本地 abort/error 标记。
- `stores/chat-turn-stream.ts` 负责当前 turn 的 Channel stream、stop 和 tool approval response。
- `stores/chat-messages.ts` 只作为历史分页加载和旧调用方 re-export 门面。
- `components/Composer/Composer.tsx` 是 UI 容器；提交分流在 `useComposerSubmission.ts`，Goal / queue 控制在 `useComposerRunControls.ts`，新会话创建在 `useComposerSession.ts`，Plan / Goal / Multi-agent 指令拼装在 `composer-instructions.ts`。
- `components/AgentTimeline/AgentTimelineView.tsx` 是 Turn Timeline 的公共视图；工具展示由 `builtin-tool-renderers.ts` 注册到 renderer catalog，具体行组件拆到 `GenericToolStep.tsx`、`TerminalToolStep.tsx`、`SidechainToolStep.tsx`、`TimelineToolLabel.tsx`、`TimelineToolTarget.tsx`。工具数据解析、分类、路径、标签和详情格式化分别落在 `tool-record.ts`、`tool-kind.ts`、`tool-path.ts`、`tool-label.ts`、`tool-detail.ts`，`tool-presentation.ts` 只作为统一导出入口。主对话区和右侧面板复用这些组件，不在各自容器里重新实现文件 / diff / terminal 展示。
- `layouts/Sidebar.tsx` 只保留左侧栏状态编排、事件刷新和菜单动作；模式页签、模式菜单、会话行、Gateway 固定入口分别由 `components/Sidebar/SidebarModeTabs.tsx`、`SidebarModeMenu.tsx`、`SidebarSessionRow.tsx`、`SidebarGatewayMenu.tsx` 承接；模式菜单常量和 session mode 到 work mode 的映射在 `components/Sidebar/sidebar-model.ts`。
- `components/Settings/GatewayConfigEditor.tsx` 只保留 Gateway 配置页状态编排和保存 / 发现模型动作；Provider rail、模型列表和配置纯函数分别拆到 `GatewayProviderRail.tsx`、`GatewayModelList.tsx`、`gateway-config-model.ts`。

后端 UI 层也按投影职责拆分：

- `ui/tasks.rs` 是 Task 相关 IPC 的模块入口，只导出真实子模块。Task 列表 / 停止 / TODO / Stream cancel / Tool approval 在 `ui/tasks/task_commands.rs`；Composer queue 在 `ui/tasks/composer_commands.rs`；Goal runner 控制在 `ui/tasks/goal_runner_commands.rs`；上下文用量在 `ui/tasks/context_usage.rs`；Session Git diff 在 `ui/tasks/git_diff.rs`；Task / Todo DTO 投影在 `ui/tasks/task_projection.rs`；共享校验在 `ui/tasks/common.rs`。
- `ui/composer_projection.rs` 负责 `ComposerRuntime` 与 `UiComposerRunState` / `UiComposerTask` 的投影转换；Plan 面板、Composer B 区和队列条只能消费这份 projection。
- `ui/runtime/agent_tool_loop.rs` 承接当前 UI stream 运行时的工具循环、工具审批、Agent control tool host 注入和 tool result 回注；`ui/mod.rs` 不再内联主工具循环。
- `ui/runtime/session_change_capture.rs` 负责 edit/write 工具前后文件内容捕获和 `SessionChange` 写入，回复区 diff / Review 事实只能来自这一路径。
- `ui/runtime/sidechain_task.rs` 负责 sidechain child session 创建后的运行生命周期；`ui/mod.rs` 不再内联 sidechain async runner。
- `ui/session_metadata.rs` 负责 Session UI metadata 的读写和派生投影，包括 worktree 名称、mode、权限策略、transcript view、reasoning effort、运行/完成任务标记；`ui/composer_run_state.rs` 单独负责 composer run state 的默认值与归一化，避免 metadata 基础层继续承载运行队列语义。
- `ui/tasks/context_model.rs` 负责 context usage 命令和消息流共用的模型选择、context window、token 估算辅助；`ui/mod.rs` 不再保存这类 Settings/Gateway/Session 混合 helper。
- `ai/agent/goal_runner.rs` 通过 `GoalRunnerCommand` 返回 `GoalRunnerStatePatch`，UI 只写入 Session composer metadata，不直接决定 autonomous task 生命周期。

### 2.1 公共组件库

#### Icon 统一图标库（`components/Icon/`）

所有 SVG 图标集中到 `components/Icon/index.tsx`，以 named export 导出。图标按来源分组：

**Router 图标**（对话区 / Composer 相关）：`ScreenIcon`、`PanelIcon`、`SendIcon`、`StopIcon`、`PlusIcon`、`ChevronDown`、`PaperclipIcon`、`FolderIcon`、`FolderPlusIcon`（CSS mask）、`ConnectorIcon`、`ChecklistIcon`、`TargetIcon`、`MultiAgentIcon`、`ExtensionIcon`、`SlashIcon`、`EditIcon`、`PauseCircleIcon`、`TrashIcon`、`QuoteIcon`

**Toolbar 图标**（顶部栏 / 全局）：`IconHamburger`、`IconSidebar`、`IconSearch`、`IconArrowLeft`、`IconArrowRight`、`IconMinimize`、`IconMaximize`、`IconRestore`

**RightWorkspaceHeader 紧凑图标**：`PanelIconCompact`、`ChevronDownCompact`

**CloseIcon** 单独文件（`CloseIcon.tsx`），使用 CSS mask + `close.svg` 资源，由 `WorkspacePanelFrame` 和窗口关闭按钮消费。

`FolderPlusIcon` 和 `CloseIcon` 使用 CSS mask 方式（`url()` + `--*-url` 变量），不使用内联 `<svg>`，其余图标均为内联 SVG `<Component>`。

路由文件 `router/index.tsx` 和顶部栏 `layouts/Toolbar.tsx` 中的内联 SVG 已全部迁移至此库，不再自行定义图标组件。

#### LoadingSpinner（`components/ui/LoadingSpinner.tsx`）

公共加载旋转指示器，接受可选 `text` prop（默认 `'Loading...'`）。同时导出 `LoadingPlaceholder` 别名，用于 `<Suspense>` fallback 和懒加载占位。

```tsx
<LoadingSpinner text="Loading session..." />
<LoadingSpinner />   // 默认文本
```

#### EmptyState（`components/ui/EmptyState.tsx`）

公共空状态占位组件，提供标题 + 说明两段式布局，使用 `navis-workspace-empty` CSS 类名。所有调用方直接使用 `EmptyState`。

```tsx
<EmptyState title="No results" body="Try adjusting your search criteria." />
```

#### SlashCommandDropdown（`components/SlashCommandDropdown.tsx`）

输入框上方斜杠命令下拉列表，在输入 `/` 时弹出。支持键盘导航（`ArrowUp` / `ArrowDown` / `Enter` / `Escape`）、点击外部关闭和模糊过滤（匹配 label、description、tags）。使用 CSS 类名 `slash-command-dropdown` / `slash-command-item` / `is-selected`，样式定义在 `styles/composer/slash-dropdown.css`。

#### RightWorkspaceHeader（`components/RightWorkspaceHeader.tsx`）

右侧面板区顶部工具栏，包含紧凑下拉按钮（`PanelIconCompact` + `ChevronDownCompact`）和 `FloatingMenu`。消费 `right-workspace-menu` store 的面板命令，标记已打开面板项的选中态。从 `router/index.tsx` 中提取为独立组件。

---

## 三、布局系统

Navis Go 当前主界面以 Chat 工作台为中心，不再采用“通用 IDE 壳 + 单个右侧 Tab 面板”的抽象作为第一视图。

主界面按以下分区组织：

1. 顶部栏
2. 左侧栏
3. 对话区域
4. 对话输入区
5. 右侧动态面板区
6. 可选系统状态栏

### 3.1 主界面分区

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ Topbar（顶部栏）                                                             │
├───────────────┬──────────────────────────────────────────────┬──────────────┤
│ Left Sidebar  │ Chat Bench                                   │ Right        │
│               │                                              │ Dynamic      │
│ 1. 模式切换区  │ 3. 对话区标题栏                               │ Panels       │
│ 2. 模式菜单区  │ 4. 对话消息正文区                             │              │
│ 3. Group/会话区│ 5A. 对话输入框（A 区）                        │ 默认关闭      │
│ 4. Gateway区  │ 5B. 输入区多功能栏（B 区）                    │ 按列动态展开  │
│               │                                              │              │
└───────────────┴──────────────────────────────────────────────┴──────────────┘
│ Optional Status Bar（可选系统状态栏，仅承载全局系统态）                       │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### UI 区域与交互入口总表

以下区域名是 UI 框架和扩展系统共同使用的区域级心智模型。实现层可以继续使用更细的 DOM id，但 DOM id 不是 Extension 的公开 contract；对外说明、扩展文档和设计图应优先使用这些区域名。

| 区域 | 主要职责 | 内建按钮 / 菜单 / 状态 | 扩展可扩展内容 |
|------|----------|------------------------|----------------|
| `topbar` | 顶部栏，承载窗口级和全局导航级操作 | 顶部栏菜单、侧栏开关、搜索入口、后退 / 前进、窗口最小化 / 最大化 / 关闭 | 顶部栏菜单项、少量全局动作入口 |
| `leftSidebar` | 左侧栏，承载模式、会话组织和 Gateway 固定入口 | Cowork / Code / Custom 模式切换、模式菜单、Custom 模式扩展菜单、Group 展开折叠、Group 菜单、会话右键菜单、Gateway 按钮和菜单 | Custom 模式扩展菜单、模式菜单项、Group 菜单项、会话右键菜单项、Gateway 菜单项 |
| `chatHeader` | 当前会话标题和右侧面板区入口 | 会话类型图标、标题、紧跟标题的标题菜单按钮、右侧面板按钮、标题区动作 | 会话标题菜单项、右侧面板开关菜单项、标题区有限动作 |
| `chatMessages` | 对话正文、消息级内容表现和 Agent 工作流时间线 | 消息列表、滚动容器、基础消息块、Navis Go Turn Timeline、消息级操作入口 | 消息渲染、结果卡片、表格、验证信息、引用块、消息级操作、扩展工作流过程块 |
| `startWorkspace` | 新建会话 / 新建任务时的开始页 UI surface | 创建会话开始页、创建任务开始页、工作目录预选、首条输入入口 | 模式扩展可注册 `startWorkspace:<mode>` 开始页主体，尤其是 `startWorkspace:custom:<runtimeId>`；名称只作为 UI placement 保留 |
| `composer.workspace` | 会话工作目录行，承载当前会话的本地 worktree 选择 | 当前工作目录按钮、文件夹 `+` 最近目录入口、最近打开目录菜单（最多 10 个）、选择新的工作目录入口 | 后续可扩展远程环境 / 容器环境来源，但不得替代 `Session.worktree_root` |
| `composer.input` | A 区，承载当前输入内容和询问模式下的工具授权覆盖层 | 输入框、发送按钮、轻量附件预览条、引用 token、结构化片段、扩展结果插入位置、Ask for approval 工具确认面板 | 扩展引用结果、结构化 token、输入片段、快捷触发后的插入内容 |
| `composer.toolbar` | B 区，承载当前输入相关控制和状态 | `+` 菜单、权限选择、模型 + 推理强度选择、上下文状态指示、上下文 hover 详情、附件 / 工具入口 | `+` 菜单项、输入相关动作入口、上下文辅助状态、工具快捷入口 |
| `rightWorkspace` | 右侧动态分列面板区 | 动态分列布局、面板容器、面板关闭 / 聚焦 / 持久化 | 右侧面板、工具输出、引用详情、Inspector、扩展工作面板；名称只作为 UI placement 保留 |
| `statusbar` | 可选系统状态栏，只承载应用级状态 | 应用状态、连接状态、后台任务状态、全局错误 / 同步状态 | 应用级状态项，不承载当前会话主交互 |

#### 菜单入口约束

菜单是 UI 层入口，不是内核原语。新增菜单不能创建新的 Registry、Pipeline、EventBus 或 Policy，也不能绕过现有菜单 store 在任意组件中硬编码入口。

当前前端只承接这些 `MenuTarget`：`Tools`、`InputPlus`、`ChatTitle`、`RightPanel`、`Gateway`、`GroupContext`、`SessionContext`。新增位置必须同时补齐后端 `MenuTarget` 输出、前端触发按钮、执行器和 `npm run test:menus` 覆盖；未承接的 `File/Edit/View/Help/Context` 继续保留为扩展模型预留，不出现在用户可见菜单。

#### UI 功能贡献承接器

UI Framework 需要提供一个统一的功能贡献承接器，用来把“新增功能”接入已有区域，而不是让每个功能分别修改顶部栏、菜单、命令面板和右侧面板区代码。承接器属于 UI / extension 层，不属于 Kernel；Kernel 只继续提供 Registry / Pipeline / EventBus / Policy。

功能贡献的输入可以来自宿主内建声明，也可以来自已启用扩展 manifest。进入前端前统一归一为同一类贡献数据：

```text
FeatureContribution
├─ menus       -> 已承接的 MenuTarget
├─ commands    -> Command Palette（仅宿主支持的声明式 action）
├─ keybindings -> 后端公共快捷键投影（复用 command/action 与 HostView contract）
├─ views       -> rightWorkspace / chatAside / bottomDrawer / settings 等已有视图区域
└─ actions     -> OpenView / ToggleView / OpenSettings / RunPipeline
```

承接规则：

- 贡献只能落到当前已有 UI 区域，不能声明任意 DOM 挂载点。
- `menus.command` 必须引用已注册 command；扩展 command 只声明 `BuiltinAction`，当前 UI 投影只执行满足 HostView contract 的 `OpenView` / `ToggleView`。
- 扩展 keybinding 由后端 `ui_list_extension_keybindings` 从已启用扩展的 commands/keybindings 建立 `<extension_id>/<command_id>` namespaced 投影；存在 `keybinding.when` 或 `command.when` 时当前没有上下文评估能力，必须 fail-closed。
- `views.placement` 必须落在宿主已承接的 UI surface；`views.renderer` 必须是宿主已支持的 renderer。没有 placement 或 renderer 的 view 不进入菜单和 Command Palette。
- `RunPipeline` 只触发已注册的 Tool / Agent Pipeline，不直接在 UI 里执行系统能力；进度和结果通过 Stream / Event 投影回 UI。
- 新增功能优先新增贡献声明；只有新增 `MenuTarget`、新增 action 类型或新增 renderer 类型时才修改宿主代码。

Design 设计文档索引、模块边界和架构检查应作为一组内建 `FeatureContribution` 接入：菜单可落在 `Tools` 或对应 surface 菜单，命令为 `design.open`，视图落到合适的宿主 UI surface。读取文档复用 worktree/file 能力；后续自动审计进入现有 Agent Tool Runtime / Tool Pipeline，不新建执行链。

补充说明：

- `Settings` 不属于主工作台区域，使用公共紧凑 dialog 承载；它是扩展安装、启停、权限、配置和日志的统一管理面。普通菜单打开 Settings 时只定位到目标 tab，不在标题下方额外显示解释性 message；只有业务错误引导（例如缺少 Gateway 模型配置）才允许传入一次性原因说明。
- `Settings > Coding` 的可见配置块命名为 `Coding Editor`，承载真实 Coding Editor settings；前端 `settings` store、后端 `ui_get_editor_settings` / `ui_save_editor_settings` 与 `/editor` 的 CodeMirror 配置共用同一份状态，不允许再维护独立假 store。
- 区域名 `composer.workspace` 对应内建会话工作目录行；区域名 `composer.input` 对应输入区 contribution surface；区域名 `composer.toolbar` 对应输入工具栏 contribution surface。
- `rightWorkspace` 的布局算法由宿主内建，扩展只注册面板内容，不直接改写分列规则。
- `rightWorkspace` 内建 `session-transcript` 面板用于 `Open in > Right workspace`：面板携带 `sessionId`，通过后端 `ui_list_session_messages` 读取真实会话消息；当面板对应当前活跃会话时，发送新消息后随当前消息状态刷新。`Right workspace` 是用户可见的历史 UI surface 名，不是业务域。扩展面板必须通过 HostView contract 声明 placement、renderer 和 config；满足宿主 contract 的 `host:panel` 或 `html:sandbox` view 才能由 `OpenView` / `ToggleView` 投影到菜单或 Command Palette。`html:sandbox` 只加载扩展 `ExtensionUI/` 目录下的相对 `entry`，不加载远程 URL 或任意扩展模块。
- `rightWorkspace` 内建面板第一版可见 UI 文案统一使用英文。`Diff`、`Background tasks`、`Plan`、`Design`、`session-transcript` 和未知 renderer fallback 的 loading、empty、error、action 文案不得混用中文；设计文档可以继续使用中文说明，但用户可见面板保持英文产品界面。右侧面板菜单（`rightWorkspace` 内建）当前提供四项：`Diff`、`Background tasks`、`Plan`、`Design`，其中 `Background tasks` 替代了早期设计中暂定的 `Subagents` 名称，二者指向同一个 `background-tasks` 视图 ID，底层数据来源均为后端 `TaskManager`。
- `rightWorkspace` 内建 `diff` 面板读取当前活跃 Session 的 `worktree_root`，通过后端 `ui_get_session_git_diff` 调用 Git 模块获取真实 `git diff` 或 `git diff --staged`，并在面板中展示仓库状态、文件变更摘要、插入/删除统计和原始 diff；未绑定工作目录、非 Git 仓库和无变更都显示真实状态。非 Git 仓库返回 `isRepo=false` 而不是伪造空 diff 或抛出通用错误，面板提供 `ui_create_session_git_repo` 创建仓库入口，创建后重新读取真实 diff 状态。面板的本轮 / 最近文件改动摘要读取 `SessionChange`，通过 `ui_list_session_changes` 查询 `session_changes`，不再从 `AgentTimelinePart.metadata.reviewPanel` 推导 Review 事实；Git diff 只是 VCS projection，`SessionChange` 才是 Agent edit/write 的 Review/Diff/Revert 事实源。
- `rightWorkspace` 内建 `tool-diff` 面板展示工具级别的 diff 详情，与 `diff` 面板共享 Git 数据来源但聚焦于单次工具调用的文件变更，通过 `ToolDiffPanel` 懒加载渲染。
- `rightWorkspace` 内建 `background-tasks` 面板通过后端 `ui_list_tasks` 读取 `TaskManager` 中当前 Session 的真实 Task 投影，展示任务描述、状态、运行时长、最新消息、错误信息、消息数和工具调用数；运行 / 等待确认态使用 Navis Go 的闪烁状态语义。`ui_stream_session_message` 必须在发送生命周期中创建 Task、标记运行、写入用户 / 助手消息，并在 Gateway、Stream 或存储失败时标记失败。
- `rightWorkspace` 内建 `background-tasks` 面板同时承担子 agent 实时状态的第一版 Host panel renderer：它从同一个 `TaskManager` 投影读取 `kind`、`owner`、`activeForm`、`blocks`、`blockedBy`、`parentTaskId` 和 `sidechainSessionId`，展示任务归属、阻塞关系、运行活动和可跳转 transcript。后续如果该界面移动到 bottom drawer、chat aside 或独立 tab，只能换 UI surface，不能换事实源。
- 当前前端不再为发送流程单独维护一套 `Agent.currentTask` 假任务状态；运行态统一由真实聊天流状态和后端 `TaskManager` 推导，避免状态栏、输入区和右侧后台任务面板出现双份真相。
- 对话区的“助手正在做什么”必须来自 `AgentTimelinePart` 或历史 `AgentTimelineParts`。发送一轮消息后，后端创建 assistant shell 并立即推送 `kind = reasoning`、`source = turn_prelude` 的轻量状态 AgentTimelinePart；该 AgentTimelinePart 对齐 Claude Code 的 thinking/status line，只表达 `Thinking` 这类运行中状态，不写“准备本轮任务”这类模板化工作项，不解释内部上下文策略，也不得复述用户原始指令。模型在工具调用前输出的自然语言说明以 `source = gateway_tool_prelude` 进入同一 Timeline；如果模型第一块就是 tool call，前端不得伪造“我将...”正文，只展示 active `Thinking` 和真实工具行。前端仅在本轮尚无真实 assistant text 可显示且 prelude 仍 active 时展示 `turn_prelude`；一旦模型真实说明或最终 assistant 文本出现，就由真实文本接管展示。后端写入 `turn_finalizer` 前必须把同轮 prelude 更新为 `completed`，该 AgentTimelinePart 不计入工具统计，不显示 `Done` 徽标，也不能用前端临时 loading 文案替代。
- 成功结束一轮消息后，后端必须在发送最终 `messages` payload 前写入并推送 `kind = summary`、`source = turn_finalizer` 的收尾 AgentTimelinePart。前端把它渲染为独立完成行，例如 `Finished response · 2 tool calls · 4096 tokens`；该 AgentTimelinePart 不计入工具统计，不复用 `Compacted context` 的展示语义，且只能出现在该轮真实工具/text 之后。
- `rightWorkspace` 内建 `plan` 面板读取当前 Session 的 Composer projection。计划模式、计划执行确认、待审阅计划和目标文本等可恢复 UI 偏好通过 `ui_get_session_composer_run_state` / `ui_set_session_composer_run_state` 写入 `Session.metadata.ui.composerRun`；当前运行任务和提示词队列不写入 metadata，由后端 `ComposerRuntime` 按 Session 持有并通过同一 projection 输出。Composer B 区计划 / 目标胶囊、输入框上方目标 / 队列条和右侧 Plan 面板必须共享这份后端 projection，前端不得自行判断 running / queue 事实。
- `rightWorkspace` 内建 `design` 面板展示设计文档索引、Kernel 四原语边界和模块契约说明。该面板是 UI / 文档 / 架构辅助入口，不是新的后端大域，也不是新的执行链。当前第一版只展示已有设计索引和边界说明，不伪造架构扫描结果；如果后续加入自动审计，审计动作进入 Agent Tool Runtime / Tool Pipeline，结果再投影到右侧面板。
- `/editor` 路由当前使用与会话 worktree 同一套宿主文件服务：前端 `worktree` store 通过 `ui_get_session_worktree_snapshot` 读取当前 Session 绑定 worktree 的真实文件树和扁平文件列表，通过 `ui_read_session_worktree_file` / `ui_write_session_worktree_file` 读取和保存 UTF-8 文本文件；后端文件访问与 MCP 文件工具共享 `file::worktree_fs` 路径校验和目录过滤规则，避免编辑器和 Agent 看到两套不同的 worktree 边界。
- `/editor` 的打开文档、保存中状态和错误状态通过 editor feature store 统一管理；当当前 Session 不变但 `worktreeRoot` 被重新绑定到另一工作目录时，编辑器必须清空旧标签和旧文档缓存，再加载新的 worktree 快照，不能继续显示上一个目录的文件树或脏状态。
- 标题菜单 `Open in` 不再打开右侧 `File` 面板；它只用于把当前会话 worktree 交给用户在 `Settings > Coding` 配置的外部编程工具，例如 Zed、VS Code 或 JetBrains IDE。子菜单固定保留 `Current worktree` 与 `Right workspace`，并追加已配置的外部编程工具；没有工具时只显示 `Configure coding tools...`，打开 Settings > Coding。左侧会话右键菜单不暴露 `Open in`，避免把会话打开方式与文件上下文入口混在一起。
- 对话区 read / inspect / edit 等文件 AgentTimelinePart 的路径 label 是文件上下文入口：当路径属于当前 Session worktree 时，点击该路径打开或聚焦右侧 `File` 面板并请求打开对应文件；Command Palette 的 `@` 文件和 `#` 符号结果也走同一条右侧 `File` 面板链路。右侧 `File` 面板使用单文件布局，隐藏 worktree tree 和 tabs，只展示文件名、绝对路径工具条、保存按钮和文件内容。绝对路径必须先归属当前 worktree 后转为相对路径，不允许前端猜测或打开 worktree 外文件。
- 对话区工具行默认折叠，行结构固定为 icon、title、expand chevron、result meta、duration、status；read/list/search 的 `Read n lines`、`Listed n entries`、`Found n results` 等统计属于行内 meta，必须位于 `>` 后、duration 前，不在标题下方另起一行。点击 expand 后，read/list/search/inspect/terminal 复用同一个公共灰色详情面板并在面板内部滚动；edit diff 仍使用独立浅米色 diff 面板和行号。扩展工具继续通过 `ToolRendererCatalog` 注入 renderer，不允许直接操作消息 DOM。
- `startWorkspace` 是开始页 UI surface，不属于 `composer.input` / `composer.toolbar` 本体。开始页可以复用 composer 的输入组件、工作目录行、模型选择等控件，但页面布局、首屏标题、概览卡片、任务启动卡片由 `startWorkspace` 负责。宿主内建 `cowork` / `code` 默认开始页；Custom 模式扩展可以注册 `startWorkspace:custom:<runtimeId>` 替换开始页主体，宿主仍保留工作目录、模型、权限、发送等基础交互，避免每个模式扩展重复实现底座控件。
- 扩展菜单 action 中的 `OpenView` / `ToggleView` 必须先经过后端可渲染性 gate：只有目标 view 的 placement 和 renderer 都属于 HostView contract 白名单时，才输出到菜单或 Command Palette，并统一交给对应 UI surface 分发。当前已真实挂载的 Host view placement 只有 `rightWorkspace`、`chatAside`、`bottomDrawer`、`settingsSection`；`topbar`、`leftSidebar`、`chatHeader`、`composer.*`、`statusbar` 等区域继续走菜单、工具栏或对应 contribution surface，不作为完整 Host view 打开。当前已落地 renderer 为 `host:panel` 与 `html:sandbox`：前者通过 view contract、config、UI IPC、Kernel EventBus 投影或 `foundation::stream` 承接宿主界面，后者只加载扩展 `ExtensionUI/` 目录下的相对 `entry`，并运行在仅允许脚本执行的 sandbox iframe 中。远程 URL、`file://` URL、动态 Solid 组件和任意 ES module renderer 均不接受；非白名单 renderer/placement 或无效 entry 直接 fail-closed。`OpenView` 打开或聚焦目标视图，`ToggleView` 在目标视图已打开时关闭、未打开时打开。surface 的选中态由宿主维护，不允许扩展自己维护一套独立打开状态。
- 顶部栏 `☰` 是全局 Tools 入口：后端 `ui_list_menus` 默认返回 Command palette、Settings、Gateway、Coding、Extensions 这些真实内建项，点击该按钮打开 `Tools` 浮层菜单；`Settings` 默认进入 Gateway catalog，`Coding` 进入 Settings > Coding，页面内可见配置块仍命名为 `Coding Editor`。Settings 顶层当前只暴露 `Gateway / Coding / Extensions` 三个真实可操作 section，未落地的 Personal / Integrations 不显示为占位 tab。如果前端菜单状态尚未加载，按钮只刷新真实后端菜单，不能 fallback 到 Command Palette。扩展项仍走统一 `OpenView / ToggleView` action 分发并受真实 renderer 过滤。前端不声明或渲染尚未承接的 `File/Edit/View/Help/Context` 入口，避免扩展菜单看似可见但无真实触发源。
- `Custom` 只是模式扩展集合页签，不是一个可单独运行的工作模式。页签下方展示已安装且启用的 `work_modes` 模式扩展；显示文本优先来自 `WorkModeRegistration.name`，缺省使用 `ExtensionManifest.name`；点击后切换当前会话为对应扩展的完整工作模式，运行标识为 `custom:<runtimeId>`，其中 `runtimeId = <extensionId>/<modeId>`。选中态圆点表示当前正在使用的 Custom 模式扩展。
- 菜单项数据由后端 `ui_list_menus` 提供，该 command 合并宿主内建菜单与已启用扩展的 `contributes.menus`；数据结构复用扩展模型 `MenuRegistration` / `MenuTarget`，前端只负责按触发源渲染和分发命令。扩展模型中预留的 `File/Edit/View/Help/Context` target 不得从 `ui_list_menus` 输出，直到对应 UI 触发源、执行器和真实行为全部落地；扩展 `OpenView` / `ToggleView` 还必须引用当前宿主可渲染的 view，不能只因为 manifest 声明了 view 就变成用户可见入口。这类预留声明可以保留在 manifest 校验层，但不能变成用户可见假菜单。
- 菜单执行器按入口拆分但共享同一后端菜单契约：`group-menu`、`session-menu`、`gateway-menu`、`composer-menu`、`right-workspace-menu` 分别处理对应触发上下文，视图组件不直接解释命令字符串。扩展声明式 `OpenView` / `ToggleView` 在这些执行器中统一交给 UI surface host，不允许每个视图各自写一套 fallback。
- 内建菜单 command 覆盖必须可验证：前端 `menu-command-coverage` 维护每个已落地入口的 direct command、submenu parent 和动态 submenu prefix；`npm run test:menus` 从后端 `builtin_menus()` 解析真实内建菜单并校验每个 command 已被覆盖。新增内建菜单时必须先补真实 executor 或 submenu 生成逻辑，再更新覆盖表；不允许新增可见但无动作的菜单项，也不允许靠点击后打开 Command Palette 这类泛化 fallback 掩盖缺失实现。

### 3.2 Chat 工作台布局原则

- `A 区` 负责当前消息本身，只承载输入、引用结果和发送。
- `B 区` 负责与当前消息相关的控制、状态和扩展入口。
- `composer.workspace` 位于输入框上方，负责当前会话工作目录选择：当前工作目录按钮显示 `Local` 或当前目录名；当前工作目录按钮和文件夹 `+` 都打开最近绑定过的 Worktree 菜单；最近 Worktree 最多保留 10 个，按最近使用倒序展示；菜单底部的 `选择新的工作目录` 才调用系统目录选择器。最近目录由后端 `ProjectManager` / `RecentWorktreesManager` 管理并持久化到用户配置 `project.recentWorktrees`，前端不得使用独立 localStorage 列表。选择结果写入后端 `Session.worktree_root` / 数据库 `worktree_path`，切换会话时从 Session 恢复，不允许只保存在前端临时状态。
- 左侧栏 `New task` / `New session` 先进入 `startWorkspace` 开始页，不立即创建空会话；两者外观可以复用输入组件，但语义和页面布局不同。`New session` 是会话开始页，主区域可显示概览 / 统计 / 空会话引导，输入入口贴近底部真实会话态；`New task` 是任务开始页，主区域居中展示大任务输入卡片。开始页仍显示工作目录预选能力，用户可先从最近打开过的目录（最近 10 个）中选择工作目录，或通过底部入口选择新的工作目录。选择目录或发送第一条消息时，才按当前 mode 创建真实 Session 并绑定 `worktreeRoot`。
- `composer.toolbar` 的上下文圆环由后端 `ui_get_session_context_usage` 驱动：Session 消息和 TokenCounter 决定已使用上下文，Gateway 模型配置决定总上下文窗口，Config 中 `context.autoCompressThreshold` 决定压缩阈值。
- `composer.toolbar` 的模型入口拆成 Provider 与 Model + Effort 两个相邻控件：Provider 控件展示所有 Gateway catalog projection 中的 Provider ID；Model + Effort 控件只展示当前 Provider 下的模型，并在下半区显示 `Low / Medium / High / Extra high / Max` 推理强度。选中态沿用圆点，不用勾号。Session 事实源保存 `providerId + modelId + reasoningEffort`，切换 Session 后必须恢复各自选择；发送消息、工具循环、最终流式回复和审批后的恢复请求都必须读取同一个 Session `reasoningEffort`。Gateway 只在模型配置声明 `supportsReasoningEffort = true`，或 ChatCompletions 模型显式要求 `reasoning_effort` 字段时，才把 Effort 映射进 Provider 请求；不支持的模型保留 UI 选择但不注入未知字段。Settings 模型行中的 `Quality` 是该模型的默认质量档，只有 Effort 开启时展示，默认 `High`。
- 输入区下方的 `+` 是扩展、命令、连接器、附件的主发现路径。
- `composer.input` 支持通过 `+ > Add photos and files` 选择文件、通过 `+ > Add folder` 选择文件夹，也支持直接粘贴或拖放图片 / 文件进入输入框。附件以输入框顶部的轻量预览条展示：图片显示缩略图，普通文件显示文件卡，文件夹显示目录卡；每个附件可单独移除。第一版发送时将附件转为明确引用文本随消息进入 Agent；后续 Gateway multimodal 完成后再把有二进制内容的图片 / 文件升级为原生多模态 payload。
- 当当前 Session 的权限策略要求确认，且 Agent 即将执行写文件、编辑文件、删除或命令等修改类工具时，`ui_stream_session_message` 必须先写入 `kind = permission`、`status = waiting_permission`、`source = permission_runtime` 的 AgentTimelinePart，再发送 `toolApproval` stream chunk 并阻塞该 tool call。前端在 `composer.input` 内用 Agent 确认面板覆盖原输入框；用户点击 `Allow once` 后回传 `allow_once` 并继续本次 tool call；点击 `Allow this session` 后回传 `allow_session` 并在后端当前 session 内缓存同一 permission pattern；点击 `Allow this project` 后回传 `allow_project` 并写入 `projectId + permission pattern` 的项目级 allow 规则；点击 `Deny always` 后回传 `deny_always` 并写入同 key 的项目级 deny 规则，后续同类请求优先拒绝。确认面板不走全局 Dialog 队列，不浮在页面中央，避免把用户注意力从当前输入上下文移走。
- 右侧面板区默认关闭，不占宽度；关闭时，对话区与输入区横向填满主体空间。
- 状态栏如果保留，只表达应用级状态，不承载当前会话的主交互。
- `/editor` 路由按需懒加载，避免 CodeMirror 和编辑器文件树逻辑进入聊天主工作台首屏主包。

当前实现基准：

- 顶部栏高度为 `28px`，只承载全局菜单、侧栏、搜索、后退 / 前进、窗口控制，并保留可拖拽窗口区域。
- 顶部栏鼠标保持系统默认指针，不使用小手指针；拖拽区域负责移动窗口，按钮区域只执行对应点击动作。
- 左侧栏默认宽度为 `280px`，最小宽度为 `240px`，最大宽度为 `420px`；左侧栏与对话工作台之间提供可拖拽分隔条，用户调整后的宽度进入 `AppState.sidebarWidth` 并随应用状态持久化。鼠标悬停到交界线命中区域时，在交界线中部显示灰色竖杠；左键按住拖拽时竖杠变黑。
- 左侧栏采用统一浅容器布局，模式菜单区、Project / Group 区、`Navis Go` 区复用公共 `PanelSurface`，使用一致的淡边框、圆角和留白；模式切换区保留浅灰 segmented control。各区之间只保留必要间距，避免多层重卡片边框或裸露/有框混搭造成空间浪费。
- Project / Group 区按当前工作模式过滤会话。`mode` 归属独立写入 `Session.metadata.ui.mode`：内建模式使用 `cowork` / `code`，Custom 模式扩展使用 `custom:<runtimeId>`，其中 `runtimeId = <extensionId>/<modeId>`；`group` 只表示真实 Project / Group 名称，不能再用 `code`、`cowork` 这类模式名充当分组名。Custom 页签下方和 Project 区标题需要展示具体模式扩展名称，避免把所有 Custom 会话混到一个泛用 `custom` 桶里。
- 启动、刷新或切换会话后，左侧栏必须从后端 active Session 的 `metadata.ui.mode` 恢复当前模式页签和 Group 过滤；不得只使用前端默认 `code` 页签作为显示来源。
- 对话标题栏高度为 `36px`，采用紧凑布局。
- 对话标题栏从左到右为：会话类型图标、`Navis Go / 当前会话标题`、紧跟标题的标题菜单按钮、弹性占位、右侧面板按钮。
- 标题菜单按钮使用 `src/assets/gateway-v.svg` 作为 mask，颜色继承原灰色按钮色；默认旋转为 `>`，展开状态为 `v`。
- 右侧面板按钮保持在标题栏最右侧，不与标题菜单按钮合并。
- 应用 / 任务栏图标由 `src/assets/gateway-app-icon.svg` 生成 `src-tauri/icons/*`，运行时窗口图标由 Tauri `window.set_icon(...)` 设置；左侧栏底部平台入口显示为 `Navis Go`，使用 `src/assets/navis-product-icon.svg`，菜单数据目标名为 `Gateway`，作为宿主菜单目标标识。

### 3.3 右侧动态面板区

右侧区域采用动态分列面板布局，而不是单个面板 Tab 容器。`rightWorkspace` 仍是历史 UI placement 名，不代表 Project / Worktree / Session 之外的业务概念。

递推规则：

1. 打开 1 个面板：最右侧生成 1 列，竖向填充。
2. 打开 2 个面板：保持 1 列，上下平分。
3. 打开 3 个面板：在最右侧新增 1 列，第 3 个面板独占该列。
4. 打开 4 个面板：第 4 个面板进入第 3 个所在列，该列上下平分。
5. 后续继续按“优先填满最右侧列，再按需新增列”递推。

约束：

- 面板区整体靠右对齐。
- 同层级列宽保持一致。
- 每个面板有最小宽度限制。
- 每个右侧面板使用公共面板外壳（WorkspacePanelShell）；该外壳名称作为 UI 组件历史名保留，复用 `PanelSurface`，在公共浅容器基础上叠加紧凑标题栏、关闭按钮和聚焦态。
- 同一列内多个面板保持独立容器，不直接粘连；面板之间保留小间距。
- 中央对话区优先填充剩余宽度，不允许出现大块无意义留白。
- 打开或重复选择某个面板时，应设置该面板为当前聚焦面板。
- 面板头部提供关闭按钮；关闭最后一个面板时，右侧面板区整体隐藏并释放宽度。
- 右侧面板菜单需要标识已打开的面板项。

#### WorkspacePanel 模块（`components/WorkspacePanel/`）

右侧面板区的组件实现在 `components/WorkspacePanel/` 目录下，核心分工如下：

| 组件 | 职责 |
|------|------|
| `BuiltinRightWorkspaceContent.tsx` | 面板内容分发调度器。接收 `RightWorkspacePanel` prop，通过 `Switch/Match` 按 `viewId` 分发到内建面板；非内建面板走 `HostViewRenderer`。所有非 Design 面板均使用 `lazy()` 懒加载 |
| `DiffPanel.tsx` | Git Diff 面板，支持 Unstaged / Staged 切换，消费 `ui_get_session_git_diff` 和 `ui_list_session_changes`，展示仓库状态、文件变更摘要和 Unified Diff |
| `BackgroundTasksPanel.tsx` | 后台任务面板，消费 `TaskProjection` store，分 Running / Finished 两区展示任务卡片，支持停止运行任务和清除已完成任务；支持 `selectedTaskId` / `selectedSidechainSessionId` 配置项自动滚动定位 |
| `PlanPanel.tsx` | 计划面板，展示 Composer 运行状态中的 pending plan review 和 Session Todos（通过 `sessionTodos` store 轮询） |
| `SessionTranscriptPanel.tsx` | 会话记录面板，消费 `ui_list_session_messages` 读取消息列表，复用 `ConversationTranscript` 组件渲染，支持 standard / compact / raw 三种视图模式 |
| `ToolDiffPanel.tsx` | 工具 Diff 面板，展示工具级别 diff 详情 |
| `WorkspacePanelShell.tsx` | 面板外壳历史命名，委托 `WorkspacePanelFrame` 渲染 |
| `WorkspacePanelFrame.tsx` | 面板框架：使用 `PanelSurface` 渲染外层容器，提供标题栏（标题文本 + `CloseIcon` 关闭按钮）和内容区；导出 `WorkspacePanelScrollArea`（滚动容器）、`WorkspacePanelSectionHeader`（区段标题 + 可选操作按钮）、`WorkspacePanelCard`（任务卡片容器） |
| `shared.tsx` | 公共类型定义（`UiSessionGitDiff`、`UiSessionChange`、`DesignDocLink` 等）、常量（`DESIGN_DOCS` 设计文档索引、`kernelPrimitiveRows` Kernel 四原语）、辅助函数（`taskStatusLabel`、`openTaskTranscript`、`visibleSessionChanges` 等）和 `WorkspaceSectionList` / `SessionTodosSection` 共享组件 |

所有内建面板通过 `BuiltinRightWorkspaceContent` 的 `lazy()` 按需加载，不进入首屏 bundle。`DesignPanel` 保持内联，因其代码量轻且无独立状态。`BUILTIN_VIEW_IDS` 集合定义了当前所有内建视图 ID：`diff`、`tool-diff`、`background-tasks`、`plan`、`design`、`session-transcript`、`editor`。

### 3.4 右侧面板区与扩展关系

- 右侧面板区的布局引擎由宿主内建，扩展不能直接改写列计算规则。
- 扩展可以声明打开为 `rightWorkspace` 面板，由框架分配到合适的列和行；这里的 `rightWorkspace` 只表示 UI placement。
- 扩展可以请求打开、关闭、聚焦自身面板，但不能绕过宿主自行创建自由布局。

### 3.5 Settings 与扩展管理

扩展安装和管理统一放在 Settings 中，主界面只提供能力入口，不承载扩展管理流程。

Settings 使用公共紧凑 dialog 打开；Dialog 内建 `Extensions` 页面，负责：

- 安装本地扩展目录（包含 `extension.json`）
- 启用 / 禁用 / 卸载扩展
- 查看扩展贡献能力统计
- 查看扩展权限声明
- 查看扩展运行状态和启用错误
- 管理 Background Extension（无 UI 扩展）

在目标实现中，`Settings > Extensions` 复用前端 `extensionState` 和后端 `ui_list_extensions` / `ui_install_extension` / `ui_set_extension_enabled` / `ui_uninstall_extension`，安装、启停和卸载后会刷新 `ui_list_menus` 与 Command Palette 扩展声明式命令，确保扩展贡献的菜单、模式和命令进入真实链路；其中扩展 view 命令还必须通过后端可渲染性 gate，不能因为 manifest 声明了 view 就出现在用户入口。扩展页提供 `All extensions`、`Mode extensions`、`Connectors` 三个过滤入口：Mode extensions 使用 `contributionCounts.workModes > 0`，Connectors 当前只把 `contributionCounts.mcpServers > 0` 视为可启用连接器；`contributionCounts.gatewayProviders` 只表示通过 Gateway catalog projection 校验后的 Provider 数量；未注册 Adapter、坏引用或缺失能力的 Gateway contribution 不进入连接器入口。扩展专属配置页和日志页由后续 Settings 子区或扩展贡献页面承载；没有后端配置 / 日志 API 前，不在宿主 UI 中展示假配置和假日志。

`+` 菜单中的 `Add extensions...` 和 `Add connectors` 都打开公共紧凑 Settings dialog 并跳转到 `Settings > Extensions`，但扩展安装流程本身只在 Settings 内完成。`Add connectors` 必须打开 Extensions 页的 Connectors 过滤视图；连接器扩展只定义为贡献 `contributes.mcp_servers` 的扩展，后端 `ui_list_extensions` 通过 `contributionCounts.mcpServers` 暴露该事实，前端不得通过扩展名称或描述猜测。`contributes.gateway` 只有在 Gateway lifecycle 成功注册 Adapter、Provider 和 Model 后，才进入对应的 Gateway catalog projection；manifest 声明本身不代表可用能力。Gateway 语言菜单当前不显示 `Language extensions...`，因为宿主尚未定义 UI language-pack 的后端 manifest/loader 契约；等 i18n 扩展契约落地后再作为真实入口加入。

`/settings` 路由只作为深链和调试入口存在，必须复用同一个 `SettingsDialogContent` 内容组件；Gateway 菜单、`+` 菜单和路由不能各自维护不同的设置界面，避免配置来源、扩展生命周期和表单校验出现分叉。

Gateway 当前只保留两个内建菜单项：

- `Settings`：打开公共紧凑 Settings dialog。
- `Language`：打开二级语言菜单；当前只支持 `zh-CN` 与 `en-US` 两个内建语言。UI 语言包扩展入口在后端 manifest/loader 契约落地前不显示。

Gateway 默认不提供账户、帮助、更新、changelog、登出或推理配置入口。

---

## 四、主题系统

```css
/* CSS 变量定义 */
:root {
  /* 颜色 */
  --color-bg-primary: #ffffff;
  --color-bg-secondary: #f5f5f5;
  --color-text-primary: #1a1a1a;
  --color-text-secondary: #666666;
  --color-accent: #2563eb;
  --color-border: #e5e7eb;
  --color-error: #ef4444;
  --color-warning: #f59e0b;
  --color-success: #22c55e;
  --color-info: #3b82f6;

  /* 间距 */
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  /* 字体 */
  --font-sans: 'Inter', sans-serif;
  --font-mono: 'JetBrains Mono', monospace;
  --font-size-xs: 12px;
  --font-size-sm: 14px;
  --font-size-md: 16px;
  --font-size-lg: 18px;

  /* 圆角 */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;

  /* 阴影 */
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.05);
  --shadow-md: 0 4px 6px rgba(0,0,0,0.1);
}

/* 暗色主题 */
[data-theme="dark"] {
  --color-bg-primary: #1a1a1a;
  --color-bg-secondary: #2a2a2a;
  --color-text-primary: #e5e5e5;
  --color-text-secondary: #999999;
  --color-border: #333333;
}
```

### 4.1 Extension 主题变量接入

- Extension 视图自动继承宿主应用的主题 CSS 变量（`--color-*`、`--spacing-*`、`--font-*` 等），无需额外配置。
- Extension 可通过 `extension.json` 声明自定义 CSS 变量，框架会在加载 Extension 时自动注入到其 DOM 作用域中。

---

## 五、HostView Surface 与扩展视图承接

### 5.1 当前模型

当前实现已经不再使用前端本地组件注入注册表来承接扩展完整界面。扩展 UI 的真实入口是 `contributes.views`，由宿主根据 view contract 打开目标 surface，并选择内置 renderer 渲染。

这套模型的核心约束如下：

- 扩展声明的是 view contract，不是直接注入 DOM 的组件实例。
- 菜单、命令、快捷键和 view 是分离的 contribution；打开完整界面时走 `OpenView` / `ToggleView` 这类声明式 action。快捷键不由扩展直接注册前端 Hotkey handler，而是消费后端公共投影。
- surface 决定“界面放在哪里”，renderer 决定“界面怎么渲染”。
- 宿主负责 placement、可渲染性、生命周期和布局；扩展不能直接篡改主布局。

### 5.2 Surface 与 renderer

当前 HostView 只承接完整界面，不承接菜单项、状态栏 item 或工具栏按钮。这些轻量入口继续使用既有 contribution 类型。

| 字段 | 当前支持 | 说明 |
|------|---------|------|
| `placement` | `rightWorkspace`、`chatAside`、`bottomDrawer`、`settingsSection` | 决定完整界面挂载到哪个宿主 surface |
| `renderer` | `host:panel`、`html:sandbox` | 决定宿主用哪种内置 renderer 承接该 view |

约束：

- `rightWorkspace` 用于右侧动态面板区。
- `chatAside` 用于贴近对话区的实时辅助界面。
- `bottomDrawer` 用于底部抽屉式辅助区。
- `settingsSection` 用于 Settings 内部区段。
- 当前 renderer 基线为 `host:panel` 与 `html:sandbox`；新增 renderer 属于 UI Framework 范围，不进入 Kernel。

### 5.3 打开链路

完整界面的打开路径如下：

1. 扩展在 manifest 中声明 `views`、`commands`、`menus`。
2. 后端 extension host 解析 `contributes.views`，并对 placement / renderer 做可渲染性 gate。
3. `ui_list_extension_views` 独立投影所有 enabled Extension View 列表，统一提供 `UiExtensionView[]`；菜单、command 和 keybinding projection 不再承担 view 发现职责。
4. `ui_list_menus`、命令面板投影和前端菜单 store 只消费通过 gate 的贡献。
5. 用户从菜单、命令面板或其他宿主入口触发 `OpenView` / `ToggleView`。
6. `src/stores/menu-actions.ts` 与 `src/stores/app.ts` 更新 view 打开状态、placement 与聚焦面板。
7. `HostViewSurface` 与 `HostViewRenderer` 读取 view contract 并渲染完整界面。

统一 projection 规则：只返回 enabled Extension；非法 renderer、placement、entry 或静态资源的 view fail-closed；结果稳定排序。`default_visible` 只在加载、安装或启用后的 projection 恢复时生效，不能通过菜单声明间接触发；禁用或卸载时清理该 Extension 的所有 HostView projection。

数据来源规则：

- `views[].config` 只描述标题、作用域、数据源 key、展示偏好等 UI 参数。
- 实时事实数据来自 UI IPC、Kernel EventBus 的 UI 投影或 `foundation::stream`。
- 需要执行文件扫描、任务控制、审计、修复等动作时，renderer 走已有 IPC 或 Tool / Agent Pipeline，不自建执行链。

### 5.4 与右侧面板区的关系

右侧面板区仍然由宿主布局引擎统一控制。`WorkspacePanel` 与 `HostView` 的边界如下：

| 模块 | 职责 |
|------|------|
| `WorkspacePanel/*` | 承接宿主内建 view，例如 diff、plan、background tasks、session transcript |
| `HostView/*` | 承接通过 `contributes.views` 打开的扩展完整界面 |
| `BuiltinRightWorkspaceContent.tsx` | 在内建 view 与 `HostViewRenderer` 之间做分发 |

因此，扩展可以声明把 view 打开到 `rightWorkspace`，但不能直接指定任意像素坐标、列算法或拖拽行为；这些规则都由宿主决定。

### 5.5 边界原则

- HostView contract 属于 UI 域与 extension 域之间的事实约定，不是新的 Kernel 原语。
- Kernel 不感知 DOM、surface、placement、renderer 字符串或面板布局。
- 新增 placement、renderer、surface 只修改 UI Framework 与 extension host。
- 新增 view 所需的业务数据来源时，优先复用既有 IPC、EventBus projection、stream 或 Tool / Agent 能力，不重复造轮子。

## 六、全局状态

### 6.0 状态管理架构

前端状态分四层管理，各层职责明确，避免跨层直接引用：

```
┌─ Layer 1: AppState（顶层全局 Store）──────────────────────┐
│  Solid.js createStore，管理跨模块共享的全局状态             │
│  被所有组件通过 Context Provider 访问                       │
├─────────────────────────────────────────────────────────────┤
│  activeSessionId: string | null    ← 单一来源，所有模块从此读取
│  activeProjectId: string | null    ← 当前项目
│  theme: 'light' | 'dark' | 'system'
│  isOffline: boolean                ← Gateway 离线状态
│  updateAvailable: boolean
│  globalError: Error | null
│  globalLoading: boolean
│  sidebarVisible: boolean
│  rightWorkspaceVisible: boolean
│  rightWorkspaceWidth: number
│  rightWorkspaceColumns: WorkspaceColumn[]
└─────────────────────────────────────────────────────────────┘

┌─ Layer 2: 模块 Store（独立业务状态）───────────────────────┐
│  每个模块维护自己的 Store，通过 Context Provider 注入子树    │
├─────────────────────────────────────────────────────────────┤
│  AgentStore   → AgentStatus, currentTask, streamingText
│  SessionStore → sessions[], messages[], checkpoints[]
│  ProjectStore → currentProject, recentWorktrees, config
│  TaskStore    → activeTasks[], subTasks[]
└─────────────────────────────────────────────────────────────┘

┌─ Layer 3: IPC Event 同步层 ────────────────────────────────┐
│  后端事件通过 useEvent/useStream hooks 自动同步到 Store     │
├─────────────────────────────────────────────────────────────┤
│  session.switched    → AppState.activeSessionId 更新
│  agent.state.changed → AgentStore.state 更新
│  gateway.offline     → AppState.isOffline 更新
│  project.switched    → AppState.activeProjectId 更新
└─────────────────────────────────────────────────────────────┘

┌─ Layer 4: 持久化层 ───────────────────────────────────────┐
│  Config 模块持久化偏好设置                                   │
├─────────────────────────────────────────────────────────────┤
│  theme, keybindings, sidebarVisible → config.set()
│  启动时从 config 恢复到 AppState
└─────────────────────────────────────────────────────────────┘
```

**跨 Store 同步规则：**

```typescript
// ✅ 正确：模块 Store 通过 AppState 间接引用其他模块的状态
const session = useAppState();  // 获取 activeSessionId
const agent = useAgentStore(session.activeSessionId);  // 传入 ID 查询

// ❌ 错误：模块 Store 直接引用其他模块的 Store
const agent = useAgentStore(sessionStore.activeId);  // 禁止跨 Store 直接引用
```

**状态初始化顺序：**
```
应用启动
  │
  ├── 1. 从 Config 恢复 theme/sidebarVisible → AppState
  ├── 2. session.getActive() → AppState.activeSessionId
  ├── 3. project.discover() → AppState.activeProjectId
  ├── 4. agent.getState(activeSessionId) → AgentStore
  └── 5. 订阅 IPC Events → 自动同步所有 Store
```

```typescript
// stores/app.ts（顶层全局 Store）
interface AppState {
  theme: 'light' | 'dark' | 'system'
  windowState: WindowState
  activeSessionId: string | null
  activeProjectId: string | null
  globalLoading: boolean
  globalError: Error | null
  isOffline: boolean
  updateAvailable: boolean
  sidebarVisible: boolean
  rightWorkspaceVisible: boolean
  rightWorkspaceWidth: number
  rightWorkspaceColumns: WorkspaceColumn[]
}

// stores/agent.ts（模块级 Store，通过 Context Provider 注入）
interface AgentState {
  state: AgentStatus           // 枚举：idle | thinking | tool_calling | waiting_permission | streaming | recovering | error
  currentTask: Task | null
  streamingText: string
}

// stores/project.ts（模块级 Store）
interface ProjectState {
  currentProjectId: string | null
  recentWorktrees: RecentWorktree[]
}
```

### 6.1 左 / 右分隔条与右侧面板区调整

#### 分隔条（Resize Handle）

左侧栏与中央对话工作台之间、中央内容区与右侧面板区之间都有可拖拽的分隔条：

```
┌──────────────┬────────────────────────────────────┬────────────────────┐
│ Left Sidebar │ Content Area                       │ Right Panels       │
│              │ （填充剩余空间）                    │ （按列动态展开）    │
└──────────────┴────────────────────────────────────┴────────────────────┘
              ↑                                    ↑
        左侧分隔条（可拖拽）                  右侧分隔条（可拖拽）
```

#### 布局状态

```typescript
interface RightWorkspaceLayout {
  visible: boolean
  width: PanelSize
  minPanelWidth: number
  maxSizeRatio: number
  columns: WorkspaceColumn[]
}

interface WorkspaceColumn {
  width: number
  panels: WorkspacePanel[]
}

interface WorkspacePanel {
  id: string
  title: string
  minHeight: number
  allowClose: boolean
}

interface PanelSize {
  value: number                   // 数值
  unit: SizeUnit                  // 'px' | 'percent'
}
```

`RightWorkspaceLayout`、`WorkspaceColumn`、`WorkspacePanel` 是 UI layout 类型历史命名，只表示右侧动态面板区布局，不作为业务域模型。

#### 交互行为

| 操作 | 行为 |
|------|------|
| 拖拽左侧分隔条 | 实时调整左侧栏宽度，中央内容区联动填充剩余空间 |
| 拖拽右侧分隔条 | 实时调整右侧面板区每一列的统一宽度，中央内容区联动填充剩余空间 |
| 鼠标悬停分隔条 | 在交界线命中区域中间显示灰色竖杠，提示可拖动 |
| 按住分隔条拖拽 | 竖杠变黑，直到释放鼠标 |
| 双击主分隔条 | 右侧面板区宽度切换到上次记忆的大小（或默认宽度） |
| 拖拽到最小值以下 | 右侧面板区自动隐藏 |
| 打开第 1/2/3/4 个面板 | 按动态分列规则自动重排 |
| 关闭面板 | 布局自动回收空位，必要时减少列数 |
| 聚焦面板 | 仅切换焦点态，不改变列结构 |

#### 大小持久化

右侧面板区布局状态通过 `AppState` 持久化到本地存储，下次启动时恢复：
- `AppState.rightWorkspaceVisible` -- 是否可见
- `AppState.rightWorkspaceWidth` -- 右侧面板区整体宽度
- `AppState.rightWorkspaceColumns` -- 当前列布局快照

#### 扩展面板在右侧面板区中的行为

扩展把 view 打开到 `rightWorkspace` 面板区时：

- 共享同一个右侧分隔条和宽度状态
- 每个扩展面板是独立布局单元，不再默认压成单个 Tab 栈
- 右侧面板外壳复用左侧浅容器 `PanelSurface`，保持淡边框、圆角和轻量标题行
- 右侧面板区贴近对话区边界，只保留圆角裁切需要的极小内边距，避免出现弹窗式大留白
- 面板在列中的摆放顺序由宿主 `WorkspaceLayoutManager` 决定
- 扩展可以声明 `default_visible`、`allow_close`，但不能直接指定任意像素位置
- `allow_close` 默认值为 `true`；当值为 `false` 时，宿主不显示关闭控件，并在状态层拒绝关闭请求，保证 UI 约束与生命周期状态一致。
- `default_visible` 默认值为 `false`；它只描述宿主在首次投影、安装或启用后是否恢复打开，不覆盖用户后续手动关闭的状态。

#### HostView Contract 与 Renderer Registry

HostView 是 UI Framework 与扩展域之间的稳定 contract，不是 Kernel 原语。contract 集中在 `src-tauri/src/extension/host_view.rs`，由公共能力判断统一校验 renderer 与 placement；菜单、Command Palette 和快捷键投影都复用同一判断，不建立各自的白名单或 registry。

扩展 view 由 UI Host view renderer 解释。renderer 是 UI Framework 的内置前端视图渲染策略，不进入 Kernel，也不作为新的后端桥接层。它不是完整Extension，不包含独立前后端运行时；扩展或宿主内建 contribution 只声明 view、command、menu 和 placement，宿主负责打开目标 UI surface、读取 view contract、通过 `HostViewRenderer` 选择 renderer 并渲染完整界面。

`renderer` 和 `placement` 必须分离，并由同一 HostView contract 校验：

| 字段 | 说明 |
|------|------|
| `placement` | 完整 Host view 放在哪里，当前只允许 `rightWorkspace`、`chatAside`、`bottomDrawer`、`settingsSection` |
| `renderer` | 视图怎么渲染；当前支持 `host:panel` 与 `html:sandbox` |

`chatAside` 表示贴近对话区右侧的轻量实时界面，适合计划任务、子 agent 实时状态、当前运行队列等需要常驻但不应挤进消息 DOM 的内容。它不是右侧动态面板区的同义词；是否显示、宽度、折叠和与对话区的关系由 UI Framework 管理。菜单、状态栏和输入工具栏不是 Host view surface；它们使用已有 contribution，不通过 `OpenView` 打开完整界面。

当前 registry 基线：

| renderer | 用途 | 数据来源 |
|----------|------|----------|
| `host:panel` | 通用宿主面板，承接扩展信息、设计面板、计划任务、实时状态、子 agent 状态等完整界面 | View contract + UI IPC / Kernel EventBus projection / foundation::stream |
| `html:sandbox` | 扩展提供的静态 HTML 界面 | `entry` 指向扩展 `ExtensionUI/` 目录；宿主以 sandbox iframe 加载 |

`html:sandbox` 的资源合同是静态文件合同：HTML、CSS、JavaScript、jQuery 等依赖必须由 Extension 自行打包到安装目录的 `ExtensionUI/**` 中，并通过相对路径互相引用。宿主只接受经过资源边界校验并投影出的 `resource_path`，拒绝远程 URL、`file://`、路径穿越和符号链接逃逸；不注入 CDN、不提供通用 Tauri IPC bridge，也不把 iframe 内容当作可执行 Extension runtime。

计划任务、实时状态、子 agent 状态等宿主数据界面使用 UI 域的通用 Host view renderer：`host:panel`；扩展静态 HTML 界面使用 `html:sandbox`。renderer ID 只是宿主选择前端渲染策略的标识，不是 Extension 名称，也不是 Kernel capability 类型；具体展示由 `views[].config`、UI IPC、Kernel EventBus 投影或 `foundation::stream` 提供数据。未知 renderer、未知 placement、缺失 placement 或无效 entry 必须 fail-closed。

Host view renderer 复用成立的前提：

- 入口落在已承接的 `MenuTarget`，例如 `Tools` 或 `RightPanel`。
- 命令使用已有声明式 action，例如 `OpenView` / `ToggleView`。
- view 的 placement 和 renderer 都属于宿主白名单。
- renderer 所需配置来自 `views[].config`，实时数据来自后端 IPC、Kernel EventBus 的 UI 投影或 `foundation::stream`。
- 需要执行扫描、修复、审计、任务控制等动作时，命令只触发已注册 Tool / Agent Pipeline，不在 UI renderer 内自建执行链。

需要修改宿主代码的情况只有四类：新增 `MenuTarget`、新增 placement、新增 action 类型、或新增宿主 renderer 类型。新增 placement/renderer 仍属于 UI Framework，由 UI 域承接，不修改 Kernel。未来如果某个 renderer runtime 需要安装、启停或替换，UI 域也只是复用通用 capability 生命周期，Kernel 不增加 renderer 语义。

因此，后期如果要增加一个靠近对话区右侧、实时展示计划任务或子 agent 状态的界面，设计上不需要改 Kernel：后端把 task/subagent 状态作为事实源和事件/stream 投影出来，UI 域增加或复用 `chatAside` placement 和对应 Host view renderer，并用现有菜单/命令/view 声明打开即可。

示例：

```json
{
  "contributes": {
    "views": [
      {
        "id": "background-tasks.panel",
        "name": "Background tasks",

        "placement": "chatAside",
        "renderer": "host:panel",
        "config": {
          "title": "Background tasks",
          "sessionScoped": true,
          "source": "task_projection"
        },
        "activation_events": [],
        "allow_close": true,
        "default_visible": false
      }
    ],
    "commands": [
      {
        "id": "background-tasks.open",
        "label": "Background tasks",
        "category": "Status",
        "action": { "type": "OpenView", "view_id": "background-tasks.panel" }
      }
    ],
    "menus": [
      {
        "id": "tools.background-tasks",
        "label": "Background tasks",
        "target": "Tools",
        "command": "background-tasks.open",
        "group": "status"
      }
    ]
  }
}
```

渲染约束：

- UI Host view renderer 只能消费宿主传入的 view contract 和经过 IPC/Tool/Event/Stream 能力读取的数据。
- `config` 只描述 renderer 需要的标题、作用域、数据源 key、展示偏好等 UI 参数，不作为事实源。
- renderer 不直接操作 Kernel Registry、Pipeline、EventBus、Policy；需要读取文件或执行架构检查时，走已有 UI IPC 或 Tool/Agent Pipeline。
- 第三方扩展当前不能仅凭 manifest 提供自定义 renderer；新增 renderer 必须先由 UI Framework 增加对应 contract、registry 条目和 renderer 实现。Kernel 不增加 renderer 概念，也不感知 DOM、菜单、布局或 renderer 字符串。

#### Agent 可视化面板边界

扩展 Agent 可视化面板当前只能作为满足 HostView contract 的声明式 view 接入。扩展声明 `placement`、`renderer` 和 `config`；使用 `host:panel` 时不声明 `entry`，使用 `html:sandbox` 时必须声明扩展 `ExtensionUI/` 目录下的相对 `entry`。宿主通过 `HostViewRenderer` 将其放入对应 UI surface，并对 HTML view 使用仅允许脚本执行的 sandbox iframe；不支持远程 URL、`file://` URL、动态 Solid 组件或任意 ES Module renderer。

面板生命周期：

1. 用户通过已承接菜单、command 或快捷键入口请求 `OpenView` / `ToggleView`。
2. 后端先复用 HostView contract 和声明式 action 映射；不满足 contract 的 view 或 action fail-closed，不产生用户可见入口。
3. 宿主将合法 view 放入 `rightWorkspace`、`chatAside`、`bottomDrawer` 或 `settingsSection`，布局由 UI Framework 管理。
4. 面板只消费宿主传入的 view contract，以及经过 UI IPC、Kernel EventBus UI 投影或 `foundation::stream` 提供的事实。
5. 面板关闭或扩展禁用时，宿主移除对应 surface 投影和订阅；扩展不能直接控制绝对位置或布局规则。

---

## 七、事件定义

```typescript
type UIEvents = {
  'ui.theme.changed':      { theme: string }
  'ui.sidebar.toggled':    { visible: boolean }
  'ui.right-workspace.toggled': { visible: boolean }
  'ui.right-workspace.layout.changed': { columnCount: number; panelCount: number }
  'ui.view.changed':       { view: string }
  'ui.error.boundary':     { error: Error; component: string }
}
```

---

## 八、测试策略

```
单元测试：组件渲染、状态管理、主题切换、右侧面板区布局递推
集成测试：布局响应式、HostView surface 渲染、右侧动态面板区开关与重排、路由导航
```

