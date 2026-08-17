# Navis Go 架构总览

> 本文档描述当前代码的分层、事实源和扩展边界。模块编号仅用于检索，具体合同以各模块设计文档和实际代码为准。

---

## 一、架构原则

1. 高内聚：业务规则留在所属域，Gateway 负责模型调用，MCP 负责工具协议，Editor 负责编辑器运行时，Extension 基于 Cordis 负责扩展 plugin/service 生命周期与装配。
2. 低耦合：跨域只依赖 typed DTO、Cordis service interface、Registry、Pipeline、EventBus、Policy 或 capability port，不引用对方内部存储和具体实现。
3. Extension-only：新的 Provider、Model、协议适配器、MCP server、LSP language、Editor contribution 和 UI view 都通过 Extension contribution 或宿主已有注册合同接入，不新增旧命名兼容层。
4. fail-closed：未知 schema、未知协议、无效权限、重复 ID、缺失依赖和不完整生命周期都拒绝加载。
5. 单一事实源：Project/Worktree/Session、Config、Auth、Kernel Registry、Cordis Context/plugin registry、Extension lifecycle 各自有明确事实源，禁止平行状态。
6. 可投影：前端只读取后端 projection 和事件，不读取业务对象、模板、secret 或内核实现细节。

---

## 二、分层

```text
UI / Tauri IPC projection
        │
Application runtime / use cases
        │
Business domains
  ai / project / tool / extension / security
        │
Cordis extension runtime
  Context / Plugin / Service / Inject / Fiber
        │
Foundation services
  config / storage / stream / logging / IPC base
        │
Kernel primitives
  Registry / Pipeline / EventBus / Policy
```

- Cordis 是扩展组合与服务容器：插件通过 `Context` 提供/获取类型化 service，通过 `Inject` 声明依赖，通过 `Fiber` 管理生命周期，通过 `effect`/disposer 回收副作用。Cordis 不替代 Kernel 原语。
- Kernel 提供通用原语，不知道 Provider、MCP server、Editor、Extension manifest 或 UI placement。
- Application composition root 在 app/mod.rs 装配具体业务实现和 capability ports。
- UI 通过 ui/** 命令和事件/Channel 与后端通信。
- Domain 之间通过 capability port 或 typed contract 连接，不能通过全局具体对象互相穿透。

---

## 三、事实源

| 事实 | 唯一事实源 | 说明 |
|---|---|---|
| Project / Worktree / Session | project domain | 会话事实、工作目录和项目身份不由 UI store 代替 |
| 用户配置 | Config | Gateway 连接元数据和 secret_ref 持久化在配置层 |
| secret 内容 | Auth Store / SecretResolver | Gateway 只获得短生命周期 secret |
| 运行时能力 | Cordis Context service registry + Kernel Registry / 业务 Registry | Gateway Adapter、MCP Tool、LSP 等由 Cordis service 注入后进入各自 Registry |
| Extension 生命周期 | Cordis Fiber + extension lifecycle | 负责插件声明、启用、禁用、dispose 和 rollback |
| 前端展示状态 | UI projection/store | 只读投影，不反向成为后端事实源 |

---

## 四、Gateway × Extension

Gateway 将三种职责拆开：

- Adapter：把统一 ChatRequest/Response 转换为目标协议。
- Provider：一个连接实例，包含 base_url、secret_ref 和 Model 列表。
- Model：Provider 下的模型 ID、上下文窗口、输出限制和能力声明。

ProtocolAdapterRegistry 统一注册内建 Chat Completions、Responses 和 Extension Adapter。Gateway 主流程只 resolve registry，不添加按协议的 match 分支。

Extension manifest 使用 contributes.gateway.adapters 和 contributes.gateway.providers。Extension Cordis plugin 通过 GatewayCapabilityPort 进行 protocol acquire、custom protocol registration、Provider upsert 和 release。Gateway 不反向依赖 Extension lifecycle。

新增不同模型协议时，只需增加 Adapter 实现或声明式 Adapter contribution，并声明 Provider/Model；UI catalog、Gateway router、Kernel 和其他 Extension 不需要增加分支。

---

## 五、其他宿主域

### MCP

MCP 负责远程/本地 server、transport、tool discovery、tool call 和生命周期。Extension 只声明 MCP server/transport 数据，由 McpCapabilityPort 转换成 MCP runtime 配置。MCP Tool projection 到 Agent schema 的规则属于 Agent/MCP 边界，不进入 Gateway。

### LSP 与 Editor

LSP 负责语言服务进程、诊断、补全和语言能力注册；Extension lifecycle 通过 LspCapabilityPort 注册语言。Editor 只承接已校验的 CodeMirror themes、languages 和 editor extensions，保留 runtime activation cache，不拥有 Extension lifecycle 或 Kernel Registry。

### UI Host View

Extension view 通过 views、commands、menus 等声明接入 HostView。placement、renderer 和安全边界由 UI 宿主解释。Kernel 不感知 DOM、菜单、布局和 renderer 字符串。

---

## 六、运行时通信

- 离散业务事实：Kernel EventBus，前端通过 Tauri event/useEvent 订阅；Cordis `emit/parallel/serial/bail/waterfall` 只用于扩展装配与插件作用域内事件，不承载跨域业务事实。
- 高频数据：foundation::stream 的 Tauri Channel。
- 前端命令：Tauri invoke，命令位于 src-tauri/src/ui/**。
- 取消与超时：由对应 application/runtime 或 domain service 管理，不由 UI 自行杀任务。
- 审计与日志：Rust 使用 tracing；secret、原始请求体和敏感模板不得进入日志或事件。

---

## 七、目录边界

```text
src-tauri/src/
├── app/         # composition root、状态装配、命令注册
├── ai/          # Gateway、Agent、Context
├── extension/   # Cordis 扩展基座、Extension manifest、loader、store、lifecycle、skills
├── project/     # Project、Worktree、Session、Knowledge
├── tool/        # MCP、Agent tools、LSP、文件、编辑、Git、终端等
├── foundation/  # Config、Storage、Stream、Logger、IPC 基础件
├── security/    # Auth、Sandbox、权限和审计约束
├── kernel/      # Registry、Pipeline、EventBus、Policy
└── ui/          # Tauri commands、DTO、projection、events
```

扩展代码固定目录：

- 前端扩展点：`extensions/{id}/ExtensionUI/`
- 后端扩展点：`extensions/{id}/ExtensionBackend/`

---

## 八、扩展接入规则

新增能力的选择顺序：

1. 现有宿主域已有 Cordis service/contribution schema：只增加 Extension plugin 声明与 manifest。
2. 需要新协议但已有 Registry：增加业务 Adapter，并通过 capability port 注册为 Cordis service。
3. 需要新的投影字段：扩展 DTO/projection contract，不把业务对象暴露给 UI。
4. 需要新的生命周期资源：复用 Cordis `Fiber`/`effect` 或 Kernel ResourceLease/Lifecycle，不创建平行生命周期系统。
5. 只有当多个业务域都证明需要同一通用原语时，才考虑扩展 Kernel；Provider、MCP、LSP、Editor 等业务对象作为 Cordis service 接入，不能直接进入 Kernel。

Extension 禁用、卸载和回滚后，不能残留 Registry entry、Provider、Model、Tool、Language、UI projection、Policy constraint 或事件订阅。

---

## 九、验收标准

- 新 Provider/Model/协议不修改 Gateway 主流程和 UI Settings 分支。
- 新 MCP server、LSP language、Editor contribution、UI view 不修改 Kernel。
- Extension Cordis plugin 只依赖 capability ports 与类型化 service contract，不依赖 Gateway/MCP/LSP 具体类型。
- Config 不保存明文 secret；代码和文档不保留 api_key 双读双写兼容逻辑。
- Registry 重复注册、未知协议和 schema 未知字段 fail-closed。
- Gateway catalog、MCP tool catalog、LSP language projection 和 UI HostView projection 都由后端事实源生成。
- cargo fmt --check、cargo check、cargo test、npm run build 通过。
