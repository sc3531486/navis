# Navis Go 设计文档总目录

> Navis Go - 通用桌面应用白板与扩展运行时；Navis Code 是其第一个产品套件。
> 技术栈：Solid.js + Kobalte + Tailwind CSS + Rust (Tauri)
> 最后更新：2026-08-18

---

## 阅读边界

本目录同时包含 Navis 通用框架设计和 Navis Code 业务设计。领域模块名称表示扩展能力，不表示它们属于 `src/` 或 `src-tauri/src/` 宿主源码目录。`08`-`21`、`26` 等领域文档描述 Navis Code 扩展；`01`-`07`、`22`-`27`、`34`-`38` 主要描述宿主、扩展和 UI 基础设施。

当前 `src/router/`、`src/layouts/` 和部分宿主 store 仍是 Navis Code 产品壳的迁移过渡区。引用这些路径时必须明确写成“当前过渡代码”，不能称为通用框架业务能力。

## 文档结构

```
design/
├── README.md                          ← 本文件（总目录）
├── 00-architecture-overview.md        ← 整体架构设计
│
│  ── foundation（基础能力）──
├── 01-logger.md                       ← 日志系统
├── 02-ipc.md                          ← IPC 通信层
├── 02b-stream.md                      ← 流式数据通道（Stream 规范）
├── 03-config.md                       ← 配置管理
├── 04-storage.md                      ← 持久化存储
│
│  ── security（安全边界）──
├── 05-auth.md                         ← 身份认证
├── 06-sandbox.md                      ← 安全沙箱
│
│  ── extension（扩展系统）──
├── 07-extension.md                       ← Cordis 扩展基座与清单
├── 19-skills.md                       ← Skills 技能管理
│
│  ── project / worktree / session（项目、工作树和会话事实）──
├── 08-session.md                      ← 会话管理
├── 10-project.md                      ← 项目与工作树管理
├── 20-rag-knowledge.md                ← Knowledge 项目知识管理（RAG/本地知识库）
│
│  ── ai（模型与 Agent）──
├── 12-gateway.md                      ← 模型网关
├── 16-agent.md                        ← Agent 决策引擎
├── 17-task-sidechain.md               ← Task Sidechain 子任务编排
├── 18-context-manager.md              ← 上下文管理
│
│  ── tool（开发工具能力）──
├── 09-file.md                         ← 文件系统
├── 11-terminal.md                     ← 终端管理
├── 13-mcp.md                          ← MCP 工具协议引擎
├── 14-lsp.md                          ← LSP 语言服务协议
├── 15-edit.md                         ← 代码编辑引擎
├── 21-git.md                          ← 版本控制
├── 32-clipboard.md                    ← 剪贴板
│
│  ── cross-cutting（跨域架构基线）──
├── 33-extension-gateway-review.md     ← Extension × Gateway 架构复核与验收基线
├── 34-extension-ui-open-architecture.md ← 扩展 UI 开放架构 + 全系统改造基线：Cordis 装配 / Zone 开放命名空间 / 多轨渲染 / 白名单桥 / 独立弹框 / 扩展存储·网络·发现 / 扩展间通信 / 三线审计改造清单（§15）
├── 35-whiteboard-container.md        ← 白板容器与万物皆扩展：容器领域无关、ExtensionUI/ExtensionBackend、受控操作执行
├── 36-extension-development.md       ← 扩展开发手册：Cordis plugin 开发模型、manifest/contributes、固定目录契约
├── 37-component-extension.md         ← Cordis × WASM 组件化扩展执行基座：组件轨/native 逃生舱
├── 38-deepseek-harness-inspiration.md ← DeepSeek Harness 万物皆插件设计借鉴（研究参考，Cordis 同源）
│
│  ── ui（前端与 Tauri 事件出口）──
├── 22-ui-framework.md                 ← UI 框架层
├── 23-command-palette.md              ← 命令面板
├── 24-dialog.md                       ← 对话框系统
├── 25-notification.md                 ← 通知系统
├── 26-editor.md                       ← 编辑器渲染
├── 27-hotkey.md                       ← 全局快捷键
├── 28-i18n.md                         ← 国际化
│
└── (模块树结束；详见上方分组)
```

> 注：`modules/` 目录为预留占位，当前不存在；所有模块详设直接位于 design/ 根目录。

---

## 模块总览

### 大域结构

当前 Rust 运行时的通用宿主位于 `src-tauri/src/` 的 `app`、`extension`、`foundation`、`kernel`、`security`、`ui` 等目录；Agent、会话、项目、工具等领域文档描述的是 `extensions/navis-code/` 下的业务扩展，不是宿主目录。设计文档编号用于检索，不表示运行时目录层级。

EventBus 只有 `crate::kernel::EventBus` 一套。Tauri event 是 UI 侧只读事件出口，不叫 bridge，不保存事实，也不能作为后端业务事件源。

### foundation

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 01 | Logger | [01-logger.md](./01-logger.md) | 日志系统：分级日志、轮转切割、脱敏、导出、审计 |
| 02 | IPC | [02-ipc.md](./02-ipc.md) | 前后端命令边界：Tauri command、参数校验、错误编码、内核事件只读出口 |
| 02b | Stream | [02b-stream.md](./02b-stream.md) | 流式数据通道：Stream Channel、ThrottledEmitter、与 Kernel EventBus 的边界 |
| 03 | Config | [03-config.md](./03-config.md) | 配置管理：配置存储、热更新、校验、导入导出 |
| 04 | Storage | [04-storage.md](./04-storage.md) | 持久化存储：KV、会话存储、记忆存储、缓存、加密 |

### security

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 05 | Auth | [05-auth.md](./05-auth.md) | 身份认证：密钥存储、凭证管理、用户身份 |
| 06 | Sandbox | [06-sandbox.md](./06-sandbox.md) | 安全沙箱：访问控制、命令黑白名单、Project / Worktree 信任、资源限制 |

### extension

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 07 | Extension | [07-extension.md](./07-extension.md) | Cordis 扩展基座：manifest/contributes、Cordis plugin/service 生命周期与固定目录契约 |
| 19 | Skills | [19-skills.md](./19-skills.md) | 技能管理：SkillStore、激活计划、角色模板、轻量命令 |

### project / worktree / session

> 管理项目、工作树、会话和知识事实。Project 表示项目身份、指令和知识；Worktree 表示当前会话绑定的真实目录或 Git checkout；Session 表示对话、消息、Turn Timeline 和可恢复执行事实。

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 08 | Session | [08-session.md](./08-session.md) | 会话管理：会话 CRUD、项目/工作树绑定、历史管理、检查点 |
| 10 | Project / Worktree | [10-project.md](./10-project.md) | 项目与工作树管理：navis.md 配置、知识文件、会话列表、项目发现、工作目录绑定 |
| 20 | Knowledge | [20-rag-knowledge.md](./20-rag-knowledge.md) | 项目知识管理：文档索引、向量存储、语义检索（RAG/本地知识库） |

### ai

> 模型调用、上下文组装和 Agent 执行。

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 12 | Gateway | [12-gateway.md](./12-gateway.md) | 模型网关：Provider / Model 路由、Extension Adapter、Quota 计量、离线降级 |
| 16 | Agent | [16-agent.md](./16-agent.md) | AI 决策引擎：状态机、任务编排、Extended Thinking、自我进化 |
| 17 | Task Sidechain | [17-task-sidechain.md](./17-task-sidechain.md) | 子任务编排：Task kind、sidechain session、父子任务 AgentTimelinePart 摘要、权限冒泡 |
| 18 | Context Manager | [18-context-manager.md](./18-context-manager.md) | 上下文管理：上下文组装、裁剪、压缩、快照 |

### tool

> Agent 和 UI 可调用的本地开发工具能力。

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 09 | File | [09-file.md](./09-file.md) | 文件系统：文件操作、路径管理、Watcher、大文件处理 |
| 11 | Terminal | [11-terminal.md](./11-terminal.md) | 终端管理：Shell 管理、命令执行、输出流推送、并发控制 |
| 13 | MCP | [13-mcp.md](./13-mcp.md) | 工具协议：协议引擎、工具路由、Kernel Registry 接入、熔断重试 |
| 14 | LSP | [14-lsp.md](./14-lsp.md) | 语言服务：LSP Client、多 Server 管理、代码补全/诊断/跳转 |
| 15 | Edit | [15-edit.md](./15-edit.md) | 代码编辑：精确编辑、Diff 生成、批量编辑、撤销重做 |
| 21 | Git | [21-git.md](./21-git.md) | 版本控制：Git 操作、状态检测、凭证管理、跨平台路径标准化 |
| 32 | Clipboard | [32-clipboard.md](./32-clipboard.md) | 剪贴板：系统剪贴板读写、格式处理 |

### ui

> 前端 UI 框架、组件和交互层。

| 编号 | 模块 | 文档 | 说明 |
|------|------|------|------|
| 22 | UI Framework | [22-ui-framework.md](./22-ui-framework.md) | UI 框架：布局系统、HostView surface、主题、单 Router、扩展视图承接 |
| 23 | Command Palette | [23-command-palette.md](./23-command-palette.md) | 命令面板：命令投影、模糊搜索、AI 推荐、命令面板内 `>` `@` `/` `#` 搜索前缀 |
| 24 | Dialog | [24-dialog.md](./24-dialog.md) | 对话框：模态管理、确认框、Agent 确认框 |
| 25 | Notification | [25-notification.md](./25-notification.md) | 通知系统：Toast、系统通知、通知中心、扩展渠道扩展 |
| 26 | Editor | [26-editor.md](./26-editor.md) | 编辑器：CodeMirror 6、Diff 视图、LSP 集成、编辑器扩展激活 |
| 27 | Hotkey | [27-hotkey.md](./27-hotkey.md) | 全局快捷键：快捷键投影、冲突检测、分发 |
| 28 | i18n | [28-i18n.md](./28-i18n.md) | 国际化：多语言文案、动态切换、扩展语言包 |

---

## 模块统计

| 大域 | 模块数 | 文档编号 | 说明 |
|------|--------|----------|------|
| foundation | 5 | 01-04, 02b | 日志、IPC、Stream、配置、存储 |
| security | 2 | 05-06 | 身份认证、安全沙箱 |
| extension | 2 | 07, 19 | Cordis 扩展基座、Skills |
| project | 3 | 08, 10, 20 | 会话、项目、工作树、知识 |
| ai | 4 | 12, 16-18 | Gateway、Agent、Task Sidechain、Context |
| tool | 7 | 09, 11, 13-15, 21, 32 | 文件、终端、MCP、LSP、编辑、Git、剪贴板 |
| cross-cutting | 5 | 33-37 | Extension × Gateway 复核、扩展 UI 开放架构、白板容器、扩展开发手册、组件化执行基座 |
| ui | 7 | 22-28 | 前端框架和组件 |
| **合计** | **35** | **01-28, 02b, 32-37** | 当前保留的设计文档 |

---

## Extension 贡献全景

Extension（07）基于 Cordis 承载扩展组合与服务生命周期。`manifest` 是插件元数据，`contributes` 是能力声明；loader 将 `ExtensionUI/` / `ExtensionBackend/` 固定目录中的扩展点装载为 Cordis plugin/service，由宿主 capability port 接入业务域。能力发现、执行、事件和权限统一进入 Cordis Context + Kernel Registry / Pipeline / EventBus / Policy；Cordis 是装配与生命周期底座，不替代 Kernel 原语。完整字段以 `design/07-extension.md` 和 `ExtensionContributes` 为准；Gateway Adapter、Provider、Model 字段统一位于 `contributes.gateway`，Gateway Pipeline middleware 使用独立的 `contributes.middlewares`：

| contributes 字段 | 目标模块 | 说明 |
|-----------------|---------|------|
| `gateway` | 12-Gateway | 声明 Gateway Adapter、Provider、Model 与认证 reference；启用后进入 Gateway Registry / Pipeline，未接线时 fail-closed |
| `middlewares` | 12-Gateway | 声明请求中间件阶段，Gateway Pipeline 承接 |
| `transport_adapters` | 13-MCP | 声明 MCP 传输适配器，MCP ServerManager 承接并进入 Kernel Registry |
| `mcp_servers` | 13-MCP | 声明 MCP Server，真实 `tools/list` 成功后注册工具能力 |
| `languages` | 14-LSP | 自定义语言 LSP Server 配置 |
| `skills` | 19-Skills | 写入 SkillStore，生成 Agent 激活计划 |
| `roles` | 19-Skills | 写入 RoleStore，供 Agent / Task Sidechain 注入 |
| `views` | 22-UI Framework | 声明 UI Host view renderer 视图；菜单/命令打开宿主 UI surface，完整界面由 UI 域内置 renderer 承接，renderer 不进入 Kernel |
| `menus` | 22-UI Framework | 声明菜单项，挂到既有 MenuTarget |
| `commands` | 22-UI Framework | 声明命令，Command Palette 消费投影 |
| `keybindings` | 27-Hotkey | 声明快捷键，Hotkey 模块做冲突检测和分发 |
| `themes` | 26-Editor | 自定义编辑器主题 |
| `editor_languages` | 26-Editor | 自定义语言模式/语法 |
| `editor_extensions` | 26-Editor | 自定义编辑器扩展 |
| `notification_channels` | 25-Notification | 自定义通知渠道 |

---

## 编写进度

```
[■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■] 35/35

✅ 00-architecture-overview.md
✅ 01-logger.md
✅ 02-ipc.md
✅ 02b-stream.md
✅ 03-config.md
✅ 04-storage.md
✅ 05-auth.md
✅ 06-sandbox.md
✅ 07-extension.md
✅ 08-session.md
✅ 09-file.md
✅ 10-project.md
✅ 11-terminal.md
✅ 12-gateway.md
✅ 13-mcp.md
✅ 14-lsp.md
✅ 15-edit.md
✅ 16-agent.md
✅ 17-task-sidechain.md
✅ 18-context-manager.md
✅ 19-skills.md
✅ 20-rag-knowledge.md
✅ 21-git.md
✅ 22-ui-framework.md
✅ 23-command-palette.md
✅ 24-dialog.md
✅ 25-notification.md
✅ 26-editor.md
✅ 27-hotkey.md
✅ 28-i18n.md
✅ 32-clipboard.md
✅ 33-extension-gateway-review.md
✅ 34-extension-ui-open-architecture.md
✅ 35-whiteboard-container.md
✅ 36-extension-development.md
✅ 37-component-extension.md
```

## 辅助文档（非编号，未计入进度）

以下文档不属于编号模块，供研究与追溯，不参与模块统计：

| 文档 | 说明 |
|------|------|
| [kernel.md](./kernel.md) | Kernel 四原语（Registry / Pipeline / EventBus / Policy）设计细节 |
| [analysis.md](./analysis.md) | 产品竞品确认报告（参考文档，非当前实现基线） |
| [navis-agent-flow-optimized.md](./navis-agent-flow-optimized.md) | Agent 流优化研究（对标 opencode / Claude Code） |
| [opencode-agent-flow.md](./opencode-agent-flow.md) | opencode Agent 流源码研究 |

> 注：analysis.md 为历史竞品分析，部分内容（如"Extension 16 types"计数）已过时，以 07-extension.md / 34 号 §15 为准。

---

## 编写规范

每个模块详设文档应包含：

1. **模块概述** - 定位、职责边界
2. **架构设计** - 子模块划分、架构图
3. **数据模型** - 核心数据结构
4. **接口定义** - Rust API + IPC 命令
5. **依赖关系** - 依赖的模块、被依赖的模块
6. **状态管理** - 状态定义、状态转换（如有）
7. **错误处理** - 异常场景、降级策略
8. **安全考量** - 权限、校验（如有）
9. **事件定义** - 该模块产出的事件（含 sessionId）
10. **扩展扩展支持** - contributes 类型（如有）
11. **性能指标** - 关键性能要求
12. **测试策略** - 单元测试、集成测试要点

---

## 竞品参考

| 工具 | 借鉴点 |
|------|--------|
| **Claude Desktop / Code** | MCP 协议、Tool Contract、sidechain transcript、上下文管理 |
| **Codex App** | 本地沙箱、Git 集成、任务驱动 UI |
| **OpenCode** | Go channel 任务通信、轻量设计、配置驱动 |
| **Hermes** | 工具生命周期、审批队列、危险操作分级、Session 恢复 |
| **VS Code** | 扩展系统（contributes/views/commands）、Command Palette、分屏布局 |



