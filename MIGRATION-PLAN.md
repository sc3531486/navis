# Navis Go 业务迁移执行计划

> 日期：2026-08-17
> 目标：把所有业务代码从框架层物理迁移到 extensions/ 下的对应扩展目录，实现万物皆扩展。

---

## 一、现状

### 已完成（C0-C1 + D1-D4）
- extension/ 已创建 11 个扩展骨架（navis-ai-platform, navis-agent-core, navis-session 等）
- Cordis 装配接线已落地（capability port → Cordis service、WASM 组件轨、事件订阅）

### 未完成（业务代码仍在框架层）

**Rust 后端 src-tauri/src/ 中需迁出的业务模块：**

| 模块 | 目标扩展 | 说明 |
|------|---------|------|
| ai/agent/ | navis-agent-core | Agent 决策引擎 |
| ai/context/ | navis-agent-core | 上下文组装 |
| ai/gateway/ | navis-ai-platform | 模型网关 |
| tool/mcp/ | navis-ai-platform | MCP 工具协议引擎 |
| tool/lsp/ | navis-ai-platform | LSP 语言服务协议 |
| tool/terminal/ | navis-terminal | 终端管理 |
| tool/file/ | navis-editor | 文件系统操作 |
| tool/git/ | navis-editor | 版本控制 |
| tool/clipboard/ | navis-editor | 剪贴板 |
| tool/memory/ | navis-memory | Agent 记忆工具 |
| tool/agent/ | navis-agent-core | Agent 工具运行链 |
| tool/backend/ | navis-editor | 后端扩展进程管理 |
| project/session/ | navis-session | 会话管理 |
| project/catalog/ | navis-project | 项目目录 |
| project/knowledge/ | navis-knowledge | 知识库 RAG |
| project/memory/ | navis-memory | 项目记忆 |
| application/ | navis-agent-core | Agent 运行时控制 |
| ui/ 业务命令 | 各对应扩展 | IPC 命令定义 |

**前端 src/ 中需迁出的业务组件：**

| 组件/Store | 目标扩展 |
|-----------|---------|
| components/Chat/ | navis-session |
| components/Composer/ | navis-agent-core |
| components/Editor/ | navis-editor |
| components/Terminal/ | navis-terminal |
| components/Settings/ | navis-settings |
| components/Plan/ | navis-task |
| components/AgentTimeline/ | navis-agent-core |
| components/WorkspacePanel/ | navis-task |
| stores/chat* | navis-session |
| stores/composer* | navis-agent-core |
| stores/agent* | navis-agent-core |
| stores/gateway* | navis-ai-platform |
| stores/session* | navis-session |
| stores/project* | navis-project |

---

## 二、执行策略

### 物理迁移 + re-export 保编译

1. 在 extensions/*/ExtensionBackend/src/ 创建 Rust 模块
2. 先建 re-export 桥（旧路径 → 新位置），保持编译通过
3. 物理搬迁源文件到扩展目录
4. 框架层（app/business.rs、ui/mod.rs、lib.rs）精简为纯框架

### 并行分工（4 个 Agent）

| Agent | 范围 |
|-------|------|
| Agent-A | Rust AI 域：ai/ + tool/agent/ + application/ → navis-agent-core + navis-ai-platform |
| Agent-B | Rust Tool 域：tool/mcp + lsp + terminal + file + git + clipboard + memory + backend → 各工具扩展 |
| Agent-C | Rust Project 域 + UI 命令 + 框架清理 |
| Agent-D | 前端组件 + stores 迁移到 ExtensionUI |

---

## 三、框架层最终结构（src-tauri/src/）

```
src-tauri/src/
├── lib.rs              # 只声明框架域：kernel, extension, foundation, security, app, ui
├── kernel/             # 不变 — 四原语
├── extension/          # 不变 — 扩展系统本体
├── foundation/         # 不变 — 能力缝
├── security/           # 不变 — 安全边界
├── app/                # Tauri bootstrap（移除业务装配）
└── ui/                 # 框架宿主面（只保留 extension_bridge/network/router/storage/stream/host_view/events/dto/permissions）
```

## 四、前端框架层最终结构（src/）

```
src/
├── router/             # 框架路由（HostView + ExtensionDialog 入口）
├── components/HostView/ # 扩展视图投影
├── components/ExtensionDialog/
├── components/ExtensionInline/
├── components/CommandPalette/
├── components/Dialog/
├── components/ui/      # 通用原子组件
├── lib/stream/         # 宿主流
├── lib/hotkey/         # 快捷键
├── stores/app.ts       # AppState
├── stores/extension*   # 扩展宿主状态
├── styles/             # 基础样式
├── theme/              # 主题
├── i18n/               # 国际化
└── layouts/            # 框架布局
```
