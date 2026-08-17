# 37 — 组件化扩展执行基座（Cordis × Component-as-Extension）

> 状态：详设（终局架构，决策基线）
> 日期：2026-08-16
> 2026-08-17 修订：扩展框架统一为 Cordis；WASM 组件模型降为固定目录内的隔离逻辑执行适配器
> 编号：37
> 前置：35-whiteboard-container.md（容器分层）、34-extension-ui-open-architecture.md（UI 开放架构 / worker 轨裁决）、36-extension-development.md（扩展开发手册，需反哺）
> 目标：以 Cordis 作为扩展组合/服务容器/生命周期底座；WASM 组件模型作为 `ExtensionUI/` / `ExtensionBackend/` 内隔离逻辑执行适配器，只承接可编译为 WASM 的逻辑。UI 保持 iframe 沙箱；native 重型扩展走协议子进程逃生舱。
> 对齐约束：**高内聚**（Cordis 统一装配）、**低耦合**（Cordis typed service + WIT 接口 + host function 门禁）、**符合开发规范**（固定目录 ExtensionUI/ExtensionBackend）、**万物皆扩展**（Agent/AI 平台/垂直业务全部扩展化）。

---

## 一、架构裁决（定稿）

1. **扩展框架 = Cordis**：后端使用 `cordis-rs`（`cordis::Context/Plugin/Service/Inject/Fiber`），前端宿主使用 `@cordisjs/core`。所有扩展点先成为 Cordis plugin/service，再由 capability port 进入宿主域。Cordis 是唯一装配与生命周期底座，不自研平行扩展框架。
2. **WASM 组件模型 = 隔离逻辑执行适配器**：只有可编译为 WASM 的逻辑（前端逻辑组件 `ExtensionUI/scripts/*.wasm`，后端逻辑组件 `ExtensionBackend/logic/*.wasm`）进入容器内 wasmtime 执行。它替代“进程内 JS 框架的 ctx/service DI”中与宿主强耦合的执行部分，不替代 Cordis 的插件/服务装配。引擎/标准/工具链使用生态成熟件（wasmtime、W3C WASM Component Model、WIT、wit-bindgen、wasm-tools）——**不自研引擎**。
3. **host function 是组件唯一出站通道**：组件不能直接触碰宿主能力；一切能力经宿主授予的 host 接口实现，门禁在容器，**不可绕过**。未授权接口 = 无能力（fail-closed）。
4. **UI 永不 WASM 渲染**：iframe 沙箱是唯一渲染面（物理约束）。前端扩展 = iframe 视图 + 容器内逻辑组件；高频数据（Agent 流）容器→UI 直连，不绕组件。
5. **逻辑统一，native 逃生舱是特例**：可编译为 WASM 的扩展逻辑一律组件轨；native 重型（OS 调用 / USB / 打印机 / 自身是 server）走协议子进程（`ExtensionBackend/native/`，jsonrpc stdio/SSE/WS，复用 transport_adapters）。逃生舱是**特例，非主路径**。
6. **组合 = Cordis service 注入 + 组件动态链接**：扩展间声明式依赖经 manifest 与 WIT 接口解析；宿主组合注册表统一校验，双端授权（类比 ExtRouter）。
7. **契约层是唯一真理源**：manifest（插件元数据/能力声明）+ Cordis service interface + WIT（组件接口）定义扩展面；Sandbox / OperationRegistry（门禁）定义容器面。三层不重叠、不绕过。

---

## 二、分层结构

```
┌─ 扩展层（业务，可替换，同一契约）─────────────────────────────────┐
│  前端扩展（ExtensionUI/）：iframe 视图 + 前端逻辑组件 .wasm          │
│    navis-session / navis-editor / navis-terminal（UI 部分）        │
│  逻辑扩展（组件轨，容器内执行）：                                   │
│    navis-agent-core（Agent 引擎组件）                              │
│    navis-ai-platform（Gateway/MCP/LSP/Skills 逻辑组件）            │
│    垂直业务组件（柜面/双录逻辑）                                    │
│  native 扩展（ExtensionBackend/native，逃生舱）：                  │
│    需 OS 能力 / 自身是 server 的后端（协议子进程）                  │
├─ 容器层（Cordis extension runtime + WASM adapter，领域无关，固化）──┤
│  Cordis Context / Plugin / Service / Inject / Fiber               │
│  WASM 组件运行时（wasmtime 托管：实例化/配额/缓存/组件链接）         │
│  host function 门禁（唯一出站通道，复用 Sandbox/审批/审计）          │
│  契约映射（manifest ↔ Cordis service ↔ WIT ↔ 接口授予）            │
│  组合注册表（组件动态链接，双端授权）                               │
│  UI surface 投影（iframe/zone/menu/dialog/command）               │
│  native 子进程管理器（ExtensionBackend 逃生舱 spawn/kill）          │
│  Tauri IPC / EventBus / 存储 / 流 / 网络策略 / 沙箱                │
└──────────────────────────────────────────────────────────────────┘
```

---

## 二A、Cordis 核心 API 映射

| 概念 | cordis-rs（后端） | 前端宿主（@cordisjs/core） |
|------|-------------------|----------------------------|
| 服务容器 | `Context::new/extend/isolate` | `new Context()` / `ctx.extend()` |
| 插件 | `Plugin` / `plugin_sync` / `plugin_async` | `{ apply(ctx) }` |
| 服务依赖 | `#[derive(Inject)]` / typed `get/require/set/provide` | `ctx.inject()` / `ctx.set()` |
| 生命周期 | `Fiber` / `FiberState` / `effect` disposer | `ctx.plugin()` child fiber / disposer |
| 事件 | `emit/parallel/serial/bail/waterfall` | 宿主事件投影（不建平行总线） |

> `cordis-rs` 不提供 loader/include/HMR；扩展发现、manifest 校验、入口装载和热更均由宿主 loader 提供。

---

## 三、目录规范（固定目录，定稿）

```
extensions/{id}/
├── extension.json                     # manifest（唯一入口）
├── ExtensionUI/                       # ★ 前端扩展面：全部前端代码
│   ├── index.html                     #   html:sandbox 视图入口
│   ├── assets/                        #   静态资源
│   ├── scripts/                       #   前端逻辑组件（.wasm，容器内执行）
│   │   └── app.component.wasm         #   由 JS/TS/Rust 经 wit-bindgen 编译
│   └── locales/                       #   i18n 资源
├── ExtensionBackend/                  # ★ 后端扩展面：全部后端扩展点代码
│   ├── logic/                         #   后端逻辑组件（.wasm，容器内执行）
│   │   └── worker.component.wasm
│   └── native/                        #   native 逃生舱（协议子进程）
│       └── my-server[.exe]            #   容器 spawn 的可执行文件
└── data/                              # 静态数据（可选）
```

### 3.1 目录语义

- **前端代码统一在 `ExtensionUI/`**：html:sandbox 入口、assets、前端逻辑组件、i18n 全部在此。**废弃 `ui/` 别名**（兼容期两者皆可接受，最终单一化为 `ExtensionUI/`）。
- **后端扩展点代码统一在 `ExtensionBackend/`**：后端逻辑组件（`logic/`）+ native 可执行文件（`native/`）。
- **目录归属（扩展点语义）≠ 执行位置**：前端逻辑组件归属 `ExtensionUI/`，但**执行在容器内组件轨**（它服务的是前端 UI，不是跑在浏览器）；仅 `ExtensionBackend/native/` 是独立进程。
- **命名约束**：扩展目录名 = manifest id；entry 为相对路径、`/` 分隔、禁 `..`/`\`/控制字符；组件 entry 须位于 `ExtensionUI/` 或 `ExtensionBackend/` 下（loader 校验）。

---

## 四、WIT 接口契约

### 4.1 host 接口（容器提供，组件消费）——唯一出站通道

```wit
package navis:host;

/// 受控操作执行（门禁在容器，对应 runtime.operation.execute）
interface operation {
    execute: func(op: operation-request) -> result<value, string>;
    list:    func() -> list<operation-description>;
}
interface context {
    get-session:        func() -> result<session-snapshot, string>;
    get-active-project: func() -> result<project-snapshot, string>;
}
interface storage {
    get:    func(key: string, scope: string) -> result<option<value>, string>;
    set:    func(key: string, value: value, scope: string) -> result<_, string>;
    delete: func(key: string, scope: string) -> result<_, string>;
}
interface network {
    fetch: func(request: http-request) -> result<http-response, string>;
}
interface event {
    subscribe: func(pattern: string) -> result<subscription, string>;
    emit:      func(topic: string, payload: value) -> result<_, string>;
}
interface log {
    write: func(level: log-level, message: string) -> result<_, string>;
}
```

**能力授予**：组件在 manifest 声明 `components[].capabilities`，容器实例化时**按声明映射注入对应 host 接口实现**；未声明 = 不注入该接口 = 调用即失败 + 审计（fail-closed）。

### 4.2 扩展组件导出接口

```wit
package navis:ext;
interface lifecycle {
    init:       func(handle: host-handle) -> result<_, string>;  // enable 时
    activate:   func() -> result<_, string>;                     // 激活时
    deactivate: func() -> result<_, string>;                     // disable/失活时
}
interface message {
    handle: func(payload: value) -> result<value, string>;       // 宿主/其他组件消息
}
```

### 4.3 组件间组合

- 依赖方：manifest `components[].dependencies: [{ target, interface }]`（声明"我要用谁"）。
- 导出方：manifest `components[].exports: [{ interface }]`（显式导出，类比 `extension_exports`）。
- 运行时：组件动态链接由宿主组合注册表解析；**双端授权**——任一端未声明即拒绝 + 审计（fail-closed，类比 ExtRouter）。

---

## 五、Manifest 契约

### 5.1 新增字段

```json
{
  "contributes": {
    "components": [
      {
        "id": "app",
        "entry": "ExtensionUI/scripts/app.component.wasm",
        "kind": "logic",
        "runOn": ["activation", "message"],
        "capabilities": {
          "invoke": ["operation.execute", "context.getSession"],
          "storage": ["global"],
          "network": { "type": "allowlist", "hosts": [{ "host": "api.example.com" }] },
          "events": ["session.completed"]
        },
        "autostart": false
      }
    ],
    "backendServices": [ ]              // native 逃生舱（wire key 修正为 camelCase）
  }
}
```

- `components[].entry`：`.wasm` 相对路径，须位于 `ExtensionUI/` 或 `ExtensionBackend/` 下。
- `kind`：`logic`（容器内组件轨）| `native`（逃生舱，走协议子进程，可省组件字段改走 `backendServices`）。
- `capabilities`：声明式能力白名单 → 映射为 host 接口授予。

### 5.2 与既有 contributes 的关系（演进裁决）

| 既有 | 去向 |
|------|------|
| `scripts`（worker 轨） | **被 `components[kind:logic]` 取代**；浏览器 worker 仅保留给纯前端胶水（可选） |
| `backend_services` | **保留**，专指 native 逃生舱；wire key 修正为 `backendServices`（P0） |
| `views` / `menus` / `commands` / `keybindings` | 不变（UI 声明式，投影到 iframe/zone/menu） |
| `mcp_servers` / `tools` / `languages` / `skills` / `gateway.*` | 不变（宿主注册，组件经 host function 消费） |
| `event_subscriptions` | 保留，改经组件 `event` host 接口订阅 |

---

## 六、生命周期

### 6.1 组件轨（容器内，Cordis Fiber 托管）

```
Cordis plugin apply(ctx)
  → declared（校验 manifest + WIT，加载 .wasm，编译缓存）
  → instantiated（实例化 + 按 capabilities 注入 host 接口实现）
  → activated（调用 activate；enable 流程 commit 时）
  → suspended（不可见/无消息：冻结或保留实例）
  → disposed（disable/卸载：deactivate → 回收实例 → 释放接口）
```

- 与现有 `families.rs` 事务式 commit/rollback 对齐：新增 `ComponentFamilyHandler`（preflight/normalize/validate/prepare/commit/rollback/disable）；组件实例作为 Cordis `Fiber` 子生命周期注册，dispose 时随插件 Fiber 一并回收。
- enable 原子性：任一组件实例化失败 → 回滚已实例化组件并释放接口。

### 6.2 native 逃生舱

- 复用 `BackendProcessManager`（已落地）：enable 时按 `autostart` spawn，disable/卸载 kill_all。
- 协议：jsonrpc-over-stdio/SSE/WS（复用 transport_adapters 契约），不新造。

---

## 七、安全模型

1. **不可信组件**：wasmtime 内存/trap 隔离；组件崩溃不波及容器（trap → 记录 + 降级）。
2. **能力 = 接口授予**：未声明的 host 接口不注入实现，调用 fail-closed + 审计（capability-based）。
3. **host function 门禁**：每个出站调用构造 `OperationRequest{actor:"extension:{id}"}` 过 Sandbox（复用 permission.rs 分级/审批/审计），`require_allowed` 拒绝即 fail-closed（不弹确认）。
4. **资源配额**：组件实例内存上限 + fuel（CPU 计量）+ 超时，对齐 resource_limit.rs；超限即终止 + 审计。
5. **网络 fail-closed**：未声明 network 不注入 fetch；allowlist/proxy 复用网络策略 + SSRF 防护。
6. **组合双端授权**：组件链接任一端未授权即拒绝（fail-closed）。
7. **UI iframe**：`sandbox="allow-scripts"` 无 `allow-same-origin`，白名单桥，与组件轨经宿主桥组合。

---

## 八、实施阶段与验收

### 阶段 C0：契约与目录地基（先行）

> **状态（2026-08-17）**：C0-1~C0-4 契约/目录已落地。但 C0-0（Cordis 基座接入）与 C1（执行基座原型）的**接线未完成**——`extension/context.rs` 的 `HostExtensionContext` 已创建未接线（死代码，无 `set_service`/`install_extension` 调用点），`ComponentRegistry` 已建未接执行链路。**接线任务收敛到 35 阶段 D（D1 装配接线 / D2 组件轨接线 / D3 事件订阅落地）**，本文档 C1 以下阶段以其为前置。

| # | 改动 | 说明 |
|---|------|------|
| C0-1 | `backendServices` wire key 修正（demo / 35 / 36 反哺） | P0：snake_case 被 serde 静默丢弃，扩展启用零报错 |
| C0-2 | `resource.rs` 目录归一为 `ExtensionUI`（弃用 `ui/`） | P0：校验层接受 ExtensionUI、服务层只认 `ui/`，运行态失败 |
| C0-3 | manifest 新增 `components` 字段 + loader 校验（entry 位于 ExtensionUI/ 或 ExtensionBackend/） | 契约层 |
| C0-0 | Cordis 扩展基座接入（`cordis-rs` + 前端 `@cordisjs/core`），loader 装载为 plugin/service | **契约已具、接线见 35 D1** |
| C0-4 | 目录规范定稿（ExtensionUI/ExtensionBackend）+ 34 §2.3 worker 轨裁决更新 | 文档 |

### 阶段 C1：执行基座原型

| # | 改动 | 说明 |
|---|------|------|
| C1-1 | wasmtime + 组件模型接入；`ComponentRegistry` + `ComponentFamilyHandler` | 容器壳 |
| C1-2 | host 接口实现（operation/context/storage/network/event/log） | host function 门禁复用 OperationRegistry / Sandbox |
| C1-3 | 原型组件：navis-demo 逻辑迁 `.wasm`，暴露 `runtime.operation.execute` 为 host function | 验收五项：隔离/延迟/内存/分发/调试 |

### 阶段 C2：逻辑收敛

| # | 业务 | 迁组件轨 |
|---|------|----------|
| C2-1 | navis-agent-core（Agent 引擎） | 35 C4 迁移目标改组件轨 |
| C2-2 | 轻量后端逻辑（数据处理/编排） | `ExtensionBackend/logic/*.wasm` |
| C2-3 | AI 平台服务逻辑（Gateway/MCP/LSP/Skills） | 渐进组件化 |

### 阶段 C3：UI 桥组合

| # | 改动 |
|---|------|
| C3-1 | iframe 视图 ↔ 前端逻辑组件经宿主桥组合（UI 渲染 + 组件逻辑） |
| C3-2 | Agent 高频流容器→UI 直连（不绕组件） |
| C3-3 | `__NAVIS__` 桥与组件轨统一（同一门禁，两种消费面） |

### 阶段 C4：native 逃生舱定型

| # | 改动 |
|---|------|
| C4-1 | `BackendProcessManager` 降级为逃生舱专用（协议子进程，transport_adapters 契约） |
| C4-2 | 组件无法编译的场景（OS/USB/打印机/自身是 server）显式走 native |

### 阶段 C5：验收

| # | 验收 |
|---|------|
| C5-1 | `cargo check` + 组件轨/契约层测试全绿 |
| C5-2 | 性能：host function 调用延迟（ns 级）对比现状桥/stdio 基线 |
| C5-3 | 隔离：恶意组件 trap / 越权被门禁拒绝，宿主不崩 |
| C5-4 | 领域无关：一个非 AI 组件扩展在容器上可用，不依赖 AI 扩展 |
| C5-5 | 目录合规：全部扩展点位于 ExtensionUI/ExtensionBackend，`ui/` 别名废弃 |

---

## 九、约束铁律

1. **Cordis 是唯一扩展框架**：不引入第二扩展框架；WASM 只是逻辑执行适配器。
2. **逻辑统一组件轨**：不引入第二执行模型（除 native 逃生舱特例）。
2. **host function 是唯一出站通道**：门禁永在容器（Sandbox / OperationRegistry / 审批 / 审计）。
3. **UI 永不 WASM 渲染**：iframe 是唯一渲染面。
4. **组合双端授权**：fail-closed。
5. **目录固定**：前端 `ExtensionUI`，后端 `ExtensionBackend`；`ui/` 别名仅兼容期。
6. **不自研引擎/标准/工具链**：容器只做 Cordis 装配、契约映射 / 门禁 / 生命周期 / 组合。
7. **反哺规则**：本详设落地后，34 §2.3 worker 轨、35 C4 迁移目标、36 §三/十二 目录规范均须反哺更新（36 是开发手册权威，不一致须回写）。

---

## 十、风险与缓解

| 风险 | 缓解 |
|------|------|
| 组件模型成熟度（wasip2/WIT/调试） | C1 原型五项实测后再扩；保底为 Extism wasip1 近似态 |
| JS 逻辑让渡原生产物 | 组件可编译自 JS（Javy / extism-js），接受体积/性能折损；纯胶水保留浏览器 worker |
| 调试能力弱 | 调试符号管线（wasm-tools + sourcemap），作为长期工程投入 |
| 组合授权安全 | 双端授权 + 宿主审计 + 可撤销（禁用任一端即断链） |
| 能力声明与门禁职责混淆 | 组件 `capabilities` 是唯一能力声明；Sandbox/OperationRegistry 是门禁执行，二者不重叠 |
| 既有贡献面兼容 | C0 先行；scripts 轨替换期双轨并存，36 反哺记录 |

---

## 十一、文档关系

- 前置：35（容器分层 / 目录规范）、34（UI 开放架构 / worker 轨）、36（开发手册）。
- 取代：34 §2.3 worker 轨裁决（浏览器统一运行时 → 容器组件轨为主，浏览器 worker 降为胶水可选）；35 C4 Agent 迁移目标改组件轨；36 §三/十二 目录规范更新为 ExtensionUI/ExtensionBackend。
- 反哺：36 需新增"组件轨开发"章节（WIT 接口、wit-bindgen、组件编译、host function 消费）。
- **变更记录**：v1（2026-08-16）定稿：基于 34/35/36 及三轮框架选型（Cordis/Xi/Extism）确定固定目录 ExtensionUI/ExtensionBackend。
- v2（2026-08-17）修订：扩展框架统一为 Cordis（`cordis-rs` / `@cordisjs/core`），WASM 组件模型降为 `ExtensionUI/` / `ExtensionBackend/` 内的隔离逻辑执行适配器，移除“自研壳”表述。
