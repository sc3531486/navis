# 07 - Cordis Extension 基座与清单 详细设计

> 边界说明：本文描述通用 Navis 扩展运行时。Gateway、Agent、Session 等名称表示扩展贡献或能力合同，不表示这些业务属于 `src-tauri/src/`；Navis Code 业务位于 `extensions/navis-code/`。

> 模块编号：07 | 层级：extension 大域
> 依赖：Cordis（Rust：cordis-rs；前端宿主：@cordisjs/core）, 01-Logger, 02-IPC, 02b-Stream, kernel::EventBus, 06-Sandbox, 03-Config, 04-Storage
> 被依赖：22-UI-Framework, 23-Command-Palette, 27-Hotkey, 12-Gateway, 13-MCP, 16-Agent

---

## 一、模块概述

### 1.1 定位

Extension 是应用层基于 Cordis 的扩展组合与服务生命周期层。它读取 Extension manifest、校验 contributes、管理安装 / 启用 / 禁用 / 卸载状态，并把每个扩展点从固定目录 `ExtensionUI/` / `ExtensionBackend/` 装载为 Cordis plugin/service，通过 capability port 交给已有宿主域承接。

Cordis `Context` 提供类型化服务容器，`Plugin`/`Service` 表达扩展单元，`Inject` 声明服务依赖，`Fiber` 管理插件生命周期，`effect`/disposer 回收运行时副作用。Kernel 仍保留 Registry / Pipeline / EventBus / Policy 四个通用原语；Cordis 负责扩展装配与生命周期，不复制这四原语。Extension 声明的 Tool、Provider、UI、Hook、Skill、Settings 等能力，启用后必须作为 Cordis service 注入并进入对应业务域与 Kernel 原语：

| 声明能力 | Cordis 装配 | 承接方 / Kernel 接入 |
|----------|-------------|----------------------|
| MCP / Tool | 插件 service 注入 Tool Projection / MCP | Registry / Pipeline / Policy / EventBus |
| Gateway Adapter / Provider / Model / middleware | 插件 service 注入 `ai/gateway` | Registry / Pipeline / Policy |
| UI menu / command / view | 前端宿主 Cordis service + UI 功能贡献承接器 | EventBus 只读通知；动作进入现有 command / view / pipeline |
| Hook | 插件 service 注入 `ai` / `tool` 宿主管线 | PreToolUse deny → Policy constraint；观察/改写型 hook → Pipeline stage |
| Skill / role | 插件 service 注入 `extension/skills` + Agent | SkillStore + Agent Pipeline；真实工具仍走 Registry / Policy |
| 配置 / 权限声明 | 插件声明映射到 Config / Sandbox | Policy |

因此，本文档保留的是“Extension manifest、固定目录契约和 Cordis 插件生命周期规则”，不是自研平行扩展框架，也不是 Kernel 原语。当前产品 UI 名称可以继续显示为 `Extensions`，但设计语义统一按 Extension 理解。

Extension 不是 Tool 的同义词。Extension 可以声明 Tool、Skill、UI、Hook、Settings 或外部 provider；Tool 才是模型可调用、需要权限、耗时、结果回注和 AgentTimelinePart 展示的能力。Skill 是专业工作流/提示词/工具白名单包，不直接执行系统能力。Extension 如果要让 Agent 调用浏览器、电脑控制、数据库、外部自动化或 memory provider，必须声明 Tool；外部能力优先通过 MCP server 进入 Kernel Registry，并由 Tool Projection 暴露给模型。

### 1.2 核心原则

```
1. Cordis 是唯一扩展装配底座：plugin/service/Inject/Fiber/effect 全走 Cordis；不复制 Registry / Pipeline / EventBus / Policy
2. 清单只声明能力：manifest/contributes 是插件元数据，不是运行时执行器
3. 启用时分发承接：loader 将 `ExtensionUI/` / `ExtensionBackend/` 下的扩展点装载为 Cordis plugin/service，能力进入 MCP / Gateway / UI / Skills / Hook 等宿主域
4. 权限统一治理：权限声明进入 Sandbox / Policy，不由 Extension 私自判断
5. 固定目录：前端扩展点 `ExtensionUI/`，后端扩展点 `ExtensionBackend/`
6. MCP 协议标准化：不自定义私有协议，不改 JSON Schema

**当前实现状态：**

- 扩展 command 当前只通过声明式 `BuiltinAction + HostView contract` 进入宿主；前端 `Command.handler` 仅是宿主内部回调，不是扩展执行入口。
- `contributes.triggers` 与 `contributes.behaviors` 当前只保留为 manifest/schema 预留声明，宿主不会加载或执行其中的扩展 ES Module，也不会为它们创建运行时索引或 action dispatcher。
- 当前 `BuiltinAction` schema 只包含 `OpenView` 和 `ToggleView`。命令缺少 `action`，或其目标 view 不满足 HostView contract 时，loader / UI projection 会 fail-closed，不进入用户可见入口。
- `BehaviorAction` / `TriggerAction` 中列出的其他动作属于独立的 manifest/schema planned contract，不属于当前扩展 command 执行链；下文示例不会被当前宿主加载或执行。
```

### 1.3 Extension 类型与管理入口

Extension按是否需要主界面 UI 分为两类：

| 类型 | 定义 | 示例 |
|------|------|------|
| UI Extension | 在 Navis Go 的某个 UI 区域注册入口、渲染器或面板 | 右侧面板区面板、消息渲染器、Custom 模式菜单、`+` 菜单项 |
| Background Extension | 不注册主界面 UI，只提供后台能力 | Gateway Adapter / Provider、MCP Server、Context Provider、Agent Hook、File Watcher、Notification Channel |

统一规则：

- `ui` 不是 Extension 必需能力，Background Extension 可以完全没有界面挂载。
- Extension 安装、启用、禁用、卸载、权限查看、贡献统计和启用错误查看统一在 `Settings > Extensions` 中完成。UI 名称可以暂用 Extensions，但设计语义是 Extensions。
- `+` 菜单中的 `Add extensions...` 只作为快捷入口，打开 `Settings > Extensions`。
- 左侧 Cowork / Code 的 `Customize` 和 Custom 空态的 `Add mode extensions...` 只作为模式扩展快捷入口，打开 `Settings > Extensions` 的 Mode extensions 过滤视图；该过滤来自 `contributes.work_modes`，不得靠扩展名称猜测。
- 所有Extension，无论是否有 UI，都必须在 `Settings > Extensions` 中可见、可启停、可卸载。
- Extension可以注册 Settings 子区作为自己的配置页或日志页；在宿主提供对应后端 API 前，不展示假配置和假日志。
- Extension 不能绕过统一管理页自行实现安装/卸载流程。

### 1.4 简化后的能力心智模型

为了降低扩展作者理解成本，`contributes` 虽然仍按具体字段声明，但概念上收敛为四类；每类最终都映射为 Cordis plugin/service 或宿主 capability port：

| 能力组 | 说明 | 典型 contributes |
|--------|------|------------------|
| `actions` | 可被菜单、快捷键、命令面板或输入区触发的动作 | `commands`, `keybindings`, `menus` |
| `ui` | 按界面区域挂载的 UI 能力 | `views`, `menus`, `inline_extensions`, `toolbar_items`, `statusbar_items` |
| `hooks` | 参与 Agent、Context、文件、搜索等宿主流程 | `hooks`, `context_providers`, `search_providers`, `file_watchers` |
| `integrations` | 接入外部系统或底层能力 | `mcp_servers`, `tools`, `gateway`, `middlewares`, `transport_adapters`, `languages`, `notification_channels`, `tray_items` |

UI 类能力优先按区域理解，而不是按 DOM 实现理解：

| UI 区域 | 扩展可扩展内容 | 典型 contributes |
|---------|----------------|------------------|
| `topbar` | 顶部栏菜单、按钮、搜索入口、窗口级操作入口中的少量全局动作 | `menus`, `commands`, `keybindings` |
| `leftSidebar` | 模式菜单、Custom 模式扩展菜单、Group 菜单、会话右键菜单、Gateway 菜单 | `menus`, `views`, `commands` |
| `chatHeader` | 会话标题菜单、右侧面板区开关菜单、标题区动作 | `menus`, `commands` |
| `chatMessages` | 消息渲染、结果卡片、表格、验证信息、消息级操作 | `inline_extensions`, `menus`, `commands` |
| `composer.input` | A 区：输入内容承载、引用 token、结构化片段、扩展结果插入 | `inline_extensions`, `triggers`, `commands` |
| `composer.toolbar` | B 区：`+` 菜单、权限、模型、上下文状态、输入相关控制 | `menus`, `toolbar_items`, `commands` |
| `rightWorkspace` | 右侧动态分列面板区 | `views`, `commands` |
| `chatAside` | 对话区右侧贴边轻量界面，用于实时计划、子 agent 状态、运行队列等 | `views`, `commands` |
| `settings` | 扩展配置页、说明页、管理页入口 | `configuration`, `views` |
| `statusbar` | 可选系统状态栏，只放应用级状态 | `statusbar_items`, `commands` |

区域规则：

- 扩展 contributes 进入 UI 前必须先归一到 `22-ui-framework.md` 定义的 `FeatureContribution` 承接规则；宿主内建功能和扩展功能共用菜单、命令、视图和 action 分发链。
- `composer.toolbar` 中的 `+` 菜单是扩展、命令、连接器、附件的主发现路径。
- `composer.input` 只承载当前输入和已选择能力的引用结果，不作为扩展按钮堆叠区。
- `rightWorkspace` 只接收右侧动态面板区内容，动态分列算法由宿主框架负责；名称只作为 UI placement 保留。
- `chatAside` 是对话工作台内贴近消息区右侧的轻量 surface，不等同于右侧动态面板区；适合计划任务和子 agent 实时状态这类跟当前会话强相关的常驻状态界面。
- 扩展菜单只能注册到 UI Framework 已承接的 `MenuTarget`，不能把菜单挂到任意 DOM 位置。
- `leftSidebar` 的 Custom 模式菜单只展示已启用扩展贡献的 `work_modes`；菜单显示名优先使用 `WorkModeRegistration.name`，缺省使用 `ExtensionManifest.name`。
- `work_modes` 注册的是完整工作模式，不是普通菜单项。点击后会切换当前会话的 Agent 模式，重新加载角色、工具白名单、技能、上下文策略、模型偏好和默认 UI 入口。
- Background Extension 可以不注册任何 UI 区域，但仍必须在 `Settings > Extensions` 中可见、可配置、可禁用。
- 扩展如果只是给桌面端增加一个菜单入口和对应界面，不需要新增菜单系统或路由系统：声明 `views + commands + menus`，命令使用 `BuiltinAction::OpenView` 或 `ToggleView`，菜单挂到已支持的 `MenuTarget`。宿主负责按 `views[].placement` 把 view 放入对应 UI surface。
- 完整界面优先使用宿主 Host view renderer，而不是新增专用页面或桥接层。Host view renderer 是 UI Framework 的内置视图渲染策略：它不是完整Extension，不包含独立前后端运行时，也不是扩展贡献的新内核能力。新增 Host view renderer 或 placement 由 UI 域承接，不修改 Kernel。界面读取数据、执行扫描、权限检查仍走已有 UI IPC、Tool/Agent Pipeline、Kernel EventBus 投影、`foundation::stream` 和 Policy，不允许扩展直接操作 DOM 或绕过内核原语。
- 扩展贡献 MCP 工具时，可以通过工具自身或 `mcp_tool_overrides` 声明 `ui_hint`、`model_name`、`renderer_hint` 和 `declared_risk`。`ui_hint` 用于菜单、trace 和审计友好展示，不进入模型上下文；`model_name` 是模型可见 provider-safe 名称，缺省由 Tool Projection 从 MCP canonical name 生成；`renderer_hint.renderer` 和 `renderer_hint.detail_view` 用于前端 `ToolRendererCatalog` 精确选择扩展 renderer / detail view；`declared_risk` 只能作为自描述和默认分类，平台会根据工具名和运行时参数计算 `effective_risk`，扩展不能把高危工具降级。宿主 Tool Projection 会为 AgentTimelinePart 写入 `metadata.displayKind`；扩展只有在复用宿主内建语义时才应使用内建 displayKind，否则使用 `other` 加 `renderer_hint`，避免第三方工具假装成文件写入、终端或审批节点。
- 浏览器控制、电脑控制、外部系统自动化等 Agent 可调用能力必须作为 `contributes.mcp_servers` 提供的 MCP 工具接入，而不是伪装成普通菜单 action 或直接暴露 IPC。扩展工具进入 Kernel Registry 后，由 Tool Projection 生成本轮 provider-safe model name、按 ModeConfig / 扩展权限 / Sandbox / Permission 过滤，再进入 Gateway tools。这样扩展工具和内建 `bash`、文件工具共享同一套审批、Turn Timeline、审计和结果回注机制。
- 扩展工具的对话区展示必须通过 `ToolRendererCatalog` 扩展，不能直接操作 `chatMessages` DOM，也不能绕过 AgentTimelinePart。扩展 renderer 只消费宿主传入的 `AgentTimelinePart.input/output/metadata/progress/time` 和扩展声明的 schema；无 renderer 时使用宿主 `GenericToolStep`。长任务扩展可以选择实现 MCPTool progress 能力，progress snapshot 仍由 MCP Executor 审计后写入同一 `callId` 的 AgentTimelinePart；前端不能用本地计时器、模板文案或 fake completed 状态伪造扩展工具进度。这样浏览器控制、电脑控制、数据库查询等工具可以获得专属展示，同时不破坏 `ToolUse / Progress / Result` 三段语义。宿主内建 renderer 已覆盖 `read/list/glob/grep/search/inspect/edit/write-as-edit/bash/git/lsp/todo/task/task_output/task_stop/webfetch/websearch/permission/error`；`skill`、`mcp_resource`、`browser` 等专属 renderer 只在真实 Skill/MCP 扩展工具注册后启用，扩展 renderer 只补充宿主没有的专属展示。
- 扩展 MCP Server 启用时，宿主必须真实连接 transport 并调用 `tools/list`；发现失败即扩展能力不可用并在 Settings 扩展详情中暴露错误。扩展不得通过 manifest 伪造运行时 `ToolDefinition`，也不得用 StubTool、mock result 或本地 fallback 代替 `tools/call`。`mcp_tool_overrides` 只能覆盖宿主展示、风险和模型侧名称元数据，不能创建实际不存在的工具。
- 扩展 transport 没有实现完整请求语义时必须 fail-closed。当前可用于真实工具发现和调用的内置 transport 只有 stdio MCP 子进程；SSE、WebSocket、REST、gRPC 在真实 adapter 注册到 Kernel-backed transport registry 前不进入可用列表，也不得显示成可启用能力或注册扩展工具。扩展作者如果要接入浏览器控制、电脑控制、数据库或外部自动化，应优先提供 stdio MCP server，等对应 transport 完整落地后再切换。
- Extension 声明 `gateway`、`middlewares`、`transport_adapters`、`themes`、`editor_languages`、`editor_extensions`、`notification_channels`、`behaviors`、`context_providers`、`search_providers`、`file_watchers` 等 runtime contribution 时，如果对应宿主尚未把该 contribution 接入真实 Registry / Pipeline / Policy / EventBus 链路，扩展启用必须 fail-closed。不能只保留 manifest declaration、打印 warning 或在 UI 中显示为可用能力。`languages` 已由 ExtensionLifecycle 接入 LSP 宿主，启用时注册到 LSP Kernel-backed Registry，禁用时注销。
- 扩展启用必须是事务式的：MCP server、Skills、LSP languages 等已经进入宿主运行时的 contribution，如果后续任一 contribution 启用失败，ExtensionLifecycle 必须按反向顺序注销已注册资源，并把扩展置为 `Error`。不能留下半启用的 MCP server、Skill、LSP language 或 hook 声明。
- 外部 memory provider 未来也属于 MCP / Tool Projection 能力，不是普通菜单 action 或扩展私有 prompt 注入。扩展可以贡献 memory 相关 MCP 工具或 provider metadata，但持久化、权限、审计、AgentTimelinePart 展示和上下文 snapshot 刷新仍由宿主管理；扩展不得直接改写当前 turn 的系统提示或向 `chatMessages` DOM 注入伪 memory 展示。
- Custom 模式扩展如果需要模式级记忆，必须写入 `scope_type = mode`，`scope_id = <modeId>`。扩展不得创建独立 memory 表、独立长期记忆 prompt 或绕过 `global -> mode -> project -> session` 的宿主合并规则。

---

## 二、架构设计

```
extensions/
├── mod.rs              # 模块入口
├── context.rs          # Cordis 宿主 Context / 插件 Fiber 装配
├── loader.rs           # 扩展加载器
├── lifecycle/          # 生命周期管理（已拆分为子模块）
│   ├── mod.rs          # ExtensionLifecycle 结构体 + Builder + Rollback 辅助
│   ├── install.rs      # 热更新（update）逻辑
│   ├── register.rs     # contributes 注册/注销辅助函数
│   └── state.rs        # enable/disable 状态管理 + do_enable/do_disable
├── store.rs            # ExtensionStore：安装状态、manifest DTO、冲突检测与 hook 声明索引
└── installer.rs        # 扩展安装/卸载

extensions/{id}/
├── extension.json
├── ExtensionUI/        # 前端扩展点：全部前端代码
└── ExtensionBackend/   # 后端扩展点：全部后端代码
```

---

## 三、数据模型

Manifest 基本规则：

- `id` 是稳定机器标识，用于注册表、权限、配置和依赖引用。
- `name` 是必填人类可读名称，用于 UI 展示；Custom 模式菜单优先展示 `WorkModeRegistration.name`，缺省使用 `ExtensionManifest.name`。
- 同一个扩展的 `name` 可以随版本调整，但 `id` 不应随意变化。
- 如果后续需要国际化显示名，可在 `name` 基础上增加 `display_names`，但不能省略默认 `name`。

```rust
struct ExtensionManifest {
    id: String,                       // 扩展稳定 ID，不能用于直接展示
    name: String,                     // 必填显示名；Custom 模式菜单、Settings > Extensions 等界面优先展示该名称
    version: String,
    description: String,
    author: String,
    permissions: ExtensionPermissions,
    contributes: ExtensionContributes,
}

struct ExtensionPermissions {
    filesystem: Vec<String>,      // 路径权限（如 "read:./src/**"）
    terminal: Vec<String>,        // 命令权限（如 "npm", "git"）
    network: Vec<String>,         // 网络权限（如 "https://api.example.com"）
    ipc: Vec<String>,             // 允许调用的 IPC 命令（如 "agent.cancelTask"）
    events: Vec<String>,          // 允许订阅宿主暴露的 UI event/stream 只读投影 pattern（如 "action.*" / "session.*"）
    resources: ResourceLimits,    // 资源配额
}

struct ResourceLimits {
    max_memory_mb: u64,           // 最大内存（MB），如 512
    max_cpu_percent: f32,         // 最大 CPU 占用（%），如 50.0
    timeout_ms: u64,              // 执行超时（毫秒），如 30000
}

struct ExtensionContributes {
    // 已有字段
    mcp_servers: Option<Vec<MCPServerConfig>>,               // 注册 MCP 工具（含 user_visible 标记）
    mcp_tool_overrides: Option<Vec<McpToolOverride>>,        // MCP 工具属性覆盖（声明式配置，承载 user_visible 等标记）
    tools: Option<Vec<ToolRegistration>>,                    // 扩展直接声明 MCP 工具（通过 ToolRegistry 注册为平台工具，server_id 为 extension:{extension_id}）
    skills: Option<Vec<SkillDefinition>>,                     // 声明 Skills，启用后写入 SkillStore
    roles: Option<Vec<RoleDefinition>>,                       // 声明角色，启用后写入 RoleStore
    views: Option<Vec<ViewRegistration>>,                     // 声明 UI 视图（placement + 宿主 renderer）
    menus: Option<Vec<MenuRegistration>>,                     // 声明菜单
    commands: Option<Vec<CommandRegistration>>,               // 声明命令
    keybindings: Option<Vec<KeybindingRegistration>>,         // 声明快捷键
    configuration: Option<Value>,                                 // 注册配置项（其他类型可通过 ${config.key} 引用）
    work_modes: Option<Vec<WorkModeRegistration>>,             // 注册 Custom 模式扩展（显示在 Custom 页签下）

    // 新增：UI 扩展（由 22-ui-framework.md 的 HostView 与 contribution contract 承接）
    toolbar_items: Option<Vec<ToolbarItemRegistration>>,          // 工具栏按钮（topbar / editorToolbar / terminalToolbar）
    statusbar_items: Option<Vec<StatusBarItemRegistration>>,      // 状态栏项目（statusbar:left / statusbar:right）
    inline_extensions: Option<Vec<InlineExtensionRegistration>>,  // 视图内嵌组件（chatAside / editorView / terminalView）

    // 新增：Gateway 层扩展
    gateway: Option<GatewayContributions>,                    // Gateway Adapter / Provider / Model / Auth 声明
    middlewares: Option<Vec<MiddlewareRegistration>>,          // 独立 Gateway Pipeline contribution；不属于 Adapter / Provider schema，未接入 Pipeline 时 fail-closed

    // 新增：MCP 层扩展
    transport_adapters: Option<Vec<TransportAdapterRegistration>>,  // 自定义 MCP 传输适配器

    // LSP 层扩展（对应新编号 14-LSP）；启用时进入 LSP Kernel-backed Registry
    languages: Option<Vec<LanguageRegistration>>,             // 自定义语言 LSP Server 配置

    // 新增：Editor 层扩展（对应新编号 26-Editor）
    themes: Option<Vec<ThemeRegistration>>,                   // 自定义编辑器主题
    editor_languages: Option<Vec<EditorLanguageRegistration>>,    // 自定义语言模式/语法
    editor_extensions: Option<Vec<EditorExtensionRegistration>>, // 自定义编辑器扩展

    // 新增：Tray 层扩展（系统托盘菜单项）
    tray_items: Option<Vec<TrayItemRegistration>>,             // 自定义系统托盘菜单项（Top/Middle/Bottom）

    // 新增：Notification 层扩展（对应新编号 25-Notification）
    notification_channels: Option<Vec<NotificationChannelRegistration>>, // 自定义通知渠道

    // 新增：本地化资源（对应新编号 28-i18n；数据结构见 34-extension-ui-open-architecture §3.1，entry 须位于 ExtensionUI/locales/ 下）
    i18n: Option<Vec<I18nResource>>,                                      // 扩展本地化语言包（28 号承接前必须 reject_unbound，禁止静默忽略）

    // 新增：Chat 输入区快捷触发器
    triggers: Option<Vec<TriggerRegistration>>,                   // 自定义输入快捷触发器（建议 /xxx；扩展/能力引用主路径仍是 + 菜单）

    // ─── 新增：UI 样式与行为层 ───

    // 样式注入（自定义 CSS，限定在扩展沙箱作用域内）
    styles: Option<Vec<StyleRegistration>>,                       // 注入自定义 CSS 变量 / 样式规则
    // 布局覆盖（修改扩展自身 UI 组件的定位和尺寸）
    layout_overrides: Option<Vec<LayoutOverrideRegistration>>,    // 覆盖扩展 UI 组件的布局属性
    // 事件行为（定义 hover/click/focus 等触发的 UI 行为）
    behaviors: Option<Vec<BehaviorRegistration>>,                 // 事件驱动的 UI 行为（如 hover 打开面板）

    // ─── 新增：Agent 管道钩子层 ───

    // 管道钩子（在 Agent 请求/响应/上下文/工具选择等阶段注入逻辑）
    hooks: Option<Vec<HookRegistration>>,                         // Agent 管道钩子（拦截/修改/增强）

    // ─── 新增：数据连接层 ───

    // 上下文数据源（向 Context Manager 注入外部数据）
    context_providers: Option<Vec<ContextProviderRegistration>>,  // 自定义上下文数据源（如 Jira Issue、文档链接内容）
    // 全局搜索提供者（向全局搜索注入外部结果）
    search_providers: Option<Vec<SearchProviderRegistration>>,    // 自定义搜索源（如 GitHub、Jira、Notion）
    // 文件监听器（响应当前 Session.worktree_root 内的文件变更）
    file_watchers: Option<Vec<FileWatcherRegistration>>,          // 文件变更监听与响应
}

// Custom 模式扩展注册
struct WorkModeRegistration {
    id: String,                         // 模式 ID，扩展内唯一，如 "knowledge-search"
    name: Option<String>,               // 模式显示名；缺省使用 ExtensionManifest.name
    description: Option<String>,        // 模式说明
    icon: Option<String>,               // Lucide icon 名或扩展图标路径
    role: Option<String>,               // 默认角色，引用内建或扩展 contributes.roles 中的 RoleDefinition.id
    available_tools: Option<Vec<String>>, // 模式工具白名单，支持 read/list/glob/grep/search/inspect/write/edit/bash/git/lsp/webfetch/websearch/mcp.* 等工具 ID 或通配符
    skills: Option<Vec<String>>,        // 默认技能集，引用内建、项目、用户或扩展 contributes.skills 中的 skill id
    commands: Option<Vec<String>>,      // 模式优先命令，引用 contributes.commands 或系统命令 id
    context_policy: Option<String>,     // 上下文策略 ID，引用 context_providers 或内建策略
    behavior_rules: Option<Vec<String>>, // 模式行为约束，注入 Agent system prompt 的规则片段
    entry_view: Option<String>,         // 可选：进入模式时默认打开的 view id
    default_views: Option<Vec<String>>, // 可选：进入模式时建议打开的 view id 列表；具体 surface 由 view.placement 决定
    default_model: Option<String>,      // 可选：该模式建议模型，最终仍写入 Session 模型偏好
    model_preferences: Option<WorkModeModelPreferences>, // 可选：temperature、max_tokens、extended_thinking、language_quality_emphasis 等模式模型偏好
    capabilities: Vec<String>,          // 能力标签，如 ["rag", "visualization"]
}

// Work mode 模型偏好。
struct WorkModeModelPreferences {
    temperature: Option<f32>,               // 温度参数
    max_tokens: Option<u32>,                // 最大输出 token 数
    extended_thinking: Option<bool>,        // 是否启用扩展思考
    language_quality_emphasis: Option<f64>, // 语言质量偏好（影响模型风格）
}

// WorkModeRegistration 不是普通菜单项，而是完整工作模式的 ModeConfig overlay。
// 点击 Custom 页签下的某个模式扩展后，当前会话进入 custom:<runtimeId>，
// 其中 runtimeId = <extensionId>/<modeId>，
// 这里的 custom:<runtimeId> 是 WorkMode 的会话标识，不是 Gateway protocol ID；Gateway protocol ID 另遵循 protocolId 的 canonical 规则。
// Agent 会按该声明重新加载角色、工具白名单、技能、命令、上下文策略、模型偏好和默认 UI 入口。
// 主对话发送时，该 ModeConfig 会进入 Agent Turn Context，与 Code / Cowork 一样注入
// system prompt、worktree snapshot、工具边界和行为规则；Custom 不是普通菜单动作。
//
// 安全边界：
// - 模式扩展可以缩小或组合可用工具，但不能绕过 Sandbox、Project Trust、用户权限和扩展权限声明。
// - available_tools 是模式级白名单，实际可用工具 = 模式白名单 ∩ 扩展权限 ∩ Sandbox 允许范围 ∩ 当前项目可用工具。
// - 工具风险等级由 provider 声明，但平台保留强制覆盖权；terminal.*、fs.write*、fs.delete*、network.*、web.* 等高风险能力不能被扩展声明为 safe。
// - MCP ToolDefinition 是工具能力的真实来源；ui_hint 只给 UI/trace/审计使用，不进入模型可见 tool schema。
// - entry_view/default_views 只声明“建议打开”的 UI，右侧动态分列布局仍由宿主 rightWorkspace 统一管理。

// 扩展工具路径：
// - 重型能力使用 MCP Server。当前内置可用 transport 是 stdio；HTTP / SSE / WebSocket 等远程形态必须由真实 adapter 注册后才可用。
// - 轻量能力可以使用 Host inline function 注册，适合代码统计、文本转换、模板生成等小工具。
// - 两条路径最终都进入同一个 Tool Projection / Tool Pipeline，执行时使用相同的 ToolDefinition、ToolCallRequest、ToolCallResult、
//   权限校验、风险覆盖、Gateway 编码和 execution trace，不允许扩展绕开宿主工具运行时。

// ─── UI 样式与行为层 数据模型 ───

// 样式注册（CSS 注入，限定在扩展沙箱作用域）
struct StyleRegistration {
    id: String,                          // 样式 ID
    module: String,                      // ES Module 路径（导出 CSS 字符串或 CSS-in-JS 对象）
    scope: StyleScope,                   // 作用域
    variables: Option<HashMap<String, String>>,  // 自定义 CSS 变量（自动注入到扩展 DOM 作用域）
}

enum StyleScope {
    Extension,     // 仅作用于扩展自身组件（默认，Shadow DOM 内）
    View,       // 作用于扩展所在的宿主视图（如 Chat 视图、Editor 视图）
    Global,     // 全局生效（需要额外权限声明 "style:global"）
}

// 布局覆盖注册（控制扩展 UI 组件的定位和尺寸）
struct LayoutOverrideRegistration {
    target: String,                      // 目标组件 ID（引用 views.id / inline_extensions.id / toolbar_items.id 等）
    position: Option<PositionValue>,     // 定位方式
    offset: Option<OffsetValue>,         // 偏移量
    size: Option<SizeValue>,             // 尺寸
    z_index: Option<i32>,                // 层级
    transition: Option<String>,          // 过渡动画（如 "opacity 0.2s ease"）
}

struct PositionValue {
    type_: String,                       // "absolute" | "relative" | "fixed" | "sticky"
    top: Option<String>,                 // CSS 值，如 "10px", "50%", "auto"
    right: Option<String>,               // 如 "10px"（距右侧 10px）
    bottom: Option<String>,
    left: Option<String>,
}

struct OffsetValue {
    x: Option<String>,                   // 水平偏移，如 "10px"
    y: Option<String>,                   // 垂直偏移
}

struct SizeValue {
    width: Option<String>,               // 如 "300px", "50%", "auto"
    height: Option<String>,
    min_width: Option<String>,
    max_width: Option<String>,
    min_height: Option<String>,
    max_height: Option<String>,
}

// 行为注册（事件驱动的 UI 行为）
struct BehaviorRegistration {
    id: String,                          // 行为 ID
    trigger: BehaviorTrigger,            // 触发条件
    action: BehaviorAction,              // 触发后的动作
    target: Option<String>,              // 目标组件 ID（可选，限定在特定组件上生效）
}

enum BehaviorTrigger {
    Hover {                              // 鼠标悬停触发
        delay_ms: Option<u64>,           // 悬停延迟（默认 300ms）
        leave_delay_ms: Option<u64>,     // 离开后延迟隐藏（默认 200ms）
    },
    Focus {                              // 获得焦点触发
        target_selector: String,         // CSS 选择器
    },
    Click {                              // 点击触发
        target_selector: String,
        button: Option<String>,          // "left" | "right" | "middle"（默认 "left"）
    },
    Shortcut {                           // 快捷键触发
        key: String,                     // 按键组合
    },
    Resize {                             // 窗口/组件尺寸变化触发
        threshold: Option<f32>,          // 变化阈值（百分比）
    },
}

enum BehaviorAction {
    ShowPanel {                          // 预留：显示面板/浮动组件
        view_id: String,                 // 引用 views.id
        position: Option<String>,        // "near-cursor" | "center" | "anchored"
    },
    ShowTooltip {                        // 预留：显示工具提示
        content_module: String,          // 预留模块字段；当前宿主不加载
        position: Option<String>,        // "top" | "bottom" | "left" | "right"（默认 "bottom"）
    },
    ToggleComponent {                    // 预留：切换组件显隐
        target_id: String,               // 引用 views.id / inline_extensions.id
    },
    EmitEvent {                          // 预留：发出自定义事件
        event_name: String,              // 事件名称
        payload: Option<Value>,          // 事件载荷
    },
    RunCommand {                         // 预留：引用宿主命令
        command_id: String,              // 引用 commands.id
    },
}

// ─── Agent 管道钩子层 数据模型 ───

// 钩子注册（在 Agent 管道各阶段注入逻辑）
struct HookRegistration {
    id: String,                          // 钩子 ID
    name: String,                        // 显示名称
    phase: HookPhase,                    // 执行阶段
    priority: Option<u32>,               // 执行优先级（数字越小越先执行，默认 100）
    module: String,                      // ES Module 路径（实现对应阶段的处理函数）
    when: Option<String>,                // 条件表达式（仅在满足条件时执行）
}

enum HookPhase {
    SessionStart,      // 新会话或会话恢复进入 Agent 运行上下文前
    PreToolUse,        // 工具调用授权与执行前，可观察或请求改写工具输入
    PostToolUse,       // 工具调用完成并写回模型前，可观察或请求改写工具结果
    PreCompact,        // 上下文压缩前，可观察压缩候选并提供保留建议
}

// ─── 数据连接层 数据模型 ───

// 上下文数据源注册（向 Context Manager 注入外部数据）
struct ContextProviderRegistration {
    id: String,                          // 数据源 ID
    name: String,                        // 显示名称
    description: String,                 // 描述（用于 Agent 判断是否需要调用）
    module: String,                      // ES Module 路径（实现数据获取逻辑）
    trigger_pattern: Option<String>,     // 触发模式（正则，匹配用户消息时自动调用，如 "jira-[0-9]+"）
    inject_position: Option<InjectPosition>,  // 注入位置（在上下文中的哪个阶段注入）
    priority: Option<u32>,               // 注入优先级
    max_tokens: Option<usize>,           // 最大注入 Token 数
}

enum InjectPosition {
    BeforeHistory,    // 在历史消息之前注入
    AfterHistory,     // 在历史消息之后、用户消息之前注入
    AfterUserMessage, // 在用户消息之后注入（作为补充上下文）
}

// 全局搜索提供者注册
struct SearchProviderRegistration {
    id: String,                          // 搜索源 ID
    name: String,                        // 显示名称（如 "GitHub", "Jira"）
    icon: Option<String>,                // 图标
    module: String,                      // ES Module 路径（实现搜索逻辑）
    scope_tags: Vec<String>,             // 搜索范围标签（用户可通过 scope 过滤）
    priority: Option<u32>,               // 结果排序优先级
}

// 文件监听器注册
struct FileWatcherRegistration {
    id: String,                          // 监听器 ID
    name: String,                        // 显示名称
    patterns: Vec<String>,               // 文件匹配模式（glob，如 "**/*.test.ts"）
    events: Vec<FileWatchEvent>,         // 监听的事件类型
    module: String,                      // ES Module 路径（实现响应逻辑）
    debounce_ms: Option<u64>,            // 防抖延迟（默认 500ms）
}

enum FileWatchEvent {
    Created,     // 文件创建
    Modified,    // 文件修改
    Deleted,     // 文件删除
    Renamed,     // 文件重命名
}

// MCP 工具属性覆盖（声明式配置，用于覆盖运行时 MCP 发现的工具属性）
struct McpToolOverride {
    server: String,                     // MCP Server 名称（对应 mcp_servers[].name）
    tool: String,                       // 工具名称（对应 MCP 运行时发现的 ToolDefinition.name）
    model_name: Option<String>,         // 模型可见 provider-safe 名称；缺省由 Tool Projection 生成
    user_visible: Option<bool>,         // 是否对用户可见（在 / 触发器中显示）
    display_name: Option<String>,       // 显示名称覆盖（替代工具原始名称）
    description: Option<String>,        // 描述覆盖（替代工具原始描述）
    renderer: Option<String>,           // ToolRendererCatalog renderer ID
    detail_view: Option<String>,        // renderer 详情视图语义
    declared_risk: Option<String>,      // provider 自声明风险，不能降低平台强制风险
}

// 扩展直接声明 MCP 工具（与 mcp_servers 声明的工具不同：tools 声明的是宿主无法
// 通过 MCP 运行时发现的轻量工具，启用时由 ExtensionLifecycle 通过 MCP ToolRegistry 注册为
// 平台工具，server_id 为 extension:{extension_id}，禁用时按 server_id 整体移除。）
struct ToolRegistration {
    name: String,                       // 工具唯一名称（同一扩展内不可重复）
    description: String,                // 工具描述（供模型选择参考）
    input_schema: Value,                // JSON Schema 输入参数定义
    user_visible: bool,                 // 是否对用户可见（默认 true）
    use_builtin_risk: bool,             // 是否使用内置风险评估（默认 true）
    declared_risk: Option<String>,      // Provider 自声明风险等级（Low/Medium/High/Critical）
}

struct ViewRegistration {
    id: String,                         // 扩展内唯一 view id
    name: String,                       // 面板标题
    icon: Option<String>,
    entry: Option<String>,              // html:sandbox 的入口，相对于扩展根目录且必须位于 ExtensionUI/ 下
    placement: Option<String>,          // 宿主 UI surface，如 rightWorkspace / chatAside / bottomDrawer / settingsSection
    renderer: String,                   // 宿主 Host view renderer ID，如 host:panel
    config: Option<Value>,              // renderer 专用配置；只由 UI 域解释
    activation_events: Vec<String>,
    allow_close: Option<bool>,
    default_visible: Option<bool>,
}

// Host view renderer config 示例：
//
// {
//   "title": "Subagents",
//   "placement": "chatAside",
//   "session_scoped": true,
//   "source": "subagent_status"
// }
//
// 约束：
// - placement 必须属于宿主白名单。
// - renderer ID 必须属于宿主白名单。
// - config 是 UI 声明，只由对应 Host view renderer 解释。
// - 实时数据来自宿主 IPC、Kernel EventBus 的 UI 投影或 foundation::stream。
// - renderer config 不是 Kernel 类型。

// ─── contributes 的宿主承接边界 ───
//
//  贡献类型                  当前承接边界
//  ────────────────────────  ─────────────────────────────────────────────────────
//  views                     HostView contract；placement 选择 surface，renderer 选择宿主渲染策略
//  commands                  声明式 BuiltinAction；当前仅 OpenView / ToggleView
//  menus                     已支持的 MenuTarget；只引用可投影的 command
//  keybindings               App scope；复用 command/action 投影，不建立独立 registry
//  toolbar/statusbar/items   对应宿主工具栏或状态栏 contribution；不是 HostView surface
//  inline_extensions         对应宿主内嵌 contribution contract；不是路由或动态组件入口
//  configuration/themes      Settings / UI Framework 的既有配置与主题承接边界
//  triggers/behaviors        当前仅为 manifest/schema 预留，不加载或执行扩展模块
//  hooks/gateway/tools      分别进入 Agent、Gateway、Tool/MCP 等已有宿主域和 Kernel 原语
//
// views 不再映射到独立的 panel:* / view:* UI ID，也不注册独立前端路由。

// ─── contributes 各类型关联全景图 ───
//
//   UI 入口层                命令枢纽层              能力层              基础设施层（宿主域 + Kernel 原语）
//   ──────────              ──────────            ────────            ──────────────────
//   menus ──────────┐
//   keybindings ────┤       ┌──────────┐
//   toolbar_items ──┤       │          │
//   statusbar_items─┤       │          │
//   triggers ───────┤       └────┬─────┘
//   inline_extensions┐          │
//                    │            │
//                    │            │ action: OpenView / ToggleView
//                    │            │ activation_events: onCommand:xxx
//                    │            ▼
//                    │         views ◄──── triggers (TriggerAction::OpenView)
//                    │
//                    │       ┌──────────┐
//                    │       │  roles   │── skills/commands 字段 ──► skills
//                    │       └──────────┘              ▲
//                    │                                 │ tools_whitelist
//                    │       ┌──────────────┐
//                    │       │ mcp_servers  │── skills/commands 字段 ──► skills
//                    │       │ tools        │── 启用时通过 MCP ToolRegistry 注册为平台工具
//                    │       └──────────────┘
//                    │
//                    │       ┌──────────────┐
//                    │       │configuration │◄── ${config.key} 被其他类型引用
//                    │       └──────────────┘
//                    │
//                    │       ┌──────────────┐
//                    │       │  gateway     │──► Gateway Registry（Adapter / Provider / Model；当前 fail-closed）
//                    │       │  middlewares  │──► Gateway（目标 Pipeline stage；当前 fail-closed）
//                    │       │transport_adpt│──► MCP（Registry / EventBus）
//                    │       │  languages   │──► LSP（Kernel Registry / EventBus）
//                    │       │   themes     │──► Editor（UI 域承接）
//                    │       │ editor_langs │──► Editor（Registry / EventBus）
//                    │       │ editor_exts  │──► Editor（UI 域承接）
//                    │       │notif_channels│──► 通知系统（Registry / EventBus）
//                    │       │ tray_items   │──► 系统托盘菜单（Top/Middle/Bottom）
//                    │       └──────────────┘
//                    │
//                    │       ┌──────────────┐
//                    │       │    hooks     │──► Agent 生命周期（SessionStart/PreToolUse/PostToolUse/PreCompact）
//                    │       └──────────────┘
//                    │
//                    │       ┌──────────────┐
//                    │       │   context_   │──► Context Manager（外部数据源注入）
//                    │       │  providers   │
//                    │       ├──────────────┤
//                    │       │   search_    │──► 全局搜索系统（外部搜索结果）
//                    │       │  providers   │
//                    │       ├──────────────┤
//                    │       │    file_     │──► File 模块（文件变更响应）
//                    │       │  watchers    │
//                    │       └──────────────┘
//                    │
//                    │       ┌──────────────┐
//                    │       │   styles     │──► 扩展 UI 组件样式注入
//                    │       │   layout_    │──► 扩展 UI 组件布局覆盖
//                    │       │  overrides   │
//                    │       │  behaviors   │──► 事件驱动 UI 行为（hover/focus/click）
//                    │       └──────────────┘
//                    │
//                    │   联动规则：
//                    │   ├── menus / keybindings / toolbar_items / statusbar_items → 通过 command 字段引用 commands.id
//                    │   ├── commands → 通过 action 或 activation_events 驱动 views
//                    │   ├── triggers → 通过 TriggerAction 引用 commands.id 或 views.id
//                    │   ├── roles → 通过 skills/commands 字段 引用 skills.id 和 commands.name
//                    │   ├── skills → 通过 tools_whitelist 引用 mcp_servers 的工具名
//                    │   ├── configuration → 其他类型通过 ${config.key} 模板语法引用
//                    │   └── 基础设施层类型（gateway/languages/themes 等）→ 交给对应宿主承接，不被其他 contributes 引用

struct ExtensionState {
    id: String,
    status: ExtensionStatus,
    manifest: ExtensionManifest,
    install_path: PathBuf,
    installed_at: DateTime<Utc>,
    enabled_at: Option<DateTime<Utc>>,
    error: Option<String>,
}

struct ExtensionLogEntry {
    extension_id: String,
    level: String,                       // info / warn / error
    message: String,
    timestamp: DateTime<Utc>,
    source: Option<String>,              // loader / runtime / sandbox / host_api
}

enum ExtensionStatus {
    Installed,
    Loading,
    Enabled,
    Disabling,
    Disabled,
    Unloading,
    Error,
}

// 视图注册声明（宿主 UI surface + Host view renderer）
struct ViewRegistration {
    id: String,                     // 视图唯一 ID，如 "my-extension.dashboard"
    name: String,                   // 显示名称，如 "Dashboard"
    icon: Option<String>,           // 图标（Lucide icon 名或 SVG 路径）
    entry: Option<String>,           // html:sandbox 的入口，相对于扩展根目录且必须位于 ExtensionUI/ 下
    placement: Option<String>,      // 宿主 UI surface，如 "rightWorkspace" / "chatAside"
    renderer: String,               // 宿主 Host view renderer ID，如 "host:panel"
    config: Option<Value>,          // renderer 专用配置；不进入 Kernel
    activation_events: Vec<String>, // 激活条件，如 ["onCommand:myExtension.show"]
                                   // 若命令使用 BuiltinAction::OpenView，此处可省略（自动关联）
    allow_close: Option<bool>,     // 默认 true，设为 false 时用户无法关闭该右侧面板/辅助面板
    default_visible: Option<bool>, // 默认 false，设为 true 时面板自动显示
}

// 菜单注册声明
struct MenuRegistration {
    id: String,
    label: String,
    target: MenuTarget,            // 菜单位置
    command: String,               // 关联的命令 ID
    group: Option<String>,         // 分组名（用于菜单分隔线归类）
    when: Option<String>,          // 条件表达式，如 "editorIsOpen" / "gitRepo" / "extensionView:github.pr-view"
    icon: Option<String>,
    shortcut: Option<String>,      // 快捷键提示文本（实际绑定走 KeybindingRegistration）
    risk: Option<MenuRisk>,        // 菜单风险等级；Delete / Delete group 等危险操作使用 High
}

enum MenuRisk {
    Low,
    Medium,
    High,
}

enum MenuTarget {
    File,          // menu:file — 文件菜单
    Edit,          // menu:edit — 编辑菜单
    View,          // menu:view — 视图菜单
    Tools,         // menu:tools — 顶部栏全局工具菜单
    Help,          // menu:help — 帮助菜单
    Context,       // menu:context — 右键上下文菜单
    InputPlus,     // menu:input-plus — B 区 + 菜单（输入区主发现路径）
    ChatTitle,     // menu:chat-title — 对话区标题栏菜单
    RightPanel,    // menu:right-panel — 对话标题栏右侧面板菜单
    Gateway,       // menu:gateway — 左侧底部 Gateway 菜单
    GroupContext,  // menu:group-context — 左侧 Group 标题右键 / 更多菜单
    SessionContext,// menu:session-context — 左侧会话项右键 / 更多菜单
}

实现约束：

- `MenuRegistration` 是菜单项的权威数据模型，内置菜单和扩展贡献菜单都使用该结构。
- 前端菜单 store 只渲染后端 `ui_list_menus` 返回的数据，不维护另一份内置菜单副本。
- 菜单、命令、视图和 action 引用关系必须在 Extension Loader / UI contribution loader 阶段校验；校验失败的贡献 fail-closed，不输出到用户可见入口。
- 当前前端已经承接的菜单 target 为 `Tools`、`InputPlus`、`ChatTitle`、`RightPanel`、`Gateway`、`GroupContext`、`SessionContext`。其中 `Tools` 挂在顶部栏 `☰` 入口；宿主内建 Command palette、Settings、Gateway、Coding、Extensions 这些真实全局入口，扩展可以追加真实 `OpenView / ToggleView` 菜单项。左侧栏底部产品入口显示为 `Navis Go`，但扩展菜单 target 仍为 `Gateway`，避免重命名破坏扩展契约。扩展 `OpenView / ToggleView` 只有在目标 view 明确声明宿主白名单 placement 和 renderer 时才进入用户可见菜单；`host:panel` 由 Navis Go 宿主渲染，`html:sandbox` 只加载扩展 `ExtensionUI/` 目录下的相对 `entry`。`File/Edit/View/Help/Context` 仍属于后端扩展模型预留，只有补齐对应 UI 入口后才允许作为用户可见菜单入口宣传。
- 扩展新增菜单项必须通过 `contributes.menus` 注册；危险操作必须声明 `risk = High`。

// 命令注册声明
struct CommandRegistration {
    id: String,                    // 命令 ID，如 "myExtension.refresh"
    label: String,                 // 命令面板中显示的标题（与其他注册类型一致）
    description: Option<String>,   // 命令描述（用于模糊搜索和 tooltip）
    icon: Option<String>,          // 图标（Lucide icon 名）
    category: Option<String>,      // 分类，如 "My Extension"
    when: Option<String>,          // 条件表达式（与 menus/keybindings 共用 when 语法）
    action: BuiltinAction,          // 宿主可识别的声明式动作；不承载扩展代码入口
}

// 内置声明式动作。它们是 manifest 数据，不是扩展 handler。
// 当前 command contract 只包含以下两个动作；目标 view 必须满足 HostView contract。
enum BuiltinAction {
    OpenView {                         // 打开满足 HostView contract 的扩展视图
        view_id: String,               // 引用 contributes.views 中的 id；具体 surface 由 view.placement 决定
    },
    ToggleView {                       // 切换满足 HostView contract 的扩展视图显隐
        view_id: String,
    },
}
// 快捷键注册声明
struct KeybindingRegistration {
    command: String,               // 关联的命令 ID
    key: String,                   // 按键组合，如 "Ctrl+Shift+R"
    when: Option<String>,          // 生效条件；当前没有上下文评估能力时 fail-closed
    scope: KeybindingScope,        // 只允许 App 范围
}

enum KeybindingScope {
    App,     // 应用级（扩展只能注册此范围）
    // Global 暂不开放给扩展
}

// ─── when 条件语法（menus / keybindings / toolbar_items / statusbar_items / inline_extensions 共用）───
// 支持的条件表达式：
//   "editorIsOpen"              - 编辑器已打开
//   "gitRepo"                   - 当前 Session.worktree_root 是 Git 仓库
//   "extensionView:<viewId>"       - 指定扩展视图处于活动状态（如 "extensionView:github.pr-view"）
//   "!<condition>"              - 取反（如 "!editorIsOpen"）
//   "<a> && <b>"                - 逻辑与
//   "<a> || <b>"                - 逻辑或

// ─── Gateway Adapter / Provider / Model 注册 ───
struct GatewayContributions {
    adapters: Vec<GatewayAdapterRegistration>,
    providers: Vec<GatewayProviderRegistration>,
}

struct GatewayAdapterRegistration {
    id: String,                         // Extension 内唯一的 Adapter contribution ID
    name: String,                       // 显示名称
    protocol_id: String,                // Registry 全局 canonical 协议 ID；直接使用 manifest protocolId
    kind: GatewayAdapterKind,           // builtin / declarative
    config: Option<DeclarativeAdapterConfig>,
}

struct GatewayProviderRegistration {
    id: String,                         // Extension 内唯一的 Provider contribution ID
    name: String,                       // Provider 显示名称
    adapter_id: String,                 // 引用同一 Extension 的 GatewayAdapterRegistration.id
    base_url: String,                   // 绝对 HTTP(S) URL
    auth: GatewayAuthRegistration,
    models: Vec<GatewayModelRegistration>,
    default_model: String,              // 必须指向 models 中已声明的模型
}

struct GatewayModelRegistration {
    id: String,
    name: String,
    capabilities: ProviderCapabilities,
    context_window: u32,
    max_output_tokens: u32,
}

struct GatewayAuthRegistration {
    scheme: String,
    secret_ref: Option<String>,         // opaque reference；Extension 不预置 secret
    header: String,
}

struct ProviderCapabilities {
    tools: bool,
    streaming: bool,
    multimodal: bool,
    reasoning: bool,
    structured_output: bool,
    usage: bool,
}

// Adapter 负责协议转换、响应归一化和流式 framing；Provider 只描述连接实例；Model 只描述模型能力。
// gateway.adapters[].id 必须唯一；protocol_id 是 Registry 的 canonical 路由键，使用 manifest protocolId 的 trim 后原值，不能自动添加 custom: 前缀。
// 上述 custom: 前缀限制只适用于 Gateway protocol ID，不影响 WorkMode 的 custom:<runtimeId> 会话标识。
// gateway.providers[].adapter_id 必须引用同一 Extension 内已注册的 Adapter；Provider 注册到 Gateway 后使用 extension:<extensionId>/<providerId> 完整运行时 ID。
// builtin 协议只允许 chat_completions 和 responses；Extension 声明的 protocolId 不得使用显式 custom: 前缀，并必须通过 Registry 注册。
// module、任意本地路径、远程代码 URL、函数源码和任意 IPC handler 不属于 Gateway manifest 合同。
// schema、引用、endpoint、header 或 capability 校验失败时，整个 Gateway contribution fail-closed，不能部分启用。
// Gateway 通过 Auth SecretResolver 读取 secret；Extension 不保存、不接收、不记录明文 secret。

// ─── Gateway 中间件注册 ───
struct MiddlewareRegistration {
    id: String,                          // 中间件 ID
    name: String,                        // 显示名称
    phase: MiddlewarePhase,              // 执行阶段
    module: String,                      // 实现模块路径
}

enum MiddlewarePhase {
    PreRequest,    // 请求前（注入 header、改写 body）
    PostResponse,  // 响应后（转换格式、过滤字段）
    Error,         // 错误处理（重试、降级）
}

// ─── MCP 传输适配器注册 ───
struct TransportAdapterRegistration {
    id: String,                          // 适配器 ID
    name: String,                        // 显示名称
    transport_type: String,              // 传输类型标识（如 "grpc", "amqp"）
    module: String,                      // 实现 TransportAdapter trait 的模块
}

// ─── 托盘菜单项注册 ───
struct TrayItemRegistration {
    id: String,                          // 菜单项 ID
    label: String,                       // 显示文本
    icon: Option<String>,                // 图标（Lucide icon 名）
    command: String,                     // 关联的命令 ID
    position: TrayPosition,             // 插入位置（Top / Middle / Bottom）
    when: Option<String>,               // 条件表达式（与 menus 等共用 when 语法）
}

enum TrayPosition {
    Top,           // 托盘菜单顶部
    Middle,        // 托盘菜单中部
    Bottom,        // 托盘菜单底部
}

// ─── LSP 语言注册 ───
struct LanguageRegistration {
    language_id: String,                 // 语言 ID（如 "csharp", "kotlin"）
    display_name: String,                // 显示名称（如 "C#", "Kotlin"）
    extensions: Vec<String>,             // 文件扩展名（如 [".cs", ".csx"]）
    server_command: String,              // LSP Server 启动命令
    server_args: Option<Vec<String>>,    // 启动参数
    initialization_options: Option<Value>, // 初始化配置
}

// 启用扩展时，ExtensionLifecycle 将 LanguageRegistration 转换为 LSPServerConfig，
// 调用 LSPManager.registry().register(config, LanguageSource::Extension)；
// 禁用扩展时调用 LSPManager.registry().unregister(language_id)。
// Extension 模块不保存一份运行时语言注册表，不做桥接层。

// ─── 编辑器主题注册 ───
struct ThemeRegistration {
    id: String,                          // 主题 ID
    name: String,                        // 显示名称
    type: ThemeType,                     // 主题类型
    module: String,                      // ES Module 路径（导出主题定义）
}

enum ThemeType {
    Light,
    Dark,
    HighContrast,
}

// ─── 编辑器语言模式注册 ───
struct EditorLanguageRegistration {
    id: String,                          // 语言模式 ID
    name: String,                        // 显示名称
    extensions: Vec<String>,             // 文件扩展名
    module: String,                      // ES Module 路径（导出语法定义）
}

// ─── 编辑器扩展注册 ───
struct EditorExtensionRegistration {
    id: String,                          // 扩展 ID
    name: String,                        // 显示名称
    description: String,                 // 描述
    module: String,                      // ES Module 路径（导出 CodeMirror Extension）
    activation_events: Vec<String>,      // 激活条件
}

// ─── 工具栏按钮注册（对应 topbar / editorToolbar / terminalToolbar）───
struct ToolbarItemRegistration {
    id: String,                          // 按钮 ID
    label: String,                       // Tooltip 文本
    icon: String,                        // 图标（Lucide icon 名）
    command: String,                     // 关联的命令 ID
    position: ToolbarPosition,           // 目标工具栏
    group: Option<String>,               // 分组名（用于按钮分隔归类）
    when: Option<String>,                // 条件表达式（与 menus/keybindings 共用 when 语法）
}

enum ToolbarPosition {
    Main,           // topbar — 主工具栏（顶部全局工具栏）
    Editor,         // editorToolbar — 编辑器工具栏
    Terminal,       // terminalToolbar — 终端工具栏
}

// ─── 状态栏项目注册（对应 statusbar:left / statusbar:right）───
struct StatusBarItemRegistration {
    id: String,                          // 项目 ID
    label: String,                       // 显示文本
    icon: Option<String>,                // 图标
    position: StatusBarPosition,         // 左侧或右侧
    command: Option<String>,             // 点击关联的命令 ID（可选）
    priority: Option<u32>,               // 排序优先级（数字越小越靠左，默认 100）
    when: Option<String>,                // 条件表达式
}

enum StatusBarPosition {
    Left,           // statusbar:left — 状态栏左侧（与 AgentStatus 并列）
    Right,          // statusbar:right — 状态栏右侧（与 CursorPosition 并列）
}

// ─── 视图内嵌组件注册（对应 chatAside / editorView / terminalView）───
struct InlineExtensionRegistration {
    id: String,                          // 扩展 ID
    name: String,                        // 显示名称
    target: InlineTarget,                // 目标视图
    position: InlinePosition,            // 在目标视图中的位置
    component: String,                   // ES Module 路径
    max_items: Option<u32>,              // 同一位置最多渲染几个组件（默认 3；宿主侧 maxItems 为硬上限，extension 侧为建议值）
    priority: Option<u32>,               // 排序优先级（默认 100）
    visible: Option<bool>,               // 默认是否可见（默认 true）；当前没有对应的宿主运行时切换动作
    when: Option<String>,                // 条件表达式
    sandbox: UISandboxMode,                // 沙箱隔离模式
}

enum InlineTarget {
    Chat,           // chatAside — Chat 视图内嵌
    Editor,         // editorView — Editor 视图内嵌
    Terminal,       // terminalView — Terminal 视图内嵌
}

enum InlinePosition {
    BeforeInput,    // 输入框之前（Chat: 输入框上方区域）
    AfterMessages,  // 消息列表之后（Chat: 消息列表下方）
    Sidebar,        // 视图侧边（Chat/Editor: 右侧边栏）
    Top,            // 视图顶部（内容区域最上方，滚动时始终可见）
    Bottom,         // 视图底部（内容区域最下方，紧接消息列表末尾）
}

// ─── 通知渠道注册 ───
struct NotificationChannelRegistration {
    id: String,                          // 渠道 ID
    name: String,                        // 显示名称
    description: String,                 // 描述
    config_schema: Value,                // 配置 JSON Schema（如 webhook URL）
    module: String,                      // 实现 NotificationChannel trait 的模块
}

// ─── Chat 输入框触发器注册（预留声明模型，当前未接入执行运行时） ───
struct TriggerRegistration {
    prefix: String,                      // 触发前缀，建议使用 "/xxx"，如 "/pr", "/issue", "/jira"
    label: String,                       // 显示名称，如 "Pull Request"
    description: String,                 // 描述（显示在触发器选择列表中）
    icon: Option<String>,                // 图标（Lucide icon 名）
    placeholder: Option<String>,         // 搜索框占位文本，如 "搜索 GitHub PR..."
    search_module: Option<String>,       // 预留字段；当前宿主不加载或调用扩展模块
    select_module: Option<String>,       // 预留字段；当前宿主不加载或调用扩展模块
    scope: TriggerScope,                 // 作用范围
}

enum TriggerScope {
    Input,     // 仅输入框内可用
    Global,    // 输入框 + Command Palette 均可用
}

// 触发器搜索候选项（预留 DTO，当前无扩展搜索运行时）
struct TriggerCandidate {
    id: String,                          // 候选项唯一 ID
    label: String,                       // 主显示文本
    description: Option<String>,         // 副文本（如文件路径、Issue 状态）
    icon: Option<String>,                // 图标
    metadata: Option<Value>,             // 附加数据（供未来宿主选择器使用）
}

// 触发器选中后的动作（预留 DTO，不代表当前存在执行器）
enum TriggerAction {
    InjectRef {                          // 注入结构化引用标签到输入框
        ref_type: String,                // 引用类型标识，如 "file", "session", "pr"
        ref_id: String,                  // 引用目标 ID（文件路径 / 会话 ID / PR 编号）
        label: String,                   // 输入框中显示的文本
    },
    InjectText {                         // 注入纯文本到输入框
        text: String,
    },
    RunCommand {                         // 执行已注册的命令（引用 CommandRegistration.id）
        command_id: String,              // 命令 ID，必须引用 contributes.commands 中的 id
        args: Option<Value>,             // 传递给命令的参数
    },
    OpenView {                           // 打开扩展视图（引用 ViewRegistration.id）
        view_id: String,                 // 视图 ID，必须引用 contributes.views 中的 id
        params: Option<Value>,           // 传递给视图的参数（如选中的引用目标）
    },
    ToggleInline {                       // 切换内嵌组件显隐
        extension_id: String,            // 引用 contributes.inline_extensions 中的 id
    },
    UpdateStatusBar {                    // 更新状态栏项目
        item_id: String,                 // 引用 contributes.statusbar_items 中的 id
        label: Option<String>,
        icon: Option<String>,
    },
}

```

---

## 四、接口定义

```typescript
// 扩展管理
extensions.install(path: string): Promise<ExtensionState>
extensions.uninstall(extensionId: string): Promise<void>
extensions.enable(extensionId: string): Promise<void>
extensions.disable(extensionId: string): Promise<void>
extensions.list(): Promise<ExtensionState[]>
extensions.get(extensionId: string): Promise<ExtensionState | null>
extensions.getManifest(extensionId: string): Promise<ExtensionManifest>
extensions.openSettings(extensionId?: string): Promise<void> // 打开 Settings > Extensions 或指定扩展详情

// 扩展配置（预留：宿主有配置存储 API 后再落地 Settings 子区）
extensions.getConfig(extensionId: string): Promise<Record<string, any>>
extensions.setConfig(extensionId: string, key: string, value: any): Promise<void>

// 扩展日志（预留：宿主有扩展日志查询 API 后再落地 Settings 子区）
extensions.getLogs(extensionId: string): Promise<ExtensionLogEntry[]>

```

---

## 四A、扩展执行环境

扩展代码的执行环境根据 contributes 类型分为两条路径：

| 扩展类型 | 执行环境 | 能力边界 |
|---------|---------|---------|
| UI 类（views、menus、keybindings、toolbar_items、statusbar_items、themes） | 前端 Solid.js 沙箱 | 可访问：DOM 渲染、IPC 调用、事件订阅。不可访问：文件系统、终端、网络 |
| Background / Agent 类（hooks、context_providers、search_providers、file_watchers） | 对应宿主 port / Pipeline（当前仅登记已落地的声明式 contract） | 未接入真实 runtime 的 contribution 必须 fail-closed；不可把 manifest module 当作可执行入口 |
| Integration 类（gateway、middlewares、mcp_servers、transport_adapters、notification_channels） | 对应宿主子系统 | 已接入宿主的 contribution 才能加载执行；未接入真实 Registry / Pipeline / Policy / EventBus 链路的 contribution 必须 fail-closed |

Settings 管理页不改变扩展执行环境，只负责展示状态、权限、配置、日志和安装/卸载操作。

**通信机制：**
- UI 扩展 → 后端：通过 Tauri IPC（invoke/listen），与宿主应用一致
- 宿主状态 → 前端 / 扩展 UI：通过 UI Tauri event publisher / Stream 接收只读状态通知
- UI 扩展 → Agent/Task 能力：通过声明过权限的 Tauri IPC 命令调用；扩展不能通过 EventBus 发布命令或传递结果

**沙箱隔离：**
- 前端沙箱：Solid.js 组件级隔离，扩展无法访问宿主组件的内部状态
- 后端沙箱：deno_core V8 隔离，扩展无法访问 Rust 原生内存，所有系统调用通过 Sandbox 权限检查

---

## 五、扩展生命周期

```
安装（install）
  │
  ├── 解压/复制到扩展目录
  ├── 解析 extension.json
  ├── 校验权限声明
  ├── 注册到扩展表
  │
  ▼
启用（enable）
  │
  ├── 读取并校验 Extension manifest
  ├── 注册可用的 declared contributions（Gateway/MCP/Skills/LSP/UI/命令/触发器等）
  ├── 启动扩展进程（如果需要独立进程）
  │
  ▼
运行中
  │
  ├── 扩展代码执行
  ├── 资源配额检查
  └── 权限校验（Sandbox）
  │
  ▼
禁用（disable）
  │
  ├── 注销 contributed items
  ├── 停止扩展进程
  └── 保留配置和数据
  │
  ▼
卸载（uninstall）
  │
  ├── 禁用（如果已启用）
  ├── 确认 ExtensionRuntimeHandle 中的运行时资源已清理
  ├── 删除扩展文件
  ├── 清理扩展数据
  └── 无残留时完成注销；清理失败则保留 recovery 状态
```

当前实现中的 `ExtensionRuntimeHandle` 是 Extension 级聚合句柄，不是每个 Registry 资源的 opaque handle。它保存完整 Provider runtime ID、`ApiProtocol` 和其他已提交资源的撤销事实；disable/rollback 使用这些已保存值，不依据可变 manifest 重新拼接 ID。清理失败会保留残留句柄，后续可重试；逐资源 opaque handle 仍是后续生命周期增强目标。

---

## 六、扩展权限与隔离

```
扩展运行时
├── 独立进程/线程（不污染主进程）
├── 文件路径权限：只能访问 extension.json 声明的路径
├── 命令权限：只能执行声明的命令
├── 网络权限：只能访问声明的域名
├── 资源配额：CPU/内存/超时由权限与执行器约束统一管控
└── IPC 隔离：扩展只能调用自己的 IPC 命令
```

### 6.1 Sandbox 权限校验机制

扩展加载时，Sandbox 模块读取 `extension.json` 中的 `permissions` 字段，将声明的权限注册为运行时校验规则：

- 文件系统访问：每次文件操作前检查路径是否在 `filesystem` 白名单内
- 命令执行：每次命令调用前检查命令是否在 `terminal` 白名单内
- 网络请求：每次网络请求前检查目标域名是否在 `network` 白名单内
- 资源配额：注册为执行器约束，用于 CPU/内存/超时管控

未经声明的操作将被 Sandbox 拒绝并触发 `extension.error` 事件。

---

## 六A、扩展与宿主通信边界（当前声明式合同）

当前扩展 UI 只使用声明式 `contributes.views` 合同：宿主根据受控的 `placement`、`renderer`、`config` 和后端数据投影渲染视图，不向扩展暴露通用宿主运行时对象。`host:panel` 是宿主内建 renderer；`html:sandbox` 只加载扩展安装目录 `ExtensionUI/**` 下的相对 `entry`，并由宿主放入仅允许脚本执行的 sandbox iframe。扩展信息、设计面板、计划任务状态、子 agent 实时状态等，应作为具体 view contract 接入同一 HostView 体系；placement 和 renderer 由 UI Host 解释，不是 Kernel 对象类型。

当前版本没有可执行的 Extension JavaScript Runtime，也没有通用 `host.*`、`ipc.invoke`、文件系统、事件发布、流创建或面板控制桥接。`html:sandbox` 仅用于承载受限静态 UI；未来如果确有必要引入可执行运行时，必须另行设计最小能力合同、权限边界、生命周期回收和审计规则。在这些条件落地前，不得实现或依赖隐式桥接。

### 通信架构

当前 Extension 与宿主之间没有通用 JavaScript Runtime，也没有全局 `host` 对象或任意 IPC bridge。UI 贡献只通过声明式 manifest 进入宿主投影：

```text
Extension manifest
  -> ExtensionLifecycle 校验与启停
  -> contribution registry / domain host
  -> UI projection / HostView
  -> 现有 IPC、EventBus、Stream、Pipeline、Policy
```

各能力必须进入拥有该能力的业务域：

| 能力 | 当前接入方式 |
|------|--------------|
| `views` | `HostView` contract，受限 `placement` + `renderer` 白名单 |
| `menus` / `commands` / `keybindings` | 统一 `UiExtensionViewDescriptor` 与宿主投影；动作只允许当前支持的 `BuiltinAction` |
| `mcp_servers` / `tools` | MCP 真实发现与 Tool Registry / Tool Projection |
| `languages` | LSP 宿主的 Kernel-backed Registry |
| `skills` / `roles` / `hooks` | ExtensionLifecycle 交给对应 Agent / Tool Pipeline / Policy 承接 |
| `html:sandbox` | 仅加载 Extension 安装目录 `ExtensionUI/**` 下的相对静态入口，iframe 只允许 `allow-scripts` |

`html:sandbox` 不允许远程 URL、`file://` URL、绝对路径、路径穿越、符号链接资源、动态 Solid 组件或任意 ES Module renderer，也不开放通用 Tauri IPC。宿主负责资源边界、生命周期清理和错误 fail-closed；Extension 不能绕过这些约束自行接管 DOM、权限或内核原语。

未来如果确有必要支持可执行 Extension Runtime，必须作为新的受控能力重新设计最小 API、权限声明、参数校验、取消/回收、审计和版本策略；在该合同落地前，Extension 只能使用本节定义的声明式 view、宿主 projection 和受控 renderer。

### Kernel EventBus 事件订阅合同

Extension 事件订阅复用 Kernel EventBus，不创建平行事件总线。manifest 可以声明：

```json
{
  "contributes": {
    "eventSubscriptions": [
      {
        "id": "my-extension.session-completed",
        "topic": "session.completed",
        "scopeKey": "session:active",
        "handler": {
          "module": "./runtime/events",
          "export": "onSessionCompleted"
        }
      }
    ]
  }
}
```

当前合同约束：

- `id` 在同一 Extension 内唯一；`topic` 是 Kernel EventBus 的精确 topic；`scopeKey` 是可选的 scope 过滤条件。
- `handler` 只是受控的 `EventHandlerReference` DTO（`module` + `export`），不是可执行 JavaScript，也不能被 `ExtensionLifecycle` 直接加载或调用。
- 只有未来明确的 Extension runtime execution entry 完成模块解析、权限、超时、取消、错误传播和审计后，runtime 才能通过 `EventSubscriptionPort` 注册真实 handler。
- `EventSubscriptionPort` 是 Extension lifecycle 与 Kernel EventBus 之间的唯一边界；它只接收稳定的 Extension topic 字符串、scope 字符串和 `ExtensionEventHandler` DTO-only handler 合同，不暴露或持有 `EventBus`、`Topic`、`EventEnvelope` 或 Kernel handler 类型。具体的 Kernel adapter 在 `app` composition root 装配，并仅在 adapter 内完成 topic 解析和 `EventEnvelope` 到 `ExtensionEventDto` 的转换。
- lifecycle 仅在 `EventSubscriptionPort` 成功返回后向 Extension-owned subscription ledger 写入 opaque `SubscriptionId`；ledger 不执行 subscribe，只负责按 Extension 记录并拒绝重复或跨 Extension 复用。`disable` / rollback 只消费 ledger 中的真实句柄；注销成功后删除记录，失败保留记录供重试。当前 UI 卸载路径先对 `Enabled` 扩展调用 `disable`，再调用 Installer；Installer 不直接消费 ledger，卸载前清理依赖 disable 成功完成。
- 当前版本没有可执行 Extension runtime，因此带 `eventSubscriptions` 的 Extension 必须在 preflight 阶段 fail-closed；即使 app 已装配 Kernel adapter，也不会调用 `EventSubscriptionPort` 或产生 EventBus 订阅，ledger 保持为空。

Runtime handler 接收的是稳定的 `ExtensionEventDto`，而不是 Kernel `EventEnvelope`：

```typescript
type ExtensionEventDto = {
  id: string
  topic: string
  version: { major: number; minor: number }
  scopeKey: string
  source: string
  payload: unknown | null
  created_at: string
}
```

`ExtensionEventDto` 只允许由宿主从 Kernel event 转换后传入 runtime；Extension 不得访问 EventBus、Kernel context、共享指针或其他 Rust 内部对象。当前 Rust DTO 未为 `created_at` 设置 serde rename，因此实际序列化字段名是 `created_at`；`scope_key` 才通过显式 rename 输出为 `scopeKey`。`createdAt` 不会被输出，也不作为兼容输入。
### 权限声明

扩展在 `extension.json` 中声明所需权限，未声明的权限在运行时被拒绝：

```json
{
  "permissions": {
    "filesystem": ["read:./src/**", "write:./docs/**"],
    "terminal": ["git", "npm"],
    "network": ["https://api.example.com"],
    "ipc": ["agent.cancelTask", "git"],
    "events": ["agent.*", "project.*"],
    "resources": {
      "max_memory_mb": 512,
      "max_cpu_percent": 50,
      "timeout_ms": 30000
    }
  }
}
```

权限粒度：
| 权限 | 说明 |
|------|------|
| `permissions.ipc[]` | 允许调用指定 IPC 命令 |
| `permissions.filesystem[]` | 允许读取 / 写入声明范围内的文件 |
| `permissions.terminal[]` | 允许执行指定命令 |
| `permissions.network[]` | 允许访问指定网络 origin |
| `permissions.events[]` | 允许订阅宿主暴露的 UI event/stream 只读投影 pattern |
| `permissions.resources` | 扩展资源配额 |

### 通信实现

当前 HostView 支持两种宿主 renderer：`host:panel` 和 `html:sandbox`。前者由 Navis Go 内建渲染；后者只加载扩展安装目录下 `ExtensionUI/` 内的静态入口，并由宿主使用受限 sandbox iframe 承接。两者都必须先通过 HostView contract 校验，不能加载任意远程 URL、`file://` URL、动态 Solid 组件或任意 ES Module renderer。

| 沙箱模式 | 通信方式 | 说明 |
|----------|----------|------|
| `host:panel` | 宿主 UI IPC、EventBus UI 投影或 `foundation::stream` | 当前内建 renderer，不执行扩展前端模块 |
| `html:sandbox` | 受限 iframe 文档加载 | 当前支持；`entry` 必须是扩展 `ExtensionUI/` 下的相对路径，sandbox 仅允许脚本执行 |
| Shadow DOM / None | 不适用 | 预留模型，不属于当前 HostView contract |

---

## 七、扩展 Keybindings 注册

扩展快捷键先作为 manifest 声明进入 UI 后端公共投影，不由扩展生命周期直接调用前端 Hotkey API，也不建立第二套快捷键 registry：

1. `ui_list_extension_keybindings` 遍历已启用扩展的 `contributes.keybindings` 与 `contributes.commands`。
2. command 使用 `<extension_id>/<command_id>` namespaced 标识，避免不同扩展的本地 command ID 冲突。
3. 投影复用 `ui_list_extension_commands` 的 command/action 映射以及 HostView contract，只输出当前已有声明式 UI action 可执行的快捷键；当前实际支持 `OpenView` / `ToggleView`。
4. `keybinding.when` 或关联 `command.when` 存在时，因当前没有上下文评估能力而直接 fail-closed，不输出该快捷键。
5. 空快捷键、缺失 command、带 when 条件或目标 view 不满足 HostView contract 时同样不输出。
6. 扩展只能声明 `App` 范围快捷键；当前后端投影的 scope 为 `app`，未开放 `Global`。

前端可以消费该 DTO 接入现有快捷键分发，但扩展 manifest 不获得任意 handler、脚本执行或动态注册权限。

---

## 八、扩展 UI 注入机制

### 8.1 注入流程概述

当前 `contributes.views` 通过 `src-tauri/src/extension/host_view.rs` 统一校验 HostView contract。placement 只能是 `rightWorkspace`、`chatAside`、`bottomDrawer` 或 `settingsSection`；renderer 只能是 `host:panel` 或 `html:sandbox`。`host:panel` 不允许声明 `entry`，`html:sandbox` 必须声明位于扩展 `ExtensionUI/` 目录下的相对 `entry`。未知 renderer、未知 placement、缺失 placement 或不满足 renderer/entry 组合约束时均 fail-closed，不进入菜单、Command Palette 或快捷键投影。`topbar`、`leftSidebar`、`composer.*`、`statusbar` 等区域使用各自的菜单、工具栏或其他 contribution contract，不作为 Host view surface；HostView 不加载远程 URL、`file://` URL、动态 Solid 组件或任意 ES Module renderer，也不把 renderer 语义下沉到 Kernel。

```
extension.json 声明 contributes
     │
     ▼
Extension Loader 校验 views[].placement 和 views[].renderer 属于宿主白名单
     │
     ▼
扩展启用后，后端菜单、command 和 keybinding IPC 只暴露可渲染 view 及其可执行声明式 action
     │
     ▼
前端执行 OpenView / ToggleView，按 placement 打开对应 UI surface
     │
     ▼
surface host 调用对应 UI command / DTO 渲染宿主界面
```

### 8.2 当前 UI 映射

| contributes 类型 | 宿主承接边界 | 注入方式 |
|-----------------|-------------|---------|
| `views` (placement + renderer) | HostView surface | 宿主 renderer 根据 view contract 渲染 |
| `menus` | 菜单管理器 | 菜单注入（不直接影响 div） |
| `commands` | Command Palette 弹窗 | 命令注册（无固定 DOM） |
| `menus` (target: InputPlus) | `#composer:toolbar` | 投影到输入区下方 `+` 菜单，作为主发现路径 |
| `triggers` | `#composer:input` 内输入框 | 输入框快捷触发（/ 等快捷路径；扩展/能力引用主路径是 B 区 + 菜单） |
| `configuration` | `#perm-wrapper` / 设置页 | 配置注入 |
| `themes` | 全局 CSS 变量 | 样式注入（不影响 div 结构） |
| `toolbar_items` (Main) | `#topbar` | 混合渲染（在已有工具栏右侧追加） |
| `toolbar_items` (Editor) | `#editorToolbar` | 混合渲染 |
| `toolbar_items` (Terminal) | `#terminalToolbar` | 混合渲染 |
| `statusbar_items` (Left) | `#statusbar:left` | 追加渲染（与已有项目并列） |
| `statusbar_items` (Right) | `#statusbar:right` | 追加渲染 |
| `inline_extensions` (Chat) | `#chatAside` | 视图内嵌渲染 |
| `inline_extensions` (Editor) | `#editorView` | 视图内嵌渲染 |
| `inline_extensions` (Terminal) | `#terminalView` | 视图内嵌渲染 |

### 8.3 扩展视图发现机制

扩展视图通过后端 ExtensionStore 中的声明和前端 UI surface host 打开，不注册 Solid Router 路由，也不创建动态前端组件。

#### 注册流程

```
Extension Loader 解析 extension.json contributes.views
     │
     ▼
validate_view(viewRegistration)
     │
     ├── placement 必须属于宿主白名单 surface
     └── renderer 必须属于宿主白名单 renderer
     │
     ▼
OpenView / ToggleView 命令引用该 view
     │
     ▼
ui_list_extension_views 生成 enabled Extension 的统一 UiExtensionView projection
     │
     ├── 非法 renderer / placement / entry 直接跳过
     └── 归一化 allow_close / default_visible / resource_path
     │
     ▼
ui_list_menus / ui_list_extension_commands / ui_list_extension_keybindings 暴露用户可见入口
```

#### View 标识与资源校验

- `views[].id` 是扩展内的声明标识；跨扩展引用由 `extension_id + view_id` 形成命名空间，不注册 Solid Router 路由。
- `views[].placement` 必须匹配 HostView surface 白名单，`views[].renderer` 必须匹配宿主 renderer 白名单。
- `renderer = "host:panel"` 不声明 `entry`；`renderer = "html:sandbox"` 必须声明位于扩展 `ExtensionUI/` 目录下的相对 `entry`。宿主将其解析为受限本地资源，不接受远程 URL 或 `file://` 路径。
#### 统一 View projection 与生命周期字段

- `ui_list_extension_views` 是所有 enabled Extension View 的统一列表 projection；它不依赖菜单、command 或 keybinding 是否声明，也不允许前端从其他 DTO 拼装 view。
- projection 复用 HostView contract 和静态资源解析器；未知 renderer、未知 placement、无效 HTML entry 或资源不存在的 view 直接 fail-closed，不进入 `UiExtensionView[]`。
- `allow_close` 控制宿主 surface 是否提供关闭控件，默认值为 `true`；宿主 store 同时拒绝对 `allow_close=false` 的 view 执行关闭操作，不能只依赖前端按钮隐藏。
- `default_visible` 默认值为 `false`，仅在初次加载、安装或启用后的 projection 恢复时打开一次；用户手动关闭后，普通刷新不得因为该字段再次强制打开。
- Extension 禁用或卸载时，宿主清理该 Extension 在所有 placement 上的 view instance、菜单、command 和 keybinding projection。

#### HTML 静态资源契约

- `html:sandbox` 的 `entry` 必须是扩展安装目录下 `ExtensionUI/**` 的相对路径；路径使用 `/`，禁止绝对路径、路径穿越、URL、`file://` 和符号链接逃逸。
- `ExtensionUI/index.html` 及其相对引用的 CSS、JavaScript、jQuery 或其他静态依赖由 Extension 自行放入并打包在 `ExtensionUI/**` 中，宿主不提供 CDN、远程资源代理或依赖注入分支。
- 宿主只投影已解析的 `resource_path` 并用 `sandbox="allow-scripts"` iframe 加载 HTML；不添加 `allow-same-origin`，不提供通用 Tauri IPC bridge，也不把 HTML 页面当作可执行 Extension runtime。

#### 导航入口

| Renderer | 导航入口位置 |
|----------|-------------|
| 宿主白名单 placement + `host:panel` / `html:sandbox` renderer | 对应 surface 菜单、扩展菜单或 Command Palette |

#### View 注销

扩展禁用/卸载时：
1. Extension lifecycle 原子注销该扩展的 views、commands、menus 和 keybindings。
2. 后端不再从 `ui_list_menus` / `ui_list_extension_commands` / `ui_list_extension_keybindings` 暴露对应入口。
3. 前端刷新菜单、命令面板和快捷键投影后移除入口，并清理该扩展的 HostView instance。
4. 已打开的 view 不通过 Router 保留；其 surface 状态由宿主清理，不留下失效路由或旧 View contract。

---

## 九、扩展输入区主路径与快捷触发器注册

Navis Go 对输入区扩展扩展采用“双路径”策略：

1. `B 区 + 菜单` 是主发现路径。
2. `/` 等输入触发器是高级用户快捷路径。

这意味着：

- 面向普通用户的扩展能力，优先声明到 `menus.target = InputPlus`。
- 面向高频用户的快捷命令能力，可以额外声明到 `contributes.triggers`。
- 两条路径最终都应收敛到同一套引用注入和面板打开机制。

### 9.0 输入区主路径约定

`+` 按钮位于输入区下方 `B 区`，是扩展、命令、连接器、附件的统一主入口。

内置主路径菜单项包括：

- Add files or photos
- Add folder
- Slash commands
- Add connectors（打开 `Settings > Extensions` 的 Connectors 过滤视图；当前只展示贡献 `mcp_servers` 的扩展）
- Add extensions...（打开 `Settings > Extensions`）

扩展注册建议：

- 如果扩展提供“可发现能力”，应首先声明 `InputPlus` 菜单项。
- 如果扩展提供“高频快捷命令”，可再补充 `/xxx` 触发器；扩展/能力引用仍优先通过 `+` 菜单选择。
- 通过 `+` 菜单或触发器选中的结果，最终都应插入 `A 区`，而不是直接修改 `B 区` 骨架。
- 已启用扩展中的 command 只有在 action 为 `OpenView` / `ToggleView` 且目标 view 满足 HostView contract 时，才会由 `ui_list_extension_commands` 投影到 Command Palette；相同的 action 映射由 `ui_list_extension_keybindings` 复用，并使用 `<extension_id>/<command_id>` namespaced command。带 `keybinding.when` 或 `command.when` 的快捷键在当前没有上下文评估能力时 fail-closed。当前 UI 不执行扩展 handler、动态脚本或任意命令脚本；命令 manifest 也不提供这些执行入口。

X（系统提供，不可被扩展覆盖）

| 前缀 | 类型 | 功能 | 搜索范围 | 注入行为 |
|------|------|------|---------|---------|
| `/` | 技能 | 触发 Skills / Commands | 已注册的 Skills + Commands | 注入 Skill/Command 的提示词到上下文 |

文件、文件夹、会话、连接器、扩展能力等引用型入口默认放在 `B 区 + 菜单`。如后续保留 `@`，它应是宿主内建的输入辅助，不作为扩展/能力引用的主扩展路径。

#### `/` 触发器详情

`/` 触发器聚合所有已注册的 Skills 和 Commands，用户输入 `/` 后弹出搜索列表：

```
用户输入 "/"
     │
     ▼
弹出搜索浮层，聚合以下来源（按优先级排序）：
├── 1. Commands（轻量命令，来源：项目 .navis/commands/ + 用户 ~/.navis/commands/）
├── 2. Skills（标准/增强模式，来源：项目 .navis/skills/ + 用户 ~/.navis/skills/）
├── 3. 扩展注册的 Skills（来源：contributes.skills）
└── 4. 内置 Skills（来源：builtin/）

每个候选项显示：
├── 名称（如 "review"）
├── 来源标签（[内置] [项目] [用户] [扩展]）
├── 描述（Skill.description / Command 文件首行）
└── 类型标签（Command / Skill / Enhanced Skill）

用户选中后：
├── Command → 将提示词模板注入上下文，替换 $ARGUMENTS 占位符
├── Skill → 将 Skill 提示词 + 工具白名单注入上下文
└── Enhanced Skill → 启动步骤编排流程
```

**当前实现要点**：`/` 触发器的搜索和注入由 Skills 模块（19-skills.md）提供数据，触发器系统只负责 UI 交互层。启动时 `Skills::load_all()` 注册内置 / 用户 / 项目级 Skills 与轻量命令；扩展启用/禁用时，`ExtensionLifecycle` 再把 `contributes.skills` 注册/卸载到同一共享状态。当前用户可见 `/` 范围尚未接入 `contributes.mcp_servers` 的 `user_visible` 工具。

#### MCP 工具用户可见性

MCP 工具默认仅对 Agent 可见（Agent 内部决策调用），不暴露给用户。扩展可通过两种方式让 MCP 工具对用户可见：

**方式一：标记 `user_visible`（声明式）**

> **注意**：extension.json 中 `mcp_servers[].tools` 声明是声明式配置（用于标记 `user_visible` 等属性），与运行时 MCP 发现的 `ToolDefinition` 是不同的数据结构。为避免混淆，建议使用 `contributes.mcp_tool_overrides` 字段来承载 `user_visible` 等属性覆盖，而非在 `mcp_servers[].tools` 中重复声明完整的工具定义。

在 MCP Server 的工具定义中标记 `user_visible: true`，该工具自动注册到 `/` 触发器：

```json
{
  "mcp_servers": [{
    "name": "github",
    "tools": [{
      "name": "createPR",
      "description": "创建 Pull Request",
      "user_visible": true,
      "inputSchema": { ... }
    }, {
      "name": "internalDiff",
      "description": "内部 diff 计算",
      "user_visible": false
    }]
  }]
}
```

或使用 `mcp_tool_overrides`（推荐，仅覆盖属性，不重复工具定义）：

```json
{
  "mcp_tool_overrides": [
    { "server": "github", "tool": "createPR", "user_visible": true }
  ]
}
```

Command 不是扩展脚本 handler，也不是 MCP 工具的包装执行器。`contributes.commands` 只声明标题、分类、when 和 `BuiltinAction`；当前 UI 投影仅把目标 view 满足 HostView contract 且 action 为 `OpenView` / `ToggleView` 的 command 暴露给菜单、Command Palette 和可执行快捷键。需要参数校验、交互流程或外部系统调用时，应接入已有 MCP / Tool / Agent Pipeline contract，不在 command manifest 中增加脚本入口。

### 9.3 输入区交互流程

```
主路径 A：点击 B 区的 "+"
     │
     ▼
弹出 InputPlus 菜单：
├── Add files or photos
├── Add folder
├── Slash commands
├── Add connectors（Settings > Extensions / Connectors）
├── Add extensions...（打开 Settings > Extensions）
└── 扩展注册的 InputPlus 项
     │
     ▼ 用户选择某个能力
     │
根据菜单项类型执行：
├── 直接注入引用
├── 打开二级选择器
├── 打开参数表单
└── 按 view.placement 打开对应 UI surface
     │
     ▼
结果插入 A 区，或在对应 UI surface 中打开扩展界面

快捷路径 B：用户在 A 区输入 / 等前缀
     │
     └─ "/" → 弹出技能 / 命令触发器浮层（FuzzySearch 过滤）
         ├── 📝 /explain — 解释代码         ← Command
         ├── 📝 /test — 运行测试            ← Command
         ├── 🤖 /review — 代码审查          ← Skill
         ├── 🤖 /commit — 生成提交          ← Skill
         ├── 🔌 /pr — Pull Request 快捷选择  ← 扩展注册的高频快捷入口
         ├── 🔧 /github.createPR — 创建 PR  ← MCP Tool (user_visible)
         ├── 🤖 /deploy-check — 部署检查    ← 扩展注册的 Skill
         └── ...
     │
     ▼ 用户选择触发器（或继续输入前缀自动匹配）
     │
进入搜索模式（placeholder 显示触发器的搜索提示）
├── 用户输入关键词 → 调用 search_module / search() → 返回候选列表
├── 候选列表显示 label + description + icon
│
     ▼ 用户选中候选项
     │
调用 select_module / onSelect() → 返回 TriggerAction
├── InjectRef → 在输入框中插入 [ref_type:ref_id]{label} 标签
├── InjectText → 插入纯文本
├── RunCommand → 声明已注册命令引用（引用 CommandRegistration.id）；当前扩展 UI 不执行任意 command handler
├── OpenView → 打开扩展视图（引用 ViewRegistration.id），可传参；目标 view 必须声明真实宿主 placement + renderer
     │
     ▼
引用标签以结构化组件渲染（可点击预览，不可直接编辑文本内容），或在对应 UI surface 中显示结果详情
```

### 9.4 引用标签数据格式

引用标签注入输入框后，SessionMessageContent 的 parts 数组中体现为结构化引用：

```typescript
// 输入框中的引用标签在发送时转换为 AgentTimelinePart
type ReferencePart = {
    type: 'reference'
    ref_type: string        // 'file' | 'folder' | 'session' | 'pr' | ...（扩展自定义）
    ref_id: string          // 引用目标标识
    label: string           // 显示文本
    source: 'builtin' | 'extension'
    extension_id?: string      // 来源扩展 ID（source 为 extension 时）
}

// Context Manager 收到包含 reference parts 的消息后，按 ref_type 分发：
// - file → 读取文件内容注入上下文
// - folder → 列出目录结构注入上下文
// - session → 加载会话摘要注入上下文
// - 扩展自定义类型 → 通过 host.events 通知扩展处理
```

### 9.5 触发器卸载

扩展禁用/卸载时，InputTriggerIndex 自动：
1. 从触发器列表中移除
2. 如果当前正在使用该触发器的搜索模式，自动关闭浮层
3. 已注入输入框的引用标签保留（不删除用户已有内容），但标记为 `source: 'extension'` + `stale: true`

### 9.6 跨类型引用与联动校验

扩展加载时，Extension Loader 对 contributes 中各类型之间的引用关系做完整性校验：

```
校验规则：
├── commands 字段校验
│   ├── command 只能声明 BuiltinAction；不包含可执行 handler
│   ├── action.OpenView.view_id → 必须引用满足 HostView contract 的 contributes.views id
│   ├── action.ToggleView.view_id → 必须引用满足 HostView contract 的 contributes.views id
│   └── action 为 OpenView/ToggleView 时，对应 view 的 activation_events 自动补充
│
├── commands.id 引用校验
│   ├── menus.command → 必须引用 contributes.commands 中的 id
│   ├── keybindings.command → 必须引用 contributes.commands 中的 id
│   ├── toolbar_items.command → 必须引用 contributes.commands 中的 id
│   └── statusbar_items.command → 必须引用 contributes.commands 中的 id（可选）
│
├── views.id 引用校验
│   ├── views.activation_events → "onCommand:xxx" 必须引用 contributes.commands 中的 id
│   ├── menus.when → "extensionView:xxx" 必须引用 contributes.views 中的 id
│   ├── keybindings.when → 当前没有上下文评估能力，带 when 的 keybinding fail-closed；如恢复评估，引用必须指向 contributes.views 中的 id
│   ├── toolbar_items.when → "extensionView:xxx" 必须引用 contributes.views 中的 id
│   ├── statusbar_items.when → "extensionView:xxx" 必须引用 contributes.views 中的 id
│   ├── inline_extensions.when → "extensionView:xxx" 必须引用 contributes.views 中的 id
│   └── TriggerAction.OpenView.view_id → 必须引用 contributes.views 中的 id
│
├── inline_extensions.id 引用校验
│   └── TriggerAction.ToggleInline.extension_id → 必须引用 contributes.inline_extensions 中的 id
│
├── statusbar_items.id 引用校验
│   └── TriggerAction.UpdateStatusBar.item_id → 必须引用 contributes.statusbar_items 中的 id
│
├── commands.id 引用校验（触发器）
│   └── TriggerAction.RunCommand.command_id → 必须引用 contributes.commands 中的 id
│
├── roles 绑定校验
│   ├── roles.skills[] → 必须引用已注册的 skill id（builtin / 同扩展 contributes.skills）
│   └── roles.commands[] → 必须引用已注册的 command name（builtin / 同扩展 contributes.commands）
│
└── 冲突检测
    ├── triggers.prefix → 不可与内置 / 命令、其他扩展触发器和 Command Palette 命令冲突
    ├── commands.id → 不可与其他扩展的命令 ID 冲突
    ├── views.id → 不可与同一扩展内其他视图重复；跨扩展通过 extensionId + viewId 隔离
    └── keybindings.key → 由后端公共投影保留 App scope；空 key、缺失 command、when 或无法解析为支持的 HostView action 不输出
```

### 9.7 内置角色激活入口

扩展通过 `contributes.roles` 声明的角色写入 RoleStore 后自动获得以下 UI 入口，无需手动配置：

```
contributes.roles 启用后自动生效：
├── Command Palette 中出现 "角色: xxx" 命令（source: 'extension'）
├── 输入框支持 /role 触发器（自动聚合所有已启用角色）
└── 角色列表 UI（设置面板中）显示扩展注册的角色
```

这意味着扩展只需声明 `roles`，系统自动为其生成 Command Palette 入口和 /role 触发器候选，解决了"roles 没有 UI 入口"的问题。

---

## 九B、Agent 生命周期 Hook Contract

扩展通过 `contributes.hooks` 声明 Agent 生命周期钩子。当前已落地的是应用层 contract/registration：扩展启用时由 `ExtensionLifecycle` 将声明登记进 `ExtensionStore`，禁用时清理；宿主发布 `extension.hook.declared` / `extension.hook.removed` 事件。`ExtensionStore` 只保存声明索引，不做执行裁决。

Hook 声明只在扩展状态成功进入 `Enabled` 后登记。启用过程中如果 MCP server、Skill、LSP language 或其他运行时 contribution 失败，生命周期会回滚已进入宿主的资源，并且不会登记 hook 声明。

`PreToolUse` 的声明式 `Deny` 动作已归一为 Agent Tool Pipeline 内的 Kernel Policy constraint：`tool/agent` runtime 在执行边界读取已启用 hook 声明快照，注册 `agent.extension_hooks` constraint，由 `PolicyCheckStage` 统一返回 allow / deny。UI 不直接运行 hook，也不作为权限执行点；它只展示 Pipeline / Policy 产生的工具错误和 AgentTimelinePart。后续如果支持观察、输入改写或 `PostToolUse` 改写，才作为 Pipeline stage 落地，并且必须写审计和事件。

### 阶段边界

```
SessionStart
  会话创建、恢复或进入 Agent 运行上下文前。用于准备扩展会话态和只读启动建议。

PreToolUse
  Tool Projection 已把 provider-safe 名称反查为 MCP canonical name，工具执行前。声明式 Deny 进入 Kernel Policy；观察或请求改写输入的 hook 才进入 Pipeline stage。

PostToolUse
  `tool/agent` result envelope 归一后、Gateway tool result 回注模型前。用于观察或请求改写工具结果。

PreCompact
  上下文压缩前。用于观察压缩候选并返回应保留的消息、工具结果或摘要提示。
```

### 注册约束

```
注册约束：
├── Hook ID 在同一扩展内唯一，运行时 ID 为 <extension_id>/<hook_id>
├── phase 只接受 SessionStart / PreToolUse / PostToolUse / PreCompact
├── 同一阶段按 priority 升序查询，缺省 priority = 100
├── module 必须非空，但当前只作为声明保存，不加载执行
├── when 当前支持简单等值匹配（tool.name / tool.mcpName / permission）
└── 失败注册必须整体失败，不留下半注册状态
```

### 当前落点

```text
src-tauri/src/extension/models.rs
  HookRegistration / HookPhase contract

src-tauri/src/extension/store.rs
  RegisteredHook / register_hooks / unregister_hooks / list_hooks

src-tauri/src/extension/lifecycle/state.rs
  enable/disable 状态管理：do_enable 注册 hooks，do_disable 注销 hooks

src-tauri/src/extension/lifecycle/register.rs
  register_enabled_hook_declarations / apply_mcp_tool_overrides / register_lsp_languages

src-tauri/src/extension/loader.rs
  Hook ID 去重和 module 非空校验

src-tauri/src/tool/agent/hooks.rs
  ExtensionHookConstraint：将 PreToolUse / Deny 声明投影为 Kernel Policy constraint
```

### 当前最小闭环

1. `PreToolUse` / `Deny` 接入 `tool/agent` 的 Kernel Policy：hook 快照进入 `ToolPipelineData.extension_hooks`，由 `agent.extension_hooks` constraint 决策。
2. Hook constraint 只读取 `ExtensionStore::list_hooks` 快照，按 priority 和 phase 匹配已启用扩展声明，不扫描 manifest，不调用 UI。
3. `PreToolUse` 输入包含 `sessionId`、`callId`、`toolName`、`gatewayTool/providerName`、`input`、`permission`；当前输出只允许 `continue` / `deny`。
4. 暂不加载 JS 模块时，manifest hook 的 `module` 只作为声明和校验字段；非声明式动作不会进入当前执行链，待真实 JS 运行时 contract 落地后再实现。
5. 每个执行型 hook 结果必须通过 Pipeline / Policy / Audit / EventBus 表达；失败发 `extension.hook.error`，不得伪造工具成功结果。

---

## 九C、UI 样式与行为系统

### 样式注入

扩展通过 `contributes.styles` 注入自定义 CSS，三种作用域：

```
StyleScope::Extension（默认）
├── 样式仅在扩展 Shadow DOM 内生效
├── 不影响宿主应用和其他扩展
└── 最安全，无需额外权限

StyleScope::View
├── 样式作用于扩展所在的宿主视图（如 Chat 视图、Editor 视图）
├── 可修改宿主视图中与扩展组件相邻的元素
└── 需要声明 "style:view" 权限

StyleScope::Global
├── 样式全局生效
├── 可修改宿主应用的任何元素
└── 需要声明 "style:global" 权限，仅限可信扩展
```

### 布局覆盖

扩展通过 `contributes.layout_overrides` 精确控制自身 UI 组件的定位：

```json
{
  "layout_overrides": [
    {
      "target": "my-extension.hover-panel",
      "position": { "type": "fixed", "top": "auto", "right": "10px", "bottom": "60px" },
      "size": { "width": "320px", "height": "400px", "max_height": "80vh" },
      "z_index": 1000,
      "transition": "opacity 0.2s ease, transform 0.2s ease"
    }
  ]
}
```

### 事件行为

`contributes.behaviors` 当前仅是 manifest/schema 预留；宿主不会加载或执行其中的扩展模块。以下内容是 planned contract，不能作为当前可用的扩展 UI 执行能力：

```json
{
  "behaviors": [
    {
      "id": "my-extension.hover-preview",
      "trigger": { "type": "Hover", "delay_ms": 500, "leave_delay_ms": 300 },
      "action": { "type": "ShowPanel", "view_id": "my-extension.preview-panel", "position": "near-cursor" },
      "target": "my-extension.status-indicator"
    },
    {
      "id": "my-extension.quick-action",
      "trigger": { "type": "Shortcut", "key": "Ctrl+Shift+J" },
      "action": { "type": "ToggleComponent", "target_id": "my-extension.sidebar-widget" }
    }
  ]
}
```

---

## 九D、数据连接系统

### 上下文数据源

扩展通过 `contributes.context_providers` 向 Context Manager 注入外部数据：

```
场景示例：
├── Jira 扩展：用户消息包含 "PROJ-123" → 自动注入该 Issue 的标题、描述、状态
├── GitHub 扩展：用户消息包含 "#456" → 自动注入该 PR 的 diff 和评论
├── 文档扩展：用户消息包含文档链接 → 自动抓取链接内容注入上下文
└── 数据库扩展：用户查询数据 → 自动注入表结构和示例数据
```

**触发模式**：
```
trigger_pattern 匹配用户消息 → 调用扩展的 fetch 函数 → 返回结构化数据
→ 注入到 Context Manager 的指定位置（BeforeHistory / AfterHistory / AfterUserMessage）
→ 受 max_tokens 限制，超出部分截断
```

### 全局搜索提供者

扩展通过 `contributes.search_providers` 向全局搜索注入外部结果：

```
场景示例：
├── GitHub 扩展：搜索 "auth bug" → 返回 GitHub Issues/PRs 中的匹配结果
├── Jira 扩展：搜索 "登录" → 返回 Jira Issue 中的匹配结果
├── Notion 扩展：搜索 "API 设计" → 返回 Notion 文档中的匹配结果
└── Slack 扩展：搜索 "部署" → 返回 Slack 消息中的匹配结果
```

### 文件监听器

扩展通过 `contributes.file_watchers` 响应当前 `Session.worktree_root` 内的文件变更：

```
场景示例：
├── 测试扩展：监听 **/*.test.ts 变更 → 自动运行相关测试
├── 文档扩展：监听 **/*.md 变更 → 自动更新文档索引
├── 格式化扩展：监听 **/*.py 保存 → 自动检查格式
└── CI 扩展：监听 .github/workflows/* 变更 → 验证 YAML 语法
```

---

## 十、Manifest 文件规范

扩展清单文件统一命名为 `extension.json`，位于扩展根目录。示例：

```json
{
  "id": "com.example.my-extension",
  "name": "My Extension",
  "version": "1.0.0",
  "description": "示例扩展",
  "author": "Example",
  "permissions": {
    "filesystem": ["read:./src/**"],
    "terminal": ["npm", "git"],
    "network": ["https://api.example.com"],
    "resources": { "max_memory_mb": 512, "max_cpu_percent": 50.0, "timeout_ms": 30000 }
  },
  "contributes": {
    "views": [
      {
        "id": "my-extension.dashboard",
        "name": "Dashboard",
        "placement": "rightWorkspace",
        "renderer": "host:panel"
      }
    ],
    "menus": [
      {
        "label": "打开 Dashboard",
        "target": "Tools",
        "command": "my-extension.openDashboard",
        "icon": "layout-dashboard"
      },
      {
        "label": "引用 Pull Request",
        "target": "InputPlus",
        "command": "my-extension.pickPullRequest",
        "icon": "git-pull-request"
      }
    ],
    "toolbar_items": [
      {
        "id": "my-extension.toolbar.sync",
        "label": "同步数据",
        "icon": "refresh-cw",
        "command": "my-extension.sync",
        "position": "Main"
      }
    ],
    "statusbar_items": [
      {
        "id": "my-extension.status.sync",
        "label": "已同步",
        "icon": "check-circle",
        "position": "Right",
        "command": "my-extension.openDashboard",
        "priority": 50
      }
    ],
    "inline_extensions": [
      {
        "id": "my-extension.chat.summary",
        "name": "任务摘要",
        "target": "Chat",
        "position": "AfterMessages",
        "component": "./views/TaskSummary.tsx",
        "priority": 100
      }
    ],
    "commands": [
      {
        "id": "my-extension.openDashboard",
        "label": "My Extension: 打开 Dashboard",
        "action": { "type": "OpenView", "view_id": "my-extension.dashboard" }
      }
    ],
    "work_modes": [
      {
        "id": "pull-request-review",
        "name": "PR Review",
        "description": "面向 Pull Request 阅读、审查和变更总结的 Custom 工作模式",
        "icon": "git-pull-request",
        "role": "developer",
        "available_tools": ["read", "git", "lsp.diagnostic", "mcp.github.*"],
        "skills": ["review", "explain", "bug-fix"],
        "commands": ["my-extension.openDashboard"],
        "context_policy": "github.issue-provider",
        "behavior_rules": ["优先说明变更风险", "引用代码时带文件路径和行号"],
        "entry_view": "my-extension.dashboard",
        "default_views": ["my-extension.dashboard"],
        "default_model": "claude-sonnet-4-6",
        "model_preferences": {
          "temperature": 0.2,
          "extended_thinking": true
        },
        "capabilities": ["review", "github", "code-analysis"]
      }
    ],
    "triggers": [
      {
        "prefix": "/pr",
        "label": "Pull Request",
        "description": "引用 GitHub PR",
        "icon": "git-pull-request",
        "placeholder": "搜索 Pull Request...",
        "search_module": "./triggers/pr-search.js",
        "select_module": "./triggers/pr-select.js",
        "scope": "Global"
      }
    ],
    "hooks": [
      {
        "id": "github.issue-context",
        "name": "GitHub Issue 工具输入守卫",
        "phase": "PreToolUse",
        "priority": 50,
        "module": "./hooks/issue-context.js"
      },
      {
        "id": "github.pr-link-detector",
        "name": "GitHub PR 结果整理",
        "phase": "PostToolUse",
        "module": "./hooks/pr-link-detector.js"
      }
    ],
    "context_providers": [
      {
        "id": "github.issue-provider",
        "name": "GitHub Issue",
        "description": "获取 GitHub Issue 详情",
        "module": "./providers/issue-provider.js",
        "trigger_pattern": "#[0-9]+",
        "inject_position": "AfterUserMessage",
        "max_tokens": 2000
      }
    ],
    "search_providers": [
      {
        "id": "github.search",
        "name": "GitHub",
        "icon": "github",
        "module": "./providers/github-search.js",
        "scope_tags": ["code", "issues", "prs"]
      }
    ],
    "styles": [
      {
        "id": "github.theme",
        "module": "./styles/github-theme.css",
        "scope": "Extension",
        "variables": {
          "--github-accent": "#238636",
          "--github-danger": "#da3633"
        }
      }
    ],
    "behaviors": [
      {
        "id": "github.hover-pr",
        "trigger": { "type": "Hover", "delay_ms": 500 },
        "action": { "type": "ShowPanel", "view_id": "my-extension.pr-detail", "position": "near-cursor" }
      }
    ]
  }
}
```

---

## 十一、事件定义

```typescript
type ExtensionEvents = {
  'extension.installed':    { extensionId: string; version: string }
  'extension.uninstalled':  { extensionId: string }
  'extension.enabled':      { extensionId: string }
  'extension.disabled':     { extensionId: string }
  'extension.updated':      { extensionId: string; fromVersion: string; toVersion: string }
  'extension.error':        { extensionId: string; error: string }
  'extension.loading':      { extensionId: string }   // 扩展正在加载
  'extension.enabling':     { extensionId: string }   // 扩展正在启用
  'extension.disabling':    { extensionId: string }   // 扩展正在禁用
  'extension.unloading':    { extensionId: string }   // 扩展正在卸载
  'extension.trigger.registered':   { extensionId: string; prefix: string }   // 触发器注册
  'extension.trigger.unregistered': { extensionId: string; prefix: string }   // 触发器注销
  'extension.view.registered':      { extensionId: string; viewId: string; position: string }  // 视图注册
  'extension.view.unregistered':    { extensionId: string; viewId: string }  // 视图注销
  'extension.command.registered':   { extensionId: string; commandId: string }  // 命令注册
  'extension.command.unregistered': { extensionId: string; commandId: string }  // 命令注销
  'extension.behavior.triggered':   { extensionId: string; behaviorId: string; triggerType: string }  // 行为触发
  'extension.style.injected':       { extensionId: string; scope: string }  // 样式注入
  'extension.hook.registered':      { extensionId: string; hookId: string; phase: string; priority: number }  // 钩子注册
  'extension.hook.unregistered':    { extensionId: string; hookId: string; phase: string }  // 钩子注销
  'extension.hook.executed':        { extensionId: string; hookId: string; phase: string; duration: number }  // 钩子执行完成
  'extension.hook.error':           { extensionId: string; hookId: string; phase: string; error: string }     // 钩子执行失败
  'extension.hook.skipped':         { extensionId: string; hookId: string; phase: string; reason: string }    // 钩子跳过（超时/条件不满足）
  'extension.context_provider.executed': { extensionId: string; providerId: string; tokensInjected: number }  // 上下文数据源执行
  'extension.search_provider.executed':  { extensionId: string; providerId: string; resultCount: number }     // 搜索提供者执行
  'extension.file_watcher.triggered':    { extensionId: string; watcherId: string; filePath: string; event: string }  // 文件监听器触发
}
```

上表是宿主发布的 Extension 生命周期/诊断事件，不等同于 `contributes.eventSubscriptions` 的订阅声明。订阅声明的 handler 执行入口尚未落地前，宿主不会把这些声明注册到 Kernel EventBus。

---

## 十二、测试策略

```
单元测试：manifest 解析（含 eventSubscriptions）、未知字段拒绝、ExtensionEventDto 序列化（包括 `created_at` / `scopeKey` 字段名）、权限校验、生命周期状态转换、事件 subscription ledger、触发器前缀冲突检测、MenuTarget / host view renderer 映射校验
集成测试：扩展安装/卸载、沙箱隔离、资源限制、事件订阅 fail-closed 与 cleanup 重试、触发器注册/注销/搜索/注入流程、工具栏/状态栏/内嵌组件注册与渲染
```

