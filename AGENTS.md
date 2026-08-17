# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## 项目概述

Navis Go 是一个**万物皆扩展**的桌面应用框架，基于 Tauri 2 构建。框架本身只提供应用程序白板（窗口管理、扩展加载、能力注册），所有业务功能——AI、编辑器、终端、项目管理等——均以扩展形式接入。当前基于此框架构建的第一个应用是 **Agent IDE**（对标 Codex Desktop）。

技术栈：SolidJS + Kobalte + Tailwind CSS（前端）+ Rust（后端）。深色主题优先，视觉风格对标 Codex Desktop，强调高信息密度和开发者工具美学。

**设计哲学**：参考 Cordis 扩展框架与 DeepSeek Harness 万物皆插件设计。底层框架高内聚、低耦合；业务领域全部视为扩展点，可在同一框架上构建柜面系统、双录系统等不同应用。

## 常用命令

```bash
# 前端开发（Vite dev server，端口 1420）
npm run dev

# 前端构建（TypeScript 检查 + Vite 构建）
npm run build

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

```
┌─────────────────────────────────────────────────────┐
│                  应用层（扩展）                        │
│  extensions/navis-*   src/components/Agent*|Chat|... │
├─────────────────────────────────────────────────────┤
│                框架层（不可含业务逻辑）                  │
│  src-tauri/src/{kernel,extension,foundation,...}     │
│  src/{router,HostView,stores/app,lib/stream,...}    │
├─────────────────────────────────────────────────────┤
│              Tauri 2 平台 + 系统能力                    │
└─────────────────────────────────────────────────────┘
```

框架层只提供：窗口管理、扩展加载/生命周期、能力注册/发现、IPC 基础件、事件总线、流式通道、安全沙箱、配置/存储接口。**一切业务逻辑均为扩展**。

### 后端框架层 (`src-tauri/src/`)

| 模块 | 职责 | 关键内容 |
|------|------|----------|
| `kernel/` | 四大原语 | Registry（能力注册）、Pipeline（中间件管道）、EventBus（事件总线）、Policy（策略） |
| `extension/` | 扩展系统本体 | loader（加载）、lifecycle（装配）、context（根 Context + 能力缝擦除）、component（WASM）、skills |
| `foundation/` | 能力缝接口 | config（可替换配置）、ipc（命令基础件）、logger、storage（StoragePort）、stream（高频 Channel）、status |
| `security/` | 安全边界 | auth（认证）、sandbox（沙箱权限约束） |
| `app/` | 运行壳 | Tauri bootstrap、根上下文创建、扩展注册、`app.manage()` |
| `ui/` | 框架宿主面 | extension_bridge、network、router、storage、stream、host_view、tauri_events、permissions、dto、menus |

### 后端业务扩展层 (`extensions/`)

每个扩展是一个独立目录，包含 `extension.json` 声明和 Rust 源码：

| 扩展 | 职责 | 能力域 |
|------|------|--------|
| `navis-ai-platform/` | AI 平台服务 | Gateway（模型路由/Quota/中间件/协议适配）、MCP（多传输、熔断重试）、LSP |
| `navis-agent-core/` | Agent 引擎 | turn 编排、上下文管理、工具运行时、thinking、sidechain、self_evolution |
| `navis-session/` | 会话管理 | 会话 CRUD、消息存储、Session UI |
| `navis-project/` | 项目管理 | Project、Worktree、Knowledge |
| `navis-task/` | 任务系统 | 任务投影、后台任务、进度追踪 |
| `navis-editor/` | 编辑器 | CodeMirror 6（DiffView、LSP、Minimap）、文件操作、Git、剪贴板 |
| `navis-terminal/` | 终端 | xterm.js、命令执行 |
| `navis-settings/` | 设置 | 设置 UI、用户偏好 |
| `navis-knowledge/` | 知识库 | RAG 检索、文档处理 |
| `navis-memory/` | Agent 记忆 | 记忆存储、回忆、演化 |
| `navis-demo/` | 演示 | 扩展示例和开发参考 |

### 前端框架层 (`src/`)

| 目录 | 职责 |
|------|------|
| `router/` | 框架路由（`src/router/index.tsx` 单文件） |
| `components/HostView/` | 扩展视图投影（承接 `contributes.views` 的面板/抽屉/侧栏） |
| `components/ExtensionDialog/` | 扩展对话框渲染 |
| `components/ExtensionInline/` | 内联扩展组件 |
| `components/CommandPalette/` | 命令面板壳（模糊搜索 + AI 推荐） |
| `components/Dialog/` | 通用模态对话框系统 |
| `components/ui/` | 通用原子组件（按钮、输入框、卡片等） |
| `lib/stream/` | 宿主流（`runChannelStream` / `useEvent`） |
| `lib/hotkey/` | 全局快捷键（注册、冲突检测、分分发） |
| `stores/app.ts` | AppState（顶层全局 Store） |
| `stores/extension*.ts` | 扩展宿主状态（extension、bridge、menu、discovery） |
| `styles/` | 基础样式 + CSS 变量 |
| `theme/` | 主题注册机制（默认主题，可替换扩展） |
| `i18n/` | 国际化（默认 locale，可替换扩展） |
| `layouts/` | 框架布局壳 |

### 前端业务扩展层

以下前端组件属于业务扩展，不属于框架层：

| 组件 | 所属扩展 |
|------|----------|
| `components/AgentTimeline/` | navis-agent-core |
| `components/Chat/` | navis-session |
| `components/Composer/` | navis-session |
| `components/Editor/` | navis-editor |
| `components/Terminal/` | navis-terminal |
| `components/Plan/` | navis-agent-core |
| `components/Settings/` | navis-settings |
| `components/Sidebar/` | navis-session |
| `components/WorkspacePanel/` | navis-project |
| `components/SearchSurface/` | navis-agent-core |
| `components/GlobalSearchPalette/` | navis-editor |
| `stores/agent*.ts` | navis-agent-core |
| `stores/session*.ts` | navis-session |
| `stores/project*.ts` | navis-project |
| `stores/gateway*.ts` | navis-ai-platform |
| `stores/terminal*.ts` | navis-terminal |
| `stores/settings*.ts` | navis-settings |

### 产品主概念

对齐 Claude Code：`Project / Worktree / Session`。

- **Project** — 项目身份、指令和知识
- **Worktree** — 当前会话绑定的真实目录或 Git checkout
- **Session** — 对话、消息和可恢复执行事实

`workspace` 不再作为业务域概念使用，只保留在包管理 workspace、UI placement（如 `rightWorkspace` / `startWorkspace` / `composer.workspace`）或第三方协议术语中。

### 状态管理四层架构

（见 `src/stores/index.ts`）

1. `AppState`（`src/stores/app.ts`）— 顶层全局 Store，`activeSessionId` / `activeProjectId` 为跨模块唯一真实来源
2. 模块 Store — 各扩展自己的 store，使用 SolidJS `createStore`
3. IPC 事件同步层 — `useEvent` / `useStream` 自动同步后端状态
4. 持久化层 — localStorage 存储偏好，Config 模块存储设置

## 扩展开发

### 如何添加新扩展

**后端扩展**：

1. 在 `extensions/` 下创建目录，如 `navis-my-feature/`
2. 创建 `extension.json` 声明扩展元数据、能力、贡献点
3. 在 `ExtensionBackend/src/` 下编写扩展实现代码（通过 re-export 或直接实现）
4. 在扩展的 `mod.rs` 中实现 `BusinessAssembly` trait（如有业务装配需求）
5. 在 `app/` 的扩展注册流程中加载

**前端扩展**：

1. 在 `src/components/` 下创建扩展视图组件
2. 在 `src/stores/` 下创建扩展状态管理
3. 通过 `contributes.views` 在 `HostView` 中注册面板/抽屉/侧栏
4. 通过 `contributes.commands` 注册命令，`contributes.keybindings` 注册快捷键

### 扩展声明 (`extension.json`)

```json
{
  "name": "navis-my-feature",
  "version": "0.1.0",
  "description": "我的功能扩展",
  "capabilities": ["feature.a", "feature.b"],
  "contributes": {
    "views": [{ "id": "my-panel", "type": "panel", "title": "我的面板" }],
    "commands": [{ "id": "my-feature.toggle", "title": "切换我的功能" }],
    "keybindings": [{ "command": "my-feature.toggle", "key": "Ctrl+Shift+M" }]
  }
}
```

### 扩展边界规则

- **框架层**（`src-tauri/src/{kernel,extension,foundation,security,app,ui}/`、`src/{router,components/HostView,components/ui,lib/stream,lib/hotkey,stores/app.ts}/`）**禁止包含业务逻辑**
- **扩展层**（`extensions/`、`src/components/Agent*|Chat|Editor|Terminal|...`）**禁止直接访问其他扩展内部状态**，需通过框架提供的能力缝或 EventBus 通信
- 扩展之间通过 Kernel EventBus 解耦，通过 Foundation 能力缝获取共享服务

## 关键约定

- **日志**: Rust 端全部使用 `tracing` 宏（`debug!`、`info!`、`warn!`、`error!`），不用 `log` 宏
- **IPC 命令**: 所有前端→后端通信通过 Tauri `invoke`，框架层命令定义在 `src-tauri/src/ui/**`，扩展命令由扩展自行注册
- **事件流**: 离散事实统一通过 Kernel EventBus；前端只读投影使用 Tauri event / `useEvent`；高频数据统一通过 `foundation::stream` 的 Tauri Channel
- **模块文档**: 每个 Rust 模块的 `mod.rs` 顶部包含职责边界和子模块说明（中文注释）
- **Store 模式**: 前端 store 使用 `createStore` + 纯函数 action（`set*` 前缀），通过 `stores/index.ts` 统一导出
- **路由**: 当前所有路由视图内联在 `src/router/index.tsx` 单文件中，前端只保留这一层 `Router`
- **无原生装饰**: 窗口使用 `decorations: false`，自定义标题栏
- **设计规范**: 视觉规范见 `DESIGN.md`（颜色、字体、组件 token、布局系统），模块详细设计见 `design/` 目录
- **扩展命名**: 后端扩展目录统一在 `extensions/navis-*/ExtensionBackend/src/` 下；前端扩展组件统一在 `src/components/` 下以扩展名前缀区分

## 外部依赖要点

- **Tauri 2.9**: `@tauri-apps/api` ^2.11, `@tauri-apps/plugin-dialog` ^2.7
- **Rust**: tokio（全异步）、rusqlite（bundled SQLite）、reqwest（流式）、aes-gcm（加密）、sysinfo（健康监控）、notify（文件监听）
- **前端**: SolidJS 1.9、Vite 8.0、TypeScript 6.0（strict）、Tailwind CSS 4.3
