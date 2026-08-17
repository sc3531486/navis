# 34 - 扩展 UI 开放架构 + 全系统改造基线 详细设计

> 模块编号：34 | 层级：UI 层 × Extension 层（cross-cutting）× 全系统改造基线
> 依赖：07-Extension、Cordis（前端宿主 @cordisjs/core；后端 cordis-rs）、22-UI-Framework、24-Dialog、02-IPC、02b-Stream、06-Sandbox（门禁+网络策略）、04-Storage
> 被依赖：25-Notification（渠道扩展）、27-Hotkey（keybinding 复用）、26-Editor（编辑器域贡献）、33-extension-gateway-review（Gateway 接线）
> 状态：总改造基线（2026-08-16）
> 关联复核：33-extension-gateway-review.md（Extension 全链路验收基线）
> 2026-08-15 版：扩展 UI 开放架构（UI 层）。
> 2026-08-16 升格：全系统三线审计（前端/后端/设计文档）结果并入本文档（§15），成为「万物皆扩展 + 高性能」总改造基线；状态从「设计审核稿」升级为「总改造基线」，后续所有涉及扩展性/性能的改造以本章节为唯一裁决点。
> 2026-08-16 补齐：扩展持久化存储（§2.5）/ 网络策略（§2.6）/ 扩展发现（§2.7）/ 数据通道选择原则（§2.8）/ iframe 生命周期（§2.3.1）/ host:panel 动态数据绑定（阶段 9）/ i18n 移交边界（§10.4、风险 14）/ 全系统改造清单与性能依赖引入（§15）
> 2026-08-17 裁决：扩展装配与生命周期统一落在 Cordis `Context/Plugin/Service/Inject/Fiber`；Cordis 是插件/服务宿主，不是渲染器或 JS 引擎。固定目录为 `ExtensionUI/`（前端扩展点）与 `ExtensionBackend/`（后端扩展点），`ui/` 别名废弃。

---

## 一、模块概述

### 1.1 定位

本设计解决 Navis Go 的"万物皆扩展"在 UI 层的落地，并承接 2026-08-16 全系统审计的改造基线（§15）：扩展通过 manifest **声明菜单（任意位置）与界面（HTML / Vue 等任意前端框架）**，界面嵌入当前客户端界面（内嵌区域或独立弹框），样式完全自定义，交互经**白名单桥**授权访问宿主能力；后端 Gateway 中间件 / MCP 传输适配器 / 扩展 KV / 编辑器 / 通知等被审计为 fail-closed 的贡献，统一以本设计为总裁决点逐步转为承接或显式拒绝。

核心约束：**不新开窗口、不脱离桌面端**。渲染层使用宿主内嵌 WebView（与主 UI 同构），能力层全部由 Rust 掌控。扩展代码全部运行在本地桌面进程内，离线可用，能力一律经 Rust 白名单授权。

Cordis 是前后端扩展的**组合与服务生命周期底座**：manifest 是插件元数据，loader 把 `ExtensionUI/` / `ExtensionBackend/` 中的扩展点装载为 Cordis plugin/service；UI 投影 Registry、白名单桥、iframe/worker/WASM 组件都只是 Cordis 服务的消费面，不再自建平行扩展壳。

### 1.2 职责边界

```
负责：
├── Cordis 扩展装配与服务生命周期（Context/Plugin/Service/Inject/Fiber/effect）
├── Zone 开放命名空间（内置 zone + 扩展自定义 zone + 泛型 slot）
├── Renderer 多轨渲染（iframe 轨 + 浏览器 worker 胶水轨 + WASM 组件轨 + host:panel 轨）
├── 白名单 IPC 桥（iframe / Worker 内 window.__NAVIS__ API）
├── 动作契约泛化（命令/菜单/快捷键统一触发 OpenView | ToggleView | OpenDialog | RunScript | SendMessage）
├── 扩展间通信（命名空间路由：ext:A 调用 ext:B 的视图/命令/事件）
├── 独立 ExtensionDialog 弹框体系（多弹框并行、可拖拽、可缩放）
├── 菜单任意位置承接（开放 MenuTarget + 顶部应用菜单栏 + when/group 求值）
├── UI 扩展点承接（toolbar_items / statusbar_items / inline_extensions / configuration / layout_overrides / styles / triggers）
├── Gateway middleware / MCP transport_adapter 贡献接线（§10.2、§15 P0-1/P0-2，从 reject 转承接）
├── 扩展 KV 存储（§2.5 ExtensionStorage facade，扩展域自持三态，foundation/storage 只做落盘原语）
├── 性能依赖引入裁决（§15.4：ripgrep / @tanstack/solid-virtual / dashmap / moka，遇瓶颈即替换自造方案）
└── 与 security/sandbox 的桥接（OperationRequest{actor:"extension:{id}"}）

不负责：
├── 扩展生命周期管理 → 07-Extension
├── 宿主内建视图/面板内容 → 22-UI-Framework 各 View 组件
├── 宿主业务对话框（确认/输入/选择/Agent 确认） → 24-Dialog
├── 新增 Kernel 原语 → 禁止，Kernel 只保留 Registry/Pipeline/EventBus/Policy
├── 扩展工具执行/审批/Turn Timeline → Tool / Agent 域
├── 编辑器域贡献（themes / editor_languages / editor_extensions / styles 的 CodeMirror 承接） → 26-Editor
├── 通知渠道运行时 → 25-Notification
├── 系统托盘运行时 → 窗口域（tray_items 需承接或显式拒绝，禁止静默忽略）
├── LSP 命令命名统一 / Gateway client 复用 / async 阻塞 I/O → 后端改造清单（§15.2 P1/P2）
└── 上下文压缩语义 / i18n 结构 / hotkey 模型 → 各业务域文档修复（§15.3）
```

### 1.3 设计原则

1. **开放命名空间**：`placement` → `zone`、`MenuTarget` 从枚举白名单放开为 `{extId}:{zoneId}` 命名空间。扩展可定义新位置、可加进原位置。
2. **动作契约泛化**：`BuiltinAction` 从 `OpenView/ToggleView` 扩展为动作族（`OpenView/ToggleView/OpenDialog/RunScript/SendMessage`）。命令/菜单/快捷键不再只是"开视图"，而是统一触发入口，动作目标可指向宿主能力、扩展自身脚本或另一个扩展。
3. **宿主布局最终裁决权**：扩展 zone 的锚定语义（parent+position 或泛型 slot）只描述意图，宿主布局算法负责裁决冲突、拒绝非法位置。
4. **严格沙箱 + 白名单桥**：扩展界面 iframe 保持 `sandbox="allow-scripts"`（无 `allow-same-origin`），跨源 postMessage + origin 校验，扩展只能调 manifest 声明的白名单能力。
5. **多轨并行**：iframe 轨承担 HTML/Vue UI；浏览器 Web Worker 仅承担前端胶水；WASM 组件轨承担重逻辑/后端逻辑；三面共用同一白名单桥。
6. **安全复用，不新造**：桥请求构造 `OperationRequest{actor:"extension:{id}"}` 走现有 `security::sandbox` 门禁（白名单 / 权限分级 / 审批 / 审计）。
7. **渲染层浏览器化 ≠ 脱离桌面**：主 UI 与扩展 UI 都在宿主 WebView 内，窗口、菜单栏、托盘、文件/终端/Git/AI 全部由 Rust 提供，扩展永远无法绕过 Rust 直碰系统能力。
8. **fail-closed 是姿态、不是终点**：未承接的 contributes 必须显式拒绝（enable 失败并说明），禁止"声明成功但运行态静默忽略"（当前 `tray_items` 违反此原则）。每一类 UI 扩展点必须：声明模型 → 承接判定 → 运行时投影，三态齐全。

### 1.4 与审计的关系（2026-08-15 补）

本设计基于一次全量贡献面承接审计（详见 §十）。审计结论：当前"万物皆扩展"只兑现了**后端能力面（MCP/Tools/Gateway/LSP/Skills/Hooks）与 UI 声明式投影（views/commands/menus/keybindings）**，而 **9 项 UI 扩展点全部 fail-closed**（toolbar/statusbar/inline/layout/configuration/styles/triggers/themes/editor_*），另有 **tray_items 静默忽略**。本设计以"UI 扩展面全量承接"为补齐目标，并明确编辑器域与 AI 域移交 26/25/33 号处理。

---

## 二、架构设计

### 2.1 整体分层

```
[扩展 manifest]  ← 声明层（07-Extension 校验后进入运行时）
  contributes:
    menus:      [{ id, label, target, command, when, group, icon, risk }]
    commands:   [{ id, label, action }]   ← action 泛化：OpenView | ToggleView | OpenDialog | RunScript | SendMessage
    views:      [{ id, name, entry, zone, renderer, config, allowClose }]
    scripts:    [{ id, entry: "worker.js" }]                    ← 新增：逻辑轨
    capabilities:[{ invoke: ["..."], events: ["..."], read: ["..."] }]  ← 新增：白名单
    toolbar_items / statusbar_items / inline_extensions        ← 新增承接：UI 扩展点
    configuration / layout_overrides / triggers                ← 新增承接：声明式 UI 配置

[宿主运行时]  ← Cordis Context / Plugin / Service / Inject / Fiber + 三张 UI 投影 Registry（宿主前端投影，不进 Kernel）
  ZoneRegistry     内置 zone: rightWorkspace/chatAside/bottomDrawer/settingsSection/dialog
                  泛型 slot: sidePanel/drawer/popover/topBar/footer
                  扩展 zone: {extId}:{zoneId}（anchor: parent+position 或 slot）
  RendererRegistry 内置: host:panel | html:sandbox(+IPC桥) | runtime:worker(新)
  MenuRegistry     内置 target + 扩展 target {extId}:{targetId} + 顶部菜单栏
  ActionRegistry   命令动作族（OpenView/ToggleView/OpenDialog/RunScript/SendMessage）+ 目标解析
  ExtRouter        扩展间通信路由（ext:A → ext:B 视图/命令/事件）

[渲染管线]  ← Cordis 装配后的能力消费面
  iframe 轨  : 扩展静态资源(html/vue产物) + 注入 __NAVIS__ 桥 → postMessage → 宿主桥 → 白名单 → Tauri
  worker 轨   : 扩展 entry.js 跑在 Web Worker（仅前端胶水），同一 __NAVIS__ 桥（postMessage 透传）
  component 轨: 扩展逻辑组件 .wasm 跑在容器内 wasmtime（37 主路径；前端逻辑组件位于 ExtensionUI/scripts/）
  host 轨     : 宿主 SolidJS 渲染声明式面板（保留现状）

[安全边界]  ← 复用现有 sandbox
  每个桥请求 → OperationRequest{ actor:"extension:{id}", session_id, worktree_path }
  → sandbox 门禁（permission.rs CheckResult: allowed/denied/needs_confirm）
  → 审计
  跨扩展调用 → 目标扩展白名单 + 调用方权限声明，双端校验
```

### 2.2 Zone 锚定语义（定稿）

```
内置 zone（宿主锚点）  : rightWorkspace / chatAside / bottomDrawer / settingsSection / dialog
泛型 slot（宿主预置）   : sidePanel / drawer / popover / topBar / footer

扩展 zone 声明二选一：
  { "id": "my-ext:custom-panel", "zone": { "anchor": {
      "type": "parent", "parent": "rightWorkspace", "position": "below", "size": "40%" } } }
  { "id": "my-ext:quick-panel", "zone": { "anchor": { "type": "slot", "slot": "sidePanel" } } }
```

- **parent + position**：扩展 zone 挂在宿主已知 zone 上，用 `position`（`left`/`right`/`above`/`below`）+ `size`（百分比或像素）描述相对几何。宿主据此插入动态布局树。
- **泛型 slot**：宿主预置有限形态的通用容器，扩展 zone 声明放入哪个 slot，形态由宿主强约束。
- 校验规则：
  - `parent` 必须引用已声明且可用的 zone（内置 zone 恒可用；扩展 zone 须已被宿主接受）。
  - `slot` 必须命中宿主预置 slot 列表。
  - 不满足 → manifest 校验 fail-closed，view 不进入菜单 / Command Palette / zone 投影。
- 布局树：`rightWorkspace` 现有一维 `RightWorkspaceColumn`（src/stores/app.ts:77-81）演进为嵌套 split/stack 布局树；扩展 zone 的 parent+position 作为树上的新节点插入，宿主算法计算最终几何并裁决冲突。

### 2.3 多轨渲染

| 轨 | 承载内容 | 运行时 | 桥 |
|----|----------|--------|-----|
| iframe 轨 | 扩展 HTML/Vue 界面（编译产物） | 宿主 WebView 内 iframe | `__NAVIS__`（postMessage） |
| worker 轨 | 前端胶水脚本（贴近 UI 的轻逻辑） | Web Worker（浏览器统一运行时） | `__NAVIS__`（postMessage 透传） |
| host 轨 | 宿主声明式面板（现状保留） | 宿主 SolidJS | 宿主内联 |

- Cordis 不提供渲染器、JS 引擎、Web Worker 或 wasmtime 组件运行时；它只负责 plugin/service 装配、依赖注入与生命周期。
- **不引入原生 JS 引擎**（deno_core / rquickjs / boa）：二进制 +1~50MB、C/构建依赖、能力边界与 iframe 轨割裂，均不划算。
- 浏览器统一运行时轨 = Web Worker，仅保留给纯前端胶水；重逻辑、跨平台、需隔离逻辑一律走 37 定义的 WASM 组件轨（`components`），不作为第二执行模型。
- 无头长期后台任务、数据处理与后端逻辑统一评估 WASM 组件轨，而不是无限扩展 Web Worker。

### 2.3.1 iframe 生命周期（效果/内存定稿）

每个 iframe 是独立 JS 上下文 + 渲染树（内存重），必须显式管理生命周期，否则扩展视图/弹框堆积会拖垮主线程。状态机：

```
declared（manifest 已声明）
  → mounted（可见，创建 iframe + 注入桥 + 订阅其声明的 stream/事件）
  → suspended（不可见，卸载 iframe / 冻结 JS 上下文，保留桥元数据）
  → disposed（关闭/禁用/卸载，销毁 iframe + 释放订阅 + 清 ephemeral）

规则：
  1. 懒加载：首次可见才创建 iframe（`declared → mounted` 由可见性触发）。
  2. 不可见即挂起：zone 视图切走 / 弹框失焦非激活 → `mounted → suspended`（默认）
     （host:panel 轨与 iframe 轨同步挂起，保持一致性）。
  3. 可配置常驻：扩展可在 manifest 声明 `keep_alive: true`（如音频/长连接场景），
     但计费到该扩展配额，防滥用。
  4. 弹框降级：ExtensionDialog 非激活弹框 → suspended（渲染暂停），激活时恢复；
     避免"8 个弹框 = 8 个活跃 iframe"的内存/GPU 峰值。
```

### 2.4 扩展间通信（ExtRouter，定稿）

审计发现当前**没有扩展 A → 扩展 B 的调用/消息通道**（`contributions.rs:119-133` 甚至校验命令 view_id 必须在同一扩展内）。"万物皆扩展"必须支持扩展协作。设计：

```
调用方 ext:A 需要：
  capabilities.extension_calls: [{ target: "ext:B", actions: ["view.open", "command.execute", "event.emit"] }]

被调用方 ext:B 需要：
  extension_exports: { views: ["b-view"], commands: ["b-cmd"] }   // 显式导出的跨扩展面

调用路径：
  ext:A 命令/脚本 → ActionRegistry 解析 action.target = "ext:B" 
  → ExtRouter: 校验 ext:A 的 extension_calls 白名单 ∩ ext:B 的 extension_exports
  → 双端校验通过 → 转发（打开 ext:B 视图 / 执行 ext:B 命令 / 订阅 ext:B 事件）
  → 任一端未授权 → 拒绝 + 审计
```

- **双端授权**：调用方声明想用什么、被调用方声明愿意暴露什么，两端都过白名单（扩展是安全边界，扩展间权限不能由单方单方面授予）。
- **事件流**：`ext:B` 发布事件（`ext:B.event.foo`），`ext:A` 在 `capabilities.events` 中声明订阅 `ext:B.event.foo`；宿主 EventBus 命名空间化事件名，路由时校验订阅授权。
- **命令与视图**：`action.SendMessage{ target:"ext:B", message }` / `OpenView{ view:"ext:B:b-view" }` 均走 ExtRouter 双端校验。非授权目标解析失败时 fail-closed（命令可渲染性 gate 拦截）。
- **不建平行总线**：跨扩展消息仍走 Kernel EventBus + 命名空间事件名，不引入独立消息总线原语。

### 2.5 扩展持久化存储（ExtensionStorage，定稿）

审计发现扩展**只有配置、没有状态存储**——扩展无法保存运行时状态（上次选择、缓存、偏好），"万物皆扩展"在状态持久化维度是瘸腿的。设计（对标 VS Code `context.globalState` / `workspaceState`）：

```
存储三态：
  global    : 跨项目、跨 worktree 的扩展状态（{extension_id}/global/）
  worktree  : 按当前 worktree 隔离的扩展状态（{extension_id}/worktree/{worktree_id}/）
  ephemeral : 仅内存、进程生命周期内（extension_id 内存 Map，退出即清）

位置：扩展数据目录（<app_data>/extensions/{extension_id}/），由 Rust 统一管理，
      不开放任意文件系统路径。
```

- 数据模型：`{ key: String, value: Value }`（JSON 值），`ExtensionStorageState` 枚举三态 + 命名空间（按 extension_id）。
- API（前端桥）：`__NAVIS__.storage.get(key)` / `set(key, value)` / `delete(key)` / `clear()`，`storageState: "global" | "worktree" | "ephemeral"` 参数。
- 安全：存储按 extension_id 命名空间隔离，扩展只能读写自己的 key 空间；值过 Rust 校验（大小上限、JSON 深度）；存储不落明文 secret（secret 仍走 05-Auth）。
- 生命周期：扩展禁用/卸载时，ephemeral 立即清空；global/worktree 保留（卸载时确认是否清除），禁用仅停止访问。

### 2.6 扩展网络策略（ExtensionNetPolicy，定稿）

审计发现扩展 UI 能否发网络请求**完全未定义**（iframe 是 `allow-scripts` 无 `allow-same-origin`，fetch 行为无约定）。设计：

```
三种网络模式（capabilities.network 声明，fail-closed 默认拒绝）：
  none            : 完全禁止网络（默认）。iframe/worker 内 fetch/XHR/WebSocket 被宿主拦截。
  allowlist       : 仅允许列出的域名（{ host, allow_subdomains, protocols } 白名单）。
  proxy           : 全部网络请求经宿主代理（走 06-Sandbox 网络策略 + 审计），扩展不直连。

实现：
  allowlist : 前端桥拦截 iframe/worker 内 fetch，与 manifest 域名白名单比对；命中则转发宿主 fetch，
              否则拒绝。非桥接资源（img/script src 等被动加载）由 CSP 控制。
  proxy     : 扩展请求经 ui_extension_network_proxy → sandbox 网络策略判定 → 代理发出。
```

- **默认 fail-closed**：不声明 `capabilities.network` → `none`，扩展任何网络请求被拒并审计。
- **被动资源（CSP）**：iframe 注入 CSP（`img-src` / `font-src` / `connect-src` 按 network 模式生成），防扩展通过 img/script 标签绕过主动网络拦截。
- **与 06-Sandbox 关系**：网络策略复用 06-Sandbox 网络策略层（06-sandbox.md §七），扩展声明映射到 sandbox 网络规则，不新造平行网络门禁。
- **代理安全**：proxy 模式全部请求经宿主，走 OperationRequest 审计链，扩展无法直连外网。

### 2.7 扩展发现机制（ExtensionDiscovery，定稿）

审计发现扩展间是**静态耦合**——调用方必须在 manifest 里写死被调用方 id，无法查询"当前环境有没有提供某能力的扩展"。设计：

```
运行时能力发现（只读）：
  __NAVIS__.extensions.query({ capability: "view" | "command" | "script" | "network" | "storage", 
                               provides: "file-index" | ... })
  → 返回 [ { extension_id, name, version, exports: [...] , enabled: true } ]

数据源：宿主 ExtensionStore 只读投影（ui_list_extensions 扩展），不新建 Registry。
查询范围：仅返回已启用扩展；未启用扩展对调用方不可见（fail-closed）。
```

- **声明式能力标签**：扩展在 manifest 声明 `capabilities.provides: ["file-index", "git-viz"]`（能力标签），发现机制按标签检索。
- **运行时变化**：扩展启用/禁用/卸载时，`extension.registry.changed` 事件通知调用方刷新发现结果。
- **与 ExtRouter 关系**：Discovery 负责"发现谁可以提供"，ExtRouter 负责"授权后调用"。调用前仍需 ExtRouter 双端白名单（发现不绕过授权）。

### 2.8 数据通道选择原则（效果优先定稿）

**效果（实时性、完整性、交互流畅度）是第一约束，性能是手段、不是牺牲效果的借口。** 节流（ThrottledEmitter）只允许用于"高频且逐条无感知价值"的辅助数据（终端滚动输出、编译日志），**绝不用于用户直接感知的实时数据**（Agent 动作、任务状态、Gateway 流式内容）——那些必须逐条实时到达，50ms 延迟就是体验损失。

| 数据形态 | 示例 | 通道 | 投递语义 |
|---------|------|------|---------|
| 实时感知流 | Agent 动作（AgentTimelinePart）、任务状态、Gateway 流式内容 | **Stream 通道**（`extension.stream.subscribeSource`，02b-stream §3.8） | **逐条实时**，不节流不合并；效果优先 |
| 高频低价值流 | 终端滚动输出、编译/测试日志 | Stream 通道 | **允许节流**（ThrottledEmitter 窗口合并，02b-stream §4）——逐条无感知价值 |
| 低频离散事实 | zone 可用性变化、扩展生命周期、配置变更、审计拒绝 | **事件桥**（Kernel EventBus → Tauri event → 桥） | 逐条；低频无压力 |

- **性能保障 = 架构手段，不是节流**：
  1. **按需订阅**：扩展不订阅则该流零开销（Stream 是推模型，无订阅者不产生转发）——这是第一性能手段，从源头消灭无效投递。
  2. **生命周期**：iframe 不可见即 suspended（§2.3.1），不活跃的接收端不消耗资源——避免"8 个弹框 8 个活跃消费端"。
  3. **背压与投递率**：扩展接收端处理不过来时走**丢弃 + 计数**（扩展收到 `{dropped: n}` 可提示降级），而不是延迟投递——保实时性，不缓存堆积。
  4. **选择性投递**：同一流只投给**已订阅且可见**的实例（订阅注册 + 可见性过滤），而非广播全部。
- **禁止路径**：实时感知数据不得用事件桥逐条转发（每 token 一次 fan-out 到 N 个 iframe/worker → IPC 爆炸、掉帧）；也不得为性能对 Agent 动作节流。
- **通道选型校验**：桥在加载期校验扩展订阅的 channel pattern——实时感知 pattern（agent/stream）必须走 stream 逐条语义，事件桥只允许低频离散 pattern，防误用（fail-closed）。

---

## 三、数据模型

### 3.1 manifest 新增字段（07-Extension `ExtensionContributes` 扩展）

```rust
// 现有字段保持（views/menus/commands/keybindings/...），新增：
struct ExtensionContributes {
    // ── 既有 ──
    views: Option<Vec<ViewRegistration>>,
    menus: Option<Vec<MenuRegistration>>,
    commands: Option<Vec<CommandRegistration>>,
    keybindings: Option<Vec<KeybindingRegistration>>,
    // ── 新增：34 ──
    scripts: Option<Vec<ScriptRegistration>>,        // 前端胶水轨（Web Worker；重逻辑走 components/WASM）
    capabilities: Option<CapabilityDeclaration>,      // 白名单授权声明
    zones: Option<Vec<ZoneRegistration>>,             // 扩展自定义 zone（含锚定语义）
    toolbar_items: Option<Vec<ToolbarItemRegistration>>,   // ← 从 fail-closed 转为承接
    statusbar_items: Option<Vec<StatusBarItemRegistration>>, // ← 同上
    inline_extensions: Option<Vec<InlineExtensionRegistration>>, // ← 同上
    configuration: Option<Value>,                     // JSON Schema → 设置面板渲染
    layout_overrides: Option<Vec<LayoutOverride>>,    // ← 同上
    triggers: Option<Vec<TriggerRegistration>>,       // ← 同上
    extension_exports: Option<ExtensionExports>,      // 跨扩展面显式导出（定稿 §2.4）
    storage: Option<StorageDeclaration>,              // 状态存储声明（定稿 §2.5）
    network: Option<NetworkPolicy>,                   // 网络模式声明（定稿 §2.6，默认 none）
    provides: Option<Vec<String>>,                    // 能力标签，供发现机制检索（定稿 §2.7）
    i18n: Option<Vec<I18nResource>>,                  // 扩展本地化资源（移交 28-i18n 承接）
}
```

### 3.2 ViewRegistration 变更

```rust
struct ViewRegistration {
    id: String,
    name: String,
    icon: Option<String>,
    // placement: String        → 弃用
    zone: String,                // 内置 zone 名 或 "{extId}:{zoneId}"
    renderer: RendererKind,      // host:panel | html:sandbox | runtime:worker
    entry: Option<String>,       // html:sandbox/runtime:worker 必填；host:panel 禁止
    config: Option<Value>,
    activation_events: Vec<String>,
    allow_close: bool,           // 默认 true
    default_visible: bool,
}
```

### 3.3 MenuRegistration 变更

```rust
struct MenuRegistration {
    id: String,
    label: String,
    // target: MenuTarget        → 放开为开放字符串
    target: String,              // 内置 target（Tools/...）或 "{extId}:{targetId}"
    command: String,             // 引用已声明 command
    group: Option<String>,       // 分组（前端落地渲染）
    when: Option<String>,        // 条件表达式（前端落地求值）
    icon: Option<String>,
    shortcut: Option<String>,
    risk: Option<MenuRisk>,
}
```

### 3.4 新增类型

```rust
// 白名单授权声明
struct CapabilityDeclaration {
    invoke: Vec<String>,          // 允许调用的宿主 IPC 命令白名单（需已注册）
    events: Vec<String>,          // 允许订阅的宿主 UI event/stream 只读投影 pattern
    read: Vec<String>,            // 允许读取的上下文（session/project/context 快照）
    extension_calls: Vec<ExtensionCall>, // 跨扩展调用白名单（定稿 §2.4）
    provides: Vec<String>,        // 能力标签，供发现机制检索（定稿 §2.7）
    network: NetworkPolicy,       // 网络模式（定稿 §2.6，缺省 None）
}
struct ExtensionCall { target: String, actions: Vec<String> } // actions: view.open/command.execute/event.emit/event.subscribe

// 跨扩展面显式导出
struct ExtensionExports { views: Vec<String>, commands: Vec<String> }  // 仅被显式列出的才可被其他扩展调用

// 命令动作族（替代单一 OpenView/ToggleView）
enum CommandAction {
    OpenView    { view: String },          // 本扩展视图或 "ext:B:b-view"（需 ExtRouter 授权）
    ToggleView  { view: String },
    OpenDialog  { view: String, size: Option<String>, position: Option<String>, modal: Option<bool> },
    RunScript   { script: String, payload: Option<Value> },   // 触发扩展自身 worker 逻辑
    SendMessage { target: String, message: Value },           // 跨扩展消息（ext:B 或宿主命名空间）
}

// 逻辑轨脚本
struct ScriptRegistration {
    id: String,
    entry: String,                // 相对路径，须位于 ExtensionUI/scripts/ 下
    run_on: Option<Vec<RunTrigger>>, // 可选：activation / view-open / worker-spawn / message
}

// 扩展自定义 zone
struct ZoneRegistration {
    id: String,                   // "{extId}:{zoneId}"（宿主投影时命名空间化）
    name: String,
    anchor: ZoneAnchor,
}
enum ZoneAnchor {
    Parent { parent: String, position: Position, size: String },  // parent+position
    Slot { slot: String },                                         // 泛型 slot
}
enum Position { Left, Right, Above, Below }

// 状态存储声明（定稿 §2.5）
struct StorageDeclaration { scopes: Vec<StorageScope> } // global / worktree / ephemeral
enum StorageScope { Global, Worktree, Ephemeral }

// 网络策略声明（定稿 §2.6，缺省 none）
enum NetworkPolicy {
    None,
    Allowlist { hosts: Vec<NetworkHost> },   // { host, allow_subdomains, protocols }
    Proxy,                                    // 全部经宿主代理 + sandbox 网络策略
}

// 能力标签（定稿 §2.7）
// provides: ["file-index", "git-viz", ...] 供 __NAVIS__.extensions.query 检索

// i18n 资源（移交 28-i18n）
struct I18nResource { lang: String, entry: String } // 相对路径，须位于 ExtensionUI/locales/ 下

// UI 扩展点承接（从 fail-closed 转承接）
struct ToolbarItemRegistration { id, label, icon, command, position: ToolbarPosition, group, when }
struct StatusBarItemRegistration { id, label, icon, position: StatusBarPosition, command, priority, when }
struct InlineExtensionRegistration { id, name, view: String, mount: InlineMount }  // chatAside/editorView/terminal
struct LayoutOverride { zone: String, size: Option<String>, visible: Option<bool>, order: Option<u32> }
struct TriggerRegistration { id, name, entry: String }   // 声明式触发器（复用 ScriptRegistration 逻辑轨）

// 菜单开放命名空间化后的 target
// 内置: "Tools" | "InputPlus" | "ChatTitle" | "RightPanel" | "Gateway"
//       | "WorktreeContext" | "SessionContext" | "GroupContext"
//       | "Menubar.File" | "Menubar.Edit" | "Menubar.View" | "Menubar.Help" | "Menubar.Tools"
// 扩展: "{extId}:{targetId}"
```

### 3.5 DTO（Rust → 前端，src-tauri/src/ui/dto.rs）

```rust
struct UiZone {
    id: String,                    // 内置 zone 名 或 "{extId}:{zoneId}"
    name: String,
    kind: ZoneKind,                // Builtin | Extension
    anchor: Option<UiZoneAnchor>,  // Extension zone 才有
    available: bool,               // 当前宿主是否已承接（可渲染）
}
struct UiCapabilities { invoke: Vec<String>, events: Vec<String>, read: Vec<String>, extension_calls: Vec<String> }
struct UiExtensionInfo { extension_id: String, name: String, version: String, provides: Vec<String>, enabled: bool }
struct UiStorageEntry { key: String, value: Value, scope: StorageScope }
// UiMenuRegistration 增加 group/when 透传；UiExtensionViewDescriptor 增加 capabilities
// 新增 UiCommandAction（动作族序列化）、UiToolbarItem / UiStatusBarItem / UiInlineExtension（UI 扩展点投影）
// 新增 UiExtensionInfo（发现机制返回）、UiStorageEntry（存储读写返回）
```

---

## 四、接口定义

### 4.1 新增 IPC 命令（Rust）

| 命令 | 位置 | 职责 |
|------|------|------|
| `ui_list_zones` | `src-tauri/src/ui/host_view.rs`（或新增 `ui/zones.rs`） | 返回全部可用 zone：内置 zone + 已启用扩展声明的扩展 zone（含 anchor、available 状态） |
| `ui_extension_bridge_invoke` | 新增 `src-tauri/src/ui/extension_bridge.rs` | iframe/Worker 白名单桥入口。参数 `{extension_id, cmd, args}`。流程：校验扩展 Enabled → 查 `capabilities.invoke` 白名单 → 构造 `OperationRequest{actor:"extension:{id}"}` 过 sandbox 门禁 → 派发到目标命令。未声明能力 → 拒绝 + 审计 |
| `ui_list_extension_scripts` | `src-tauri/src/ui/extensions.rs`（或并入现有） | 返回已启用扩展的 scripts 投影（`{extension_id, script_id, entry, run_on}`） |
| `ui_extension_route_call` | 新增 `src-tauri/src/ui/ext_router.rs` | 跨扩展调用入口（ExtRouter）：`{from, target, action, payload}` → 双端白名单校验（from.extension_calls ∩ target.extension_exports）→ 派发。任一端未授权 → 拒绝 + 审计 |
| `ui_get_extension_config` | `src-tauri/src/ui/extensions.rs` | 返回指定扩展的 `configuration` JSON Schema（供设置面板渲染表单） |
| `ui_set_extension_config` | 同上 | 保存扩展配置值 → 落 Config 域 + 触发 `extension.config.updated` 事件（扩展经桥订阅） |
| `ui_list_toolbar_items` / `ui_list_statusbar_items` / `ui_list_inline_extensions` | `src-tauri/src/ui/extensions.rs` | UI 扩展点投影（复用 extension_store 已注册声明，从 fail-closed 转可用） |
| `ui_extension_storage_get` / `ui_extension_storage_set` / `ui_extension_storage_delete` / `ui_extension_storage_clear` | 新增 `src-tauri/src/ui/extension_storage.rs` | 扩展 KV 存储（§2.5）：按 extension_id 命名空间 + scope 读写，Rust 落盘扩展数据目录；set 过 Rust 校验（大小/JSON 深度） |
| `ui_extension_network_proxy` | 新增 `src-tauri/src/ui/extension_network.rs` | proxy 模式的网络代理入口：`{extension_id, request}` → 校验 network 模式（allowlist 域名命中 / proxy）→ 复用 06-Sandbox 网络策略 → 代理发出 → 回传响应 |
| `ui_extension_discovery_query` | `src-tauri/src/ui/extensions.rs`（或新增 `ui/discovery.rs`） | 运行时能力发现（§2.7）：`{capability, provides?}` → 返回已启用扩展的只读投影（含 provides 匹配） |
| `ui_list_extensions`（扩展） | `src-tauri/src/ui/extensions.rs` | 增加 `provides` 字段透传（Discovery 数据源） |

### 4.2 变更的 IPC 命令

| 命令 | 变更 |
|------|------|
| `ui_list_menus` | 透传任意 target（含 `Menubar.*` 与 `{extId}:{targetId}`）；透传 `when`/`group` 字段；`action` 可渲染性 gate 保持（非白名单 renderer/zone 的 view 不输出）；动作族 `OpenDialog/RunScript/SendMessage` 的目标解析结果一并投影 |
| `ui_list_extension_views` | DTO 增加 `zone`（替代 placement）、`capabilities` |
| `ui_list_extension_commands` | 透传动作族；`canDispatchCommandAction`（extension-commands.ts:27）扩展为动作族分发 |

### 4.2.1 动作契约分发（ActionRegistry）

```
CommandAction  → 前端执行器（src/stores/menu-actions.ts + extension-commands.ts 扩展）
  OpenView/ToggleView → 既有 hostView 逻辑（app.ts:406-408 hostViewInstanceId 命名空间化）
  OpenDialog          → ExtensionDialog.open(view, size, position, modal)
  RunScript           → 前端胶水 worker 轨 spawn + postMessage({ type:"invoke", script, payload })
  SendMessage         → ui_extension_route_call（ExtRouter 双端校验）
命令可渲染性 gate：动作目标解析失败（view 未授权 / script 未声明 / target 未授权）→ 命令不进入菜单/Command Palette。
```

### 4.3 桥协议（前端 ↔ iframe/Worker）

```
扩展内：window.__NAVIS__.invoke(cmd, args) → Promise<T>
       window.__NAVIS__.listen(pattern, cb) → 取消订阅函数
       window.__NAVIS__.getContext()        → Promise<ContextSnapshot>
       window.__NAVIS__.getConfig()         → Promise<ConfigSnapshot>   // 读取扩展配置
       window.__NAVIS__.setConfig(patch)    → Promise<void>             // 更新扩展配置
       window.__NAVIS__.call(target, action, payload) → Promise<T>      // 跨扩展调用（ExtRouter）
       window.__NAVIS__.storage.get/set/delete/clear(scope?) → Promise  // 扩展 KV 存储（§2.5）
       window.__NAVIS__.fetch(url, init)    → Promise<Response>         // 经网络策略代理（§2.6，none 时拒绝）
       window.__NAVIS__.extensions.query({capability, provides?}) → Promise<UiExtensionInfo[]> // 发现（§2.7）

宿主桥（HtmlSandboxRenderer / Worker host）：
  接收 postMessage { bridgeId, type: "invoke"|"listen"|"context"|"config"|"call"|"storage"|"fetch"|"discovery"|"stream", cmd, args }
  → 校验 event.origin 为扩展资源来源（asset 协议白名单）
  → 转发 ui_extension_bridge_invoke（或事件订阅 / ui_get/set_extension_config / ui_extension_route_call
     / ui_extension_storage_* / ui_extension_network_proxy / ui_extension_discovery_query
     / extension.stream.subscribeSource 订阅 Agent/任务等高频流）
  → 回传 { bridgeId, ok, data | error }
```

- 请求-响应：`bridgeId` + Promise 映射 + 超时（默认 30s，可配置）。
- 来源校验：仅接受属于当前扩展实例的 iframe/Worker 来源；伪造来源直接丢弃。
- 事件订阅：只读投影，进 `kernel::EventBus` → Tauri event → 桥 → 扩展；扩展不可反写。
- **Stream 直通（高频，§2.8）**：`type:"stream"` 订阅走 `extension.stream.subscribeSource`（02b-stream §3.8）。宿主桥把 Tauri Channel 的 `onmessage` **批量合并转发**（同窗口合并成数组、分帧投递），不逐条 postMessage。扩展不订阅则该流零开销（推模型）。
- **通道选型校验**：桥在加载期校验扩展订阅的 channel pattern——命中高频流（agent/terminal/task）必须走 stream，事件桥只允许低频离散 pattern，防误用（fail-closed）。

### 4.4 前端桥注入

- `HtmlSandboxRenderer.tsx` 改造：iframe 保持严格 sandbox；宿主侧 `window.addEventListener("message")` 桥接，校验 `event.origin`；注入桥脚本到扩展入口页（通过 `asset:` 协议同源加载注入脚本，或宿主侧 postMessage 协议）。
- 新增 `src/stores/bridge.ts`：postMessage 协议、`bridgeId` 分配、Promise 映射、超时、来源白名单。
- 新增 `src/lib/extension-ui.ts` 类型扩展：`UiExtensionView` 增加 `zone`/`capabilities`。

---

## 五、依赖关系

| 模块 | 依赖 | 被依赖 |
|------|------|--------|
| 34-Extension UI Open Architecture | 07-Extension（manifest/生命周期）、22-UI-Framework（surface/布局）、06-Sandbox（白名单门禁 + 网络策略）、02-IPC、02b-Stream | 25-Notification（渠道扩展投影）、27-Hotkey（keybinding 复用 OpenView） |
| 白名单桥（extension_bridge.rs） | extension/lifecycle（Enabled 判断）、security/sandbox（OperationRequest/CheckResult） | 前端 bridge.ts |
| ExtRouter（ext_router.rs） | extension/lifecycle、extension/models（capabilities.extension_calls / extension_exports） | 前端 bridge.ts `call()`、ActionRegistry |
| ExtensionStorage（extension_storage.rs） | foundation/storage（扩展数据目录）、extension/models（StorageDeclaration） | 前端 bridge.ts `storage.*` |
| ExtensionNetPolicy（extension_network.rs） | security/sandbox（网络策略层）、extension/models（NetworkPolicy） | 前端 bridge.ts `fetch()` |
| ExtensionDiscovery（discovery.rs） | extension/store（ExtensionStore 只读投影）、extension/models（provides） | 前端 bridge.ts `extensions.query` |
| Zone Registry | extension/models（ZoneRegistration）、ui/host_view.rs（DTO 投影） | 前端 zone store、HostViewSurface |

**不依赖**：Kernel 四原语只作为事件/能力出口消费，本模块不新增任何 Kernel 原语。

---

## 六、状态管理

### 6.1 前端

| Store | 位置 | 职责 |
|-------|------|------|
| Zone Registry | 新增 `src/stores/zone.ts` | 内置 zone 锚点映射 + 扩展 zone 快照（来自 `ui_list_zones`），`getZoneById` / `getAvailableZones` |
| Menu | `src/stores/menu.ts`（改造） | 支持任意 target；`getZoneMenuItems(zoneId)` 按 zone 收集；`when` 求值结果缓存 |
| Bridge | 新增 `src/stores/bridge.ts` | postMessage 协议、Promise 映射、超时、来源校验；`storage.*` / `fetch()` / `extensions.query` 透传 |
| ExtensionDialog | 新增 `src/components/ExtensionDialog/store.ts` | 多弹框并行、z-order 栈、打开/关闭/聚焦/缩放/拖拽状态 |
| HostView | `src/stores/app.ts`（改造） | `hostViewInstances` 增加 zone 字段；新增 `extensionDialogs: ExtensionDialogInstance[]` |
| Discovery | 新增 `src/stores/discovery.ts` | `extensions.query` 结果缓存 + `extension.registry.changed` 订阅刷新；`findByProvides(label)` |

### 6.2 状态转换

- zone：`declared`（manifest 已声明）→ `available`（宿主已承接，可渲染）→ `mounted`（当前有 view 实例挂载）。不可用 zone 的 view 不进入用户可见入口。
- 弹框：`opening` → `open`（活跃）→ `focused`（z-order 栈顶）→ `closed`。多弹框并行，关闭最后一个释放弹框层。

---

## 七、错误处理

| 场景 | 行为 |
|------|------|
| 扩展 zone 的 parent 引用未知 zone | manifest 校验 fail-closed，view 不进入菜单 / zone 投影 |
| 扩展 zone 的 slot 未命中预置列表 | 同上 |
| iframe 来源校验失败 | 桥丢弃请求，审计记录 |
| `__NAVIS__.invoke` 调用未声明命令 | 拒绝 + 审计，返回错误 `PermissionDenied` |
| sandbox 门禁拒绝（操作级别不足/需审批） | 返回 `CheckResult` 对应错误；`needs_confirm` 时挂起并弹宿主确认（复用 06-Sandbox 审批流） |
| 跨扩展调用未授权（from.extension_calls 缺失 / target.extension_exports 未导出） | ExtRouter 拒绝 + 审计；命令可渲染性 gate 在菜单层即拦截 |
| 动作目标解析失败（view/script/target 不存在或未授权） | 命令不进入菜单 / Command Palette（声明即不可见，非运行时报错） |
| 扩展配置读取/写入失败 | `ui_get/set_extension_config` 返回错误；写失败不落库，回滚到上次已保存值 |
| 存储访问越界（读其他扩展的 key / 超大小上限） | `ui_extension_storage_*` 拒绝 + 审计，返回 `StorageDenied` |
| 网络请求违反策略（未声明 network / 域名不在 allowlist） | `ui_extension_network_proxy` 拒绝 + 审计；none 模式下 iframe/worker 内 fetch 由桥拦截 |
| 发现查询返回空 | 正常（不是错误）——扩展按空结果降级（隐藏功能），不报错 |
| 桥请求超时 | 前端 reject，保留审计 |
| 扩展禁用/卸载时 | 关闭该扩展全部弹框 + 卸载 zone/桥实例（`removeHostViewsForExtension` 扩展 + 弹框清理）；跨扩展路由表删除该扩展的 exports 与 calls 记录；ephemeral 存储立即清空，global/worktree 保留（卸载确认清除） |

---

## 八、安全考量

1. **严格沙箱**：iframe 保持 `sandbox="allow-scripts"`，无 `allow-same-origin`，无 `withGlobalTauri` / `__TAURI_INTERNALS__` 注入。
2. **白名单授权**：扩展 UI 只能调 `capabilities.invoke` 声明且已注册的命令；未声明 → fail-closed。
3. **来源校验**：桥只接受属于当前扩展实例的 iframe/Worker 来源，`event.origin` 必须命中扩展资源来源白名单。
4. **sandbox 复用**：每个桥请求构造 `OperationRequest{actor:"extension:{id}"}`，走现有权限分级 / 审批 / 审计（permission.rs:125-175 已内置 `extension_id()`）。
5. **资源边界**：扩展 entry 只能位于 `ExtensionUI/`（脚本位于 `ExtensionUI/scripts/`，i18n 位于 `ExtensionUI/locales/`）目录，`resolve_extension_ui_resource` 的 canonicalize + 符号链接检测保持不变（resource.rs:30-84）。
6. **不直通 IPC**：扩展 UI 永不能直接 `invoke` 任意 Tauri 命令；全部经白名单桥。
7. **事件只读**：扩展订阅的是 Kernel EventBus 的只读投影，不可反写宿主事实。
8. **跨扩展双端授权**：`ext:A → ext:B` 必须同时满足 ext:A 的 `capabilities.extension_calls` 与 ext:B 的 `extension_exports`，扩展是安全边界，不接受单方单方面授予。
9. **配置隔离**：扩展配置读写按 extension_id 命名空间隔离，扩展只能读写自己的配置，宿主设置面板只展示已启用扩展的 schema。
10. **存储隔离**：扩展 KV 存储按 extension_id 命名空间 + scope 隔离，扩展只能读写自己的 key 空间；值过 Rust 校验（大小上限/JSON 深度）；存储不落明文 secret。
11. **网络 fail-closed**：不声明 `capabilities.network` → 完全禁网；allowlist/proxy 模式经宿主代理 + 06-Sandbox 网络策略 + 审计，扩展无法直连外网或绕过策略（CSP 封被动资源绕过）。
12. **发现不授权**：Discovery 只回答"谁提供了什么能力"（已启用扩展的只读投影），不授予任何调用权；实际调用仍须过 ExtRouter 双端白名单。

---

## 九、事件定义

| 事件 | 方向 | 说明 |
|------|------|------|
| `extension.zone.changed` | Kernel EventBus → UI | 扩展 zone 可用性变化（enable/disable 后刷新） |
| `extension.dialog.closed` | 宿主桥 → 扩展 | 弹框被用户关闭，通知扩展清理 |
| `extension.bridge.denied` | 宿主桥 → 审计 | 白名单/来源拒绝，带 extension_id + 原因 |
| `extension.scripts.loaded` | 后端 → 前端 | 脚本投影就绪，触发前端胶水 worker spawn |
| `extension.config.updated` | Kernel EventBus → 扩展 | 扩展配置被宿主/用户修改，经桥通知订阅方（扩展命名空间化事件名） |
| `ext.{extId}.event.*` | 宿主 EventBus（命名空间化） | 扩展发布的事件；其他扩展经 `capabilities.events` 声明订阅，宿主校验授权后路由 |
| `extension.registry.changed` | Kernel EventBus → UI/扩展 | 扩展启用/禁用/卸载/版本变化，通知发现机制刷新（§2.7） |
| `extension.storage.changed` | 宿主桥 → 扩展 | 扩展存储 key 被外部（同扩展其他实例）修改，经桥通知订阅方（读后清订阅） |
| `extension.network.denied` | 宿主桥 → 审计 | 网络策略拒绝，带 extension_id + URL + 模式 |

---

## 十、扩展支持（contributes 承接全景 + 审计基线）

> 本表是 2026-08-15 全量审计结果与本设计的补齐目标。审计依据：`state.rs::ensure_supported_runtime_contributes`（reject_unbound! 宏）与前端消费代码搜索。

### 10.1 已承接（本次不动）

| contributes | 承接位置 | 说明 |
|-------------|----------|------|
| `mcp_servers` / `tools` / `mcp_tool_overrides` | `state.rs:415-472` → MCP ToolRegistry | 真实注册 |
| `skills` | `state.rs:475-492` → Skills | 真实注册 |
| `languages`（LSP） | `state.rs:494-499` + `register.rs:582-599` → LSP registry | 真实注册 |
| `hooks` | `register.rs:602-613` + `tool/agent/hooks.rs:45-62` | PreToolUse Deny/Continue 真实执行 |
| `gateway`（adapters/providers/validation） | `families.rs` → `register.rs:127-191` → `ai/gateway/mod.rs` | 真实写入 provider/protocol registry |
| `views` | Zone Registry + Renderer Registry（本设计） | `zone` 决定 surface，`renderer` 决定渲染轨 |
| `menus` | MenuRegistry（本设计，开放 target + when/group） | 任意位置；`Menubar.*` 进顶部菜单栏 |
| `commands` | CommandPalette + 菜单 action（本设计，动作族） | 动作族：OpenView/ToggleView/OpenDialog/RunScript/SendMessage |
| `keybindings` | 27-Hotkey（复用动作族分发） | 扩展只可注册 App 范围 |
| `work_modes` | 已承接 | 前端消费 |

### 10.2 fail-closed → 本设计转为承接（补齐目标）

> 行号依据 2026-08-16 复核：`state.rs::ensure_supported_runtime_contributes` 的 reject_unbound! 块实际位于 **L690-794**（宏 L697），`do_disable` 自 L796 起。行号易漂移，实施时以函数名 + 特性名为准。

| contributes | 现状（审计） | 本设计承接 | 阶段 |
|-------------|--------------|-----------|------|
| `toolbar_items` | `state.rs:754-758` 拒绝；前端无消费 | ToolbarRegistry + 前端 Toolbar 投影（ComposerToolbar/面板工具栏合并渲染） | 阶段 6 |
| `statusbar_items` | `state.rs:759-763` 拒绝；前端无消费（`layouts/StatusBar.tsx` 从未被挂载，死代码） | StatusBarRegistry + 前端 StatusBar 投影（先挂载 StatusBar 再承接） | 阶段 6 |
| `inline_extensions` | `state.rs:764-768` 拒绝；前端无消费 | InlineRegistry：chatAside/editorView/terminal 挂载点投影 | 阶段 6 |
| `configuration` | `state.rs:770-776` 拒绝；前端无设置面板 | Config host：`ui_get/set_extension_config` + 设置面板 JSON Schema 表单渲染 | 阶段 6 |
| `layout_overrides` | `state.rs:779-783` 拒绝 | LayoutOverride：扩展调整已授权 zone 的 size/visible/order（宿主裁决） | 阶段 6 |
| `styles`（editor.style） | `state.rs:778` 拒绝 | **移交 26-Editor**（CodeMirror 承接，本设计只承接声明投影与桥传递） | 26 号 |
| `triggers` | `state.rs:777` 拒绝；前端走 Skills 斜杠命令；`state.rs:936-953` disable 有处理但 enable 拒绝（死代码） | 复用 ScriptRegistration 逻辑轨（worker 触发），不单建宿主；清理 disable 死代码 | 阶段 5 |
| `themes` / `editor_languages` / `editor_extensions` | `state.rs:717-727` 拒绝 | **移交 26-Editor** | 26 号 |
| `notification_channels` | `state.rs:728-732` 拒绝；前端 `registerNotificationChannel`（Notification/channel.ts）整轨零调用方 | **移交 25-Notification** | 25 号 |
| `event_subscriptions` | `state.rs:749-753` 拒绝；基础设施（port/adapter/ledger）已装配，缺 runtime handler | 经白名单桥 `listen()` 承接（宿主半就绪，补入口即可） | 阶段 1 |
| `middlewares` / `transport_adapters` | `state.rs:707-716` 拒绝；Gateway 管道 / MCP transport registry 真实存在，缺 manifest→宿主转换 | **本设计承接**（见 §15 后端改造清单 P0-1/P0-2）：接入 `GatewayPipelineConfig::add_extension` 与 `ServerManager::register_transport` | 阶段 7 |
| `behaviors` / `context_providers` / `search_providers` / `roles` | `state.rs:733-743` + `769` 拒绝 | **移交各自业务域**（本设计不承接，保持 fail-closed 直至对应域承接） | 后续 |
| `file_watchers` | `state.rs:744-748` 拒绝 | **移交 Tool 域**（不属 UI 层） | 后续 |

### 10.3 静默忽略 → 必须修复（违反 fail-closed 铁律）

| contributes | 现状 | 修复 |
|-------------|------|------|
| `tray_items` | models.rs:154 定义；loader.rs:249-256 仅校验命令引用；**无 reject 也无承接** → 用户看到"已启用"但托盘永不出现 | 二选一：接入窗口域托盘 registry（真实承接），或加入 reject_unbound!（显式拒绝）。禁止静默 |

### 10.4 本设计新增 contributes

| contributes | 承接方 | 说明 |
|-------------|--------|------|
| `scripts` | worker 胶水轨 | 前端胶水脚本，白名单桥 |
| `capabilities` | 白名单桥 + ExtRouter + ExtensionStorage + ExtensionNetPolicy + ExtensionDiscovery | invoke/events/read/extension_calls/provides/network 授权声明 |
| `zones` | Zone Registry | 自定义位置 + 锚定语义 |
| `extension_exports` | ExtRouter | 跨扩展面显式导出 |
| `storage` | ExtensionStorage | KV 状态存储（global/worktree/ephemeral 三态） |
| `network` | ExtensionNetPolicy | 网络模式（none/allowlist/proxy） |
| `provides` | ExtensionDiscovery | 能力标签，运行时发现 |
| `i18n` | **移交 28-i18n** | 扩展本地化资源声明与语言切换由 28 号承接（本设计只透传声明，禁止静默忽略） |

---

## 十一、性能指标

| 指标 | 目标 |
|------|------|
| iframe 首屏加载 | 扩展入口页 <= 300ms（宿主 `asset:` 协议本地资源，懒加载触发） |
| 桥 invoke 往返 | 单命令 <= 20ms（不含命令自身耗时） |
| 并发弹框 | 支持 >= 8 个并行，z-order 切换 <= 16ms；非激活弹框 suspended（不常驻活跃渲染） |
| 桥协议解析 | 单消息 <= 1ms，不阻塞主线程（Web Worker / message 异步） |
| 布局树重排 | 单次 zone 增删/改尺寸 <= 16ms（宿主布局算法） |
| **高频流投递（Agent 动作）** | ThrottledEmitter 50ms 窗口合并（复用 02b-stream 默认），单窗口批量转 1 次 postMessage；扩展无订阅时该流零转发开销 |
| **iframe 生命周期** | mounted→suspended <= 50ms；suspended 释放 JS 上下文，常驻活跃 iframe 峰值受配额限制（默认 <= 4，弹框 <= 8） |
| 扩展存储读 | 内存缓存命中 <= 1ms；未命中经 IPC <= 20ms |

---

## 十二、测试策略

| 层 | 要点 |
|----|------|
| Rust 单元 | `extension_bridge`：白名单命中/拒绝、来源校验、`OperationRequest` actor 构造；`ext_router`：双端授权命中/拒绝（from.extension_calls ∩ target.extension_exports）、目标解析失败 fail-closed；`extension_storage`：命名空间隔离 / scope 语义 / 大小上限 / ephemeral 清空；`extension_network`：none 拒绝 / allowlist 命中与未命中 / proxy 走 sandbox 网络策略；`extension_discovery`：provides 匹配 / 仅返回已启用；`zones` 校验：parent 引用未知 zone / slot 未命中 / 命名空间化 |
| Rust 集成 | `ui_list_zones` 投影（内置+扩展）；`ui_list_menus` 开放 target 透传 + 可渲染性 gate；`ui_get/set_extension_config` 读写 + 配置命名空间隔离；`ui_extension_storage_*` 落盘与隔离；`ui_extension_network_proxy` 策略命中；fail-closed 场景（未声明 capabilities 的 invoke、未授权跨扩展调用、未声明 network 的 fetch）；`subscribeSource` Agent 流订阅逐条实时投递 + 按需订阅（无订阅者零推送）验证 |
| 前端单元 | `bridge.ts`：Promise 映射 / 超时 / origin 拒绝 / stream 批量转发；`menu.ts`：任意 target 收集 / when 求值；`zone.ts`：zone 快照与可用性；动作族分发（OpenDialog/RunScript/SendMessage 目标解析）；`discovery.ts`：query 缓存 / registry.changed 刷新 |
| 前端集成 | `npm run test:menus` 覆盖表扩展（新增 `Menubar.*`、扩展 target、动作族 executor 覆盖） |
| 端到端验收 | 扩展声明 menu+view（html:sandbox）→ 菜单出现 → 点击打开 → iframe 内 `__NAVIS__.invoke` 调白名单命令成功 / 未声明命令被拒；扩展声明 dialog view → 弹框可拖拽/缩放/多开；扩展 A 经 ExtRouter 打开扩展 B 视图 / 执行 B 命令（授权通过）与未授权拒绝；扩展 `storage.set` 后 reload 仍可 `get`（global）而 worktree 切换后隔离；扩展 `fetch` 在 none 模式被拒 / allowlist 命中可通；扩展经 `subscribeSource({kind:"agent"})` 逐条实时收到 Agent 动作（按需订阅生效：无订阅者时后端零推送） |
| 性能回归 | 高频流场景（Agent 流 + 3 个订阅 iframe）主线程无卡顿、无 fan-out 爆炸（按需订阅 + 生命周期挂起生效）；iframe 挂起/恢复无泄漏；仅高频低价值流（终端/日志）可经 ThrottledEmitter 合并，Agent 流逐条实时不节流 |
| 安全 | 伪造 iframe 来源被拒；未授权命令/跨扩展调用/网络请求审计日志存在；扩展禁用后全部弹框/zone/桥实例/路由表清理 + ephemeral 清空；扩展配置/存储越界（读别的扩展）被拒 |

---

## 十三、实施阶段与验收

### 实施顺序原则（桥优先）

"万物皆扩展"的运行态交互全部汇聚在**白名单桥**（数据进扩展 / 扩展出命令 / 弹框与前端胶水 worker 复用），桥是共同咽喉，必须**第一优先级**落地。开放命名空间（Zone/Menu）、弹框、菜单栏、worker 胶水轨都建立在桥之上；桥未建成前，扩展 UI 收不到任何运行时数据，其余阶段只是"可声明但运行态是死的"。因此顺序调整为：

```
阶段 1 桥（运行态咽喉）→ 阶段 2 开放命名空间（声明层）→ 阶段 3 弹框
→ 阶段 4 菜单 when/group + 菜单栏 → 阶段 5 worker 胶水轨 + 动作族 → 阶段 6 UI 扩展点承接
→ 阶段 7 ExtRouter + Gateway/MCP 接线（B-P0-1/2）→ 阶段 8 扩展存储 + 网络策略（含 B-P0-3）
→ 阶段 9 扩展发现 + 动态数据绑定 → 阶段 10 tray_items 合规修复
```

> 后端改造清单（§15.2）按依赖关系嵌入上述阶段：B-P1-6（async 阻塞 I/O）为阶段 1 前置（Agent 流直通依赖异步热路径健康）；B-P1-4（保持实时不节流）随阶段 1 一并验收；B-P1-5/B-P1-7/B-P2-8/9/11/12 为独立治理项，不阻塞阶段进度。前端清单（§15.1）中 F-B1/B2/B3 由阶段 1-5 覆盖，F-I/F-M 项随对应阶段或独立治理项处理。

每阶段独立可验收；阶段 1 验收即打通"Agent 动作 → 扩展 UI"这一万物皆扩展的核心能力。

### 阶段 1 — 白名单桥（运行态咽喉，第一优先级）

**Rust**
- `src-tauri/src/extension/models.rs`：新增 `capabilities`（invoke/events/read 白名单），loader 校验命令引用存在。
- 新增 `src-tauri/src/ui/extension_bridge.rs`：`ui_extension_bridge_invoke`——校验 Enabled → 查 capabilities → 构造 `OperationRequest{actor:"extension:{id}"}` 过 sandbox 门禁 → 派发。
- `event_subscriptions` 承接：补 runtime handler 入口（基础设施 port/adapter/ledger 已装配，`state.rs:834-838` 的 reject 移除，改经桥 `listen()` 承接）。
- `src-tauri/src/app/mod.rs`：注册 `ui_extension_bridge_invoke`。

**前端**
- 新增 `src/stores/bridge.ts`；`HtmlSandboxRenderer.tsx` 改造（严格 sandbox + host 侧 message 监听 + origin 校验 + 桥注入）。
- 桥注入脚本提供 `window.__NAVIS__.invoke/listen/getContext`。

**前置验证**：实测 `convertFileSrc`（asset 协议）下 iframe `event.origin` 形态，决定桥接注入方式。

**验收（核心）**：
1. iframe 内 `__NAVIS__.invoke('file.read', {path})` 经白名单+门禁返回结果；未声明能力被拒并审计。
2. **Agent 动作实时投递（Stream 直通，非事件桥）**：扩展 iframe 内经桥订阅 `extension.stream.subscribeSource({ kind: "agent", sessionId })`（02b-stream §3.8 既有设计，`session_message_stream.rs:278` 已用 `stream_kind::AGENT` + session meta 建流）——宿主桥把 Agent 流 Tauri Channel **直通转发**，扩展**逐条实时**收到 AgentTimelinePart（**保持实时不节流**，用户裁决）；**不走 EventBus 事件桥逐条转发**（避免每 token 一次 fan-out 到多 iframe 的 IPC 爆炸与掉帧）；性能靠按需订阅（无订阅者零推送）+ iframe 生命周期（§2.3.1）承载。主 UI 已有 Stream 数据源零改动复用。
3. `__NAVIS__.getContext()` 返回当前 session/project 快照。
4. `event_subscriptions` 经桥订阅的事件能收到（宿主半就绪入口补齐）。

### 阶段 2 — 开放命名空间化（声明层扩展）

**Rust**
- `src-tauri/src/extension/host_view.rs`：placement 白名单（L15-18）放开为 zone 命名空间；`validate_extension_view`（L40-78）增加 zone/anchor 校验（parent 引用已知 zone / slot 命中预置列表）。
- `src-tauri/src/extension/models.rs`：`ViewRegistration.placement` → `zone`（兼容解析旧名）；`MenuRegistration.target` 开放字符串；新增 `ZoneRegistration`/`ZoneAnchor`。
- `src-tauri/src/extension/lifecycle/contributions.rs`：`is_supported_menu_target`（L208-219）移除白名单拒绝，改命名空间校验（内置 target 走白名单，`{extId}:{targetId}` 走前缀校验）。
- `src-tauri/src/ui/menus.rs`：`ui_list_menus`（L537-577）透传任意 target + when/group；新增 `ui_list_zones`。
- `src-tauri/src/ui/dto.rs`：新增 `UiZone`；`UiMenuRegistration`/`UiExtensionViewDescriptor` 字段同步。
- `src-tauri/src/app/mod.rs`：注册 `ui_list_zones`。

**前端**
- `src/stores/app.ts`：`HostViewPlacement`（L58-63）结构化（内置枚举 + `{extId}:{zoneId}`）；新增 `src/stores/zone.ts`。
- `src/stores/menu.ts`：`getMenuItems` 支持任意 target；`when` 求值器接入。
- `src/layouts/`：MainLayout.tsx:172/175、SettingsDialogContent.tsx:73 硬编码锚点改为遍历 zone registry 动态渲染 `HostViewSurface`。
- `src/components/HostView/registry.ts`：surface registry 枚举 → zone 路由。

**验收**：扩展声明 `target:"my-ext:custom"` 菜单项出现；视图挂到任意已声明 zone；未声明 zone 的 view fail-closed。

### 阶段 3 — 独立弹框体系 ExtensionDialog

**前端**（新建，不复用 24-Dialog）
- `src/components/ExtensionDialog/`：Manager（多弹框并行、z-order 栈）、Surface（标题栏/拖拽把手/缩放手柄）、Store。
- 桥 API 新增 `__NAVIS__.dialog.open({viewId,size,position,modal})` / `dialog.close()`。
- 弹框内复用 `HostViewRenderer`；`app.ts` 新增 `extensionDialogs` 状态。

**验收**：扩展菜单点击弹出可拖拽/缩放/多开的扩展 dialog，内容可交互。

### 阶段 4 — 菜单 when/group + 顶部应用菜单栏

- `when` 求值器（`{activeSession}` / `{activeProject}` / `{platform}` / 扩展上下文变量）；`FloatingMenu.tsx` 与各 surface 菜单按 `group` 分组渲染。
- 新增 `src/components/MenuBar/MenuBar.tsx`（File/Edit/View/Help/Tools 骨架，扩展可贡献），挂 MainLayout 顶部，复用 `executeDeclarativeMenuAction`（menu-actions.ts:138-151）。
- 更新 `npm run test:menus` 覆盖表。

**验收**：菜单按 when 显隐、按 group 分组；顶部菜单栏出现扩展贡献项且能开视图。

### 阶段 5 — 逻辑轨 + 动作族泛化

- 扩展 `contributes.scripts` → 前端 Web Worker 承载，worker 内注入 `__NAVIS__` 桥（postMessage 透传），与 UI 轨共用同一白名单门禁。`triggers` 复用逻辑轨（worker 触发），不单建宿主。
- `BuiltinAction` 泛化为动作族（`OpenView | ToggleView | OpenDialog | RunScript | SendMessage`）：
  - `src-tauri/src/extension/models.rs`：`BuiltinAction` 枚举扩展；loader 校验动作目标（view/script 存在）。
  - 前端 `extension-commands.ts:27 canDispatchCommandAction` 与 `menu-actions.ts:126` 扩展为动作族分发；`ActionRegistry` 解析动作目标。
- 无原生 JS 引擎；无头长期后台超需时后续评估 wasmtime 轨。

**验收**：扩展 worker 后台处理数据并经白名单桥调宿主能力；命令动作族可触发 OpenDialog / RunScript；`canDispatchCommandAction` 不再只认 OpenView/ToggleView。

### 阶段 6 — UI 扩展点承接（toolbar/statusbar/inline/configuration/layout）

- 前端消费组件：ComposerToolbar/面板工具栏合并渲染 `toolbar_items`；StatusBar 投影 `statusbar_items`；chatAside/editorView/terminal 挂载 `inline_extensions`。
- Config host：`ui_get/set_extension_config` + 设置面板 JSON Schema 表单渲染（新增 Settings tab），`extension.config.updated` 事件通知扩展。
- `layout_overrides`：扩展调整已授权 zone 的 size/visible/order，宿主布局算法裁决。
- 同步移除 `state.rs:839-868` 对应 reject_unbound! 项（保留未移交项的拒绝）。

**验收**：扩展声明的工具栏按钮/状态栏项/inline 组件出现在宿主对应 surface；扩展配置在设置面板可编辑、扩展经桥实时收到变更。

### 阶段 7 — ExtRouter 扩展间通信

- 新增 `src-tauri/src/ui/ext_router.rs`：`ui_extension_route_call` 双端授权（from.extension_calls ∩ target.extension_exports）→ 转发视图打开/命令执行/事件订阅。
- 事件命名空间化 `ext.{extId}.event.*`，订阅校验授权。
- 前端桥 `__NAVIS__.call(target, action, payload)`；`action.SendMessage` 目标解析。

**验收**：扩展 A 经 ExtRouter 打开扩展 B 视图 / 执行 B 命令 / 订阅 B 事件（授权通过）；未授权调用被拒并审计；禁用 B 后 A 的路由请求 fail-closed。

### 阶段 8 — 扩展存储 + 网络策略

- 新增 `src-tauri/src/ui/extension_storage.rs`：`ui_extension_storage_*`（get/set/delete/clear），按 extension_id 命名空间 + scope 读写，落盘扩展数据目录；set 过 Rust 校验（大小上限/JSON 深度）。
- 新增 `src-tauri/src/ui/extension_network.rs`：`ui_extension_network_proxy`（proxy 模式）/ allowlist 校验；CSP 注入（按 network 模式生成 img-src/font-src/connect-src）。
- 桥 API 新增 `__NAVIS__.storage.*` / `__NAVIS__.fetch`。
- 同步移除 `state.rs` 中 `network`/`storage` 相关未承接拒绝（若加入 reject 列表的话），改由 §2.5/§2.6 承接。

**验收**：扩展 `storage.set` 后 reload 仍可 `get`（global）；worktree 切换后 worktree scope 隔离；扩展 `fetch` 在 none 模式被拒并审计、allowlist 命中可通、proxy 模式请求过 sandbox 网络策略。

### 阶段 9 — 扩展发现 + host:panel 动态数据绑定

- 新增 `src-tauri/src/ui/discovery.rs`：`ui_extension_discovery_query`（按 capability/provides 检索已启用扩展）；`ui_list_extensions` 透传 `provides`。
- 新增 `src/stores/discovery.ts`：query 缓存 + `extension.registry.changed` 订阅刷新。
- host:panel 数据绑定：宿主声明式面板支持声明数据源投影（`host:panel` renderer 增加 `data_source: { kind: "event"|"stream"|"storage", pattern, transform }`），面板随数据流实时更新——补齐审计缺口 4（host 轨接动态数据，不必绕路 iframe）。
- 桥 API 新增 `__NAVIS__.extensions.query`。

**验收**：扩展 `__NAVIS__.extensions.query({provides:"file-index"})` 返回已启用匹配扩展；扩展启用/禁用后发现结果自动刷新；host:panel 声明式面板订阅宿主事件/stream 实时更新（数据绑定路径打通）。

### 阶段 10 — tray_items 合规修复

- 二选一落地（见 §10.3）：接入窗口域托盘 registry 或加入 reject_unbound!。禁止静默忽略。

**验收**：扩展声明 tray_item 后托盘真实出现该菜单项；或 enable 时显式报"不支持 tray_items"。

---

## 十四、风险与开放问题

1. **iframe 桥 origin 校验**：`convertFileSrc` 走 `asset:` 协议，需实测 `event.origin` 形态（`tauri://localhost` 或 asset 协议变体）——阶段 1 前置验证，决定桥注入方式。
2. **动态布局树改造耦合度**：parent+position 需要 `rightWorkspace` 一维 split（app.ts:77-81）演进为嵌套布局树，与现有 surface 锚点改造耦合——阶段 2 先行定义布局树数据结构。
3. **ExtensionDialog 与 24-Dialog 并存**：两者独立，需协调 z-index 层级（弹框层位于宿主 dialog 层之下或独立栈）。
4. **`when` 求值上下文范围**：会话/项目/platform/扩展变量，阶段 4 定义完整且 fail-closed（无法求值时隐藏）。
5. **worker 胶水轨与扩展 UI 通信**：worker 与 iframe 之间是否需要直连，还是全部经宿主桥中转（推荐后者，避免绕过白名单）。
6. **菜单栏 `Menubar.*` 与现有浮动菜单并存**：顶部菜单栏是宿主新增 surface，内建菜单项与扩展项需统一经 `ui_list_menus`，不新增平行菜单源。
7. **动作族安全边界**：`RunScript`/`SendMessage` 引入命令触发逻辑（而非仅开视图），必须保持与桥相同的白名单+审计路径，防止动作族成为旁路。
8. **扩展间信任链**：ExtRouter 双端授权依赖扩展声明可信；恶意扩展可通过互相声明绕过单方限制，需在双端校验基础上保留宿主审计与可撤销（禁用任一端即断链）。
9. **UI 扩展点承接的回归风险**：toolbar/statusbar/inline 消费组件落地时需与宿主内建 UI 统一渲染层，避免扩展项与内建项样式/行为割裂（复用现有 Toolbar/StatusBar 组件，不新建平行实现）。
10. **扩展配置 schema 演进**：`contributes.configuration` 使用 JSON Schema，需定义版本化与校验错误反馈（表单错误态），防止扩展升级破坏既有用户配置。
11. **扩展存储持久化损坏**：扩展 KV 数据写坏（断电/版本升级）需容错——Rust 侧按 key 校验 + 备份机制，扩展侧读到坏值降级而非崩溃。
12. **网络策略绕过面**：proxy/allowlist 之外的绕过途径（`<img>`/`<script>`/`<link>` 被动加载、Worker importScripts、iframe 嵌套子资源）需 CSP 封死，阶段 8 验收须含绕过测试。
13. **host:panel 数据绑定性能**：声明式面板订阅高频事件/stream 需节流与变换白名单，防止拖慢主线程——复用 `ThrottledEmitter`（02b-stream.md §四）。
14. **i18n 移交边界**：`contributes.i18n` 在 28 号承接前必须加入 reject_unbound!（禁止静默忽略），28 号承接后移除拒绝。
15. **Agent 流实时性 vs IPC 频次**：保持实时不节流（用户裁决），IPC 频次优化只能靠按需订阅 + iframe 生命周期（§2.3.1）+ 背压（丢弃+计数），禁止为性能节流 Agent 动作。
16. **Store 投影同步 vs Stream 直通**：主 UI 已有权威投影的数据（Agent 动作、会话状态）经 Store 投影同步（§2.8 双层数据面）；主 UI 不投影的原始流（终端、自定义 stream）经 Stream 直通 `subscribeSource`。两者分治，避免同一数据双通道导致不一致。
17. **扩展 KV 归属裁决**：`ExtensionStore` facade 由 extension 域自持（三态隔离），`foundation/storage` 只提供落盘原语——避免继续膨胀 Storage 上帝对象（后端 P2-10）。

---

## 十五、全系统改造基线（2026-08-16 三线审计）

> 本节是三线审计（前端 `src/` / 后端 `src-tauri/` / 设计文档 `design/`）的**唯一裁决落点**。所有涉及"万物皆扩展 / 高性能"的改造必须在此登记，改动前先查本节是否已覆盖。
> 优先级约定：🔴 阻断（不修则目标不成立）/ 🟠 重要（架构违规、影响可维护与规模）/ 🟡 次要（体验与规模优化）。
> 性能依赖铁律：**自造方案只有在审计确认是瓶颈时才保留；凡有成熟高性能依赖可替换，优先引入（§15.4）。**

### 15.1 前端改造清单（审计 `src/`）

#### 🔴 阻断级

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| F-B1 | `components/HostView/registry.ts:26-36` + `extension/host_view.rs:11-18` | HostView renderer/surface 前后端双封闭（无注册 API），未知 renderer 静默降级到 host:panel | 前端 `registerHostViewRenderer(spec)` / `registerHostViewSurface(placement, spec)`；未知 renderer 显式报错，不静默降级 |
| F-B2 | `stores/menu.ts:22-25`、`extension-commands.ts:27-33`、`extension-keybindings.ts:31-41` | 菜单/命令/热键动作仅 `OpenView/ToggleView`；`tools-menu.ts` 5 分支 switch 无扩展分支 | 引入 `registerMenuActionHandler(kind, handler)` 动作注册表；tools-menu 改注册表分发（对齐 §4.2.1 ActionRegistry） |
| F-B3 | `components/HostView/HtmlSandboxRenderer.tsx:25-33` | iframe 无桥、无 `allow-same-origin`，扩展无法访问 Tauri API / 宿主状态 / 订阅宿主流 | 阶段 1 白名单桥（postMessage 双向 + origin 校验 + channel 订阅透传） |
| F-B4 | `components/Notification/channel.ts` | `registerNotificationChannel`/`initializeAllChannels` 整轨零调用方（死代码） | 删除死代码或按 `store.ts` 的 `registerChannel` 活路径重构为统一通知注册 API（配合 25 号） |
| F-B5 | `components/Editor/types.ts:544-587`、`EditorView.tsx:40-53`、`theme-extension.ts:355`、`lsp-extension.ts:36-52` | 编辑器语言/主题/扩展类型齐全但零加载路径；languageExtension switch 写死 ~7 语言；LSP 前缀硬编码 `'lsp'` | 实现 `applyEditorRegistrations(registrations)` 注入 Compartment/themeCatalog；LSP 前缀由扩展贡献参数化（配合 26 号） |

#### 🟠 重要级

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| F-I1 | `stores/extension.ts → extension-commands.ts → menu-actions.ts → extension.ts`（两环） | 循环依赖（"碰巧能跑"） | dispatch 逻辑下沉独立 `lib/menu-dispatch.ts`，extension 只持数据 |
| F-I2 | `stores/chat-message-state.ts:11` + `chat-messages.ts:64-75` | chatMessageState 单实例，切换会话整组覆写、丢本地增量 | 改 `Record<sessionId, ChatMessageState>` + 会话级订阅，或 per-session store 工厂（对齐 §15 双层数据面的 Store 投影源） |
| F-I3 | `stores/task-projection.ts:74`、`session-todos.ts:64`、`lib/status/polling.ts:10` | 1s IPC 轮询 ×2（ui_list_tasks / ui_list_session_todos） | 后端事件驱动推送（EventBus→Channel）或 `isStatusLive` 自适应退避/停轮询；polling.ts 支持动态 interval |
| F-I4 | `layouts/StatusBar.tsx`（从未挂载）+ `layouts/Toolbar.tsx:286-377` | StatusBar 死代码；Toolbar 无扩展插槽 | 挂载 StatusBar 并抽象左右插槽（对齐 contributes.views） |
| F-I5 | `components/Composer/ComposerToolbar.tsx` + `Composer.tsx:262` | 工具栏全内建无插槽；`setInterval(...,1000)` 每秒重渲染 goalStrip | 提供 toolbar slot 协议（`registerComposerToolbarButton`）；goalStrip 依赖既有 tick 或派生时间 |
| F-I6 | `i18n/index.ts:306`、`i18n/types.ts:314` | `registerExtensionLocale` 零调用方、无后端语言包加载 IPC | 接 `ui_list_extension_locales` 类 IPC，`loadExtensions` 后调用（配合 28 号） |
| F-I7 | `layouts/Sidebar.tsx:127`、`TerminalPanel.tsx:66`、`CommandPalette/store.ts:14` | 组件绕开 AppState 直接跨 store 引用（违反 app.ts:20-21） | 收敛到事件同步层（useEvent/useChannel）或 AppState |

#### 🟡 次要级

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| F-M1 | `components/ChatMessages.tsx`、`chat-messages.ts:64-75` | 长会话 DOM 膨胀无虚拟化；60/页整组覆写 | 引入 `@tanstack/solid-virtual`（§15.4）；增量 upsert |
| F-M2 | `components/Menu/FloatingMenu.tsx:11-18` | `ICON_MAP` 6 个写死图标 | 图标名→组件映射可注册化 |
| F-M3 | `components/Settings/SettingsDialogContent.tsx:16,23-27` | sections 硬编码 3 tab | 由设置贡献模型生成 tab（配合阶段 6 Config host） |
| F-M4 | `components/RightWorkspace/BuiltinRightWorkspaceContent.tsx` + `right-workspace-menu.ts` | `BUILTIN_VIEW_IDS`(7)/`BUILTIN_RIGHT_WORKSPACE_PANELS`(4) 硬编码双源 | 与扩展 hostViewInstances 统一到一个声明源 |
| F-M5 | `router/index.tsx` + `stores/view-navigation.ts:4-18` | 路由单文件内联、`BuiltinAppView` 仅 3 个 | 预留扩展 view 路由表（`{extId}:{viewId}` 命名空间） |

### 15.2 后端改造清单（审计 `src-tauri/`）

#### 🔴 阻断级（P0）

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| B-P0-1 | `extension/lifecycle/state.rs:707-711` + `extension/models.rs:142` | `contributes.middlewares` 声而不用：Gateway 管道已实现（mod.rs:1731 `add_middleware`、middleware.rs 完整），扩展声明入口被 reject_unbound 砍掉 | 接入 `GatewayPipelineConfig::add_extension`（ai/gateway/middleware.rs:196 已预留），从 reject 移除。归属：本设计阶段 7 |
| B-P0-2 | `extension/lifecycle/state.rs:712-716` + `tool/mcp/transport/mod.rs:32-41` | `contributes.transport_adapters` 声而不用：MCP 远程传输（SSE/WS/REST/gRPC）需真实 adapter，`ServerManager::register_transport`（server_manager.rs:379）已可编程注册，入口被砍 | 接入 `register_transport`，从 reject 移除。归属：本设计阶段 7 |
| B-P0-3 | `foundation/storage/kv.rs` | 扩展 KV 仅测试使用，无生产出口，扩展无全局状态能力 | 按 §2.5 新建 `ExtensionStore` facade（extension 域自持三态 KV），foundation/storage 只留落盘原语；暴露 `ui_extension_storage_*`。归属：本设计阶段 8 |

#### 🟠 重要级（P1）

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| B-P1-4 | agent 流式全链路（`session_message_stream.rs` 等 51 处 `send_channel_value`） | 每 token/delta 裸 Channel 推送，无节流 | **保持实时不节流（用户裁决）**；不引入 ThrottledEmitter。优化方向：按需订阅（无订阅者零推送）+ 背压（丢弃+计数）。归属：本设计阶段 1 前置 |
| B-P1-5 | `ui/runtime/agent_tool_loop.rs`（~390 行）+ `session_message_stream.rs`（~970 行） | 巨型函数职责过载（工具循环/审批/捕获/发射 5+ 职责） | 按设计迁往 application/use-case 层，拆分成独立步骤 |
| B-P1-6 | `session_change_capture.rs:56,99`、`instruction_resolver.rs:54`、`compression_template.rs:55,63` | 同步 `fs::read_to_string` 跑在 async 热路径 | 包 `spawn_blocking` 或改 async 读 |
| B-P1-7 | `extension/provider_validation.rs:198` | `ExtensionProviderValidationRegistry` 自建 `RwLock<HashMap>` = Kernel 平行原语违规 | 改用 kernel `InMemoryRegistry`，或降级为纯校验器（无注册表状态） |

#### 🟡 次要级（P2）

| # | 位置 | 问题 | 方案 |
|---|------|------|------|
| B-P2-8 | `ui/lsp.rs` | 命令 `lsp_` 前缀不一致（应为 `ui_`） | 统一前缀（同步更新前端调用） |
| B-P2-9 | `ui/gateway.rs:384` | `ui_discover_gateway_models` 每调用新建 reqwest client | 复用共享 client |
| B-P2-10 | `foundation/storage/mod.rs` | Storage 上帝对象 + 单 Mutex 串行；AuthStore 独立连接碎片化 | 继续 MemoryStore/SessionStore facade 抽取；收敛 AuthStore 连接 |
| B-P2-11 | `ui/runtime/agent_tool_loop.rs:146` | `AGENT_TOOL_LOOP_MAX_ROUNDS=24` 硬编码 | 提为配置/常量声明 |
| B-P2-12 | `extension/lifecycle/state.rs:936-953` | triggers disable 有处理但 enable 拒绝（死代码） | 清理或补接线（对齐 §10.2 triggers 行） |

#### 后端确认合规（不改）

Auth secret_ref 零明文、Sandbox/Policy 约束链、kernel-backed registry facade、Tauri 事件单桥接、MCP 熔断重试、terminal spawn_blocking。

### 15.3 设计文档修复清单（审计 `design/`）

#### 🔴 文档损坏（立即修复）

| # | 文档 | 问题 | 方案 |
|---|------|------|------|
| D-P0-1 | `09-file.md` | 全文"文→件"字面损坏（文件系统→件件系统、文本→件本） | **已修复**（2026-08-16："件件"→"文件"、"件本"→"文本"，全量替换） |
| D-P0-2 | `24-dialog.md` | 全文"p→e"字面损坏（Comeoser、eermission、tyeescriet 等 56+ 匹配） | **已修复**（2026-08-16：精确词表替换还原 Composer/permission/approval/input/option/typescript 等） |
| D-P0-3 | `34` §10.2 | state.rs 行号系统性偏移 85-110 行（引用 do_disable 区间） | **已修复**（2026-08-16，改函数名+特性名为主） |

#### 🟠 语义冲突（裁决统一）

| # | 文档 | 冲突 | 裁决 |
|---|------|------|------|
| D-P1-1 | `18-context-manager.md` L322/L351 vs `08-session.md` L189-190、`04-storage.md` L148-149 | 上下文压缩：18 说"删除 old_messages"，08/04 说"软压缩不删原始" | **已修复**（2026-08-16：18 §7.2 流程图 + §7.3 代码改为 compacted_ranges 软压缩标记，不删原始） |
| D-P1-2 | `28-i18n.md` L222-230 vs `34` §3.1 | i18n 数据结构：28 顶层 `"i18n": "./i18n"`（扩展目录）vs 34 `contributes.i18n = Option<Vec<I18nResource>>` | **已修复**（2026-08-16：28 统一为 34 的 `contributes.i18n` 结构，目录改 `ExtensionUI/locales/`）；`07-extension.md` 需同步补入字段 |
| D-P1-3 | `28-i18n.md` 头部 | 标题/编号写「# 32 - i18n」但文件名是 28（与 32-clipboard 撞号） | **已修复**（2026-08-16：标题/编号改回 28，依赖补 07/34） |
| D-P1-4 | `27-hotkey.md` L175 | 「Extension 模块调用 hotkey.register()」——代码中不存在 | **已修复**（2026-08-16：改写为公共投影模型 ui_list_extension_keybindings + 动作族分发） |
| D-P1-5 | `25-notification.md` | 用 `contributes.notificationChannels`（camelCase）+ 可执行 JS 模块路径，07 用 `notification_channels`（snake_case）且禁止扩展 JS 运行时 | **已修复**（2026-08-16：改 snake_case，去 module 路径，注明宿主提供发送实现） |

#### 🟡 索引与一致性

| # | 文档 | 问题 | 方案 |
|---|------|------|------|
| D-P2-1 | `design/README.md` | 进度「30/30」与实际 33 份不符；模块表「合计 31」与明细 32 不符；33/34 未列入模块表 | **已修复**（2026-08-16：进度 33/33、合计 32、33/34 已列入） |
| D-P2-2 | `design/README.md` | 树中 `modules/` 目录实际不存在 | **已修复**（2026-08-16：删除占位并加注说明） |
| D-P2-3 | `20-rag-knowledge.md` | 命名漂移（README 标「RAG/本地知识库」，文档标题「Knowledge 项目知识管理」） | **已修复**（2026-08-16：模块名统一为「Knowledge 项目知识管理」，RAG 作为检索机制名保留，README 树/模块表已同步；18 号代码字段 rag_* 保留不动） |
| D-P2-4 | `navis-agent-flow-optimized.md` L946-947 | 16-agent / 22-ui-framework 仍含「worktree tree 每轮注入」过时描述 | **已复核无需修改**：16-agent L217 已声明"不自动注入 worktree tree"，22-ui-framework 已无注入描述；L35/216/220/413 的 worktree_root 注入是运行期元数据（正确） |
| D-P2-5 | 非编号文档（kernel.md/analysis.md/DESIGN.md 等） | 未纳入 README 索引；analysis「Extension 16 types」疑似过时 | **已修复**（2026-08-16：README 新增"辅助文档"章节收录 4 份非编号文档并标注 analysis 过时；DESIGN.md 属视觉规范、README 已有指引） |

### 15.4 高性能依赖引入清单（遇瓶颈即引入，替代自造方案）

> 铁律：**优先引入成熟依赖**，不自造轮子。以下为审计确认的候选，引入需在 §15 登记并更新 Cargo.toml / package.json。

| 场景 | 依赖 | 替代的自造方案 | 触发条件 | 归属 |
|------|------|---------------|---------|------|
| 全文/代码搜索 | `ripgrep`（rg crate） | 自写目录扫描 + 符号索引 | 现有搜索实现被审计为瓶颈时（当前未审计出热路径，标记观察） | Tool 域 |
| 大列表滚动 | `@tanstack/solid-virtual` | 手写虚拟滚动 | F-M1（ChatMessages 虚拟化）实施时直接引入 | 前端 |
| 并发容器 | `dashmap` | 全局 `Mutex<HashMap>` | 出现锁竞争热点（当前 StreamIndex 每 chunk 写锁，若优化按需订阅后可评估） | foundation |
| 缓存 | `moka` | 自写缓存/重复查询 | Discovery query 缓存 / Gateway 模型目录缓存实施时 | foundation / AI |
| 并行批处理 | `rayon` | 手动线程池 | 出现 CPU 密集批量任务时 | 按需 |
| 前端模糊搜索 | `flexsearch` | 手写 fuzzy 匹配 | CommandPalette 规模增长时 | 前端 |

> 注意：Agent 流、Store 投影同步、iframe 生命周期均为架构手段，**不**引入节流依赖（用户裁决实时性优先）。
