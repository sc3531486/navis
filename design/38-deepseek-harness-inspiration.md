# 38 — DeepSeek Harness 万物皆插件设计借鉴（Inspiration）

> 状态：研究参考（非独立模块设计；以 35/36/37 为执行基线）
> 日期：2026-08-17（v2 定稿：补"地基 vs 扩展"边界对齐 + 现状映射更新）
> 参考：https://github.com/deepseek-ai/deepseek-harness（DeepSeek Harness，Everything is a Plugin，基于 Cordis）
> 结论：deepseek-harness 与 Navis Go 的终局架构同源（Cordis + 万物皆扩展）。本文件提炼其可借鉴的
> 设计模式，并映射到 Navis Go 现有机制与 C2-C4 迁移动作。**执行仍以 35/36/37 为纲，本文只提供启发。**

---

## 一、deepseek-harness 的关键设计

1. **每一部分都是插件**：模型适配器、工具注册表、会话日志、agent loop 本身都是插件，可从配置整体替换。
2. **无特权内核**：扩展 dsh 的方式是"把插件挂载到其他插件旁边"；所有注册都是副作用，插件卸载时自动撤销。
3. **Cordis 是元框架**：插件向共享 Context 贡献服务、类型化事件和可逆副作用；Cordis 只负责插件装载/卸载与依赖管理。
4. **运行态是插件树**：由 profile（具名组装）+ bundle（组合包）+ patch（按 id 覆盖配置）分层叠加而成。
5. **事件就是扩展点**：持久会话事件（日志）、实时 Agent 事件（agent/*）、能力事件（给 seam 挂策略/适配器，无导入环）。
6. **能力 seam**：Service Definition（接口） + Service Provider（实现） + Consumer（使用者）三者一起设计；换 Provider 即换整个产品。
7. **会话日志是模型所见上下文的唯一来源**："模型可见即已记录"，回放/恢复/UI 都派生自事件流。
8. **轮次/步骤流是瀑布事件**：`agent/pre-step` → `llm/stream` → `tools/pre-execute/execute/post-execute` → `agent/turn-stopping`，监听器 `next()` 委托。

---

## 一·五、地基 vs 扩展：边界对齐（v2 定稿）【2026-08-17】

> **万物皆扩展 = 只提供最基础的扩展能力（地基），除此之外的一切都以扩展形式集成。**
> deepseek-harness 原文（architecture.md，已核证）：**"There is no privileged core to patch: you extend dsh by mounting a plugin beside the others."** —— 没有特权核心，所有能力都是可替换的插件。

### 1.5.1 deepseek-harness 的地基 vs 扩展

| 层 | deepseek 内容 | 说明 |
|----|--------------|------|
| **地基（kernel + 平台设施）** | Cordis kernel（plugin 装载/卸载/依赖/可逆副作用）、seam 定义（Service Definition）、`ctx` 共享上下文、平台事件 | 只提供"让插件工作"的机制，无业务决策 |
| **扩展（一切能力）** | model adapter（`ctx.llm`）、tools（`ctx.tools`）、sandbox backend（`ctx.sandbox`）、session log（`ctx.sessions`）、agent loop（`ctx.agentLoop`）、UI、scheduling、credentials、telemetry | `dsh-base` bundle 即默认业务集合，本身也是**可配置插件**，无特权 |

**关键判别**：deepseek 的 model adapter、sandbox、agent loop 都是插件（可替换、配置驱动）；Navis Go 当前把它们当"容器内装配的业务 State"（`app/business.rs::AiIdeBusiness` manage 11 个 State）是**最大的偏离**。

### 1.5.2 Navis Go 边界判定（落定到 35 §2.5）

| # | 铁律 | 判定 | 反例（不属于地基） |
|---|------|------|------|
| F1 | 只含扩展机制本身 | Cordis kernel、loader、manifest、生命周期 fiber | 任何业务决策 |
| F2 | 只含能力缝定义（无 Provider） | `ctx.llm`/`ctx.tools`/`ctx.sandbox`/`ctx.agents` 接口定义 | Provider 实现（Gateway/MCP/LSP 等） |
| F3 | 领域无关平台原语 | 窗口/存储/配置/日志/事件/流/密钥/沙箱门禁机制 | AI/柜面/双录特定逻辑 |
| F4 | 无特权核心 | 不存在容器内固定装配的业务 State | `AiIdeBusiness` 的 11 个 State |

**推论**：Gateway/MCP/LSP/Skills/Session/AgentLoop 都必须是**可替换的扩展能力**，容器只保留 seam 定义与机制。

### 1.5.3 现状差距（2026-08-17 实测）

- **已完成**：D1（capability port → Cordis service，全量迁移）、D2（WASM 组件轨接线）、D3（事件订阅落地）、D4（`agentLoop` seam 定义）。
- **待做**：框架污染清除（35 §2.5.3 的 0b-0e：families/provider_validation/constraint/session_store 反向依赖业务）；业务物理迁出（35 C3/C4，settings 为试点）。

---

## 二、对 Navis Go 的启发与映射

### 2.1 能力 seam ⇄ capability port

deepseek 的 seam（Service Definition / Provider / Consumer）对应 Navis Go 已具备的 **capability port** 模式：

| deepseek 概念 | Navis Go 现状 | 落地差距 |
|---|---|---|
| Service Definition | `extension::lifecycle::{GatewayCapabilityPort, McpCapabilityPort, LspCapabilityPort}` + `AgentLoopPort` | 端口定义齐全；仍缺"服务目录/发现"统一视图 |
| Service Provider | `gateway::Gateway`、`mcp::MCP`、`lsp::LSPManager` 的 impl | Provider 仍是容器内业务装配（`app/business.rs::AiIdeBusiness`），**D1 后经 capability service 暴露**，但未成为可替换扩展插件（C3/C4 目标） |
| Consumer | `ExtensionLifecycle` enable/disable 接线（Cordis fiber） | 一致（D1 落地） |
| 换 Provider 即换产品 | 未来柜面/双录/AI 各自的 Provider | C3/C4 迁移后达成 |

**动作**：capability port 已提升为 Cordis service（D1 落地：`register_capability_service`/`get_capability_service`/`require_capability_service` + `ErasedCapability`），Provider 以扩展插件注册/卸载；`ExtensionLifecycle` 只消费 service，不依赖具体类型。这与 deepseek 的 `ctx.llm / ctx.tools / ctx.shell` 等价。

### 2.2 事件就是扩展点 ⇄ Kernel EventBus + event_subscriptions

deepseek 的三类事件（会话持久 / Agent 实时 / 能力 seam）对应 Navis Go：

| deepseek | Navis Go 现状 | 落地差距 |
|---|---|---|
| 持久会话事件 | `session_store` 的 `SessionEvent` / 消息 | 已是事实源；需补齐"模型可见即已记录"不变量断言 |
| Agent 实时事件（agent/*） | Kernel EventBus 的 `agent.*` 事件 + `event_subscriptions` 声明 | `event_subscriptions` 目前"等待 runtime handler 落地"，是主要缺口 |
| 能力事件（fs/*、tools/*、telemetry/*） | `extension.trigger.*` / 生命周期事件 | 可扩展；需按 seam 定义稳定事件域 |

**动作（C2/C3 期间）**：把 `event_subscriptions` 从"声明索引"落地为真实订阅（经 `KernelEventSubscriptionAdapter`），
并定义 Navis 的瀑布事件（`agent/pre-step`、`tool/pre-execute/execute/post-execute`、`agent/turn-stopping`），
让 Agent 编排与工具流水线可被扩展拦截，而不是只靠 `AgentDefaultAllowConstraint` 等策略。

### 2.3 插件树 / profile / bundle / patch ⇄ BusinessAssembly + 扩展 enable/配置

deepseek 的"profile 列出组合包 + patch 按 id 覆盖配置"对应 Navis Go：

| deepseek | Navis Go 现状 | 落地差距 |
|---|---|---|
| 插件树（Context 叠加） | `HostExtensionContext`（Cordis root + fiber per extension） | 已具备 |
| profile / bundle | `BusinessAssembly` 清单（`builtin_business_assemblies`）+ `extensions/*` 包 | 已具备骨架 |
| patch（按 id 覆盖配置） | `extension.configuration` + `ui_set_extension_config` | 已具备 |
| 卸载即撤销副作用 | `ExtensionLifecycle` disable rollback（`runtime_handles`） | 已具备 |

**动作**：给内建业务扩展与第三方扩展统一"组合层"语义——启动顺序 = 扩展 enable 顺序；一个扩展的
`configuration` 可覆盖下层；卸载时撤销其注册的 service/event/资源。这已在设计中，C2/C3 落地时保持。

### 2.4 会话日志为唯一事实源

deepseek："模型可见即已记录"，`deriveMessages()` 从事件流投影模型历史。Navis Go 已把
Session/Message 作为事实源（`session_store`），但上下文组装（`ai/context/assembler`）目前直接读
store 而非从事件流投影。

**动作（C4-2）**：上下文组装改为"从会话事件流投影 + 提示词片段插件化"，新增模型可见输入必须新增
会话事件；用不变量断言"抵达模型的一切都能从日志重建"。

### 2.5 新行为归属表（adapted）

deepseek 有一张"新行为归属"表；Navis Go 对应表（C2-C4 目标态）：

| 目标 | 机制（Navis Go） |
|---|---|
| 添加模型 Provider/Adapter | 在 Cordis service 注册 `gateway.adapters/providers`（`GatewayCapabilityPort`） |
| 添加面向模型的能力 | 在 `McpCapabilityPort` 注册 tool，schema 进上下文组装 |
| 添加 shell/终端执行 | `TerminalManager` + `runtime.operation.execute`（操作由扩展注册） |
| 添加用户命令 | `contributes.commands` + Command Palette 投影 |
| 添加后台工作 | `contributes.jobs`/任务运行器（扩展定义） |
| 限制进程/文件 | Sandbox 门禁 + OperationRegistry（机制在容器，操作在扩展） |
| 拦截请求/工具/轮次 | 事件订阅（`event_subscriptions` 落地后）+ 中间件 contributes |
| 添加模型可见上下文 | `context.*` 快照 + 会话事件投影 |
| 添加 UI/编辑器集成 | `contributes.views` + HostView 投影 |
| fork/恢复会话 | `session_store` 事件流派生 |
| 替换 Agent 循环 | 注册 `agentLoop` service（C4：组件轨 or 扩展插件实现） |

---

## 三、对现有文档/代码的一致性核对（反哺）

1. `design/36` §二 的菜单示例与实现不一致：`menus[].action` 已废弃，菜单引用 `command`，命令声明
   `action`（`BuiltinAction`）。已按代码修正 `extensions/navis-demo/extension.json`。
2. `extensions/navis-demo` 原 manifest：目录名 `navis-demo` 与 id `navis.demo` 不一致、缺 `permissions`、
   视图缺 `activation_events`——均已修正，并新增 `src-tauri/tests/extension_contract.rs` 集成测试固化契约。
3. `event_subscriptions` 已从"声明索引"落地为真实 Kernel EventBus 订阅（D3 完成：`KernelEventSubscriptionAdapter::subscribe_declared/unsubscribe_all` + 瀑布事件常量）。

---

## 四、结论

deepseek-harness 验证了 Navis Go 的终局架构方向：Cordis 元框架 + 万物皆扩展 + 无特权内核 + 事件即扩展点 +
能力 seam。Navis Go 已具备地基（capability service / Cordis context / ExtensionLifecycle fiber / BusinessAssembly
注册边界 / 固定目录契约），且 **D1-D4 已落地**（capability service、WASM 组件轨、事件订阅、agentLoop seam）。
C3/C4 迁移应优先补齐：**框架反向依赖清除**（35 §2.5.3 的 0b-0e）、**业务物理迁出为扩展插件**（settings 试点 →
批量）、**上下文组装改事件流投影**、**Agent 循环正式迁入 agentLoop 扩展**。