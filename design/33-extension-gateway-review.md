# Navis Go 架构复核：Extension × Gateway × HostView

> 复核目标：高内聚、低耦合、符合代码开发规范、高扩展，实现“万物皆可 Extension 接入”。不同模型、不同请求协议、不同响应协议和不同流式返回格式，均应通过新增 Extension 或新增声明式适配配置接入，不修改 Gateway 主流程。
> 复核日期：2026-08-15
> 依据：对 `src-tauri/src`、`src/` 与 `design/` 现有文档的代码走查。
> 编号：33（本文件是 Extension × Gateway 的架构合同与实施验收基线）。

---

## 一、文档定位与设计原则

本文件同时承担三项职责：

1. 记录当前实现与目标架构之间的偏差。
2. 定义 Extension、Gateway、Auth、UI Projection 之间的稳定边界。
3. 为后续重构提供可验证的实施顺序和验收标准。

本文件中的“必须”是实现合同；“当前”描述代码现状；“目标”描述重构完成后的行为。

### 1.1 核心原则

| 原则 | 约束 |
|------|------|
| 高内聚 | Gateway 只负责统一请求编排、协议适配调用、重试/超时/计量和结果归一化；协议细节归 Adapter；凭据归 Auth；扩展生命周期归 Extension；UI 只消费 projection。 |
| 低耦合 | Extension 不依赖 Gateway、MCP、LSP 的具体实现类型；Gateway 不依赖 Extension 生命周期；前端不依赖 Rust 内部配置结构或协议 match。 |
| 开闭原则 | 新增协议、新增 Provider、新增模型、新增 framing 不修改 Gateway 请求/流式主流程；只注册新的能力或声明新的配置。 |
| 单一事实源 | Registry 是运行能力事实源；Auth 是密钥事实源；Gateway catalog projection 是前端展示事实源；manifest 只是声明，不代表能力已经生效。 |
| Fail-closed | 未注册协议、未支持 contribution、无效 schema、未知 renderer、缺失能力或不安全资源必须拒绝，不能静默成功。 |
| 无兼容代码 | 新合同落地后使用单一新 schema；不保留旧字段双读、双写、旧分支和兼容别名。若需要迁移，只允许在切换前执行一次性迁移，运行时不保留旧格式支持。 |
| 安全默认 | Extension 不直接读取密钥、文件系统、网络或任意宿主对象；声明式模板不能访问解密后的 secret；所有外部输入均有限制、可校验、可审计。 |

### 1.2 明确不属于本次合同的内容

- 不把 UI、DOM、HostView placement 或 renderer 放入 Kernel。
- 不通过 manifest 加载任意本地路径、远程 URL 或任意 JavaScript/Rust 动态模块。
- 不把每个 Provider 的特殊逻辑重新塞回 Gateway `match`。
- 不把 Provider、Adapter、Model、Credential 混成一个结构。
- 不为了抽象而复制一套与现有 Kernel Registry、Pipeline、EventBus、Auth Store 平行的基础设施。



### 1.3 状态标记与证据规则

- **已完成**：当前代码已有真实运行路径，并有对应测试或可复核的调用链证据。
- **底座完成**：类型、port、ledger、Registry 或 schema 校验已存在，但真实 runtime 执行链、宿主接线或完整语义尚未完成；不能当作功能已启用。
- **未完成**：目标合同尚未落地，或当前实现只覆盖子集。
- **Fail-closed**：声明可以被解析，但因为没有真实宿主能力而被明确拒绝；这不是“已支持”。
- **未验证/失败**：命令未执行、存在 warning，或测试未通过；不能写成验收通过。

本文件中的验收证据以 2026-08-15 的当前工作区为准。后续代码变化必须同步更新状态，不得仅因为 DTO 或 helper 已定义就把执行能力标记为完成。

---

## 二、当前实现问题清单

### 2.1 问题分级

| 级别 | 含义 |
|------|------|
| **P1** | 阻断“万物皆 Extension / 协议可扩展”目标，或造成运行状态、注册状态不一致。 |
| **P2** | 违反分层、依赖方向或安全边界，长期会导致架构腐化。 |
| **P3** | 结构、可维护性、可测试性或用户体验问题。 |
| **P4** | 文档漂移、诊断语义不准确或潜在可用性风险。 |

### 2.2 Gateway 协议分发与 capability projection 已形成底座（P2，当前范围已完成）

- 已完成：`src-tauri/src/ai/gateway/mod.rs` 初始化 `ProtocolAdapterRegistry`，请求路径通过 Registry `resolve()` 获取 Adapter；新增已注册协议不再要求修改 Gateway 请求主流程。
- 已完成：`src-tauri/src/ai/gateway/protocol/registry.rs` 底层复用 Kernel `InMemoryRegistry`，维护 owner 引用和协议快照，并提供注册、解析、获取和注销入口。
- 已完成：`ApiProtocol::as_str()` 是当前协议 canonical ID 的单一转换入口：内建协议为 `chat_completions`、`responses`，`Custom(name)` 使用 trim 后的原值，不自动添加 `custom:` 前缀。
- 已完成：Extension manifest 的 `protocolId` 经生命周期校验后直接转换为 `ApiProtocol` 并注册到 Registry；运行时不会根据 `extensionId + adapterId` 重新生成协议 ID。
- 已完成当前范围：`protocol/capability.rs` 提供独立的 capability evaluator，接收 host policy、Adapter、Provider、Model 四方输入，计算 effective capability intersection，并输出 capability source、version 和 diagnostics。
- 已完成当前范围：`GatewayCapabilityPolicies` 保存 Provider capability source；Extension lifecycle 通过 capability port 调用 `set_provider_capabilities` / `remove_provider_capabilities`；Gateway 请求校验与 `Gateway::capability_projection()` 复用同一 evaluator。
- 已完成当前范围：`ui/gateway.rs::ui_list_gateway_providers` 不再直接暴露原始 `ProviderConfig`，而是消费 Gateway capability projection；Provider DTO 已包含 capability version、diagnostics、configured、model counts 和当前 status。
- 尚未完成：更丰富的参数化能力约束、所有请求字段与 capability 的一一覆盖，以及统一的 Active/Rejected/Disabled/Degraded 状态合同；当前 Provider status 仍是 `CatalogOnly`、`Available`、`PartiallyAvailable`、`Unavailable`。

### 2.3 `CustomAdapter` 的 schema 与声明能力合同已接入当前子集（P1/P2）

- 底座完成：`CustomProtocolConfig` 已具备 schema/config version、协议名和相对 endpoint 校验、request/response/stream/error/capability 字段、SSE/NDJSON/JSON Lines/JSON framing 声明、字段路径和模板大小/深度/节点等基础校验。
- 已接入当前子集：request/response/stream mapping 已覆盖文本、reasoning、tool calls、structured output、finish reason、可选 usage 和 provider error 等声明式字段；framing 支持 SSE、NDJSON、JSON Lines 和单 JSON 的既有路径。
- 已补齐当前声明能力合同：模板只有在真实字符串插值中消费 `request.tools`、reasoning、structured output 或 `request.stream` 时，才能声明对应 capability；未声明能力却使用对应字段仍 fail-closed。
- 当前 usage 语义已通过 `usage_available=false` 表达不可用；Custom Adapter 缺少 usage 时不会调用 `with_usage()`，也不会把零值视为有效统计。仍未完成：下游计量/projection 对 usage 可用性的统一消费，以及覆盖所有声明式字段的完整 HTTP provider fixture。
- `StreamCancelToken`、`StreamSender` 取消入口、bytes/select 所有权和 framing EOF 校验已经落在实现中；Gateway 不在 EOF 时伪造成功终态，终态后的数据/重复终态会显式失败，provider error/incomplete/cancel 会进入失败或取消审计路径。本轮未重新执行构建或测试，因此这些实现状态不作为本轮编译验收证据。声明式配置继续对 request capability contract 保持 fail-closed。

### 2.4 Extension Provider 声明、capability source/lifecycle/UI projection 已接入（P2，剩余执行边界）

- 位置：`src-tauri/src/extension/lifecycle/register.rs`、`src-tauri/src/ai/gateway/request.rs`。
- 当前模型是 `GatewayAdapterRegistration` / `GatewayProviderRegistration`；本文原先使用的 `ProviderAdapterRegistration` 已不是当前代码结构名。
- `GatewayAdapterRegistration.protocolId` 是 manifest 声明的 Registry 全局 canonical 协议标识。生命周期校验后直接构造 `ApiProtocol::from_str(protocolId)`，声明式 `CustomProtocolConfig` 也复用同一字符串。
- Provider 的运行时主键由 `extension_provider_id()` 生成完整的 `extension:<extensionId>/<providerId>`；模型的 `api_protocol` 使用同一个 `ApiProtocol` 值，Gateway 按该值 resolve Adapter。
- 已完成当前范围：Provider capability 作为独立 source 由 `GatewayCapabilityPolicies` 保存，Extension lifecycle 在 enable/rollback/disable 中分别注册、恢复或移除；Gateway capability evaluator 将其与 host policy、Adapter、Model 一起计算。
- 已完成当前范围：Gateway capability projection 和 UI projection 使用同一 effective capability 结果，Provider DTO 包含 capability version、diagnostics、configured、model counts 和 status，不直接泄露原始 `ProviderConfig`。
- `adapterId` 只负责 Provider 对同一 Extension 内 Adapter contribution 的引用；`provider_type` 不参与 Extension Provider 合并或协议路由。未知协议、显式 `custom:` 前缀和未注册 Adapter 必须 fail-closed，不能静默改写。

### 2.5 Extension Runtime Handle 当前为聚合句柄，逐资源 opaque handle 仍是目标（P1）

- 当前 enable 成功后，`src-tauri/src/extension/lifecycle/mod.rs` 保存 Extension 级 `ExtensionRuntimeHandle`；其中以完整 Provider ID、`ApiProtocol` 和其他资源 ID/撤销事实记录已提交资源。
- 当前 ProtocolAdapterRegistry 的 `register_adapter`、`acquire`、`unregister` 仍按 owner + canonical protocol ID 工作，不返回逐资源 opaque Registry handle；因此不能把当前实现描述为“Registry 已返回并持有每个资源的 Runtime Handle”。
- `disable` / rollback 消费已保存的聚合 handle，不根据可变 manifest 重新生成 Provider 或 Protocol ID。Gateway、MCP、LSP 或其他资源清理失败时，未清空的 handle 会保留在生命周期状态中，下一次操作可以重试清理。
- 目标合同仍然有效：未来如引入逐资源 handle，应包含 owner、kind、完整 ID、注册顺序和撤销参数；当前实现只能以 ExtensionRuntimeHandle 的聚合事实满足其中的部分约束。

### 2.6 Extension 反向依赖业务域（已完成底座，持续收敛）

- `extension/lifecycle/register.rs` 仍需要使用宿主 DTO（例如 `ProviderConfig`、`MCPServerConfig`、`LSPServerConfig`）完成边界转换；这些类型不是生命周期直接持有的业务宿主实现。
- `ExtensionLifecycle` 当前只持有 `GatewayCapabilityPort`、`McpCapabilityPort`、`LspCapabilityPort`、`EventSubscriptionPort` 等能力 port，不持有 `Gateway`、`MCP` 或 `LSPManager` 实例。
- 具体 Gateway/MCP/LSP/EventBus 适配均由 `app/mod.rs` 装配；新增宿主能力应继续以独立 port 接入，不能把具体实现倒灌回 Extension lifecycle。

### 2.7 巨型 `ExtensionContributes`（P3）

- `src-tauri/src/extension/models.rs` 的单结构承载多类 UI、Integration、Agent、配置和资源贡献。
- `lifecycle/state.rs` 统一处理注册、注销、回滚，新增 contribution 会持续扩大耦合面。
- 目标是按能力边界拆分 contribution DTO 和 registrar，但不新增重复的通用注册基础设施。
- 高扩展要求：manifest 边界 DTO 可以保持聚合，内部必须按 contribution family 拆分 normalize/validate/prepare/commit/rollback/disable handler；新 family 只新增 DTO、port/registrar、projection 和测试，不修改生命周期主流程。

### 2.8 密钥事实源与发送阶段 Auth 已接入，脱敏和集成验证仍需收敛（P2）

- 当前 `ProviderConfig` 只使用 `secret_ref`，运行时不再读取或写入 `api_key`。
- `security/auth/key_store.rs` 是密钥事实源；Gateway 通过 `SecretResolver` 在发送阶段获取短生命周期 `SecretValue`。内建 Provider 和 Extension Provider 均通过 `ProviderAuthProfile` 统一构造认证 header，认证 secret 不进入 Adapter、模板、日志或 UI projection；`SecretValue` 在离开作用域时执行 zeroize。
- 尚未完成的内容主要是完整 HTTP provider fixture、发送阶段与 validation lifecycle 的集成测试、日志/事件/错误的全链路脱敏审计，以及 redirect/DNS rebinding 等安全边界验证；不恢复旧字段兼容读取。

### 2.9 Provider validation lifecycle 与发送阶段 Auth 已接入，集成覆盖仍不完整（P2/P4）

- `security/auth/key_validator.rs` 已有 `ValidationStatus::{Unknown, Reachable, Valid, Invalid}`，并通过注入的 `ValidationTransport` 执行协议级 HTTP 校验。
- 当前内建 `openai` 与 `anthropic` provider 会构造受控的 `/v1/models` 请求，分别使用 `Authorization: Bearer` 与 `x-api-key`/`anthropic-version` 头；2xx 映射为 `Valid`，401/403 映射为 `Invalid`，其他可达响应映射为 `Reachable`，传输失败映射为 `Unknown`。
- `AuthManager::validate_key()` 已调用该 validator，将结果写回 Auth Store，并发布 `auth.key.validated`（以及 `Invalid` 时的失败事件）；Extension Provider validation registry 复用同一 `ProviderAuthProfile` 和 `SecretResolver`，Gateway 真实请求路径也通过 `ProviderConfig.auth_profile` 统一注入认证 header。
- 已完成当前范围：`extension/provider_validation.rs` 定义并规范化 Extension Provider validation request、验证 Registry 和复用现有 `ValidationTransport` 的执行边界；Provider validation contribution 已进入 lifecycle plan。
- 已完成当前范围：`lifecycle/register.rs` 负责构造 validation plan，`lifecycle/state.rs` 在 enable 时注册 validation，在 rollback/disable 时注销 validation，`app/mod.rs` 负责注入 validation registry；Extension validation lifecycle 不需要在 `AuthManager` 或 Gateway 主流程增加 Provider 名称分支。
- 当前未知 Provider 或没有受控验证入口的 Provider 不会通过默认地址发送验证请求，仍保持 `Unknown` 的 fail-closed 语义。
- 当前未完成项不再是 Gateway Auth port 或 Extension Provider auth profile 接线，而是完整发送集成测试、认证失败/缺失 secret 的端到端验证、日志/事件/错误脱敏审计，以及 UI 配置写回路径与 Extension runtime Provider 边界的一致性检查；不能把这些缺口倒算为 validation lifecycle 未接入。

### 2.10 未接线 contribution 已改为 fail-closed，family runtime 与 Extension runtime entry 仍不完整（P2/P3）

- 当前 `ensure_supported_runtime_contributes()` 会对未接入宿主的 event subscriptions、toolbar/statusbar/inline、roles、configuration、triggers、styles、layout overrides、behaviors、context/search/file watcher 等 contribution fail-closed；它们不是已支持能力。
- 已完成：不再允许“只 debug、不注册也不拒绝”的静默成功路径。
- 已完成底座：contribution family 已有统一的 normalize/validate/prepare/commit/rollback/disable plan/handler 抽象、registration fact 和测试承载点；UI 与 Gateway family 已完成真实 lifecycle 接线，Gateway 的 Provider、Protocol、Capability Source、Provider Validation 清理顺序由 family handler 统一负责。
- 仍未完成：MCP、LSP、Skills 等保留现有显式 lifecycle/port 路径，事件订阅仍只有声明、ledger 和 EventSubscriptionPort 底座，因缺少 Extension runtime handler entry 在 preflight fail-closed；所有 family 的统一 projection status、诊断和 recovery 事实也尚未完全收口。新增未接线 family 不能只声明 manifest，必须先提供 handler 与 runtime entry。

### 2.11 HostView contract 的边界

- 当前支持 `host:panel`、`html:sandbox` 两种 renderer，以及受控 placement。
- 这符合 `design/07-extension.md` 与 `design/22-ui-framework.md` 的当前安全模型：第三方扩展不能仅凭 manifest 提供任意 renderer。
- 新增 Extension 使用既有 renderer/placement 时不应修改宿主业务分支；若未来增加 renderer，必须新增受限 capability contract，而不是放开任意 DOM/ES Module。

### 2.12 本轮审计证据摘要

- 本轮按要求未执行 `cargo check`、`npm run build`、`cargo fmt` 或任何测试命令；以下代码状态均未在本轮重新验证。
- 已完成实现层改造：Gateway family lifecycle 接线、统一 Gateway rollback/disable cleanup、Custom Adapter request capability token 合同，以及 EventSubscriptionPort/ledger 的 fail-closed 底座。
- 事件订阅当前仍只有声明、ledger 和 EventSubscriptionPort；没有真实 Extension handler runtime entry，因此不能标记为已完成运行时能力。
---

## 三、目标架构总览

```text
Extension Manifest
        │ validate / normalize
        ▼
Extension Contribution Plan
        │ transaction
        ├── ProtocolAdapterRegistry  <── built-in adapter / declarative adapter
        ├── InMemoryRegistry<ProviderConfig> <── provider instance + model catalog
        ├── Auth SecretReference      <── encrypted secret store
        └── UI Projection Registry    ──> Gateway Catalog DTO ──> SolidJS Store

Unified ChatRequest
        │
        ▼
Gateway orchestration
  route → policy → adapter → transport → normalize → quota/event
        │          │
        │          ├── request transform
        │          ├── response transform
        │          └── stream framing + event transform
        ▼
Unified ChatResponse / StreamChunk / ProviderError
```

### 3.1 依赖方向

```text
Kernel primitives
   ↑
Foundation / Security
   ↑
Gateway domain ← Gateway ports ← Extension lifecycle
   ↑                                  ↑
UI projection / Tauri commands        App composition root
```

- Gateway 不调用 ExtensionLifecycle。
- ExtensionLifecycle 只调用 `ExtensionHost` port，不引用 Gateway/MCP/LSP 具体类型。
- UI 不读取 Registry 内部对象，只读取安全 DTO projection。
- Kernel 只提供 Registry、Pipeline、EventBus、Policy 等通用原语，不承载 Provider 或 HostView 业务。

---

## 四、Extension Gateway 合同

### 4.1 适配器与 Provider 的职责划分

| 对象 | 职责 | 不负责 |
|------|------|---------|
| **Protocol Adapter** | 将统一请求转换为某种 Provider 协议，解析非流式响应，解析流式 framing，归一化错误、结束原因和 usage。 | 不保存用户 secret；不管理 Provider 生命周期；不决定重试和配额策略。 |
| **Provider** | 提供 Provider 身份、base URL、认证策略、模型目录、默认模型和实例级能力覆盖。 | 不实现协议转换；不在 Gateway 主流程中写特殊分支。 |
| **Model** | 描述模型 ID、能力、上下文窗口、输出上限、协议绑定和模型级覆盖。 | 不存 API key；不携带执行代码。 |
| **Gateway** | 路由、策略、超时、取消、重试、请求执行、计量、事件和统一结果。 | 不知道具体厂商字段，不解析厂商响应路径。 |
| **Auth** | 生成、存储、解密和轮转 secret；向受控 Gateway port 提供短生命周期 secret。 | 不向 UI 或 Extension 返回明文 secret；不参与协议字段转换。 |

### 4.2 Manifest 目标 schema

当前 Extension manifest 只支持 `contributes.gateway.adapters` 与 `contributes.gateway.providers` 两个明确集合。Adapter 和 Provider 必须分离；Adapter 不携带无运行时 loader 的 `module` 字段，也不允许通过隐式模块加载获得执行能力。以下是规范化 schema；实现只支持这一种形状，不保留旧字段双读。

```json
{
  "id": "acme.mimo",
  "version": "1.0.0",
  "contributes": {
    "gateway": {
      "adapters": [
        {
          "id": "mimo-v1",
          "name": "MIMO v1",
          "protocolId": "mimo-v1",
          "kind": "declarative",
          "config": {
            "request": {
              "method": "POST",
              "path": "/v1/chat",
              "body": {
                "model": "{{model.id}}",
                "messages": "{{request.messages}}",
                "stream": "{{request.stream}}"
              }
            },
            "response": {
              "contentPath": "output.text",
              "finishReasonPath": "finish_reason",
              "usagePath": "usage",
              "promptTokensPath": "input_tokens",
              "completionTokensPath": "output_tokens"
            },
            "stream": {
              "framing": "sse",
              "dataPrefix": "data:",
              "doneMarker": "[DONE]",
              "event": {
                "contentDeltaPath": "delta.text",
                "reasoningDeltaPath": "delta.reasoning",
                "toolCallDeltaPath": "delta.tool_calls",
                "finishReasonPath": "finish_reason",
                "usagePath": "usage",
                "errorPath": "error"
              }
            },
            "capabilities": {
              "tools": true,
              "streaming": true,
              "multimodal": false,
              "reasoning": true,
              "structuredOutput": false,
              "usage": true
            }
          }
        }
      ],
      "providers": [
        {
          "id": "mimo",
          "name": "MIMO",
          "adapterId": "mimo-v1",
          "baseUrl": "https://api.example.com",
          "auth": {
            "scheme": "bearer",
            "secretRef": null,
            "header": "Authorization"
          },
          "models": [
            {
              "id": "mimo-large",
              "name": "MIMO Large",
              "capabilities": {
                "tools": true,
                "streaming": true,
                "multimodal": false,
                "reasoning": true,
                "structuredOutput": false,
                "usage": true
              },
              "contextWindow": 128000,
              "maxOutputTokens": 8192
            }
          ],
          "defaultModel": "mimo-large"
        }
      ]
    }
  }
}
```

#### 4.2.1 Schema 约束

- `gateway.adapters[].id` 在 Extension 内唯一；`protocolId` 是全局路由标识，不得使用显示名称。
- `gateway.providers[].adapterId` 必须引用同一 Extension 已声明且已注册的 Adapter。
- 一个 Extension 可以声明多个 Adapter、多个 Provider；一个 Provider 只能绑定一个 Adapter；多个 Provider 可以复用同一 Adapter。
- 一个 Adapter 可以服务多个 Provider 和多个 Model，但不得携带 Provider 实例状态。
- `baseUrl` 只能是经安全策略批准的绝对 HTTP(S) URL；manifest 的 endpoint 只能是相对路径，最终 URL 由 Gateway 组合。
- `auth.secretRef` 在 manifest 中只能是空值或用户选择后的 opaque reference；扩展包不能预置 secret，不能把 secret 写进模板、日志或 UI projection。
- Model 的能力是声明输入；Gateway 计算后的有效能力必须是 Adapter、Provider、Model、宿主 Policy 的交集。
- `module`、`entry`、任意本地路径、远程代码 URL、函数源码和任意 IPC handler 不属于此 schema。
- schema 校验失败时整个 Gateway contribution 失败，不注册其中的部分对象。

### 4.3 ID 作用域与命名

所有 ID 都是机器标识，显示名称不得参与路由、持久化引用或注销。

| ID | 作用域 | 规范格式 | 说明 |
|----|--------|---------|------|
| Extension ID | 全局 | `publisher.name` | 安装时唯一；大小写、空白和路径分隔符必须规范化。 |
| Protocol ID | Registry 全局 | `chat_completions`、`responses` 或 manifest `protocolId` 的 canonical 原值 | 负责解析协议 Adapter；Registry key 不自动添加 `custom:` 或 Extension 前缀，冲突按 Registry 规则处理。 |
| Adapter ID | Extension 内 | manifest contribution ID | 只在 Extension 内唯一；它是 Provider 的引用键，不是协议路由键。 |
| Provider ID | Gateway 全局 | builtin profile ID 或 `extension:<extensionId>/<providerId>` | Provider 实例的唯一事实源；catalog、回滚和注销必须使用完整 ID。 |
| Model ID | Provider 内 | manifest model ID | 同一 Provider 内唯一；跨 Provider 的引用使用完整 `providerId/modelId`。 |
| Runtime Handle | Extension lifecycle | 当前为 ExtensionRuntimeHandle 聚合句柄 | 保存完整 Provider ID、`ApiProtocol` 和资源撤销事实；不是逐资源 opaque Registry handle。 |

规则：

1. ProtocolAdapterRegistry 以 `ApiProtocol::as_str()` 的 canonical protocol ID 建立索引，并保存 owner 引用。
2. 同一 protocol ID 只有 registration key 相同且 owner 引用可合并时才能 acquire；冲突 Adapter 不能覆盖已有对象。
3. Extension disable/uninstall 只能使用自己 `ExtensionRuntimeHandle` 中保存的完整资源事实执行撤销。
4. Provider catalog 主键始终是完整 `provider.id`；不得因显示名称、`provider_type` 或 manifest 顺序变化而改变。
5. `provider_type` 不承担协议路由职责；协议路由只使用模型 `api_protocol` 对应的 canonical protocol ID，`adapterId` 只用于 contribution 引用。

### 4.4 Registry 合同

Protocol Adapter Registry 应复用 Kernel Registry 原语，不再在 Gateway 中维护第二套无 owner、无版本、无撤销信息的 Map。

当前实现能力：

```text
register_adapter(owner, adapter) -> Result<()>
acquire(owner, protocol_id) -> Result<()>
resolve(protocol_id) -> Adapter
list() -> Vec<ProtocolAdapterInfo>
unregister(owner, protocol_id) -> Result<()>
```

- `resolve` 只返回 Registry 中可用的 Adapter；canonical key 来自 `ApiProtocol::as_str()`。
- Registry 保存 Adapter 的协议 ID、registration key、owner 引用和能力信息。
- 内建 Adapter（Chat Completions、Responses）和 Extension Adapter 走同一注册/解析接口；内建 Adapter 不能在 Gateway 主流程中绕过 Registry。
- 未注册协议必须返回可诊断错误，不能降级到 Chat Completions。
- Adapter 不能在 `resolve` 时改变全局状态；配置校验和初始化必须在注册阶段完成。
- 当前 Registry API 不返回逐资源 Runtime Handle；ExtensionLifecycle 通过聚合 `ExtensionRuntimeHandle` 保存其已提交的 protocol/provider 事实。逐资源 opaque handle 是后续 port 演进方向，不是当前实现状态。

---

## 五、声明式 Adapter 边界

### 5.1 首选路径：声明式适配

大多数“不同协议”只是在 HTTP method/path、headers、请求字段、响应路径、流式 framing 和错误结构上不同，应使用声明式 Adapter。它必须是纯数据配置，由宿主解释执行。

声明式 Adapter 允许：

- 固定 HTTP method 和相对 endpoint。
- 从统一请求模型映射 JSON 请求字段。
- 受限变量插值：`request.*`、`model.*`、`provider.*`、`runtime.*`。
- 受控 headers 模板，但不能读取原始 secret。
- 有限深度的 JSON path 提取。
- SSE、NDJSON、JSON Lines、单 JSON 等已实现 framing。
- 内容增量、reasoning 增量、tool call 增量、finish reason、usage、错误字段映射。
- 明确的能力声明和必需字段校验。

声明式 Adapter 不允许：

- 任意表达式执行、脚本执行、函数反序列化或递归模板执行。
- 访问文件系统、环境变量、网络 socket、Tauri IPC 或 Kernel 对象。
- 通过模板拼接完整 URL、代理地址或认证信息。
- 修改 retry、timeout、quota、permission、sandbox 等宿主策略。
- 把一个 Provider 的密钥复制到另一个 Provider。

### 5.2 未来代码适配器边界

如果声明式能力不足，未来可以设计受限 Adapter Runtime，但必须单独定义：

- 固定版本的 `ProviderAdapterHost` 接口。
- 明确的输入/输出 DTO，不暴露 Rust 内部对象。
- capability-based 权限和资源配额。
- 超时、取消、内存、响应大小和日志脱敏策略。
- 进程或 sandbox 隔离，以及可撤销生命周期。

在该 Runtime 合同落地前，`module` 字段必须被删除或拒绝；不能以“字符串路径 + 隐式加载”冒充扩展性。

---

## 六、请求、响应与流式 framing 合同

### 6.1 统一请求模型

Gateway 只向 Adapter 提供统一请求模型，至少包含：

- `provider_id`、`model_id`、`request_id`。
- 有序消息和角色。
- 文本、多模态内容和附件元数据。
- tools、tool choice、structured output 等结构化能力。
- reasoning effort、temperature、top-p、max output 等可选参数。
- `stream`、timeout、cancellation token 和 trace context。

Adapter 必须依据有效能力构造请求：

- 未声明或不支持的能力不能发送对应字段。
- 必需字段缺失在发送前失败。
- 宿主策略过滤后的请求才可进入模板映射。
- secret 由 Gateway/Auth 在发送阶段注入，绝不进入统一请求 JSON 或模板变量。

### 6.2 请求变换

请求变换按以下顺序执行：

```text
统一请求
  → policy 校验/裁剪
  → model/provider 默认值
  → adapter request mapping
  → host 注入认证 headers
  → transport 发送
```

- Adapter 只能决定协议字段，不决定安全策略。
- custom headers 由受控 schema 合并；禁止 header name 注入、重复敏感 header 覆盖和 CRLF。
- endpoint 必须通过 URL 解析器组合，禁止字符串拼接绕过 base URL 安全校验。
- 请求体、header 数量、模板深度和最终 JSON 大小必须有上限。

### 6.3 非流式响应

所有非流式响应必须归一化为统一 `ChatResponse`，至少支持：

- 文本内容。
- reasoning 内容（若有）。
- tool calls 及其参数。
- finish reason。
- usage：prompt/input tokens、completion/output tokens、total tokens。
- provider request ID 和可诊断 metadata（不得包含 secret）。

Adapter 必须声明：

- HTTP 状态码与 provider error body 的错误映射。
- 成功响应 content 的提取路径。
- 缺失必需字段时的错误类型。
- usage 缺失时通过 `usage_available=false` 明确表达不可用；下游不得仅凭 token 数值是否为零推断 usage 可用性。

### 6.4 流式 framing

流式协议由两层组成，不能混为一谈：

1. **Framing**：字节流如何切分为事件。
2. **Event mapping**：事件 JSON 如何映射为统一 `StreamChunk`。

第一阶段允许的 framing：

| framing | 规则 |
|---------|------|
| `sse` | 按 SSE event/data 规则解析；支持 data prefix、空行分隔和 done marker。 |
| `ndjson` | 按换行切分 JSON 对象；限制单行大小。 |
| `json-lines` | 与 NDJSON 等价的明确别名；内部统一为一种实现。 |
| `json` | 单个完整 JSON 响应，不产生增量事件，只允许在协议声明支持时使用。 |

每个流式事件最多映射为以下一种或多种事实：

- `ContentDelta`。
- `ReasoningDelta`。
- `ToolCallDelta`。
- `Usage`。
- `Finished`。
- `ProviderError`。

流式合同：

- 解析器必须支持 chunk 边界任意切分，不能假设一次网络读取对应一个事件。
- 单个事件、累计缓冲、累计事件数和总响应大小必须有限制。
- done marker 只结束当前流，不作为普通文本输出。
- provider error 一旦出现，后续事件丢弃并关闭流。
- 正常结束必须产生且仅产生一个 `Finished`；连接提前断开必须是 `IncompleteStream`，不能伪装成正常完成。
- cancellation 必须向 transport 传播，并释放 response body、buffer 和 adapter 状态。
- usage 可能在最后一个事件或独立事件中出现；缺失时标记 unavailable。

### 6.5 错误归一化

统一错误至少区分：

| 错误 | 语义 |
|------|------|
| `InvalidConfiguration` | manifest、Provider、Model、Adapter schema 无效。 |
| `UnsupportedCapability` | 请求使用了模型未声明的能力。 |
| `UnregisteredProtocol` | protocol ID 未在 Registry 激活。 |
| `TransportError` | DNS、连接、TLS、超时、取消等传输错误。 |
| `ProviderRejected` | Provider 返回认证、权限、限流或业务拒绝。 |
| `MalformedResponse` | HTTP 成功但响应或流式事件无法按合同解析。 |
| `PolicyDenied` | Endpoint、权限、大小或安全策略拒绝。 |
| `Internal` | 宿主内部错误。 |

错误诊断可以包含 provider request ID 和安全的 provider message，但不得包含 API key、Authorization header、完整敏感请求体或 secret template 展开结果。

---

## 七、Capabilities 合同

### 7.1 能力集合

Adapter、Provider 和 Model 可声明以下能力：

- `tools`
- `streaming`
- `multimodal`
- `reasoning`
- `structuredOutput`
- `usage`
- `modelCatalog`
- `maxOutputTokens`
- `inputContentTypes`、`outputContentTypes`

能力可以携带参数，例如支持的图片 MIME 类型、最大图片大小、最大工具数量、reasoning effort 枚举或 structured output schema 限制。

### 7.2 有效能力计算（四方 evaluator 已完成当前底座）

```text
effective = hostPolicy
          ∩ adapterCapabilities
          ∩ providerCapabilities
          ∩ modelCapabilities
```

- 当前实现：`GatewayCapabilityEvaluatorPort` / `IntersectionCapabilityEvaluator` 接收 host policy、Adapter、Provider、Model 四方输入，计算 effective capability intersection，并输出 version、source projection 和 clip diagnostics。
- 当前实现：`GatewayCapabilityPolicies` 是 Provider capability source 的事实源；Extension lifecycle 通过 capability port 注册和移除 Provider capability；Gateway 请求入口与 catalog projection 复用同一 evaluator。
- 当前实现：未注册协议的模型不会进入 capability projection；UI 只消费后端 projection，不根据 manifest 原值或协议名称自行推导能力。
- 未完成：部分参数化能力的完整请求约束、所有声明字段与真实 transform/normalize 的逐项一致性，以及 Active/Rejected/Disabled/Degraded 的统一状态语义。当前 Provider projection 使用 `CatalogOnly`、`Available`、`PartiallyAvailable`、`Unavailable`。
- UI 只能展示后端 projection；前端不得根据 manifest 原值或协议名称自行推导能力。
- Adapter 不得因为收到未知字段而静默转发；未知字段应被拒绝或由统一扩展参数容器明确承载。

### 7.3 Provider profile

内建 Provider profile 和 Extension Provider 必须通过同一 profile port 提供：

- 默认协议/Adapter。
- 认证 scheme 和 header 注入规则。
- Provider 默认 endpoint。
- 能力默认值和 quirks。
- 模型目录来源。

当前 `Gateway::capability_projection()` 对内建 profile 使用 `builtin_provider_profile()`，对 `extension:` Provider 使用 manifest 注册的 Provider/Model 数据，并通过 `GatewayCapabilityPolicies` 读取 Provider capability source；这已经形成独立、版本化的 capability evaluator 与 UI projection 路径。

`builtin_provider_profile()` 只能是内建 profile 的注册实现，不能成为 Extension provider 的唯一入口。Extension provider 的 profile 必须由 manifest + Registry 规范化产生。

### 7.4 高扩展 capability 合同

为使新增模型、Provider 或协议只增加声明和适配代码，不修改 Gateway 主流程，能力系统必须满足：

- capability key、参数结构和 projection DTO 都有显式 schema version；未知 capability 默认拒绝，只有统一扩展参数容器允许受控扩展。
- Adapter、Provider、Model 和 host policy 的能力来源可独立替换；新增一层能力来源只新增 evaluator input/port，不修改所有调用方的协议分支。
- capability 计算输出必须包含 canonical provider/model/protocol ID、有效能力、被裁剪的原因和来源版本，供 UI、Gateway 入口和诊断共用。
- capability evaluator 不执行网络请求、不读取 secret、不加载 Extension 代码；策略和执行仍由 Gateway/Auth/Extension runtime 各自负责。
- 能力声明与请求字段映射必须一一对应；声明了但没有真实 transform/normalize 支持的能力必须拒绝注册或在 projection 中标记为不可用。
---

## 八、Secret Reference 与安全合同

### 8.1 Secret reference

Provider 配置不再持有明文 `api_key`。目标模型为：

```text
Provider.auth
  ├── scheme: bearer | api-key | custom
  ├── header: 受控 header 名
  └── secret_ref: Auth Store opaque key ID
```

- `secret_ref` 只引用 Auth Store 中的加密记录，不包含 secret 内容。
- Extension manifest 只能声明认证需求，不能预置或读取用户 secret。
- 前端 Gateway catalog、Extension manifest projection、日志、事件和错误均不得返回明文 secret。
- Gateway 仅在发送请求的最小范围内通过 Auth port 获取短生命周期 secret；使用后立即释放或 zeroize。
- 自定义 headers 可以声明 `secretRef` 绑定位置，但不能把 secret 暴露为模板变量。
- Adapter 只能请求宿主注入认证结果，不能直接调用 KeyStore。

### 8.2 Schema 切换策略

由于本次要求不保留兼容代码，旧的 `ProviderConfig.api_key` 不应与 `secret_ref` 长期并存：

1. 在切换版本执行一次性迁移：读取旧配置、写入 Auth Store、写入新 Provider config。
2. 迁移成功后删除旧明文字段。
3. 新运行时只读取 `secret_ref`；发现旧字段直接报告配置迁移错误，不进入隐式兼容分支。
4. 迁移日志只能记录 provider ID 和结果，不记录 key 内容。

### 8.3 Endpoint 与请求安全

- 只允许 `https`；是否允许 `http` 必须由明确的本地开发 Policy 控制。
- 禁止默认把未知 Provider 指向 `localhost:8080`。
- 对 base URL 做 SSRF、localhost、私网、回环、重定向和 DNS rebinding 策略校验。
- endpoint 只能是相对路径，禁止通过 `..`、绝对 URL 或特殊 scheme 逃逸 base URL。
- header name/value 做长度、字符和注入校验；敏感 header 不允许被 Adapter 覆盖。
- response body、单事件、累计 stream、JSON nesting 和 template expansion 都必须有上限。
- 错误日志、tracing span 和 telemetry 对 Authorization、Cookie、API key、secret reference 关联数据统一脱敏。

### 8.4 密钥校验语义

校验结果必须是明确枚举：

- `Valid`：使用该协议的真实轻量 API 请求验证成功。
- `Invalid`：Provider 明确返回认证失败或密钥无效。
- `Reachable`：网络可达，但尚未验证认证成功。
- `Unknown`：超时、策略禁止、协议不支持或无法安全判断。

当前实现边界：内建 `openai`/`anthropic` 已通过 `ValidationTransport` 执行 `/v1/models` 协议校验，并完成上述四态映射；未知 Provider 没有可推断的验证请求，当前直接返回 `Unknown` 且不发送网络请求。

目标扩展边界：TCP 可达性不能标记为 `Valid`。Extension Provider 必须通过 Extension-owned validation contract 提供受控校验 endpoint、认证头构造和状态映射，或显式引用宿主提供的验证 port；没有安全验证入口时返回 `Unknown`，不能走未知 Provider 的默认 URL，也不能在 Auth 或 Gateway 中新增按 Provider 名称分支。

---

## 九、Extension 生命周期、事务与回滚

### 9.1 宿主 port

Extension lifecycle 只依赖能力 port，例如：

```text
ExtensionHost
  ├── protocol_adapters: AdapterRegistryPort
  ├── gateway: GatewayCapabilityPort
  ├── mcp: McpHostPort
  ├── lsp: LspHostPort
  ├── event_subscriptions: EventSubscriptionPort
  ├── ui_projection: UiContributionPort
  └── policy/auth: PolicyPort + SecretReferencePort
```

这些 port 不持有或暴露 `Gateway`、`MCP`、`LSPManager` 或 `EventBus` 实例。事件订阅 port 的实现可以复用 Kernel EventBus，但不能让 Extension 自己创建或持有另一套 EventBus。当前 `EventSubscriptionPort` 已收敛为 DTO-only 边界：输入只包含 Extension topic 字符串、可选 scope 字符串和 Extension handler 合同；`Topic`、`EventEnvelope`、Kernel `EventHandler` / `AsyncEventHandler` 只允许存在于 composition adapter 内部。adapter 负责把 Kernel 事件转换为 `ExtensionEventDto` 后再交给 runtime handler。当前生命周期句柄由 `ExtensionRuntimeHandle` 以 Extension 级聚合形式承载；事件订阅的 opaque `SubscriptionId` 单独由 Extension-owned ledger 持有，它不等同于 Registry 为每个资源返回的 opaque handle。`app/mod.rs` 是唯一负责具体实现装配的 composition root。

### 9.2 Enable 事务与 contribution family 扩展点

目标执行顺序：

```text
load manifest
  → validate schema / permission / IDs
  → normalize contribution families
  → validate family capability / policy / ownership
  → prepare all family registrations
  → commit through family ports
  → record runtime facts and opaque handles
  → publish enabled projection/event
```

当前代码已实现的主干是：读取当前状态，进入 `Loading`，执行 `ensure_enable_preflight()` 和 `do_enable()`，成功后进入 `Enabled`，注册权限约束和 enabled hook 声明；失败进入 `Error`，并尝试清理已提交资源。`ensure_supported_runtime_contributes()` 会对没有真实宿主接线的 contribution fail-closed。Gateway 已由独立 `GatewayContributionFamilyHandler` 负责 normalize/validate/prepare/commit/disable；Gateway 计划转换、提交和逆依赖清理集中在 `lifecycle/register.rs`，正常 disable 与 enable rollback 共用同一套资源清理规则，`state.rs` 不再编排 Provider、capability source、validation 或 protocol。

实现合同：

- prepare 阶段不得改变全局 Registry、EventBus、projection 或 ledger。
- 同一个 contribution family 必须拥有自己的 `normalize/validate/prepare/commit/rollback/disable` handler；主生命周期只编排 family plan，不继续堆叠按字段的巨大 `match`。
- 每个 family handler 必须返回可诊断的 registration fact 或 opaque handle，包含 owner、kind、canonical ID、schema/capability version 和撤销所需事实。
- 新增 contribution 类型只能新增 DTO/schema、family handler、对应 capability port/registrar、projection 和测试；不能修改 Gateway 主流程，也不能要求修改所有已有宿主分支。
- 任一校验失败，整个 Extension 的对应 contribution plan 不可见；同一 Extension 的多个 Provider 不能部分激活。
- commit 阶段记录 Extension 级聚合 `ExtensionRuntimeHandle`，包含已确认的完整 Provider ID、`ApiProtocol` 和各宿主资源的撤销参数；当前不生成逐资源 opaque Registry handle。
- Gateway 的正常 disable 和 enable rollback 必须复用同一份 `unregister_gateway_resources` 逆依赖清理合同：Provider validation → Provider capability source → Provider → Protocol；清理失败的句柄必须留在原账本中以支持重试。
- projection 只有在 commit 完成后发布；不能先显示后注册。
- `eventSubscriptions` 只有在存在明确的 Extension runtime handler execution entry 时才进入 prepare/commit；当前 runtime 入口尚未落地，因此声明会在 preflight 阶段 fail-closed，不会调用 `EventSubscriptionPort`。

### 9.3 Disable / uninstall

- 按注册顺序逆序消费 `ExtensionRuntimeHandle` 中保存的各类资源事实。
- 事件订阅不从可变 manifest 重新推导；`disable` / rollback 只消费 Extension-owned ledger 中已经成功注册的 `SubscriptionId`，并在每个注销成功后删除对应 ledger 记录。
- Adapter 必须在其 Provider 全部撤销后才能移除；Provider 必须在 UI projection 更新后才从 catalog 消失。
- 任一撤销失败都要保留可重试状态、明确错误和 owner 信息；不能只 warning 后假装成功。
- 禁用后不得继续解析该 Extension 的 protocol/provider/model。
- 当前 `prepare_uninstall()` 的真实路径是：Enabled 调用 `disable()`；Error 状态尝试 residual cleanup；Installed/Disabled 直接检查残留；随后检查 `ExtensionRuntimeHandle`、subscription ledger 和 UI registration，存在任一残留就拒绝进入 Installer。Installer 只负责文件和 ExtensionStore，不直接访问 ledger 或 `EventSubscriptionPort`。
- 卸载成功证据必须包括 runtime handle、subscription ledger、UI registration 和 Gateway/MCP/LSP 资源事实全部清空；残留时拒绝完成卸载或进入明确 recovery 状态。
- 旧的裸 `adapter.id`、裸 `provider.id` 不得作为回滚键。

### 9.4 原子性与隔离

- Extension A 的 Adapter 注册失败不能影响 Extension B 已激活的 Adapter。
- Extension A 的多个 Provider 不能部分激活；同一 contribution plan 要么全部 commit，要么全部 rollback。
- Registry 锁、网络请求和用户交互不能在同一不可重入事务中等待。
- 当前每个 `ExtensionRuntimeHandle` 聚合句柄必须能单独诊断 owner、状态、注册时间和撤销结果；若未来拆分逐资源 opaque handle，也必须保留同等诊断信息。
- rollback 必须以已提交事实为准，而不是重新解析可变 manifest；清理失败时只移除已成功撤销的事实，残余事实必须保留并支持重试。
- owner namespace、registration key、schema version 和贡献类型必须进入诊断信息，便于判断冲突是 ID、版本、策略还是宿主未接线。

### 9.5 EventSubscriptionPort 与 Extension-owned ledger

- Kernel EventBus 仍是应用唯一事件总线；Extension 不创建、不替换、不绕过 Kernel EventBus。
- manifest 只声明 `contributes.eventSubscriptions`，每项包含稳定的 `id`、精确 `topic`、可选 `scopeKey` 和受控的 `handler { module, export }` DTO。
- `handler` DTO 不是 Rust handler，也不是可执行脚本。生命周期不得自行加载模块、调用导出函数或把 DTO 强制转换为 `EventHandler`/`AsyncEventHandler`。
- `EventSubscriptionPort` 隔离 Extension lifecycle 与 Kernel EventBus，负责在 runtime 已解析出合法 handler 后执行 subscribe/unsubscribe；Kernel 适配器只在 composition root 装配。
- `ExtensionSubscriptionLedger` 是 Extension-owned 的订阅事实源；当前由 lifecycle 在 port 成功返回后写入 `ExtensionSubscriptionRecord`，ledger 本身不执行 subscribe，只拒绝重复或跨 Extension 复用句柄。
- `ExtensionEventDto` 当前由 Rust `serde` 默认字段名序列化：`created_at` 是实际 wire key；`scope_key` 通过显式 rename 序列化为 `scopeKey`。`createdAt` 不是当前字段，也不是兼容别名。
- enable/commit、disable 和 rollback 都必须以 ledger 为准；注销成功后删除记录，失败则保留记录和 recovery 状态以便重试。uninstall 当前通过先完成 disable/recovery cleanup 再进入 Installer；Installer 不直接消费 ledger。
- 在 runtime execution entry 明确前，声明式事件订阅必须 fail-closed；即使 `app` 已装配 `KernelEventSubscriptionAdapter`，preflight 仍拒绝 `eventSubscriptions`，不会调用 `EventSubscriptionPort`，ledger 也不会写入记录；“manifest 能解析”不等于“订阅已注册”。
- runtime entry 未来必须先解析 handler、校验 ABI/DTO version、权限、超时、取消、背压和配额，再调用 port；只有拿到真实 `SubscriptionId` 后才能提交 ledger 记录。
---

## 十、UI Projection 合同

### 10.1 Gateway catalog projection

前端只通过 Tauri IPC 获取后端 projection。当前 UI Provider DTO 的真实字段为：

```text
GatewayProviderProjection {
  id,              // 完整 provider.id；Extension Provider 使用 extension:<extensionId>/<providerId>
  label,
  description,
  defaultBaseUrl,
  defaultProtocol,
  protocols,
  requiresSecret,
  capabilities,
  capabilityVersion,
  diagnostics,
  configured,
  modelCount,
  availableModelCount,
  models[],
  status
}

GatewayProtocolProjection {
  id,
  name,
  ownerExtensionId,
  capabilities,
  status
}
```

- Projection 不包含明文 `api_key`、secret、模板原文或内部 Rust 类型；`ui_list_gateway_providers` 复用 `Gateway::capability_projection()`，不直接暴露原始 `ProviderConfig`。
- 当前 status 是 `CatalogOnly`、`Available`、`PartiallyAvailable`、`Unavailable`；`Active`、`Rejected`、`Disabled`、`Degraded` 是后续统一状态合同，不能写成当前已实现。
- 当前只有 `configured` 且存在可用模型的 Provider 才会进入运行时可用路径；拒绝原因通过 `diagnostics` 保留在 projection，统一 rejected contribution 诊断视图仍未完成。
- protocol options 来自后端 Registry projection，前端不得硬编码 ChatCompletions/Responses/Custom 三选一。
- Settings 只根据 DTO 的字段渲染，新增 Extension 不需要增加前端 Provider 分支。
- provider/model capability 与 Gateway 入口使用同一 effective capability evaluator 结果；capability version 和 diagnostics 由后端 projection 提供。

### 10.2 HostView 与 Gateway UI 的边界

- Gateway catalog、协议选择和 Provider 配置是普通 UI projection，不把 Gateway 业务对象放进 HostView。
- Extension view 仍遵守 `host:panel` / `html:sandbox` 既有 contract。
- 新 Extension 复用现有 surface 时只提交 manifest 和 projection 数据，不修改宿主业务分支。
- 新 renderer 或新 placement 需要单独的受限 renderer contract、registry 和安全评审；不得通过 manifest 字符串绕过白名单。
- `toolbar_items`、`statusbar_items`、`inline_extensions` 未接入真实 projection 前必须 fail-closed。

---

## 十一、Extension 对其他宿主能力的接入规则

Gateway 适配是一个代表性案例，其他 Integration contribution 也必须遵循相同规则：

| contribution | 事实源 | 接入要求 |
|--------------|--------|---------|
| Gateway Adapter/Provider | ProtocolAdapterRegistry + InMemoryRegistry<ProviderConfig> | 必须完成 schema 校验、协议注册、Provider/Model projection 和生命周期回滚；Extension lifecycle 只能通过 GatewayCapabilityPort 操作。 |
| Middleware | Kernel Pipeline | 必须有真实 phase、owner、顺序和撤销 handle；没有 Pipeline 接入口时 fail-closed。 |
| MCP Server/Transport | MCP Host port | 必须由宿主转换为 MCP DTO/Transport；不能把 manifest module 字符串当作实现。 |
| LSP language/server | LSP Host port | Extension 只声明配置；具体进程、权限和生命周期由 LSP 宿主负责。 |
| UI views/menus/commands | UI projection / HostView | 只使用受控 renderer、placement 和内建 action；无真实 projection 时拒绝。 |
| hooks/context/search | Agent/Application port | 必须通过版本化 DTO 和权限策略接入，不直接访问 Kernel 或 Gateway 内部。 |

这套规则的目的不是让所有 Extension 共用一个巨型接口，而是让每个宿主域拥有自己的高内聚 port，同时由 Extension lifecycle 统一编排 owner、事务和回滚。

### 11.1 Contribution family 可扩展性合同

- `ExtensionContributes` 可以作为 manifest 边界 DTO 保留，但内部必须按 Gateway、MCP、LSP、UI、Agent、Event 等 family 拆分 normalize/validate/register/unregister handler；不能让一个单体结构继续承担所有运行时耦合。
- 每个 family 必须声明自己的 schema version、capability version、owner/full ID 规则、生命周期阶段、权限策略、projection status 和 recovery 事实。
- family handler 通过独立 capability port 调用宿主；具体 Registry、Pipeline、EventBus、Store 或 manager 只在 composition root 装配，Extension lifecycle 不持有这些实现类型。
- 新增 family 的最小变更集合应是“manifest DTO + family handler + capability port/registrar + projection/测试”；不应修改 Gateway 主流程、Kernel EventBus、前端协议分支或已有 family handler。
- 未知 family、未知字段、未知 schema/capability version 默认拒绝；不保留旧字段双读双写、旧命名兼容分支或隐式 fallback。
- 每个 family 必须支持 prepare/commit/rollback/disable 的可观测结果；没有真实宿主接线时必须 fail-closed，而不是在 enable 中记录 debug 后返回成功。
- 对需要执行代码的 family，声明式 DTO 不能冒充 runtime；必须先定义受限 runtime entry、版本化 DTO/ABI、权限、取消、超时、背压、内存/响应大小和资源配额合同。

---

## 十二、测试与验收标准

本章同时记录目标验收合同和当前证据。`[通过]` 只表示当前证据满足该条；`[底座]` 表示已有结构但不能代表完整运行能力；`[未完成]` 表示代码或合同仍缺失；`[失败]` 表示已有测试或命令明确未通过；`[未验证]` 表示本轮没有对应执行证据。

### 12.1 Registry 与 ID

- `[通过]` 内建 Chat Completions、Responses 通过同一 `ProtocolAdapterRegistry` 注册，并由 Gateway 请求路径 `resolve()`。
- `[通过]` Extension Adapter 通过同一 Registry 注册，不修改 Gateway 主流程；未注册 protocol ID 返回错误，不降级到 Chat Completions。
- `[通过]` `ApiProtocol::as_str()`、Extension Provider full ID `extension:<extensionId>/<providerId>`、owner 引用和 registration key 已形成当前 canonical 规则。
- `[底座]` owner、full runtime ID 和 Extension 级聚合 `ExtensionRuntimeHandle` 可用于注册、列表、注销和 rollback；Registry 尚不返回逐资源 opaque handle。
- `[通过当前范围]` Provider capability 已作为独立 source 注册到 `GatewayCapabilityPolicies`；host policy、Adapter、Provider、Model 四方 evaluator、source projection、capability version 和 diagnostics 已接入 Gateway 请求校验与 catalog projection。
- `[未完成]` 参数化能力约束的完整覆盖、声明能力与真实 transform/normalize 的逐项一致性，以及统一 Active/Rejected/Disabled/Degraded status contract。
- `[未验证]` duplicate protocol/provider/model、跨 Extension 隔离、禁用/卸载后所有资源事实清空的完整集成矩阵。

### 12.2 Declarative Adapter

- `[底座]` `CustomProtocolConfig` 的 schema/config version、protocol name、相对 endpoint、模板深度/节点/大小、field path、header 和 capability 基础校验已定义。
- `[底座]` SSE、NDJSON、JSON Lines、单 JSON 的 framing 类型和 request/response/stream/error mapping 字段已定义。
- `[未完成]` 真实 request transform 尚未接入完整 multimodal、tools/tool choice、structured output、`request.extra` 保护和请求限制；当前只使用文本化消息和有限变量。
- `[部分完成]` response/stream transform 已有 reasoning、tool calls、structured output、可选 usage 和 provider error mapping 的声明式路径；usage 缺失通过 `usage_available=false` 表达不可用，仍需补齐下游计量/projection 的统一消费。
- `[部分完成，未验证]` Custom stream 的 provider error、提前断流、取消传播、重复终态和“正常结束恰好一次”已由 Gateway/Stream 统一路径处理；本轮未执行编译或专项回归，完整声明式字段和端到端 HTTP 流测试仍未完成。
- `[部分完成，未验证]` 任意 chunk 边界、重复结束、累计缓冲/事件/响应限制已有 decoder/stream 实现约束；本轮未执行单测，错误/取消终态仍缺少完整 HTTP provider fixture 的全链路验证。
- `[实现已收口，未验证]` Gateway stream 基础执行链与 Custom Adapter request capability 合同已补齐；本轮未执行 `cargo check`，因此不能把它写成编译通过证据。usage、完整 HTTP fixture 和所有声明字段合同仍需后续验证。

### 12.3 Extension 生命周期

- `[通过]` enable/disable 状态主干存在：`Installed/Disabled/Error -> Loading -> Enabled`，`Enabled -> Disabling -> Disabled`；失败进入 `Error` 并尝试清理。
- `[通过]` 无真实宿主接线的 contribution 由 `ensure_supported_runtime_contributes()` fail-closed，不再静默成功；`eventSubscriptions` 在 runtime entry 未明确前不会调用 port。
- `[底座]` `ExtensionRuntimeHandle` 保存已提交资源事实，disable/rollback 以保存事实清理；清理失败保留 residual handle 以便恢复。
- `[通过]` `prepare_uninstall()` 会先执行 disable 或 Error cleanup，再检查 runtime handle、subscription ledger 和 UI registration；有残留则拒绝进入 Installer。Installer 不直接管理宿主资源。
- `[底座]` `EventSubscriptionPort`、`ExtensionSubscriptionLedger`、真实 `SubscriptionId` 写入时机、重复句柄拒绝和注销失败保留记录均已有合同与实现。
- `[实现已覆盖，未验证]` lifecycle 已包含 Provider capability/validation、EventSubscription ledger、rollback residual 和 runtime 未明确时的 fail-closed 路径；本轮未执行专项测试。
- `[底座完成]` contribution family 已有统一 plan/handler 的 normalize/validate/prepare/commit/rollback/disable 抽象、registration fact 和测试承载点；部分 family 已迁移，未迁移或无真实宿主接线的 family 仍 fail-closed。
- `[未完成]` 所有 family 的完整宿主接线、统一 rollback/recovery 集成测试和真正的 Extension runtime execution entry。

### 12.4 Auth 与安全

- `[通过]` Provider 运行时使用 `secret_ref`，没有恢复 `api_key` 兼容读取；Auth Store 是 secret 事实源，Gateway catalog 和 projection 不应携带明文 secret。
- `[底座]` HTTPS/base URL、userinfo/query/fragment/control char、localhost/private/link-local/metadata host、危险端口和相对 endpoint 的安全校验已存在。
- `[底座已完成]` 内建 `openai`/`anthropic` 的 `key_validator` 已通过注入式 `ValidationTransport` 执行 `/v1/models` 请求；`AuthManager::validate_key()` 已持久化四态结果并发布校验事件，相关未知 Provider 和不安全 endpoint 保持不发送请求的 fail-closed 行为。
- `[通过当前范围]` Extension Provider validation request、registry/transport adapter 和 lifecycle plan 已接入；enable 注册、rollback/disable 注销均通过现有 port 完成，不需要在 AuthManager 或 Gateway 主流程增加 Provider 名称分支。
- `[实现已接入，未验证]` Gateway 真实发送阶段已通过 `SecretResolver` 和 `ProviderAuthProfile` 注入短生命周期 secret；zeroize、统一日志/事件/错误脱敏、redirect/DNS rebinding 和完整发送集成仍待验证。
- `[未验证]` 一次性旧配置迁移、zeroize、日志/事件/错误脱敏、redirect/DNS rebinding，以及 Provider validation lifecycle 与发送阶段 Auth 的完整集成测试。

### 12.5 UI Projection

- `[通过]` `Gateway::capability_projection()` 是后端 catalog 入口；`ui/gateway.rs` 从 backend projection 生成 UI DTO，`ui_list_gateway_providers` 不直接暴露原始 `ProviderConfig`，前端不计算协议/Provider/Model capability。
- `[通过]` 未注册协议的模型不会进入 Gateway model projection；Extension Provider 使用完整 runtime ID，projection 不返回模板或 secret。
- `[通过当前范围]` projection 已使用 host policy、Adapter、Provider、Model 四方 effective capability，包含 capability version 和 diagnostics；Provider DTO 还包含 configured、model counts 和当前 status。
- `[未完成]` Active/Rejected/Disabled/Degraded 的统一 status projection、被拒绝 contribution 诊断视图，以及所有参数化能力来源/裁剪原因的完整 UI 合同。
- `[未验证]` 前端全量 UI 测试、无新增宿主分支集成测试和 projection 与实际发送字段的一致性测试。

### 12.6 回归命令与本轮结果

实现阶段至少执行：

```powershell
cd src-tauri
cargo fmt --all -- --check
cargo check
cargo test gateway --lib
cargo test extension:: --lib
cargo test ui:: --lib
cargo test --lib -- --test-threads=1

cd ..
npm run build
npm run test:menus
npm run test:tool-renderers
npm run test:stream
```

本轮未执行以下命令。它们保留为后续验收清单，不代表本轮已通过：

| 命令 | 结果 | 解释 |
|------|------|------|
| 命令 | 本轮状态 | 说明 |
|---|---|---|
| `cargo check --manifest-path src-tauri/Cargo.toml` | 未执行 | 不能据此确认 Rust 编译状态。 |
| `npm run build` | 未执行 | 不能据此确认前端 TypeScript/Vite 构建状态。 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml` | 未执行 | 本轮未做格式化。 |
| `cargo test --manifest-path src-tauri/Cargo.toml extension::lifecycle --lib -- --test-threads=1` | 未执行 | 生命周期、ledger、capability 和 residual cleanup 未在本轮运行。 |

历史工作区中可能存在早期命令输出，但不作为本轮改造的验收证据；Gateway HTTP provider fixture、完整 Auth 脱敏和部分 UI 专项仍待后续验证。
---

## 十三、实施顺序

实施顺序必须先稳定边界和事实源，再接入真实执行能力；不能先开放 manifest 声明，再用后续兼容分支补救。每一阶段完成后都必须满足本阶段的验收证据，未满足时后续阶段只能保留 fail-closed。

### 阶段 0：合同冻结与事实源确认（前置）

1. 冻结 `Extension` 唯一业务命名、canonical ID、owner namespace、schema/capability version 和未知字段策略。
2. 明确 Kernel Registry、Pipeline、EventBus、Auth Store、Gateway projection 是唯一事实源；不新增平行基础设施。
3. 为每个 contribution family 建立 DTO、capability port、registration fact、projection status 和 recovery fact 的责任表。
4. 明确哪些 contribution 需要 runtime execution entry；没有 runtime entry 的声明必须在 preflight fail-closed。
5. 通过文档审计和 schema fixture 固化“无兼容代码”：新运行时只接受新 schema，不保留旧字段双读双写和旧分支。

### 阶段 A：Gateway Registry、ID 与 capability projection（当前范围已完成）

1. `[已完成]` 复用 Kernel `InMemoryRegistry`，注册 Chat Completions、Responses 和声明式 Custom Adapter。
2. `[已完成]` Gateway 请求路径通过 Registry resolve；新增 protocol 不修改 Gateway 主流程。
3. `[已完成]` 统一 canonical protocol ID、Extension Provider full ID、owner 引用和 Extension 级聚合 Runtime Handle。
4. `[已完成当前范围]` 输出 protocol/provider/model projection，拒绝未注册协议模型；Provider capability source、host policy、四方 evaluator、capability version 和 diagnostics 已接入。
5. `[部分完成]` request capability token 与真实插值的一致性已接入；仍需补齐所有参数化能力约束、transform/normalize 的逐项覆盖，以及统一 status contract 和完整 projection 测试。

### 阶段 B：声明式 Adapter 真实执行合同

1. `[底座已完成]` 保留现有 `CustomProtocolConfig` schema、模板、路径、framing、header 和大小限制校验，不重复造 Registry/Parser 基础设施。
2. `[已完成当前范围]` request capability validation 已接入配置校验，真实插值只识别 `{{request.*}}`；tools、reasoning、structured output、streaming 的声明与 request template 消费必须一致，普通文本和 JSON key 不会被误判为能力 token。
3. `[部分完成]` content、reasoning、tool calls、structured output、可选 usage、provider error 已有普通响应和流事件 mapping 路径；usage 缺失通过 `usage_available=false` 表达不可用，仍需补齐下游计量/projection 对该语义的统一消费，以及完整 HTTP provider fixture 和声明式字段覆盖测试。
4. `[部分完成，未验证]` 现有 stream/framing 管线统一处理任意 chunk 边界、done marker、提前断流、重复结束、错误、取消、背压和累计资源限制；本轮未执行编译或回归，完整 HTTP provider fixture 与声明式字段全覆盖测试仍待完成。
5. `[部分完成]` 当前声明能力与 request template 的真实消费已保持一致；仍需继续覆盖所有 transform/normalize 参数化能力，未接入能力继续拒绝注册或从 effective projection 中移除。
6. `[验收门槛]` `cargo check` 无上述 helper/限制常量未使用 warning；Custom Adapter request/response/stream 专项测试全绿后才可把声明式 Adapter 标记为已完成。

### 阶段 C：Contribution family handler 与生命周期事务

1. `[底座已完成]` 保留现有 Extension lifecycle、Gateway/MCP/LSP/UI/EventSubscription port、聚合 Runtime Handle 和 subscription ledger。
2. `[底座已完成]` contribution family 已有统一 normalize/validate/prepare/commit/rollback/disable plan/handler 抽象；已迁移的 family 通过独立模块承载，主生命周期只编排 plan。
3. `[部分完成]` family handler 已具备 registration fact、owner/canonical ID、版本、撤销参数和 projection/recovery 承载点；所有 family 的真实宿主接线、统一状态投影和完整 recovery 集成测试仍未完成。
4. `[已完成/需回归]` 未接线 contribution fail-closed；必须修复现有生命周期失败测试并补齐多 Extension 隔离、部分 commit、rollback 和 uninstall recovery 集成测试。
5. `[未完成]` 不得把 `module/export` 字符串当成执行能力；需要执行代码的 family 先完成 runtime entry、权限、超时、取消、背压、内存和资源配额合同。

### 阶段 D：Auth 与安全策略

1. `[底座已完成]` Provider 只保存 `secret_ref`，Auth Store 作为 secret 事实源，Endpoint/header/template 基础安全校验复用现有实现。
2. `[已完成当前范围]` 通过 `SecretResolver` 在非流式和流式发送阶段解析短生命周期 secret；`ProviderAuthProfile` 统一构造内建及 Extension Provider 认证 header；`SecretValue` 在 Drop 时 zeroize。完整发送集成测试、日志/事件/错误脱敏和安全边界审计仍未完成。
3. `[已完成当前范围]` 保留现有 `SecretValidator`/`ValidationTransport` 四态合同及内建 OpenAI/Anthropic `/v1/models` 校验；Extension Provider validation request、registry/transport adapter 和 lifecycle plan 已接入，使 Provider/Adapter 能声明验证请求、credential/header 构造、状态映射、超时等，不修改 AuthManager 或 Gateway 主流程。
4. `[验收门槛]` 未知 Provider、无验证入口、策略拒绝、超时和无法安全判断必须 fail-closed 为 `Unknown`；任何仅 TCP 可达即判 `Valid` 的实现均不通过。
5. `[验收门槛]` 旧 schema 一次性迁移后删除明文字段；运行时遇到旧字段直接报迁移错误，不保留兼容读取。

### 阶段 E：后端 projection 与 UI 解耦

1. `[已完成当前范围]` `Gateway::capability_projection()` 和 `ui/gateway.rs` 已形成后端 projection 消费路径；Provider UI projection 复用 capability projection，不直接返回原始 `ProviderConfig`。
2. `[已完成当前范围]` protocol/provider/model DTO 已带 capability version、来源/裁剪 diagnostics、configured/model counts 和当前 Provider status；完整 Active/Rejected/Disabled/Degraded 合同仍未完成。
3. `[已完成当前范围]` 新增 Extension 在 Gateway Provider projection 上只增加 manifest/contribution/adapter 数据，不增加 Settings 分支、协议 if/switch 或 HostView 业务分支。
4. `[未完成]` 对未接线 UI contribution 保持 fail-closed，补齐统一 status、被拒绝 contribution 诊断视图和 projection 测试。

### 阶段 F：Extension runtime 事件执行入口（独立后续阶段）

1. 定义 runtime handler 的加载边界、权限、ABI/DTO version、超时、取消、背压、错误传播和资源配额。
2. runtime 接收稳定 `ExtensionEventDto`，不得暴露 Kernel `EventEnvelope`、Rust handler trait 或内部 Store。
3. runtime 解析并验证 handler 成功后调用 `EventSubscriptionPort`；只有返回真实 `SubscriptionId` 才写入 Extension-owned ledger。
4. 复用 Kernel EventBus 的订阅和投递能力，不新增平行事件总线；unsubscribe 失败保留 ledger 记录并进入 recovery。
5. runtime entry、审计和配额完成前，`eventSubscriptions` 保持 fail-closed；manifest 可解析不等于 handler 已激活。
---

## 十四、文档同步清单

本文件是本次 Gateway × Extension 复核的架构基线；实现阶段必须同步检查以下文档，避免出现多个互相冲突的事实源：

| 文档 | 必须同步的主题 |
|------|----------------|
| `design/05-auth.md` | Gateway 使用 `secret_ref`，Auth Store 是密钥事实源，校验结果语义和迁移策略。 |
| `design/07-extension.md` | Gateway Adapter/Provider 新 schema、声明式边界、Integration fail-closed、Extension lifecycle。 |
| `design/22-ui-framework.md` | Gateway catalog projection、HostView renderer/placement 边界和 UI 不加载任意代码。 |
| `design/README.md` | 模块索引、Gateway/Extension 文档关系和 application/runtime port 位置。 |
| `AGENTS.md` | 顶层域、命名、Registry/Pipeline/EventBus/Auth/UI projection 依赖方向。 |
| Gateway 代码注释 | Registry 为唯一协议事实源，禁止重新引入静态 protocol match。 |

同步规则：

- 文档只描述当前已实现能力或明确标记“目标合同/未实现”。
- 未接入真实 Registry、Pipeline、Policy、EventBus 或 UI projection 的声明必须标记为 fail-closed。
- 任何新增协议只新增 Adapter/manifest/测试，不新增 Gateway 主流程分支。
- 业务命名统一使用 `Extension`，不再引入旧的业务命名。

---

## 十五、最终验收结论标准

本轮当前结论不是“全部完成”，而是“Registry/port/ledger/schema 底座已形成，四方 capability evaluator、Provider capability source/lifecycle/UI projection、Provider validation lifecycle 和 Gateway 发送阶段 Auth 已接入当前范围；Custom Adapter 完整执行、Extension runtime entry、所有 family 的真实宿主接线、统一 projection status、全链路脱敏和完整集成测试仍未完成”。只有下表全部达到“通过”，才能称为高扩展 Extension Gateway 架构完成。

| 验收项 | 当前状态 | 通过证据 / 未完成条件 |
|------|------|------|
| Gateway 主流程不根据协议枚举写死 Adapter | **通过** | Gateway 请求路径经 ProtocolAdapterRegistry resolve；新增协议不应修改主流程。 |
| 内建协议和 Extension 协议通过同一 Registry 解析 | **通过** | Chat Completions、Responses、Custom Adapter 共用 Registry 注册/解析路径。 |
| Extension manifest 完整声明 Adapter、Provider、Model、capabilities 和认证需求 | **部分通过** | DTO、Provider capability source/lifecycle、四方 evaluator 和当前 UI projection 已存在；参数能力完整覆盖、统一 status 和所有声明能力的真实执行一致性仍缺失。 |
| 声明式 Adapter 完整处理请求、普通响应、stream framing、usage、错误和取消 | **部分完成，未验证** | reasoning/tool/structured output/provider error 已有 mapping 子集；stream error/incomplete/cancel/重复终态/恰好一次终态已接入统一路径；usage 缺失通过 `usage_available=false` 表达不可用，下游 usage 消费和完整 HTTP fixture 尚待完成。 |
| 不支持能力、未知协议、不安全 endpoint、坏 schema 和未接线 contribution 全部 fail-closed | **部分通过** | Registry、schema、安全、request capability contract 和未接线 contribution 有 fail-closed；仍需继续覆盖所有 transform/normalize 参数化能力。 |
| Provider/Model 使用稳定作用域 ID，生命周期回滚无残留 | **实现已覆盖，未验证** | full ID、聚合 Runtime Handle、capability source 逆序 cleanup、卸载残留检查和 residual retry 已接入；本轮未执行 lifecycle 测试，逐资源 opaque handle 仍是后续增强目标。 |
| Gateway 不保存明文 secret，前端和日志不泄漏 secret | **部分通过** | `secret_ref`、Auth Store、`SecretResolver`、`ProviderAuthProfile`、请求阶段 header 注入和 `SecretValue` zeroize 已接入；完整日志/事件/错误脱敏、发送 HTTP fixture 和跨层集成测试仍待完成。 |
| Extension lifecycle 不依赖 Gateway/MCP/LSP 具体类型 | **通过底座合同** | lifecycle 持有 capability port，具体实现由 `app/mod.rs` composition root 装配；仍需保持 family handler 不倒灌具体 manager。 |
| UI 只消费后端 projection，新 Extension 不要求宿主 Provider/协议分支修改 | **通过当前范围** | `Gateway::capability_projection()` 和 `ui/gateway.rs` 已形成消费路径；Provider DTO 已包含四方 effective capability 的 version/diagnostics/configuration/model counts/current status；统一 status 和全量 UI 测试仍未完成。 |
| EventSubscriptionPort 隔离 Kernel EventBus，ledger 管理真实句柄 | **底座完成，runtime 未完成** | port、Kernel adapter、Extension-owned ledger、真实 SubscriptionId 记录时机和 cleanup 规则已存在；runtime handler execution entry 未明确，manifest event subscription 当前 fail-closed。 |
| 新增 contribution 不修改宿主主流程，扩展只增加 family handler/port/projection | **底座完成，runtime 未完成** | 统一 family plan/handler 的 normalize/validate/prepare/commit/rollback/disable 底座已存在，部分 family 已迁移；所有 family 的真实宿主接线、统一 registration/recovery projection 和完整测试仍缺失。 |
| 测试覆盖 Registry、Adapter、framing、capability、安全、生命周期、回滚、事件订阅和 UI projection | **实现已覆盖，未验证** | 本轮未执行 `cargo check`、`cargo fmt`、Rust 测试或 `npm run build`；Custom Adapter HTTP fixture、完整 Auth 脱敏和部分 UI 专项仍待补齐。 |

最终发布门槛：

1. 先完成第 13 章阶段 A-F 中标为“未完成”的合同，再运行第 12.6 节全部回归命令。
2. 所有 `[未完成]`、`[失败]`、`[未验证]` 项必须清零；warning 不能掩盖未接入的执行 helper。
3. `eventSubscriptions` 在 runtime execution entry、权限、DTO/ABI、取消、背压、配额和审计完成前继续 fail-closed。
4. 新增协议、Provider、Model 或 contribution family 的演示必须只增加 Extension 声明、Adapter/family handler 和 projection 数据，不修改 Gateway 主流程、Kernel EventBus、前端协议分支或旧兼容分支。
5. 任何旧字段双读双写、旧业务命名、裸 contribution ID 回滚键或静默 fallback 一旦重新出现，验收立即失败。

在上述条件全部满足前，只能称为“部分接入/底座已完成”，不能称为“高扩展 Extension Gateway 架构完成”。

---

## 十六、全量代码域复核（2026-08-15 晚）

> 本节是对 `src-tauri/src` 全部业务域与 `src/` 前端的独立复核。复核时工作区代码已在同日 19:00–20:06 间完成一轮大范围重构（协议注册表、key_validator 协议级校验、extension gateway contribution、前端 catalog 化均已落地），因此本节以当前代码为准，并记录了与先前状态的关键差异。

### 16.1 已确认修复（旧版问题在本轮重构中消除）

| 旧问题 | 当前状态 | 证据 |
|--------|----------|------|
| `adapter_for_protocol` 硬编码 match，Custom 直接 `bail!` | **已修复** | 全库已无 `adapter_for_protocol`；请求路径统一经 `ProtocolAdapterRegistry.resolve()`（`ai/gateway/mod.rs:840-842`、`1040-1042`；`ai/gateway/protocol/registry.rs`） |
| key_validator 假校验（TCP 连通即 Valid，未知 Provider 默认 localhost:8080） | **已修复** | `security/auth/key_validator.rs` 已改为注入式 `ValidationTransport` 协议级 HTTP 校验；未知 Provider 返回 `Unknown` 的 fail-closed 保留（provider_validation.rs 注释与代码一致） |
| 扩展 provider 注册回滚 id 不匹配导致 provider 残留 | **已修复** | `extension/lifecycle/state.rs:439` 回滚记录完整 `provider_id`；`mod.rs:999-1008` 用完整 ID 逆序清理 |
| api_key 明文写入 config.json | **已修复** | 前端与 `UiGatewayProviderConfig` 只携带 `secret_ref`（`ui/dto.rs:279-287`），`ui_save_gateway_config` 不再写明文；secret 事实源在 `security/auth/key_store.rs`（AES-256-GCM） |
| 前端 `ConnectionType` 硬编码三选一、`protocolOptions()` 无 Custom 入口 | **已修复** | `src/components/Settings/gateway-config-model.ts` 已 catalog 化：`BUILTIN_PROVIDER_TYPES` 只用于识别内建 provider，协议选项来自后端 catalog |
| `host_view` 仅 `host:panel` 单一 renderer | **已修复** | `extension/host_view.rs` 支持 `host:panel` + `html:sandbox` 双 renderer 与 4 个 placement；前端 `HostView/registry.ts` 有对应条目 |

### 16.2 新增确认问题（本轮复核直读证据）

#### 已修复：Extension Provider 请求阶段认证接线

- Extension lifecycle 已将 manifest 的 `auth.scheme/header/secretRef` 转换为 `ProviderAuthProfile` 并写入 `ProviderConfig`。Gateway 在非流式和流式 dispatch 前统一解析 `secret_ref`，通过 `SecretResolver` 获取短生命周期 `SecretValue`，再调用 `ProviderAuthProfile::auth_headers()` 注入认证 header；缺少 secret、无效 secret reference、认证 profile 不允许绑定 secret 时均 fail-closed。
- 内建 Provider 的固定头仍由现有 Provider profile 维护，`validate_custom_header()` 继续禁止 Adapter/request 层覆盖认证保留头。
- 结论：Extension Provider 声明的 `auth.scheme/header/secret_ref` 已通过 Gateway 的 `SecretResolver` 接入真实请求阶段，解析出的短生命周期 secret 会由 `ProviderAuthProfile` 注入认证 header；缺失 secret、无效引用和不允许绑定 secret 的 profile 保持 fail-closed。剩余风险是完整发送集成 fixture、日志/事件/错误脱敏审计和跨层验证，本轮未执行测试或构建。


#### P2：CustomAdapter 声明能力与真实执行能力一致性（实现已补齐，未验证）

- `ai/gateway/protocol/custom.rs` 已有 reasoning、tool calls、structured output、可选 usage 和 provider error 的 mapping 路径；usage 缺失通过 `usage_available=false` 表达不可用。
- request capability validation 现在只把真实字符串插值 `{{request.*}}` 视为能力消费，不再把普通文本或 JSON object key 误判为 token；声明 tools、reasoning、structured output 或 streaming 的 Adapter 必须消费对应 request token，未声明能力却使用对应字段仍 fail-closed。
- 本轮同步了 Custom Adapter 的能力 fixture，但未执行测试或构建；完整 HTTP fixture、声明式字段覆盖和实际 transform 一致性仍待后续验证。
- `ai/gateway/mod.rs` 已完成 stream sender 取消入口、bytes/select 所有权、provider error、提前断流、cancel、重复终态和一次性终态的统一处理；本轮未执行 `cargo check` 或 Rust 测试。Custom Adapter 现在要求声明能力与 request template 的真实插值一致，未接入能力保持 fail-closed。

### 16.3 各业务域复核结论汇总

| 域 | 评分 | 核心结论 |
|----|------|----------|
| kernel | A | 仅 Registry/Pipeline/EventBus/Policy 四原语；`boundary_test.rs` 有 E1 导入边界、E2 业务词汇 CI 门禁；无业务泄漏，无 P1。 |
| foundation | B | P1-1 `MaskingLayer::on_event` 只读观察无法改写字段（脱敏无效）；P1-2 `logger/mod.rs:131` non_blocking `_guard` 在 `init()` 返回即 drop（文件日志不可用）；P1-3 运行时加密未接线（`app/mod.rs` 传空 Encryption）。P2：`session_store.rs` 反向依赖 project 域；cache/cleanup/backup 疑似无生产调用方；P4：`reliability.rs` 死配置。 |
| security | A- | auth + sandbox 边界清晰；key_validator 已重构为协议级（见 16.1）；P3-5 `credential.rs` 契约不一致；P4 `sandbox/resource_limit.rs` 未接入拦截路径。 |
| ai | A- | 协议注册表与四方 capability evaluator/projection 落地（16.1）；P2：`provider/profile.rs` static 数组不可扩展；`ai/agent` 对 `tool::agent`/`extension::models` 类型级依赖；`ai/context` 直接依赖 foundation storage session_store 具体结构体。Extension Provider 发送阶段 Auth 已接入，剩余为集成覆盖和全链路脱敏审计。 |
| tool | A- | MCP 是唯一可执行工具注册表，全走 Kernel 原语；tool 不依赖 ui/project/ai::agent，仅反向依赖 application/runtime 与 extension；12 类 contributes 被 `reject_unbound` fail-closed，与“万物皆扩展”目标冲突（须按阶段 C family handler 补齐）。 |
| project | B | P1 foundation↔project 循环依赖 + SQL 数据访问层放领域域（`foundation/storage/session_store`）；`project/knowledge/project_knowledge.rs`（1440 行）为死模块零生产调用方；`session/composer_runtime.rs` 不属于 Session 领域。 |
| extension | B+ | 回滚 id bug 已修复（16.1）；Gateway capability source、Provider validation lifecycle、EventBus port/ledger 底座和 family plan/handler 底座已装配在 composition root/lifecycle；发送阶段 Auth 已接入当前范围；所有 family 真实 runtime 接线与 Extension runtime entry 仍未完成。 |
| ui + app | A- | `ui/host_view.rs` 双 renderer 完整；`ui/gateway.rs` 只消费 backend projection；`app/mod.rs` 以 composition root 装配所有能力 port，无反向倒灌。 |
| application | A- | `application/runtime/agent_control.rs` 的 Todo/Sidechain/AgentToolEvent ports 已被 `ui/runtime` 与 `tool/agent/special` 使用，是 B 阶段解耦的复用范式；但 application 域不在 AGENTS.md 九域清单中（P4 文档漂移）。 |
| 前端 | B+ | secretRef 化、catalog 化、ConnectionType 硬编码已消除（16.1）；`GatewayApiProtocol` 类型上仍带 `Custom: string` 变体（遗留）；HostView renderer/placement 与后端一致。 |

### 16.4 高内聚 / 低耦合 / 规范符合度评价

- **高内聚**：良好。Kernel 最小原语、MCP 单一工具注册表、Gateway 协议分发注册表化、security 职责单一，各域内部聚合度高。
- **低耦合**：中上。能力 port（`GatewayCapabilityPort`/`McpCapabilityPort`/`LspCapabilityPort`）与 composition root 已消除 extension→具体业务类型的倒灌；剩余耦合：foundation↔project 循环、ai 对 storage/tool/extension 的类型级依赖、`project/knowledge` 死模块。
- **规范符合度**：良好。`tracing` 日志、Tauri `invoke` IPC、Kernel EventBus、store 纯函数 action 均符合 AGENTS.md；余项为 16.3 中 P2/P3/P4。
- **高扩展 / 万物皆可 Extension**：协议注册表、四方 capability projection、Provider validation lifecycle、ProviderAuthProfile 发送阶段 Auth、声明式 CustomAdapter、统一 Stream 终态处理和 catalog 已形成可复用底座；仍受 request capability validation、完整 HTTP fixture、Extension runtime entry、全链路脱敏和未接线 family（transport_adapters、middlewares、themes、roles 等 fail-closed）限制，按第 13 章剩余阶段完成后方可达标。

### 16.5 建议实施优先级

1. 补齐 Extension Provider 发送阶段 Auth 的 HTTP fixture、缺失 secret、错误 header、取消/重试和脱敏集成测试。
2. 阶段 D 其余：Gateway 发送阶段 zeroize/脱敏、运行时加密接线（foundation P1-3）。
3. 阶段 B：补齐 CustomAdapter request capability validation、下游 usage 可用性消费、Stream 统一路径 HTTP provider fixture 与声明式字段全覆盖测试。
4. 阶段 C/F：完成剩余 contribution family 宿主接线和 Extension runtime handler execution entry，解除相应 fail-closed。
5. 清理项：project/knowledge 死模块、foundation↔project 循环、logger P1-1/P1-2、reliability 死配置。
