# 23 - Command Palette 命令面板 详细设计

> 模块编号：23 | 层级：UI 层
> 依赖：22-UI-Framework, 02-Event+IPC
> 被依赖：无（顶层交互组件）

---

## 一、模块概述

### 1.1 定位

Command Palette 是快速访问入口，支持命令搜索、文件搜索、Slash commands 触发、AI 推荐。

### 1.2 职责边界

```
负责：
├── 命令注册（来自所有模块的命令）
├── 模糊搜索（命令名/描述/快捷键）
├── 文件搜索（快速打开文件）
├── Slash commands 搜索（候选可来自 Skills、轻量命令和扩展声明式命令）
├── AI 命令推荐（根据用户输入推荐相关操作）
└── 最近使用记录

不负责：
├── 命令执行 → 各模块
└── 全局搜索 → File / MCP / LSP
```

---

## 二、架构设计

```
src/components/CommandPalette/
├── CommandPalette.tsx      # 主组件
├── CommandList.tsx         # 命令列表
├── SearchInput.tsx         # 搜索输入框
├── CommandItem.tsx         # 单个命令项
├── useCommandPalette.ts    # Hook
└── store.ts                # 命令目录与状态存储

src/components/SlashCommandDropdown.tsx  # 输入框内联 Slash 命令下拉组件
```

#### SlashCommandDropdown 组件

`SlashCommandDropdown` 是独立于 Command Palette 的内联下拉组件，在 Composer 输入区直接使用。当用户在输入框中输入 `/` 时自动弹出，列出所有可用的 slash commands（来自 `CommandPalette/store` 的命令数据）。

**核心特性：**
- 键盘导航：`↑` / `↓` 移动选中项，`Enter` 确认选择，`Escape` 关闭下拉
- 模糊过滤：按 `label`、`description` 和 `tags` 匹配用户输入（忽略 `/` 前缀）
- 点击外部关闭：通过 `mousedown` 事件监听容器外点击自动收起
- 选中态跟随查询变化重置：`query` 改变时回到第一项

**Props：**
```typescript
interface SlashCommandDropdownProps {
  visible: boolean;          // 控制下拉显示/隐藏
  query: string;             // 当前输入内容（含 "/" 前缀）
  commands: Command[];       // 候选命令列表
  onSelect: (command: Command) => void;  // 用户选择命令时回调
  onDismiss: () => void;                 // 关闭下拉时回调
}
```

**与 Command Palette 的关系：**
- 两者共享 `CommandPalette/store.ts` 中的命令数据源
- Command Palette 是全局浮层（`Ctrl+Shift+P`），搜索范围包括命令、文件、符号、Slash 命令
- SlashCommandDropdown 是输入框内联下拉，仅展示 slash 命令子集，无需手动打开面板
- 用户在 SlashCommandDropdown 中选择 command 或 skill 类型命令时，将 `/trigger` 插入输入框；选择 extension 命令时立即执行其声明式动作

---

## 三、数据模型

```typescript
interface Command {
  id: string
  label: string
  description?: string
  category: string
  keybinding?: string
  icon?: string
  handler: () => void | Promise<void>
  isEnabled?: () => boolean
  source: 'builtin' | 'extension' | 'skill' | 'command'
}

**扩展命令注册边界说明：**
- 当前宿主通过后端 `ui_list_extension_commands` 暴露已启用扩展中的声明式 `BuiltinAction` 命令。
- 当前用户可见链路只支持 `OpenView` / `ToggleView`。命令执行时，宿主根据扩展的 `HostView` contract 解析 `viewId`、`placement`、`renderer` 和 `config`，再交给对应的宿主 surface 和 renderer；扩展不能直接改写宿主布局。
- `Command.handler` 是前端 Command Palette 内部的宿主回调，用于调用已注册的宿主 command/action 执行器；它不是扩展提供的执行函数，也不会加载或执行扩展 ES Module。
- `BuiltinAction` 当前只定义 `OpenView` 和 `ToggleView`，不存在需要在 Command Palette 中额外过滤的其他 action 变体。缺少 `action` 的 manifest 由 loader 拒绝；目标 view 不满足 HostView contract 时，后端 projection 不输出 command、menu 或 keybinding。

interface CommandPaletteState {
  isOpen: boolean
  query: string
  selectedIndex: number
  commands: Command[]
  filteredCommands: Command[]
  recentCommands: string[]  // 最近使用的命令 ID
}

recentCommands 存储策略：
- 持久化位置：localStorage（key: `navis.commandPalette.recentCommands`）
- 最大条数：20
- 格式：`string[]`（命令 ID 列表，按最近使用时间倒序）
```

> **extension.json commands：声明式 BuiltinAction**
>
> 扩展 command 只声明命令元数据和宿主支持的 `BuiltinAction`，不提供可由宿主加载执行的 handler 模块。当前用户可见的扩展 command action 为：
>
> - `OpenView` — 打开扩展声明的 HostView
> - `ToggleView` — 切换扩展声明的 HostView 的显示状态
>
> `OpenView` / `ToggleView` 的目标必须匹配扩展的 `HostView` contract。contract 至少提供视图 ID、挂载位置、renderer 和可选 config；宿主据此创建 view instance，并通过已注册的 surface/renderer 完成渲染。
>
> ```json
> {
>   "id": "my-extension.togglePanel",
>   "action": { "type": "ToggleView", "view_id": "panel" }
> }
> ```
>
> 前端 `Command` 对象仍可包含 `handler: () => void | Promise<void>`，但该字段只表示宿主内部回调。它负责把命令选择转交给宿主声明式 action dispatcher，不代表扩展代码执行入口。

---

## 四、交互设计

```
Ctrl+Shift+P 打开命令面板

┌─────────────────────────────────────────────────┐
│ 🔍 输入命令...                                   │
├─────────────────────────────────────────────────┤
│ 📋 新建会话                    Ctrl+Shift+N     │
│ 📋 切换会话                    Ctrl+Tab         │
│ 📋 打开设置                    Ctrl+,           │
│ 📋 切换终端                    Ctrl+`           │
│ 📋 打开文件                    Ctrl+O           │
│ 🤖 /review - 代码审查                             │
│ 🤖 /commit - 生成提交信息                          │
│ 🤖 /explain - 解释代码                            │
│ 🔌 扩展: 格式化代码                               │
└─────────────────────────────────────────────────┘

输入 ">": 过滤命令
输入 "@": 搜索文件
输入 "/": 搜索 Slash commands（Skills、轻量命令和扩展声明式命令）
输入 "#": 搜索符号
```

> 这些前缀仅用于 Command Palette 内部搜索过滤。Composer 输入区的扩展/能力引用主路径仍是 `B 区 + 菜单`，输入快捷触发器以 `/` 为主。

当前宿主内建命令由顶部栏注册并在主布局挂载的全局 Command Palette 中展示：

- `app.commandPalette.open`：打开命令面板。
- `app.sidebar.toggle`：切换左侧栏。
- `navigation.back` / `navigation.forward`：执行浏览器历史后退 / 前进。
- `window.minimize` / `window.toggleMaximize` / `window.close`：调用 Tauri Window API。

当前 Command Palette 视觉规则：

- 弹层使用紧凑命令菜单高度，内容最大高度控制在视口内的小型浮层，不占用大面积空白。
- 来源标记使用轻量文字 / 符号，不使用 emoji 图标；菜单字体、选中态和快捷键样式与全局 `FloatingMenu` 保持同一层次。
- 第一版可见 UI 文案统一使用英文：scope indicator 显示 `> Commands`、`@ Files`、`/ Slash`、`# Symbols`；空状态、底部键盘提示和无障碍标题也使用英文。`Skill` 只作为 slash 候选项的 source label，不作为 `/` 范围的名称。

当前扩展声明式命令由 `src/stores/extension-commands.ts` 在应用启动、扩展安装、启停、卸载后刷新：

- 后端 `ui_list_extension_commands` 只返回已启用扩展中带 `OpenView` / `ToggleView` action 的命令。
- 前端注册为 `source = "extension"` 的 Command Palette 命令。
- 执行时调用同一套声明式动作执行器，最终进入 `rightWorkspace.openRightWorkspacePanel()` 或关闭已有面板。

当前 Slash commands 数据由 `src/stores/slash-commands.ts` 在应用启动时通过后端 `ui_list_slash_commands` 加载：

- 后端复用共享 `Skills` 状态，启动时执行 `Skills::load_all()`，加载内置 / 用户 / 项目级 Skills 与轻量命令；扩展启用/禁用时由 `ExtensionLifecycle` 直接向该共享状态注册/卸载 `contributes.skills`。
- 候选集来自 `Skills::get_trigger_candidates()`，只暴露已启用 Skill 与可用轻量命令；`needs_review`、禁用项不会进入用户可见菜单。
- 如果轻量命令与 Skill 存在同名 `/trigger` 冲突，候选列表与真实执行链都遵守 `Command > Skill` 优先级，不展示重复项。
- 前端把轻量命令注册为 `source = "command"`，把标准 / 增强 Skill 注册为 `source = "skill"`。
- 选择 `source = "command"` 或 `source = "skill"` 项时，不自动发送消息，而是把 `/trigger ` 插入当前 composer 输入框；扩展声明式命令仍立即执行其 `OpenView` / `ToggleView` 动作。
- `slash` 范围会同时展示内置 / 用户 / 项目 / 扩展 Skills、轻量命令和扩展声明式命令，保证 `+ > Slash commands` 能发现真实可用能力。

当前 `@` 文件搜索由 `src/components/CommandPalette/store.ts` 直接使用当前 active session 的 worktree snapshot：

- 数据源是后端 `ui_get_session_worktree_snapshot` 返回的 `worktreeFiles` 扁平列表，由 `src/stores/worktree.ts` 管理。
- Palette 只在 `worktreeState.currentWorktree.path === activeSession.worktreeRoot` 时展示文件结果，避免跨会话复用旧文件列表。
- 第一次进入 `@` 且当前 worktree 未加载时，`useCommandPalette` 会触发 `loadSessionWorktree(activeSessionId)`，加载完成后重新执行当前查询。
- 文件结果注册为 `source = "file"` 的临时 Command，执行后调用 `requestEditorWorktreeFileOpen(relativePath)`，并打开或聚焦右侧宿主内建 `File` 面板；真正读取文件和创建文档状态由 `WorktreeEditor` 的 file-panel 模式消费 pending 请求完成。
- `@` 搜索不读取文件内容、不扫描额外目录、不自动发送消息，符合 opencode / Claude Code 的“按需打开/读取”边界。

当前 `#` 符号搜索采用 Navis Go 本地轻量 symbol index，而不是伪装成 LSP：

- 第一次进入 `#` 且当前 worktree 未加载时，`useCommandPalette` 会先触发 `loadSessionWorktree(activeSessionId)`；worktree snapshot 就绪后再后台构建 symbol index。
- Symbol index 只读取当前 active session worktree 内的常见代码文件，跳过 `node_modules`、`dist`、`target`、`build`、`.git`、`coverage` 等目录，并限制单轮最多索引 500 个文件。
- 索引规则覆盖常见声明形态：TS/JS 的 `class / interface / type / enum / function / const arrow`，Rust/Go/Python/Java/C# 等语言的 class/type/function/method 基础声明。它是轻量导航能力，不替代 LSP `workspace/symbol` 协议能力。
- 符号结果注册为 `source = "symbol"` 的临时 Command，显示 `{kind} · {file}:{line}`；执行后调用 `requestEditorWorktreeFileOpen(relativePath, { line })` 并打开或聚焦右侧宿主内建 `File` 面板。
- `WorktreeEditor` 的 pending open 请求承载路径与可选行列；文件打开后把 navigation target 传给 `EditorView`，由 CodeMirror 滚动并选中对应行。这样 `@` 与 `#` 共享同一条右侧 `File` 面板打开链路，只是 `#` 多一步行定位。
- 后续接入真实 LSP `workspace/symbol` 时，替换 symbol provider 即可；Command Palette、Editor pending open、扩展命令执行链路不新增旧路径分支。

`Ctrl+Shift+P` 由 Hotkey 模块的默认绑定 `commandPalette.open` 触发，并通过顶部栏挂载的 `useHotkeyCommand` 调用 `commandPaletteAPI.open()`；顶部栏搜索按钮也调用同一个 API。顶部栏 `☰` 菜单按钮打开真实 `Tools` 浮层，其中 `Command palette` 内建项调用 `commandPaletteAPI.open('commands')`，作为主命令入口。

---

## 五、AI 推荐

### 5.1 推荐流程

```
用户输入："修复"
     │
     ▼
AI 推荐引擎分析：
├── 匹配关键词 "修复"
├── 结合当前上下文（打开的文件、语言、Git 状态）
│
     ▼
推荐结果：
├── 🤖 /bug-fix - Bug 修复工作流（Skill）
├── 📋 运行测试并修复（命令）
├── 🔧 修复 ESLint 错误（命令）
└── 🔧 修复 TypeScript 类型错误（命令）
```

### 5.2 AI 推荐实现方式

**AI 推荐实现方式：**
- AI 推荐不是独立接口，而是 `searchCommands()` 的内部增强行为
- 当用户输入自然语言（如"修复"、"优化"）而非精确命令名时，searchCommands 内部调用 AI 推荐引擎对结果重排序
- AI 推荐引擎可选：本地模糊匹配为基础，远程 AI 推荐为增强（用户显式请求时触发）

### 5.3 AI 推荐实现说明

```
策略分层（优先本地，按需远程）：
├── 前端本地关键词匹配（默认，不消耗 API 额度）
│     ├── 对命令名、描述、标签进行模糊匹配
│     ├── 结合最近使用频率加权排序
│     └── 覆盖 80% 的常见推荐场景
└── 后端 AI 推荐（可选，复杂场景）
      ├── 用户显式请求深度推荐时触发
      ├── 结合当前文件上下文、Git 状态等
      └── 调用后端 AI 接口，返回结构化推荐列表
```

---

## 六、接口定义

```typescript
// 命令注册
commandPalette.register(command: Command): void
commandPalette.unregister(id: string): void
commandPalette.registerBatch(commands: Command[]): void

// Extension 命令注册链路（当前实现）
// 应用启动或扩展生命周期变化 → ui_list_extension_commands()
// → 投影已启用扩展中可解析为 OpenView / ToggleView 的声明式 BuiltinAction
// → 根据 HostView contract 解析 viewId / placement / renderer / config
// → commandPalette.registerBatch()
// → 前端宿主内部 handler 调用声明式 action dispatcher 和对应 surface/renderer
// Slash commands 注册链路（当前实现）
// 应用启动 → ui_list_slash_commands()
// → 复用 Skills::get_trigger_candidates() 输出可用 Commands / Skills
// → commandPalette.registerBatch()
// → 轻量命令 / Skill 选择后插入 composer 输入框中的 /trigger

// File search 链路（当前实现）
// 输入 @ → worktreeState.worktreeFiles
// → 生成 source=file 的临时命令
// → requestEditorWorktreeFileOpen(relativePath)
// → openRightWorkspacePanel({ viewId: 'editor', title: 'File' })
// → WorktreeEditor file-panel 消费 pending open 并读取当前 session worktree 文件

// Symbol search 链路（当前实现）
// 输入 # → ensureWorktreeSymbolIndex()
// → 读取当前 session worktree 的常见代码文件并提取轻量声明符号
// → 生成 source=symbol 的临时命令
// → requestEditorWorktreeFileOpen(relativePath, { line })
// → openRightWorkspacePanel({ viewId: 'editor', title: 'File' })
// → WorktreeEditor file-panel 打开文件，EditorView 跳转到声明行

// 打开/关闭
commandPalette.open(scope?: 'commands' | 'files' | 'slash' | 'symbols'): void
commandPalette.close(): void

// 搜索（按类型拆分，各自返回对应类型）
commandPalette.searchCommands(query: string): Command[]
commandPalette.searchFiles(query: string): FileResult[]
commandPalette.searchSymbols(query: string): SymbolResult[]

// 搜索 Slash commands（/ 前缀时调用）
commandPalette.searchSlashCommands(query: string): Promise<CommandPaletteItem[]>
```

---

## 七、测试策略

```
单元测试：模糊搜索、命令过滤、最近使用排序
集成测试：命令注册/触发、AI 推荐、快捷键交互
```
