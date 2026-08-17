# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Navis Go 是一个 AI 驱动的桌面开发工具，基于 Tauri 2 构建。技术栈：SolidJS + Kobalte + Tailwind CSS（前端）+ Rust（后端）。深色主题优先，视觉风格对标 Claude Desktop，强调高信息密度和开发者工具美学。

## 常用命令

```bash
# 前端开发（Vite dev server，端口 1420）
npm run dev

# 前端构建（TypeScript 检查 + Vite 构建）
npm run build

# 前端验证测试（基于 esbuild + node:assert 的脚本测试）
npm run test:stream          # 聊天消息 reducer / timeline 顺序
npm run test:menus           # 菜单命令覆盖率
npm run test:tool-renderers  # 工具渲染器

# Tauri 开发模式（同时启动前端 + Rust 后端）
npx tauri dev

# Tauri 构建（生成桌面应用安装包）
npx tauri build

# Rust 后端编译检查
cd src-tauri && cargo check

# Rust 后端测试
cd src-tauri && cargo test

# Rust 运行单个模块测试
cd src-tauri && cargo test module_name::

# Rust 运行单个测试函数
cd src-tauri && cargo test test_function_name
```

## 架构总览

### 整体分层

应用采用前后端分离的 Tauri 2 架构，前端通过 Tauri IPC 与 Rust 后端通信。后端按 `ai / app / extension / foundation / kernel / project / security / tool / ui` 九个业务域组织；设计文档见 `design/` 目录（`design/README.md` 为总索引，文档编号只用于检索）。

产品主概念对齐 Claude Code：`Project / Worktree / Session`。Project 表示项目身份、指令和知识；Worktree 表示当前会话绑定的真实目录或 Git checkout；Session 表示对话、消息和可恢复执行事实。`workspace` 不再作为业务域概念使用，只保留在包管理 workspace、UI placement（如 `rightWorkspace` / `startWorkspace` / `composer.workspace`）或第三方协议术语中。

### 前端架构 (`src/`)

- **框架**: SolidJS 1.9 + `@solidjs/router` + Kobalte 组件库
- **样式**: Tailwind CSS 4.3 + CSS 变量（见 `DESIGN.md` 设计规范）
- **编辑器**: CodeMirror 6（多语言支持，含 LSP、Diff、主题扩展）
- **终端**: xterm.js
- **路径别名**: `@/` → `src/`
- **入口**: `src/App.tsx` 负责挂载，唯一 `Router` 定义在 `src/router/index.tsx`

**状态管理四层架构**（见 `src/stores/index.ts`，共 30+ 个 store）：
1. `AppState`（`src/stores/app.ts`）— 顶层全局 Store，`activeSessionId` / `activeProjectId` 为跨模块唯一真实来源
2. 模块 Store（agent、session、project、gateway、extension、composer-* 等）— 使用 SolidJS `createStore`
3. IPC 事件同步层 — `src/lib/stream/` 的 `useEvent`（Tauri event）/ `useChannel`（Tauri Channel 高频流）自动同步后端状态
4. 持久化层 — localStorage 存储偏好，Config 模块存储设置

**关键目录**：
- `src/components/HostView/` — 宿主扩展视图渲染与 surface，承接 `contributes.views` 的 HostView 面板/抽屉/侧栏投影
- `src/components/Editor/` — CodeMirror 编辑器（含 DiffView、标签页、LSP、Minimap）
- `src/components/AgentTimeline/` — Agent 执行时间线渲染
- `src/components/CommandPalette/` — 命令面板（模糊搜索 + AI 推荐）
- `src/components/Composer/` — 消息输入区（含 slash 命令、菜单）
- `src/components/Dialog/` — 模态对话框系统
- `src/lib/hotkey/` — 全局快捷键（注册、冲突检测、分发）
- `src/i18n/` — 国际化（`locales/zh-CN.json`、`locales/en-US.json`）

### Rust 后端架构 (`src-tauri/src/`)

入口：`lib.rs` 只声明顶层域并导出 `app::run`；`app/mod.rs` 负责 Tauri bootstrap、扩展注册、`app.manage()` 状态装配和 `generate_handler` 命令注册。

**后端顶层域**：

| 顶层域 | 子模块 | 说明 |
|------|------|------|
| kernel | core, registry, pipeline, event, policy, audit, observability, snapshot | Registry / Pipeline / EventBus / Policy 通用原语 |
| app | mod.rs | Tauri bootstrap、状态装配、扩展初始化、命令注册 |
| ai | agent, gateway, context | Agent 决策、LLM 网关、Context 组装 |
| tool | agent, mcp, file, edit, terminal, git, lsp, clipboard, memory | 工具运行链与各类工具实现 |
| extension | extension, skills | 扩展与技能扩展贡献注册 |
| project | catalog, session, knowledge | Project 发现/配置、Session 生命周期、知识管理 |
| foundation | config, logger, storage, stream, ipc | 基础设施 |
| security | auth, sandbox | 认证、沙箱、权限约束 |
| ui | sessions, tasks, gateway, extensions, settings, menus, workspace, messages 等 | Tauri IPC commands（~70 个，分散在各子模块）、DTO、前端投影 |

**核心子系统**：
- **Gateway** (`ai/gateway/`) — 多 Provider LLM 网关，运行时依赖 `protocol/*` 与 provider profile，负责请求构造、流式传输、中间件管道、Quota 计量
- **Agent** (`ai/agent/`) — 当前以类型、thinking/turn helper、sidechain 与上下文原语为主；运行期 turn 编排仍位于 `ui/mod.rs::ui_stream_session_message` 与 `ui/runtime/agent_tool_loop.rs`，后续应迁往 application/use-case 层
- **Agent Tool Runtime** (`tool/agent/`) — 工具目录、契约、批处理、执行管线、结果回注、扩展 hook 策略
- **MCP** (`tool/mcp/`) — Model Context Protocol 引擎，内置服务器（filesystem、terminal、git、clipboard、memory），多传输适配（stdio、SSE、WebSocket、REST、gRPC），含熔断重试
- **Extension** (`extension/extension/`) — contributes 扩展类型（providers、middlewares、transport_adapters、mcp_servers、languages、skills、roles、views、menus、commands、keybindings、themes、editor_languages、editor_extensions、tray_items、notification_channels）
- **Storage** (`foundation/storage/`) — `Storage` 仍是全库入口；记忆领域已通过 `MemoryStore` 收口，`SessionStore` 等其余 facade 仍待继续抽取

## 关键约定

- **日志**: Rust 端全部使用 `tracing` 宏（`debug!`、`info!`、`warn!`、`error!`），不用 `log` 宏
- **IPC 命令**: 所有前端→后端通信通过 Tauri `invoke`，命令以 `#[tauri::command]` 定义在 `src-tauri/src/ui/` 各子模块（sessions、tasks、gateway 等），统一在 `app/mod.rs` 的 `generate_handler` 注册，前端通过 `@tauri-apps/api` 调用
- **事件流**: 离散事实统一通过 Kernel EventBus；前端只读投影使用 Tauri event / `useEvent`；高频数据统一通过 `foundation::stream` 的 Tauri Channel（前端用 `useChannel`）
- **模块文档**: 每个 Rust 模块的 `mod.rs` 顶部包含职责边界和子模块说明（中文注释）
- **Store 模式**: 前端 store 使用 `createStore` + 纯函数 action（`set*` 前缀），通过 `stores/index.ts` 统一导出
- **路由**: 当前所有路由视图内联在 `src/router/index.tsx` 单文件中，前端只保留这一层 `Router`
- **无原生装饰**: 窗口使用 `decorations: false`，自定义标题栏
- **设计规范**: 视觉规范见 `DESIGN.md`（颜色、字体、组件 token、布局系统），产品定义见 `PRODUCT.md`，模块详细设计见 `design/` 目录

## 外部依赖要点

- **Tauri 2.9**: `@tauri-apps/api` ^2.11, `@tauri-apps/plugin-dialog` ^2.7
- **Rust**: tokio（全异步）、rusqlite（bundled SQLite）、reqwest（流式）、aes-gcm（加密）、sysinfo（健康监控）、notify（文件监听）
- **前端**: SolidJS 1.9、Vite 8.0、TypeScript 6.0（strict）、Tailwind CSS 4.3
