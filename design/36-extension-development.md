# 36 — 扩展开发手册（Extension Development Guide）

> 目录事实同步（2026-08-18）：仓库开发期扩展路径为 `extensions/<product>/<extension-id>/`，Navis Code 扩展位于 `extensions/navis-code/<extension-id>/`；运行时安装路径为 `<app_data>/extensions/<extension-id>/`。`src/` 和 `src-tauri/src/` 只允许通用宿主能力。

> 状态：开发指南（权威、需反哺）
> 日期：2026-08-16（v2：反哺 37 详设，目录规范归一 ExtensionUI/ExtensionBackend，新增组件轨）
> 日期：2026-08-17（v3：扩展开发模型统一为 Cordis plugin；manifest 是插件元数据，contributes 是能力声明，loader 将固定目录中的扩展点装载为 Cordis plugin/service）
> 编号：36
> 前置：37-component-extension.md（组件化扩展执行基座）、35-whiteboard-container.md（白板容器架构）、34-extension-ui-open-architecture.md（UI 开放架构）、07-extension.md（manifest/生命周期）、02b-stream.md（流）
> 读者：扩展开发者（含 Navis 官方业务扩展）
> 约束：本手册是扩展开发的**唯一权威**。开发中若出现与手册不一致的实现，必须**反哺回本手册**（更新手册后修代码，二者保持一致）。执行基座裁决以 37 详设为纲。

---

## 一、扩展是什么

扩展 = 一个目录，含 `extension.json`（manifest）+ `ExtensionUI/`（全部前端扩展点代码）+ `ExtensionBackend/`（全部后端扩展点代码）+ 可选静态数据。Navis Go 是**白板容器**，所有业务（会话、任务、项目、知识库、Agent 引擎……）都以扩展形式承载。宿主不区分内置与第三方扩展——同一契约。

每个扩展点由 loader 装载为 Cordis plugin/service；`manifest` 是插件元数据，`contributes` 是能力声明，二者不是运行时执行器。

一个扩展可以贡献：
- **视图**（`views`）：渲染到 zone 的 UI（host:panel 声明式 或 html:sandbox 自由 HTML）
- **菜单/命令/快捷键**（`menus`/`commands`/`keybindings`）
- **逻辑组件**（`components`）：WASM 组件轨，容器内 wasmtime 执行，经 host function 门禁（37 §五/§六，逻辑扩展首选）
- **逻辑脚本**（`scripts`）：Web Worker 轨，`__NAVIS__` 白名单桥（仅纯前端胶水；重逻辑一律走 `components`）
- **能力**（`capabilities`）：invoke 白名单、事件订阅、KV 存储、网络、跨扩展调用
- **平台服务**（`mcp_servers`/`gateway.*`/`languages`/`skills`/`roles`/`themes`/`tray_items`/`notification_channels`）
- **扩展点**（`toolbar_items`/`statusbar_items`/`inline_extensions`/`configuration`/`zones`/`layout_overrides`）

---

## 一A、Cordis 插件开发模型

### 固定目录契约（铁律）

- 前端扩展点代码全部位于 `ExtensionUI/`：`index.html`、assets、`scripts/`（前端胶水 worker 或前端逻辑组件 `.wasm`）、`locales/`。
- 后端扩展点代码全部位于 `ExtensionBackend/`：`logic/*.wasm`（容器内组件轨）与 `native/*`（协议子进程逃生舱）。
- `ui/` 别名、顶层 `scripts/` 已废弃；loader / `host_view` / `resource` 只接受固定目录。

### Cordis 装配模型

| 概念 | 设计语义 | 示例 |
|------|----------|------|
| `manifest` | 插件元数据 | id/name/version/main |
| `contributes` | 能力声明 | views/menus/commands/components/backendServices |
| `Context` | 类型化服务容器 | 后端 `cordis::Context`，前端 `@cordisjs/core` 的 `ctx` |
| `Plugin` | 扩展生命周期单元 | `apply(ctx)`，在 `Context` 中声明/获取服务 |
| `Service` | 跨插件共享的类型化服务 | GatewayPort / HostViewRegistry / ExtensionStorage |
| `Inject` | 显式服务依赖 | `#[derive(Inject)]` / 前端 `ctx.inject()` |
| `Fiber` / `effect` | 生命周期与副作用回收 | 启用时创建资源，禁用/dispose 时回收 |

loader 的职责：读取 `extension.json` → 校验 `contributes` → 把 `ExtensionUI/` / `ExtensionBackend/` 中声明的入口映射为 Cordis plugin/service → 注入到 `Context` → 由 `Fiber` 管理 enable/disable 与 rollback。Cordis 不实现 loader、include、HMR；这些由宿主 loader 提供。

> **装配状态（2026-08-17）**：Cordis 完整装配接线已落地（35 阶段 D1/D2/D3/D4 全部完成）——`components`/`event_subscriptions`/`backendServices` 的运行时执行均经 Cordis fiber 统一装配（`apply_extension_fiber` + `take_extension_fiber`），capability port 经 `register_capability_service` 注册为 Cordis service（含 `agentLoop` seam），WASM 组件轨经 `ComponentRegistry.load` → `ActiveComponent` → `handle_message` 路由，事件订阅经 `KernelEventSubscriptionAdapter::subscribe_declared/unsubscribe_all` 落地。全量 `cargo test` 2143 passed（仅 3 预存在 knowledge 环境失败）。**边界铁律见 35 §2.5（地基 vs 扩展判别标准）**。

### 扩展开发顺序

1. 在 `extensions/<product>/<extension-id>/` 下创建 manifest，目录名等于 `id`。
2. 前端代码放入 `ExtensionUI/`，后端扩展点放入 `ExtensionBackend/`。
3. 在 `contributes` 中声明能力；loader 会将每条声明映射为 Cordis service 或宿主 projection。
4. 逻辑优先使用 `components`（WASM 组件轨），仅纯前端胶水使用 `scripts`（Web Worker）。
5. 未接线的 `contributes` 必须 fail-closed，不得静默忽略。

---

## 二、Manifest 参考（extension.json）

```json
{
  "id": "com.example.myext",
  "name": "My Extension",
  "version": "1.0.0",
  "description": "示例扩展",
  "author": "example",
  "main": "ExtensionUI/index.html",
  "contributes": {
    "views": [
      {
        "id": "my-panel",
        "name": "My Panel",
        "zone": "rightWorkspace",
        "renderer": "host:panel",
        "config": {}
      },
      {
        "id": "my-html",
        "name": "My HTML",
        "zone": "chatAside",
        "renderer": "html:sandbox",
        "entry": "ExtensionUI/index.html",
        "config": {}
      }
    ],
    "menus": [
      { "id": "my-menu", "target": "Tools", "group": "ext", "when": "activeSession",
        "action": { "type": "OpenView", "view": { "extensionId": "com.example.myext", "viewId": "my-panel" } } }
    ],
    "commands": [
      { "id": "my.command", "title": "Run my command" }
    ],
    "keybindings": [
      { "id": "my-kb", "command": "my.command", "keys": ["ctrl+shift+m"] }
    ],
    "scripts": [
      { "id": "worker-main", "entry": "ExtensionUI/scripts/main.js", "runOn": ["activation"] }
    ],
    "components": [
      {
        "id": "app",
        "entry": "ExtensionUI/scripts/app.component.wasm",
        "kind": "logic",
        "runOn": ["activation", "message"],
        "capabilities": {
          "invoke": ["operation.execute", "context.getSession"],
          "storage": ["global"]
        },
        "autostart": false
      }
    ],
    "capabilities": {
      "invoke": ["file.read", "context.getSession", "extensions.query"],
      "events": ["session.completed", "project.*"],
      "read": ["context:session", "context:project"],
      "extensionCalls": [
        { "target": "com.example.other", "actions": ["view.open", "command.execute"] }
      ],
      "provides": ["stream", "my-feature"],
      "network": { "type": "allowlist", "hosts": [{ "host": "api.example.com", "allowSubdomains": true }] }
    },
    "storage": { "scopes": ["global", "worktree"] },
    "toolbarItems": [{ "id": "my-tool", "label": "MT", "icon": "tool", "command": "my.command" }],
    "statusbarItems": [{ "id": "my-status", "label": "READY", "command": "my.command" }],
    "inlineExtensions": [{ "id": "my-inline", "target": "Chat", "command": "my.command" }],
    "configuration": {
      "type": "object",
      "properties": {
        "theme": { "type": "string", "enum": ["light", "dark"], "default": "dark" }
      }
    },
    "zones": [{ "id": "my-zone", "name": "My Zone", "anchor": { "parent": "rightWorkspace", "position": "below" } }]
  }
}
```

### 字段语义

| 字段 | 类型 | 说明 |
|------|------|------|
| `views[].zone` | string | 内置 zone（`rightWorkspace`/`chatAside`/`bottomDrawer`/`settingsSection`）或扩展 zone `{extId}:{zoneId}` |
| `views[].renderer` | `host:panel` / `html:sandbox` | host:panel 不声明 entry；html:sandbox 必须声明 `ExtensionUI/` 下相对 entry |
| `menus[].target` | string | 内置 target（`Tools` 等）或 `{extId}:{target}` |
| `menus[].action` | 动作族 | `OpenView`/`ToggleView`/`OpenDialog`/`RunScript`/`SendMessage` |
| `scripts[].runOn` | array | `activation`（启用自动触发）、`message`、`view-open`、`worker-spawn` |
| `components[].entry` | string | `.wasm` 相对路径，**必须位于 `ExtensionUI/` 或 `ExtensionBackend/` 下**（37 §5.1） |
| `components[].kind` | `logic`/`native` | `logic`＝容器内组件轨；`native`＝走 `backendServices` 协议子进程 |
| `components[].capabilities` | object | 声明式能力白名单，映射为 host 接口授予（invoke/storage/network/events） |
| `capabilities.invoke` | string[] | 白名单命令；未声明即 fail-closed |
| `capabilities.extensionCalls[].actions` | string[] | `view.open`/`view.toggle`/`command.execute`/`message.send`/`event.emit`/`event.subscribe`/`*` |
| `storage.scopes` | array | `global`/`worktree`/`ephemeral` |

**fail-closed 铁律**：未声明的能力调用、未知 renderer/placement、非法 entry、未授权跨扩展调用、未声明 network 的 fetch，一律拒绝 + 审计。

---

## 三、渲染与执行轨

| 轨 | 用途 | 通信 |
|----|------|------|
| `host:panel` | 宿主声明式面板（信息展示、数据绑定） | 无需 iframe；经 `__NAVIS__` 或宿主数据投影 |
| `html:sandbox` | 扩展自由 HTML/JS UI（唯一渲染面） | 严格沙箱 iframe（`allow-scripts`，无 `allow-same-origin`），注入 `__NAVIS__` 垫片 |
| `components`（WASM） | **逻辑扩展首选**：无 UI 逻辑/后台/数据处理，容器内 wasmtime 执行 | host function（唯一出站通道），门禁在容器（37 §四/§六） |
| `scripts`（worker） | 纯前端胶水逻辑（可选） | Web Worker，注入 `__NAVIS__` |

各轨共用**同一 `__NAVIS__` 白名单桥**，权限集中在容器。`scripts` 轨仅保留给"必须贴近前端、无重逻辑"的胶水场景；**重逻辑、跨平台、需隔离的逻辑一律走 `components`（WASM 组件轨，37 详设）**。

---

## 四、`__NAVIS__` 桥 API（扩展内可用）

垫片注入到 iframe / worker，提供与宿主前端同构的 API。所有调用返回 Promise。
桥是**领域无关**的——只暴露容器通用原语；领域能力（如 AI 的 Agent 编排）由业务扩展自己实现。

### 4.1 invoke — 调白名单命令

```js
// iframe 内
const result = await window.__NAVIS__.invoke('file.read', { path: 'src/main.rs', worktree: '/abs/path' });
// worker 内
const result = await self.__NAVIS__.invoke('context.getSession');
```

**可用命令**（须在 `capabilities.invoke` 声明）：

| 命令 | 说明 |
|------|------|
| `file.read` | 读取 worktree 文本文件（Sandbox 门禁） |
| `context.getSession` | 当前会话快照 |
| `context.getActiveProject` | 当前项目快照 |
| `extensions.query` | 扩展发现（`{provides, capability}`） |
| `route.call` | 跨扩展调用（双端授权） |
| `storage.get`/`set`/`delete`/`clear` | 扩展 KV 存储 |
| `network.fetch` | 经网络策略代理的 fetch |
| `runtime.operation.execute` | 执行一个已注册的受控操作（容器门禁） |
| `runtime.operation.register` | 注册自己的 Operation 定义 |
| `runtime.operation.list` | 列出已注册操作 |

**受控操作执行（§3.2 容器原语）**：容器提供"操作执行机制"（Sandbox + 审批 + 审计），操作定义由扩展注册。例如 AI 业务扩展注册 `file.edit`/`terminal.run`，柜面扩展注册 `query`/`submit`——都经 `runtime.operation.execute` 走同一门禁。

### 4.2 listen — 订阅事件

```js
const unlisten = await window.__NAVIS__.listen('session.completed', (payload) => {
  console.log(payload);
});
// 组件卸载时调用 unlisten()
```

- pattern 必须在 `capabilities.events` 声明（支持精确或 `*` 前缀通配）。
- 高频流（agent/terminal/task）**必须走 stream**（4.4），不走事件桥。

### 4.3 getContext — 上下文快照

```js
const ctx = await window.__NAVIS__.getContext();
// { session: { sessionId }, activeProject: { projectId } }
```

快照在 iframe 创建时一次性注入；会话切换后请经 `listen` 接收变更通知自行刷新。

### 4.4 stream.subscribeSource — 订阅实时流（Agent 等）

```js
const unsubscribe = window.__NAVIS__.stream.subscribeSource(
  { kind: 'agent', sessionId: 'sess-001' },
  (chunk) => { /* 逐条实时接收，不节流 */ },
);
```

- 需 `capabilities.provides` 含 `"stream"`。
- 数据逐条实时投递，**禁止节流/合并**（实时性优先）。
- 订阅者不活跃时由容器按需投递（无订阅者零推送）。

### 4.5 dialog / call / storage / fetch / extensions

```js
// 弹框
await window.__NAVIS__.dialog.open({ viewId: 'my-view', size: { width: 480, height: 360 } });
await window.__NAVIS__.dialog.close();

// 跨扩展调用（双端授权）
await window.__NAVIS__.call('com.example.other', 'command.execute', { commandId: 'their.cmd' });

// KV 存储（scope: global | worktree | ephemeral）
await window.__NAVIS__.storage.set('theme', 'dark', { scope: 'global' });
const v = await window.__NAVIS__.storage.get('theme', { scope: 'global' });

// 网络（须声明 network 能力）
const resp = await window.__NAVIS__.fetch('https://api.example.com/data', { method: 'GET' });

// 发现
const found = await window.__NAVIS__.extensions.query({ provides: 'file-index' });
```

---

## 五、存储（KV）

- **作用域**：`global`（跨项目）、`worktree`（按工作树隔离）、`ephemeral`（内存，禁用即清）。
- **落盘**：`<app_data>/extensions/{extension_id}/storage/{scope}/`（按扩展隔离）。
- **约束**：key 1..512 字节、禁 `..`/`\`/前导 `/`/控制字符；JSON 值深度 ≤16、单值 ≤256KB；TTL ≤30 天。
- **声明**：必须 `storage.scopes` 声明所用 scope，否则 fail-closed。

---

## 六、网络安全策略

- 未声明 `capabilities.network`（或 `contributes.network`）→ 完全禁止网络（fail-closed）。
- `allowlist`：仅允许声明的 host（含子域可选），iframe 注入 CSP（`img-src`/`connect-src` 等）纵深防御。
- `proxy`：全部请求经宿主代理 + 审计；SSRF 防护拦截内网/环回地址。
- 敏感头（host/cookie/authorization 等）由容器过滤。

---

## 七、跨扩展调用（ExtRouter）

- 调用方声明 `capabilities.extensionCalls: [{ target, actions }]`。
- 被调用方声明 `extension_exports: { views: [...], commands: [...] }` 显式暴露。
- **双端授权**：任一端未授权即拒绝 + 审计。

```json
// 被调用方
"extension_exports": { "views": ["their-view"], "commands": ["their-cmd"] }
```

---

## 八、受控操作执行（容器通用机制，领域无关）

容器提供通用的"受控操作执行"机制（35 §3.2）：Sandbox 门禁 + 审批 + 审计 + Registry 注册由容器持有；**具体的操作定义由扩展注册**。AI 扩展注册 `file.edit`/`terminal.run`，柜面扩展注册 `query`/`submit`，都走同一门禁。

```js
// 注册一个操作（enable 时或运行时）
await window.__NAVIS__.invoke('runtime.operation.register', {
  operation: {
    id: 'com.example.myext.query',
    label: 'Query',
    permissionLevel: 'LightCheck',   // Unrestricted | LightCheck | StrictCheck | UserConfirm
    operationType: 'NetworkRequest', // FileRead | FileWrite | FileDelete | DirCreate | DirDelete | CommandExecute | NetworkRequest
    params: { type: 'object', properties: { code: { type: 'string' } } },
    handlerKind: 'Extension',        // Builtin（容器内建，如 file.read）| Extension（扩展 worker 实现）
  }
});

// 执行操作（过容器门禁）
const result = await window.__NAVIS__.invoke('runtime.operation.execute', {
  operationId: 'com.example.myext.query',
  params: { code: 'ACC-001' },
});
```

> **反哺记录（2026-08-16）**：`permissionLevel`/`operationType` 的 wire 值为 Rust 枚举名（`Unrestricted`/`LightCheck`/`StrictCheck`/`UserConfirm`；`FileRead`/`FileWrite`/`FileDelete`/`DirCreate`/`DirDelete`/`CommandExecute`/`NetworkRequest`），非 `none`/`lightCheck` 等短名。

- **机制在容器**：`runtime.operation.execute` 构造 `OperationRequest{actor:"extension:{id}"}` 过 Sandbox，命中 `userConfirm` 等级需审批，全程审计。
- **操作在扩展**：操作的真实执行由扩展在 WASM 组件轨（`components`）或前端胶水 worker 轨实现（经 `file.read`/`network.fetch`/`storage.*` 等容器原语），或引用容器内建操作（如 `file.read`）。
- **安全铁律**：扩展不能绕过 Sandbox/审批/审计；未注册的操作执行 fail-closed。

---

## 九、后端扩展（ExtensionBackend）

后端扩展承载**纯后端逻辑**。按 37 详设分两种形态：

| 形态 | 目录 | 运行 | 适用 |
|------|------|------|------|
| **逻辑组件**（首选） | `ExtensionBackend/logic/*.wasm` | 容器内 wasmtime（`components[kind:logic]`） | 可编译为 WASM 的逻辑（数据处理/编排） |
| **native 逃生舱** | `ExtensionBackend/native/*` | 独立进程（`backendServices`，协议通信） | 需 OS 能力（USB/打印机/GUI）/自身是 server 的后端 |

### manifest 声明（native 逃生舱）

```json
{
  "contributes": {
    "backendServices": [
      {
        "id": "core-server",
        "entry": "ExtensionBackend/native/my-extension-server",
        "transport": "stdio",
        "protocol": "jsonrpc"
      }
    ]
  }
}
```

> **wire key 铁律**：`contributes.backendServices`（camelCase）。`backend_services`（snake_case）会被 serde **静默忽略**——扩展启用零报错但服务不注册（已发生，C0-1 修正）。

### 生命周期（native 逃生舱）

- enable 时容器 spawn 进程（`autostart: true` 立即拉起）；disable/卸载时 kill。
- 进程崩溃由容器记录（可选受配额重启）。
- 进程不接触容器内存/State，只能经协议调用容器暴露的能力。
- 进程管理由 `BackendProcessManager` 承载（`tool::backend`）：spawn 前过 Sandbox `CommandExecute` 门禁（fail-closed，不弹确认）。

### 通信与安全

- 复用 `transport_adapters` 契约（stdio/SSE/WebSocket/REST），不新造协议。
- 后端暴露的能力经容器注册为 tool/命令/服务；前端扩展经 `route.call` 或受控操作调用。
- 需容器 Sandbox 进程门禁 + 审计；端口/协议受网络策略约束。

### 逻辑组件

可编译为 WASM 的后端逻辑走 `components[kind:logic]`，entry 位于 `ExtensionBackend/logic/`，**容器内执行**（非独立进程），经 host function 门禁——详见 §十 WASM 组件轨。

---

## 十、逻辑轨（Worker 脚本 + WASM 组件）

逻辑扩展有两条轨：**WASM 组件轨**（`components`，主路径，容器内执行）与 **Worker 脚本轨**（`scripts`，仅前端胶水，可选）。

### 10.1 Worker 脚本（`scripts`，前端胶水）

```js
// ExtensionUI/scripts/main.js —— runOn: ["activation"]，启用时自动触发
export function onRun(args) {
  const ctx = await self.__NAVIS__.getContext();
  console.log('激活', ctx);
}

export async function onMessage(payload) {
  // 宿主发来消息时触发
  return { ok: true };
}
```

- 导出 `onRun`（run 消息）与 `onMessage`（宿主消息）。
- 用 `self.__NAVIS__`（与 iframe 的 `window.__NAVIS__` 同构）。
- worker 生命周期由宿主注册表管理；扩展禁用/卸载时自动回收。
- **仅用于**必须贴近前端、无重逻辑的胶水场景；重逻辑/跨平台/需隔离一律走 WASM 组件轨。

### 10.2 WASM 组件轨（`components`，逻辑主路径）

WASM 组件编译为 `.wasm`（wasm32-wasip2），**容器内 wasmtime 执行**，经 WIT 接口 + host function 门禁（37 详设 §四~§六）。执行位置与目录归属无关：前端逻辑组件放 `ExtensionUI/scripts/`，后端逻辑组件放 `ExtensionBackend/logic/`。

#### manifest 声明

```json
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
]
```

- `entry`：`.wasm` 相对路径，**必须位于 `ExtensionUI/` 或 `ExtensionBackend/` 下**（loader 校验）。
- `capabilities`：声明式能力白名单，实例化时映射为 host 接口授予；**未声明 = 不注入接口实现 = 调用 fail-closed**。

#### 接口契约（WIT）

组件消费容器 host 接口（唯一出站通道）：

```wit
package navis:host;
interface operation { execute: func(op: operation-request) -> result<value, string>; list: func() -> list<operation-description>; }
interface context   { get-session: func() -> result<session-snapshot, string>; get-active-project: func() -> result<project-snapshot, string>; }
interface storage   { get: func(key: string, scope: string) -> result<option<value>, string>; set: func(key: string, value: value, scope: string) -> result<_, string>; delete: func(key: string, scope: string) -> result<_, string>; }
interface network   { fetch: func(request: http-request) -> result<http-response, string>; }
interface event     { subscribe: func(pattern: string) -> result<subscription, string>; emit: func(topic: string, payload: value) -> result<_, string>; }
interface log       { write: func(level: log-level, message: string) -> result<_, string>; }
```

组件导出生命周期/消息接口：

```wit
package navis:ext;
interface lifecycle { init: func(handle: host-handle) -> result<_, string>; activate: func() -> result<_, string>; deactivate: func() -> result<_, string>; }
interface message   { handle: func(payload: value) -> result<value, string>; }
```

#### 开发要点

- 用 `wit-bindgen` 从 `.wit` 生成绑定；宿主实现 host 接口，组件消费之。
- 组件间组合：`components[].dependencies` / `exports` 声明，宿主组合注册表**双端授权**（fail-closed）。
- 生命周期：enable→加载→实例化→按 capabilities 授接口→activate；disable→deactivate→回收。
- 安全：wasmtime 内存/trap 隔离 + host function 门禁（`OperationRequest{actor:"extension:{id}"}`）+ 配额（内存/fuel/超时）；组件崩溃不波及宿主。

---

## 十一、配置（configuration）

```json
"configuration": {
  "type": "object",
  "properties": {
    "theme": { "type": "string", "enum": ["light", "dark"], "default": "dark" }
  }
}
```

- 设置面板自动渲染表单（flat schema；复杂 schema 降级 JSON 编辑）。
- 读写：`ui_get_extension_config`/`ui_set_extension_config`；变更经 `extension.config.updated` 事件通知扩展。
- schema 必须是合法 JSON Schema object（容器在 enable 时校验）。

---

## 十二、开发工作流

### 目录结构（一个扩展，37 §三）

```
extensions/my-extension/             # 仓库统一扩展根（目录名 = extension id）
├── extension.json
├── ExtensionUI/                     # ★ 前端扩展面：全部前端代码
│   ├── index.html       ← html:sandbox entry（可含相对 CSS/JS）
│   ├── assets/...
│   ├── scripts/
│   │   ├── main.js       ← 前端胶水 worker（可选）
│   │   └── app.component.wasm  ← 前端逻辑组件（容器内执行）
│   └── locales/          ← i18n 资源
└── ExtensionBackend/                # ★ 后端扩展面：全部后端扩展点代码
    ├── logic/
    │   └── worker.component.wasm  ← 后端逻辑组件（容器内执行）
    └── native/
        └── my-extension-server[.exe] ← native 逃生舱（协议子进程）
```

> **entry 约定（37 §三，废弃 `ui/`）**：`html:sandbox` 视图与组件的 `entry` 必须位于 `ExtensionUI/` 或 `ExtensionBackend/` 下（loader/host_view 校验）；`ui/` 别名已废弃。

### 安装 / 启用 / 禁用 / 卸载

- 安装：复制到 `<app_data>/extensions/<product>/<extension-id>/`（容器校验 manifest）。
- 启用：容器校验全部 contributes → 注册 projection → fail-closed 处理不支持项。
- 禁用：清理 ephemeral 存储、回收 worker、关闭弹框、移除视图投影。
- 卸载：删除扩展目录（含其存储目录）。

---

## 十三、安全清单（扩展作者必读）

1. **最小权限**：只声明需要的 `capabilities`。
2. **不过度请求网络**：能用本地就不声明 network。
3. **敏感数据**：secret 走容器 Auth（`secret_ref`），不落扩展存储。
4. **流订阅按需**：不订阅不使用的流；记得 unsubscribe。
5. **fail-closed 是设计**：未声明的调用会失败——这是保护，不是 bug。

---

## 十四、反哺规则

开发过程中若发现：
- 手册描述的 API 与实现不一致（签名、命令名、字段）；
- 新增了容器能力原语未收录到 §四；
- manifest 字段行为与手册不符；

**必须更新本手册**（36-extension-development.md）与代码对齐，两者以本手册为最终契约。更新时标注改动点与日期。

> **反哺记录（2026-08-16，v2，对齐 37 详设 C0 落地）**：
> - 目录规范归一 `ExtensionUI/` / `ExtensionBackend/`，废弃 `ui/` 别名（`resource.rs` / `host_view.rs` 已落地）。
> - manifest 新增 `components` 字段（`ComponentRegistration`，camelCase wire），loader 校验 entry 位于 ExtensionUI / ExtensionBackend 下。
> - `backendServices` wire key 修正（原示例 snake_case 被 serde 静默忽略，C0-1 已修）。
> - 新增 WASM 组件轨（§十），Worker 脚本降为前端胶水（可选）。


