# 35 — 白板容器与万物皆扩展（Whiteboard Container & Everything-as-Extension）

> 当前事实同步（2026-08-18）：Navis Code 业务已迁入 `extensions/navis-code/`，但前端 `src/router/`、`src/layouts/`、部分 `src/stores/` 和 HostView 内置投影仍有产品组合代码。本文中的“容器纯业务无关”是目标态；除非表格明确标为已完成，不得将该目标当作当前事实。

> 状态：设计基线（终局架构）—— C0 地基 + C1 最小骨架**已落地**
> 日期：2026-08-16（v3 修订：新增目录规范；C0/C1 状态标注）
> 日期：2026-08-16（v4 修订：对齐 37 详设——组件轨执行基座 + 目录归一 ExtensionUI/ExtensionBackend + backendServices wire key 修正）
> 日期：2026-08-17（v5 修订：容器层扩展装配统一由 Cordis `Context/Plugin/Service/Inject/Fiber` 承接；WASM 组件是隔离逻辑执行适配器，不是扩展框架）
> 编号：35
> 前置：34-extension-ui-open-architecture.md（扩展 UI 开放架构）、33-extension-gateway-review.md、07-extension.md、36-extension-development.md（扩展开发手册）
> 目标：把 Navis Go 重构为**纯桌面客户端容器（白板）**——领域无关，可承载任何 C 端系统（AI 工具、柜面系统、双录系统……）。所有业务——包括 Agent 引擎——都以扩展形式承载。业务与底层物理剥离，底层不绑定任何业务框架。

---

## 一、架构裁决（定稿）

1. **容器是白板且领域无关**：桌面客户端容器自身**零预置业务**，且**不假设业务领域**。容器不知道"会话""任务""Agent""柜面""双录"是什么。它只提供任何桌面系统都需要的基础设施。
2. **容器可承载任意 C 端系统**：AI 工具、柜面系统、双录系统……都是跑在容器上的**业务扩展**。容器本身不绑定 AI，也不绑定任何垂直领域。
3. **所有业务都是扩展**：AI 业务（Agent 引擎、Gateway、MCP、LSP、会话、任务、项目、知识库……）与任意垂直业务，全部作为扩展分发（含 Navis 官方业务扩展）。
4. **无内置扩展特权**：Navis 官方业务扩展与第三方扩展走同一 manifest / 桥 / 生命周期 / 沙箱契约，一视同仁。
5. **Agent 引擎是 AI 业务扩展**：turn 编排、决策策略、上下文组装、行为模式由 Agent 扩展承载（`ExtensionBackend/logic/*.wasm` 组件轨）。**不属于容器**。
6. **受控操作执行机制是容器能力，操作本身是扩展**：容器提供通用的"受控操作执行"机制（Sandbox 门禁 + 审批 + 审计 + Registry 注册）；具体的操作定义（AI 的 file.edit/terminal/git，柜面的 query/submit，双录的 record/upload）由扩展注册。安全边界不允许扩展接触 Sandbox 本身。
7. **底层不绑定业务框架**：容器 crate 不 import 任何业务模块；业务扩展通过契约原语消费容器能力。容器替换底层框架（窗口库、存储、流实现）不影响业务扩展。
8. **Cordis 是唯一扩展装配底座**：插件/服务装配、依赖注入和生命周期统一由 Cordis `Context/Plugin/Service/Inject/Fiber` 负责；WASM 组件模型只是 `ExtensionUI/` / `ExtensionBackend/` 内隔离逻辑执行的 adapter，不自研扩展壳。

---

## 二、分层结构

```
┌─ 业务扩展层（领域相关、可替换、独立分发、走同一契约）─────────────┐
│  AI 业务包：                                                    │
│    navis-ai-platform   Gateway / MCP / LSP / Skills（AI 平台）   │
│    navis-agent-core    Agent 引擎（WASM 组件轨编排）            │
│    navis-session       会话/消息 UI                              │
│    navis-project / navis-task / navis-knowledge / navis-memory   │
│    navis-terminal / navis-editor / navis-settings                │
│  垂直业务包（任意 C 端系统）：                                    │
│    柜面系统扩展 / 双录系统扩展 / ……                               │
├─ 桌面客户端容器（白板，领域无关，固化）────────────────────────────┘
│  窗口/渲染壳 / Tauri IPC / Tauri Channel / Kernel 四原语         │
│  Cordis Context / Plugin / Service / Inject / Fiber              │
│  受控操作执行机制（Sandbox+审批+审计+Registry）                   │
│  扩展生命周期 / 白名单桥基础设施 / 文件+存储原语                   │
│  进程/worker/WASM 管理 / 网络策略机制（SSRF+CSP） / UI surface 投影 │
└──────────────────────────────────────────────────────────────────┘
```

---

## 二·五、地基 vs 扩展：边界铁律（万物皆扩展的判别标准）【2026-08-17 定稿】

> **一句话**：万物皆扩展 = 我们只提供**最基础的扩展能力（地基）**，类似地基；**除此之外的一切都以扩展形式集成**。判断一段代码属于"地基"还是"扩展"，用下面四条铁律逐一判别。

### 2.5.0 判别铁律（四条）

| # | 铁律 | 判据 | 反例（不属于地基） |
|---|------|------|------|
| F1 | **只含扩展机制本身** | 代码提供"让扩展被装载/装配/依赖/卸载"的机制，不含任何业务决策 | Cordis kernel、loader、manifest 解析、生命周期 fiber |
| F2 | **只含能力缝（seam）定义** | 代码只声明接口（Service Definition），**不含 Provider 实现** | `ctx.llm` / `ctx.tools` / `ctx.sandbox` / `ctx.agents` 的接口定义 |
| F3 | **领域无关平台原语** | 任何桌面系统都需要的通用设施，不假设 AI/柜面/双录 | 窗口、存储、配置、日志、事件总线、流、密钥管理、沙箱门禁机制 |
| F4 | **无特权核心** | 不存在"容器内固定装配、不可替换"的业务能力 | 容器内 `manage` 的 Gateway/MCP/LSP/Skills/Session 等业务 State |

**凡不满足 F1-F3、或满足 F4 反例特征的代码 = 业务扩展**，必须迁出为扩展包，容器不得持有。

### 2.5.1 地基边界（容器保留清单）

| 范畴 | 内容 | 模块 |
|------|------|------|
| 扩展机制 | Cordis Context/Plugin/Service/Inject/Fiber、loader、manifest、installer、生命周期 | `extension/`（不含 provider 实现） |
| 能力缝定义 | capability port / seam 接口（`ctx.llm`、`ctx.tools`、`ctx.sandbox`、`ctx.agents`、`ctx.agentLoop` 等价物） | `extension/lifecycle/cordis.rs`、capability service 常量 |
| 平台原语 | 事件总线、Registry/Pipeline/Policy、存储原语、配置、日志、流通道、Auth 机制、Sandbox 门禁机制 | `kernel/`、`foundation/`、`security/`（机制，非业务约束） |
| UI 表面 | HostView/zone/surface 投影、白名单桥基础设施 | `ui/extension_*`、`ui/host_view.rs`、`ui/tauri_events.rs` |
| 受控操作执行 | `runtime.operation.*` 机制（Registry+审批+审计+Sandbox） | `extension/operation_runtime.rs` |

### 2.5.2 扩展边界（必须迁出清单）

| 业务 | 扩展包 | 现有宿主代码（应迁出） |
|------|--------|------------------------|
| AI 平台（Gateway/MCP/LSP/Skills） | `navis-ai-platform` | `ai/gateway`、`tool/mcp`、`tool/lsp`、`extension/skills` |
| Agent 引擎（编排/决策/上下文组装） | `navis-agent-core` | `ai/agent`、`application/`、`ui/runtime/` |
| 会话/消息/任务/项目/知识/记忆 | `navis-session` / `navis-task` / `navis-project` / `navis-knowledge` / `navis-memory` | `project/`、`ui/sessions.rs`、`ui/tasks/`、`ai/agent/task_manager.rs`、`tool/memory` |
| 工具（文件/终端/Git/剪贴板） | `navis-tools` | `tool/file`、`tool/terminal`、`tool/git`、`tool/clipboard` |
| 编辑器/终端前端/设置 | `navis-editor` / `navis-terminal` / `navis-settings` | 前端 `src/components/Editor`、`Terminal`、`Settings`、`src/stores/settings.ts` |
| 命令面（菜单/消息/网关命令） | 各自扩展包 | `ui/menus.rs`、`ui/messages.rs`、`ui/gateway.rs`、`ui/lsp.rs`、`ui/worktree.rs` |

### 2.5.3 现状审计（2026-08-17 实测）：框架污染清单

以下为容器内**违反铁律的反向依赖**，须在阶段 D 收尾后清除：

| # | 位置 | 反向依赖 | 违反 | 修复 |
|---|------|----------|------|------|
| 0a | `extension/operation_runtime.rs`（原 `ui/operation_runtime.rs`） | ~~`ui` 域 + `tool::mcp::MCP`~~ | F4 | ✅ **已完成**：下沉 `extension` 域 + MCP 依赖改 `McpOperationPort` trait |
| 0b | `extension/lifecycle/families.rs` | `tool::backend::BackendProcessManager`、`tool::lsp::registry::{LanguageSource, LSPServerConfig}` | F4 | 改经 capability port 惰性解析 |
| 0c | `extension/provider_validation.rs` | `ai::gateway::provider::profile::ProviderAuthProfile` | F2/F4 | provider 贡献契约下沉为扩展域通用 DTO；ai/gateway 只作实现者 |
| 0d | `security/sandbox/constraint.rs` | `tool::mcp::protocol::ToolMetadata`、`tool::agent::{hooks,pipeline}` | F4 | 约束参数化：业务约束由扩展注册，sandbox 只留机制 |
| 0e | `foundation/storage/session_store.rs` | `project::session::{checkpoint,history,snapshot}` | F4 | 业务 schema 迁出 foundation → 业务扩展存储适配器 |

### 2.5.4 对齐 deepseek-harness（38 §一/§二 落定）

deepseek-harness 的判据与本方案一致：**"There is no privileged core to patch: you extend dsh by mounting a plugin beside the others"**。其地基 = Cordis kernel + seam 定义 + `dsh-base` bundle 之前的平台设施；其扩展 = model adapter（`ctx.llm`）、tools（`ctx.tools`）、sandbox backend（`ctx.sandbox`）、session log（`ctx.sessions`）、agent loop（`ctx.agentLoop`）等全部插件。

Navis Go 对齐结论：**Gateway/MCP/LSP/Skills/Session/AgentLoop 都必须是可替换的扩展能力，而非容器内固定装配的业务 State**。`app/business.rs::AiIdeBusiness` 是当前最大的 F4 违例（11 个业务 State 容器内装配），C3/C4 迁出后归零。

---

## 三、目录规范（扩展存放与前后端分域）

### 3.0 原则

- **扩展是独立分发的单元**：每个扩展一个目录，自包含 manifest + 前端 UI + 逻辑组件（可选 native 逃生舱），可整体复制/打包。
- **扩展分两部分**：
  - **前端扩展**：界面 + 逻辑组件（跑在容器 WebView iframe 轨 + 容器内组件轨）。
  - **后端扩展**：后端逻辑组件（`ExtensionBackend/logic/*.wasm`，容器内 wasmtime 执行）与 native 逃生舱（`ExtensionBackend/native/*`，独立进程，协议通信）两种形态。
  - 两部分可在同一扩展包内，也可只有其一。
- **前后端分域**：扩展内 `ExtensionUI/`（全部前端代码）、`ExtensionBackend/`（全部后端扩展点代码）；宿主代码也前后端分域。
- **扩展统一根目录**：仓库内 `extensions/` 为扩展统一存放根。

### 3.1 仓库目录总览

```
Navis Go/
├── extensions/                        # ★ 扩展统一根目录（可分发、独立单元）
│   ├── navis-demo/                    #   示例扩展（目录名 = extension id）
│   │   ├── extension.json             #   manifest（唯一入口）
│   │   ├── ExtensionUI/               #   ★ 前端扩展面：全部前端代码
│   │   │   ├── index.html             #   html:sandbox 视图入口
│   │   │   ├── assets/                #   静态资源
│   │   │   ├── scripts/               #   前端逻辑组件（.wasm，容器内执行）
│   │   │   │   └── app.component.wasm
│   │   │   └── locales/               #   i18n 资源
│   │   └── ExtensionBackend/           #   ★ 后端扩展面：全部后端扩展点代码
│   │       ├── logic/                 #   后端逻辑组件（.wasm，容器内执行）
│   │       │   └── worker.component.wasm
│   │       └── native/                #   native 逃生舱（协议子进程）
│   │           └── navis-demo-server[.exe]
│   ├── navis-ai-platform/             #   （规划）AI 平台服务扩展
│   └── ...                            #   其他业务扩展（可替换）
│
├── src/                               # ★ 宿主前端（容器 UI 壳）
│   ├── components/
│   │   ├── HostView/                  #   三轨渲染（host:panel / html:sandbox / worker）
│   │   ├── ExtensionDialog/           #   扩展弹框
│   │   └── ...
│   ├── stores/
│   │   ├── bridge.ts                  #   白名单桥（宿主侧 dispatcher）
│   │   ├── extension.ts               #   扩展状态
│   │   ├── extension-points.ts        #   扩展点投影
│   │   └── ...
│   └── lib/
│       ├── extension-ui.ts            #   UiExtensionView 契约类型
│       └── ...
│
├── src-tauri/src/                     # ★ 宿主后端（容器 Rust）
│   ├── app/                           #   容器壳（app/mod.rs）+ 业务装配（business.rs）
│   ├── kernel/                        #   Kernel 四原语
│   ├── foundation/                    #   平台原语（storage/stream/config/logger）
│   ├── security/                      #   Sandbox / 权限 / 审计
│   ├── extension/                     #   扩展系统（lifecycle/loader/models/store）
│   ├── ui/                            #   IPC 命令（含 extension_*、operation_runtime、stream）
│   ├── extension/                     #   扩展系统（业务由扩展承载）
│   ├── foundation/                    #   平台基础能力
│   ├── security/                      #   安全边界
│   └── lib.rs
```

### 3.2 扩展内部目录约定

| 目录 | 用途 | 说明 |
|------|------|------|
| `extension.json` | manifest | 唯一元数据入口，容器扫描该文件识别扩展 |
| `ExtensionUI/` | 前端扩展面：全部前端代码 | `html:sandbox` 视图 entry（如 `ExtensionUI/index.html`）、assets、前端逻辑组件 `ExtensionUI/scripts/*.wasm`、locales；`ui/` 别名已废弃 |
| `ExtensionBackend/logic/` | 后端逻辑组件（`.wasm`，容器内执行） | `components[kind:logic]` 的 entry 位于此；wasmtime 容器内执行，非独立进程 |
| `ExtensionBackend/native/` | native 逃生舱（协议子进程，可选） | `contributes.backendServices[]` 声明的可执行文件；容器 spawn，经协议通信；仅需 OS 能力/自身是 server 时使用 |
| `data/` | 扩展静态数据（可选） | 非 UI 数据，如模板、字典 |

> **命名约定**：扩展目录名必须等于 manifest `id`（容器启动时校验，不符则跳过）。

### 3.3 前后端分域映射

| 分域 | 目录 | 归属 | 运行位置 |
|------|------|------|---------|
| 前端扩展 UI | `extensions/<product>/<extension-id>/ExtensionUI/`（index.html、assets、locales） | 扩展作者 | 容器 WebView iframe（html:sandbox） |
| 前端逻辑组件 | `extensions/<product>/<extension-id>/ExtensionUI/scripts/*.wasm` | 扩展作者 | 容器内组件轨（wasmtime） |
| 后端逻辑组件 | `extensions/<product>/<extension-id>/ExtensionBackend/logic/*.wasm` | 扩展作者 | 容器内组件轨（wasmtime） |
| 后端 native 逃生舱 | `extensions/<product>/<extension-id>/ExtensionBackend/native/` | 扩展作者 | 独立进程（容器 spawn，协议通信） |
| 宿主前端 | `src/` | 容器 | 容器 WebView |
| 宿主后端 | `src-tauri/src/`（app/kernel/foundation/security/extension/ui） | 容器 | 容器 Rust |
| Navis Code 业务扩展 | `extensions/navis-code/<extension-id>/` | 已完成主要物理迁移；前端产品壳仍在迁移 | — |

### 3.4 后端扩展（ExtensionBackend：逻辑组件 + native 逃生舱）

后端扩展承载**纯后端逻辑**。按 37 详设分两种形态：

| 形态 | 目录 | 运行 | 适用 |
|------|------|------|------|
| **逻辑组件**（首选） | `ExtensionBackend/logic/*.wasm` | 容器内 wasmtime（`components[kind:logic]`） | 可编译为 WASM 的逻辑（数据处理/编排） |
| **native 逃生舱** | `ExtensionBackend/native/*` | 独立进程（`backendServices`，协议通信） | 需 OS 能力（USB/打印机/GUI）/自身是 server 的后端 |

**native 逃生舱 manifest**：`contributes.backendServices` 声明：

```json
{
  "contributes": {
    "backendServices": [
      {
        "id": "core-server",
        "entry": "ExtensionBackend/native/navis-demo-server",
        "transport": "stdio",
        "protocol": "jsonrpc"
      }
    ]
  }
}
```

> **wire key 铁律**：`contributes.backendServices`（camelCase）。`backend_services`（snake_case）会被 serde **静默忽略**——扩展启用零报错但服务不注册（C0-1 已修）。

**逻辑组件**：可编译为 WASM 的后端逻辑走 `components[kind:logic]`，entry 位于 `ExtensionBackend/logic/*.wasm`，**容器内 wasmtime 执行**（非独立进程），经 host function 门禁（能力声明→接口授予，未声明 fail-closed）——详见 37 详设。

**生命周期**：逻辑组件随 enable 实例化、disable/卸载时回收（组件轨，enable 原子性校验）；native 逃生舱 enable 时 spawn 进程（`autostart: true` 立即拉起），disable/卸载时 kill；进程崩溃由容器记录并可选重启（受配额约束）。

**通信**：native 逃生舱经协议与容器通信——复用 `transport_adapters` 契约（stdio/SSE/WebSocket/REST，34 文档已有），不新造协议。后端暴露的能力经容器注册为 tool/命令/服务，前端扩展经 `route.call` 或受控操作调用。

**安全**：
- 组件轨：wasmtime 内存/trap 隔离；host function 是唯一出站通道，门禁在容器，未声明能力调用 fail-closed + 审计。
- native 进程不接触容器内存/State；只能经协议调用容器暴露的能力，需容器 Sandbox **进程门禁** + 审计。
- 后端扩展声明的协议/端口受网络策略约束（SSRF/CSP）。

**用途**：AI 服务、柜面核算、双录处理等任何后端逻辑；轻量逻辑走组件轨，需 OS 能力/自身是 server 的走 native 逃生舱。

### 3.5 运行时扩展位置

- 运行时扩展从 `<app_data>/extensions/{id}/` 装载（`app/mod.rs` 启动扫描）。
- 仓库 `extensions/` 是**分发源**；安装时复制到 `<app_data>/extensions/{id}/`（或开发模式下直接指向仓库源目录）。
- 扩展持久化数据：`<app_data>/extensions/{id}/storage/`（存储 facade 已落地）。

---

## 四、容器能力契约（白板宿主暴露的原语）

### 4.1 平台原语（领域无关）

| 原语 | 模块 | 说明 | 现状 |
|------|------|------|------|
| 窗口/渲染 | `app/`、前端路由 | 多窗口、装饰、生命周期 | 已具 |
| Tauri IPC | `ui/**` | 命令边界 | 已具 |
| 事件总线 | `kernel::EventBus` | 离散事实 | 已具 |
| Cordis 扩展装配/服务容器 | `extension/context.rs` | Context / Plugin / Service / Inject / Fiber，统一扩展生命周期装配 | **接线中（D1）**：`HostExtensionContext` 已创建未接线，无 `set_service`/`install_extension` 调用点 |
| Registry/Pipeline/Policy | `kernel/` | 注册、编排、门禁 | 已具 |
| Sandbox/权限/审计 | `security/sandbox/` | 文件/网络/进程门禁 | 已具 |
| 文件抽象 | `tool/file/path_manager.rs` | 路径归一化、worktree 读取 | 已具 |
| 存储原语 | `foundation/storage/` | KV/文件/加密原语（无业务 schema） | 已具 |
| 流通道 | `foundation/stream/` | Tauri Channel 直通 + 订阅广播 | 已具 |
| 进程/worker/WASM | `app/`（manage）+ 容器组件运行时 | 承载 worker 胶水轨与 WASM 组件轨 | 已具/演进中 |

### 4.2 受控操作执行机制（容器通用原语，非 AI 特定）

容器暴露通用的"受控操作执行"原语，**不绑定任何领域**：

| 原语 | 命令 | 说明 |
|------|------|------|
| 执行受控操作 | `runtime.operation.execute` | 执行一个已注册的 Operation，过 Sandbox 门禁 + 审批 + 审计，返回结果 |
| 操作注册 | `runtime.operation.register` | 扩展注册自己的 Operation 定义（操作名/参数 schema/权限等级） |
| 审批 | `runtime.operation.approve/deny` | 审批流 |
| 操作列表 | `runtime.operation.list` | 列出已注册操作 |

- **机制在容器**：Sandbox 检查、审批状态机、审计、Registry 注册——容器持有。
- **操作在扩展**：具体操作（读文件、终端、Git、柜面查询、录制上传）由扩展声明并调用 `runtime.operation.execute`，操作的真实执行要么是容器内建的通用操作（如 `file.read`），要么由扩展在 WASM 组件轨或 native 逃生舱实现（通过 host function/桥调用容器文件/网络/进程原语）。
- **安全铁律**：`runtime.operation.execute` 的结果必须经容器校验；扩展不能绕过 Sandbox/审批/审计。

### 4.3 桥命令（领域无关白名单）

容器桥 `__NAVIS__.invoke` 只保留通用原语：

| 命令 | 说明 |
|------|------|
| `file.read` | 读取文件（容器内建通用操作） |
| `context.getSession` / `context.getActiveProject` | 上下文快照（容器投影，业务扩展填充） |
| `extensions.query` | 扩展发现 |
| `route.call` | 跨扩展调用（双端授权） |
| `storage.*` | 扩展 KV 存储 |
| `network.fetch` | 网络策略代理 |
| `runtime.operation.execute/list/register` | 受控操作执行机制 |

**AI 特定命令不进入容器桥**：`agent.*`、`gateway.*` 等由 AI 业务扩展自己经 `runtime.operation.execute` 或自建命令实现。

### 4.4 容器 surface（UI 挂载点）

| surface | 说明 |
|---------|------|
| zone（内置 + `{extId}:{zoneId}`） | 视图挂载 |
| menu target（内置 + `{extId}:{target}`） | 菜单/菜单栏 |
| toolbar / statusbar / inline | 工具条/状态栏/内嵌 |
| dialog | 弹框 |
| settings section | 设置分区 |
| command palette entry | 命令入口 |

容器只提供这些 surface 的**布局与投影**，不提供 surface 内默认内容。

---

## 五、AI 业务扩展化设计

### 5.1 AI 平台服务（navis-ai-platform）

Gateway、MCP、LSP、Skills 是 **AI 领域服务**，作为 AI 业务扩展承载：
- 现有 `gateway.adapters/providers/middlewares`、`mcp_servers`、`transport_adapters`、`languages`、`skills` 契约已存在，属 AI 平台扩展的贡献面。
- **渐进**：当前这些服务在容器内 manage（`app/mod.rs`），先保留为容器可选装配，逐步迁移到 `navis-ai-platform` 扩展包。迁移前，它们在容器内仍是"平台服务"（34 号文档语义），但**契约上归 AI 业务**。

### 5.2 Agent 引擎（navis-agent-core）

Agent 引擎是 AI 业务扩展，WASM 组件轨承载编排（详见 37 详设）：

```
业务扩展 navis-agent-core（组件轨）
  ├─ turn 编排（决策策略：选操作/结束/重试）
  ├─ 提示词/上下文组装
  ├─ 行为模式（Code/Cowork/自定义）
  └─ 经 __NAVIS__.invoke('runtime.operation.execute', ...) 调容器受控操作

容器（受控操作执行机制）
  ├─ runtime.operation.execute（Sandbox + 审批 + 审计）
  ├─ runtime.operation.register（AI 扩展注册 file.edit/terminal/git 等操作）
  └─ 存储 / 事件 / 上下文快照原语
```

**职责切分**：
- **容器**：受控操作执行机制、审批、审计、流推送、存储、事件、上下文快照——通用边界。
- **Agent 扩展**：决定"调哪个操作、何时结束、怎么组装提示词"——AI 智能，可替换。

### 5.3 Agent 扩展 manifest 形态

```json
{
  "id": "navis.agent-core",
  "contributes": {
    "capabilities": {
      "invoke": ["runtime.operation.execute", "runtime.operation.list",
                  "context.getSession", "storage.*", "network.fetch"],
      "provides": ["agent", "operation:file", "operation:terminal", "operation:git"]
    },
    "components": [
      { "id": "agent-orchestrator",
        "entry": "ExtensionBackend/logic/agent-orchestrator.wasm",
        "kind": "logic",
        "runOn": ["activation", "message"] }
    ],
    "roles": [{ "id": "agent", "system_prompt": "..." }],
    "work_modes": [{ "id": "code", "name": "Code", "available_operations": ["file.*", "terminal.*"] }]
  }
}
```

### 5.4 前端接入（会话 UI 扩展）

- `chat-turn-stream.ts` 从"直接 invoke ui_stream_session_message"改为"经 Agent 扩展触发"：
  - 会话 UI 扩展把用户消息交给 Agent 扩展（组件轨 `message` 接口 → `route.call` 或事件）。
  - Agent 扩展调用 `runtime.operation.execute`（容器命令）→ 容器执行 + 流推送回前端 Channel。
  - 前端仍用 `runChannelStream` 消费同一 Channel 格式，**前端渲染层改动最小**。

---

## 六、要做的改动清单（分阶段）

### 阶段 C0：固化容器边界（地基，先行）——【已落地】

> 状态：C0-1~C0-6 已完成（2026-08-16/17）。

| # | 改动 | 文件 | 说明 | 状态 |
|---|------|------|------|------|
| C0-1 | 容器不 import 业务模块 | `src-tauri/src/app/mod.rs`、`app/business.rs` | 业务 State 装配抽离到 `app/business.rs`；容器壳只装配平台原语 + 扩展生命周期。**注意**：业务装配目前仍在容器壳内被调用（`business::assemble`），"容器启动不加载任何业务代码"（C5-5）待 C3 业务迁出后达成 | **部分完成** |
| C0-2 | 冻结容器能力契约 | `design/35` §四 | 领域无关原语清单成为 ABI | **已完成** |
| C0-3 | **受控操作执行机制** | `src-tauri/src/ui/operation_runtime.rs` | `runtime.operation.execute/list/register` + OperationRegistry，复用现有 Sandbox/审批/审计 | **已完成**（14 测试） |
| C0-4 | 桥白名单收口为领域无关 | `ui/extension_bridge.rs` | 桥保留通用命令（file/context/storage/network/operation），无 AI 特定命令 | **已完成** |
| C0-5 | 存储 facade + 独立扩展目录 | `extension/storage.rs` | 三态 + 独立扩展目录 | **已完成** |
| C0-6 | foundation/storage 收敛为纯落盘原语 | `foundation/storage/` | B-P0-3 | **已完成**（边界文档化，db.rs 18 张业务表逐表标注"业务 schema 渐进迁出"） |

### 阶段 C1：最小扩展骨架（验证容器机制，含一个 AI 业务扩展样板）——【已落地】

> 状态：C1-1~C1-5 已完成（2026-08-16）。

| # | 改动 | 文件 | 说明 | 状态 |
|---|------|------|------|------|
| C1-1 | 最小受控操作执行 | `ui/operation_runtime.rs` | `runtime.operation.execute` + `register` + `list` | **已完成** |
| C1-2 | 最小业务扩展包 | `extensions/navis-demo/` | manifest + `ExtensionUI/index.html` + `ExtensionUI/scripts/main.js` | **已完成** |
| C1-3 | 桥放行 operation.* | `extension_bridge.rs` | `runtime.operation.*` 进白名单 | **已完成** |
| C1-4 | 全链路测试 | `ui/operation_runtime.rs` 测试 | 扩展注册操作 → 执行 → 过门禁 → 结果回扩展（含 Builtin file.read 端到端） | **已完成**（14 测试） |
| C1-5 | 验收 | `cargo test` + 手工 | 样板扩展全链路 | **已完成** |

### 阶段 D：Cordis 装配接线（唯一装配路径，地基收尾）——【已落地】

> 状态：D1~D4 已完成，D5 试运行（2026-08-17）。**背景**：`extension/context.rs` 的 `HostExtensionContext`（Cordis root Context + fiber per extension）已在容器壳创建（`app/mod.rs`），但**从未接线**——无 `set_service` 调用点、无 `install_extension` 调用点；`ExtensionLifecycle` 仍是独立于 Cordis 的 family handler 模型（**双轨**）。本阶段把 Cordis 提升为**唯一装配底座**（对齐 37 §一.1 与 38 §2.1）：capability port 注册为 Cordis service，扩展生命周期经 Cordis fiber 装配，WASM 组件轨经 Cordis service 访问容器。

| # | 改动 | 说明 | 状态 |
|---|------|------|------|
| D1 | Cordis 唯一装配接线 | `HostExtensionContext` 成为容器扩展装配的唯一入口：现有 capability port（mcp/lsp/gateway/provider_validation/event_subscription/backend_manager/component_registry/policy_engine）注册为 Cordis service；`ExtensionLifecycle` 的 enable/disable 经 Cordis fiber（`apply_extension_fiber` + `take_extension_fiber`），family rollback 语义并入 fiber dispose（删除 `ExtensionEnableRollback`） | **已完成**（D1a-D1e 全量迁移，extension:: 300 测试绿） |
| D2 | WASM 组件轨接线 | `ComponentRegistry` 执行链路落地：loader 装载 `components` 声明 → `validate_component_run_on`（activation/message 白名单）→ wasmtime 实例化 `ComponentRegistry.load` → `ActiveComponent` 持久化 → `handle_message` 路由（fail-closed） | **已完成**（component 13 + lifecycle 82 测试绿；缺口：`ui/extension_router.rs` route.call 最终接线） |
| D3 | 事件订阅落地 | `event_subscriptions` 从"声明索引"升级为真实 Kernel EventBus 订阅（38 §2.2）：瀑布事件（`agent/pre-step`、`tool/pre-execute/execute/post-execute`、`agent/turn-stopping`）常量 + `KernelEventSubscriptionAdapter::subscribe_declared/unsubscribe_all`（批级 fail-closed + 逆序撤销） | **已完成**（16 测试绿） |
| D4 | agentLoop seam | `agentLoop` 能力缝（38 §2.5）：`AgentLoopPort` trait + `AgentLoopContext`/`AgentLoopOutcome` + `DefaultAgentLoopPort` 占位 + capability service 注册（`SERVICE_AGENT_LOOP`） | **已完成**（占位，C4 迁入真实编排） |
| D5 | 业务扩展接线试点 | `navis-task` 扩展包声明任务面板视图（`host:panel`）→ HostView `BUILTIN_VIEW_PROJECTIONS` 投影复用 `BackgroundTasksPanel`；`navis-settings` 全量试点（代码物理迁入扩展包，含 C3-7） | **已完成**（前端 D5 接线；settings 试点见 C3-7） |

**验收（D）**：`cargo test extension::lifecycle` 全绿；一个后端扩展（含 `components` + `event_subscriptions`）经 Cordis 装配启用、触发事件订阅、禁用后副作用回收。**已达成**：全量 `cargo test` 2143 passed（仅 3 预存在 knowledge 环境失败）。

### 阶段 C2：AI 平台服务包（navis-ai-platform）与接线

> 状态：待做。**裁决**：C2-1/C2-2 的贡献（`middlewares`/`transport_adapters`）属于声明式 capability contribution，不是扩展 JS；由 Cordis plugin/service 注入后经宿主 capability port 接入 `GatewayMiddleware`/`TransportAdapter`。可编译逻辑走 `ExtensionBackend/logic/*.wasm` 组件轨，必须贴近宿主 Rust trait 的适配器由宿主实现 capability port，不做 JS→Rust 平行桥。当前保持 fail-closed（`families.rs unsupported_runtime_handlers` 保留条目），不做空壳接线。

| # | 改动 | 说明 | 状态 |
|---|------|------|------|
| C2-1 | `gateway.middlewares` 接线 | 从 fail-closed 变接线（34 B-P0-1）；需 worker 桥 JS→Rust | **保留 fail-closed** |
| C2-2 | `mcp.transport_adapters` 接线 | 从 fail-closed 变接线（34 B-P0-2）；同上 | **保留 fail-closed** |
| C2-3 | `editor.languages` 完整 | 移除 unsupported 条目（34 B-P2-8）；editor 域承接 | 待做 |
| C2-4 | AI 平台服务收口到 navis-ai-platform 概念 | 契约上归 AI 业务（35 §5.1），实现仍可在容器内（渐进） | 概念已定 |

### 阶段 C3：AI 业务扩展化迁移

> 状态：C3-1 前端接线已完成（D5）；C3-7 已确定全量试点（前后端物理迁出，见 Step1）。**本阶段迁移形态**：业务代码**物理迁入** `extensions/<product>/<extension-id>/ExtensionBackend/`（Cargo workspace crate，依赖 navis-core 框架层）与 `ExtensionUI/`（前端 monorepo 导入），容器经扩展清单 + seam 装配，不再是"容器内 manage"。见 §2.5 边界铁律。

| # | 业务 | 扩展包 | 复用现有 | 状态 |
|---|------|--------|----------|------|
| C3-1 | 任务/背景面板 | `navis-task` | task-projection、BackgroundTasksPanel | 前端接线✅（D5）；后端迁出待做 |
| C3-2 | 项目/工作树 | `navis-project` | project、worktree store | 待做 |
| C3-3 | 知识库 | `navis-knowledge` | knowledge store | 待做 |
| C3-4 | 会话/消息 UI | `navis-session` | session-tree、composer、chat-messages | 待做 |
| C3-5 | 编辑器外壳 | `navis-editor` | CodeMirror 封装 | 待做 |
| C3-6 | 终端前端 | `navis-terminal` | xterm.js 封装 | 待做 |
| C3-7 | 设置/命令面板入口 | `navis-settings` | Settings、CommandPalette | **全量试点**（后端 `ui/settings.rs` → `ExtensionBackend/`；前端 `stores/settings.ts` + `components/Settings/*` → `ExtensionUI/`） |
| C3-8 | 记忆 | `navis-memory` | memory store | 待做 |

### 阶段 C4：Agent 引擎正式迁移（navis-agent-core）

| # | 改动 | 说明 |
|---|------|------|
| C4-1 | `run_agent_tool_loop` 编排逻辑迁入 navis-agent-core | Rust 编排 → `ExtensionBackend/logic/*.wasm` 组件轨；容器保留受控操作执行/审批/流/持久化 |
| C4-2 | 上下文组装迁入扩展 | 提示词策略从 Rust 迁 JS（`context.*` 快照 + `storage.*`） |
| C4-3 | 会话 UI 扩展接管消息流 | `chat-turn-stream.ts` 改经 Agent 扩展触发 |
| C4-4 | 旧 `ui_stream_session_message` 命令下线或仅作容器示例 | 移除或保留 |

### 阶段 C5：验收与性能

| # | 验收 |
|---|------|
| C5-1 | `cargo check` + `cargo test` 全绿（除预存在 3 个 knowledge 环境失败） |
| C5-2 | `npm run build` + `npm run test:menus` 通过 |
| C5-3 | 会话消息流实时不节流：AI 扩展编排 → 容器受控操作 → Channel 回前端，无感知差异 |
| C5-4 | 卸载一个业务扩展后该 surface 消失、其余不受影响 |
| C5-5 | 容器启动不加载任何业务代码（白板空壳可启动） |
| C5-6 | 一个第三方扩展能替换 `navis-session` 的会话 UI |
| C5-7 | **领域无关验证**：一个非 AI 业务扩展（如演示柜面/双录操作）在容器上可用，且不依赖 AI 扩展 |

---

## 七、约束与铁律

1. **不绑框架且不绑领域**：容器 crate 的依赖只含平台依赖，不含业务模块，也不假设业务领域。
2. **操作不进容器**：具体 Operation（AI 工具、柜面操作、录制操作）由扩展定义；容器只提供执行机制。
3. **fail-closed**：未声明能力的桥调用、未授权跨扩展调用、未声明 network 的 fetch，一律拒绝 + 审计。
4. **不节流 Agent 流**：实时性优先；性能靠按需订阅 + iframe 生命周期 + 背压。
5. **多轨渲染不变**：iframe 轨（自由 UI）+ worker 胶水轨 + WASM 组件轨（逻辑）+ host:panel 轨（声明式）共用白名单桥。
6. **内置扩展无特权**：Navis 官方业务扩展与第三方同契约。

---

## 八、风险与缓解

| 风险 | 缓解 |
|------|------|
| Agent 编排迁 WASM 组件轨引入调用开销 | 编排只经 host function 原语命令，受控操作执行/流推送在容器内直连；高频流走 Channel 直通不节流 |
| 受控操作泛化导致权限模型复杂 | Operation 定义含权限等级 + Sandbox 校验，Registry 统一管理；fail-closed |
| 业务扩展 iframe 堆叠内存 | 34 §2.3.1 生命周期（suspended）+ 配额 |
| 容器与业务耦合历史遗留 | C0-1 依赖审计先行，逐步剥离 |
| AI 平台服务迁移量大 | 渐进：先容器内可选装配（契约归 AI 业务），后迁移扩展包 |
| 替换底层框架（窗口/存储/流） | 容器能力契约成为稳定 ABI，业务只依赖契约 |

---

## 九、文档关系

- 前置：`34`（三轨渲染/桥/zone/生命周期）、`33`（B-P0-1/2/3 接线依据）、`07`（manifest/生命周期）、`36`（扩展开发手册——开发需严格遵循，不一致必须反哺 36）。
- 被依赖：`09-file`、`24-dialog`、`18-context-manager`、`25-notification`、`26-editor`、`27-hotkey` 的扩展化改造均以本方案 §五 为纲。
- 本方案定稿后，`34` §13 的"阶段 1-10"视为被本方案 C0-C5 取代。
- **变更记录**：
  - v2（2026-08-16）：容器边界由"含 agent.runtime"修订为**领域无关**；`agent.runtime` 归入 AI 业务扩展（§五）；新增"受控操作执行机制"（§4.2）；AI 平台服务（Gateway/MCP/LSP）归 AI 业务层（§5.1）。
  - v3（2026-08-16）：新增**目录规范**（§三：`extensions/` 统一根、`ExtensionUI/` 前端扩展目录、前后端分域映射）；`entry` 约定对齐 `scripts/` 与 `ExtensionUI/`；C0/C1 状态标注为已落地；C2-1/2 明确保留 fail-closed 裁决。
  - v4（2026-08-16）：对齐 37 详设（C0 落地）——后端扩展区分**逻辑组件**（`ExtensionBackend/logic/*.wasm`，容器内 wasmtime，`components[kind:logic]`）与 **native 逃生舱**（`ExtensionBackend/native/*`，`backendServices`，独立进程协议通信）；目录归一 `ExtensionUI/`（全部前端代码）与 `ExtensionBackend/`（全部后端扩展点），**废弃 `ui/` 别名与顶层 `scripts/`**；§3.4 manifest `backendServices` wire key 修正（snake_case 被 serde 静默忽略，C0-1 已修）。
  - v5（2026-08-17）：容器层扩展装配统一由 Cordis `Context/Plugin/Service/Inject/Fiber` 承接；Agent 编排从 worker 轨改为 `ExtensionBackend/logic/*.wasm` 组件轨；WASM 是隔离逻辑执行 adapter，不自研扩展壳。
  - v6（2026-08-17）：C0-1 进一步落地——容器壳不再直接调用单体 business::assemble，改为经 BusinessAssembly / builtin_business_assemblies 注册边界装配内建业务扩展；NAVIS_WHITEBOARD=1 可跳过业务装配白板启动；ExtensionLifecycle 支持 new_without_skills（无业务 Skills 主机时 fail-closed）。
  - v7（2026-08-17）：新增**阶段 D：Cordis 装配接线**——事实核查发现 `HostExtensionContext` 已创建未接线（死代码）、`ExtensionLifecycle` 与 Cordis 双轨、WASM 组件轨 `ComponentRegistry` 未接执行链路；D1-D3 把 Cordis 提升为唯一装配底座（capability port→Cordis service、lifecycle→Cordis fiber、组件轨→host function 门禁、事件订阅落地），对齐 37/38。




