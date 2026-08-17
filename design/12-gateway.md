# 12 - Gateway 模型网关详细设计

> 模块编号：12 | 层级：能力层
> 依赖：01-Logger、02-IPC、03-Config、05-Auth、Kernel Registry/Pipeline/Policy
> 被依赖：Agent、Session、Context、UI projection

---

## 一、定位与不变量

Gateway 是 Navis Go 的统一模型调用入口。Agent、上下文压缩、摘要、代码解释和其他需要 LLM 的用例都只依赖 Gateway 的统一请求/响应模型，不直接依赖某个厂商协议。

核心不变量：

1. Provider、Model 和协议 Adapter 分离，分别负责连接实例、模型能力和协议转换。
2. 内建协议与 Extension 协议都通过同一个 ProtocolAdapterRegistry 解析。
3. 新增协议只新增 Adapter 或声明式 Extension contribution，不修改 Gateway 主流程的协议分支。
4. Provider 只保存 secret_ref；Gateway 通过 SecretResolver 获取短生命周期 secret，不持有 AuthStore 或明文密钥。
5. Gateway 不依赖 Extension lifecycle；Extension lifecycle 只通过 GatewayCapabilityPort 使用 Gateway。
6. UI 只消费 Gateway catalog projection，不硬编码协议、Provider 或 Model 分支。

Gateway 不负责 Agent 决策、Prompt 组装、工具选择、Session 持久化、Extension 生命周期、Auth 存储、MCP server 生命周期、LSP 生命周期或 UI renderer 生命周期。

---

## 二、代码边界

src-tauri/src/ai/gateway/ 下的职责如下：

- mod.rs：Gateway composition、统一请求入口、Provider 生命周期边界。
- request.rs：ChatRequest、ApiProtocol、ProviderConfig、ModelConfig。
- response.rs：ChatResponse、StreamChunk、TokenUsage、OutputItem。
- router.rs：Provider/Model 路由。
- protocol/adapter_trait.rs：ProviderAdapter 协议转换合同。
- protocol/registry.rs：ProtocolAdapterRegistry，内部复用 Kernel InMemoryRegistry。
- protocol/chat_completions.rs：内建 Chat Completions Adapter。
- protocol/responses.rs：内建 Responses Adapter。
- protocol/custom.rs：受限声明式 Custom Adapter。
- middleware.rs、quota.rs、cost.rs、offline.rs、multimodal.rs：Gateway 横切能力。

Gateway 使用 Kernel 的 Registry、Pipeline、EventBus 和 Policy 原语，但 Kernel 不感知 Provider、Model、协议或 Extension manifest。

---

## 三、核心对象关系

Extension manifest -> Extension lifecycle -> GatewayCapabilityPort -> ProtocolAdapterRegistry -> ProviderAdapter。

ProviderConfig -> ModelConfig -> Gateway router -> HTTP dispatch。

ProviderConfig.secret_ref -> SecretResolver -> temporary secret。

### 3.1 Adapter

ProviderAdapter 只负责统一请求与目标协议之间的转换：name、protocol、registration_key、transform_request、transform_response、transform_stream_chunk、endpoint、inject_required_fields。

Adapter 不注册自己、不管理 owner、不访问 AuthStore、不决定 Provider 默认模型。注册和释放由 Gateway registry 与 Extension lifecycle 负责。

### 3.2 Provider

ProviderConfig 是可路由的连接实例，包含 id、provider_type、name、base_url、secret_ref、models、default_model。`id` 是 Provider 的唯一事实源；`provider_type` 仅保留 builtin profile/auth fallback 所需的类型语义，不参与 Extension Provider 合并、catalog 主键或协议路由。Provider 不携带协议转换逻辑，也不把厂商名称当作协议类型。

### 3.3 Model

ModelConfig 是 Provider 下的能力声明，至少包含 id、name、api_protocol、context_window、max_output_tokens，以及 tools、streaming、multimodal、reasoning 等能力字段。UI、Agent 和请求构造都读取同一份 effective capability，不自行推断。

### 3.4 协议标识

ApiProtocol 的 canonical string 由 `ApiProtocol::as_str()` 提供：

- ChatCompletions -> `chat_completions`。
- Responses -> `responses`。
- Custom(name) -> `name` trim 后的原值，不自动添加 `custom:` 前缀。

Extension 的 `contributes.gateway.adapters[].protocolId` 是自定义协议的 canonical ID 来源，经非空、无空白和禁止显式 `custom:` 前缀校验后，原样进入 `ApiProtocol::Custom` 与 ProtocolAdapterRegistry。当前运行时不会根据 `extensionId + adapterId` 重新生成协议 ID；`adapterId` 只用于 Provider 对同一 Extension 内 Adapter contribution 的引用。Provider 模型的 `api_protocol` 必须与 Registry 中的 canonical ID 精确匹配，未知协议 fail-closed。

---

## 四、ProtocolAdapterRegistry

ProtocolAdapterRegistry 是 Gateway 唯一的 Adapter 目录，职责包括：

- 注册内建 Chat Completions 和 Responses Adapter。
- 注册、解析和释放 Extension Adapter。
- 以 registration_key 检测冲突。
- 按 owner 保留引用计数，最后一个 owner 释放后注销 Adapter。
- 向 UI 输出不包含模板、secret 和 Rust 具体类型的 ProtocolAdapterInfo projection。

Gateway 请求主流程只执行：路由 Provider/Model -> Registry resolve api_protocol -> Adapter 转换请求 -> HTTP dispatch -> Adapter 转换响应/流。

禁止为新协议在 Gateway 主流程增加 match 分支。重复注册、未知协议、无效 owner 和未满足依赖都必须拒绝，不静默回退。

---

## 五、Extension Gateway contribution

Extension manifest 只声明数据和受限配置，不把任意模块字符串当作协议实现直接执行。结构固定为 contributes.gateway.adapters 和 contributes.gateway.providers。

Adapter 字段：id、name、protocolId、kind、config。

Provider 字段：id、name、adapterId、baseUrl、auth、models、defaultModel。manifest 内的 Provider `id` 在 Extension 内唯一；注册到 Gateway 后使用完整运行时 ID `extension:<extensionId>/<providerId>`。

Auth 字段：scheme、secretRef、header。Model 字段：id、name、capabilities、contextWindow、maxOutputTokens。

Gateway schema 使用 camelCase 和 deny_unknown_fields。Adapter、Provider、Model ID 必须在 Extension 内唯一；adapterId 必须存在；defaultModel 必须指向声明的 Model。

Extension lifecycle 启用事务：

1. 解析并校验 manifest，生成 Gateway plan。
2. 为内建协议 acquire owner；为声明式协议注册 `CustomProtocolConfig`，协议 ID 使用 manifest 的 canonical `protocolId`。
3. 注册全部 Provider，Provider 使用完整运行时 ID `extension:<extensionId>/<providerId>`。
4. Provider 全部成功后再提交其他 contribution。
5. 任一步失败，按逆序释放 Provider、协议 owner 和其他资源。

当前生命周期保存的是 Extension 级聚合 `ExtensionRuntimeHandle`，其中记录已提交的完整 Provider ID、`ApiProtocol` 值以及 MCP、LSP、Skill 和 UI 等资源事实；它不是 ProtocolAdapterRegistry 返回的逐资源 opaque handle。禁用或回滚会消费该已保存事实，清理失败时保留残留 handle，使下一次 disable/enable retry 可以继续清理。逐资源 opaque Registry handle 仍属于后续增强目标，不应在当前实现中描述为已完成。

禁用与卸载按相反顺序执行。Gateway 只提供 capability port，不反向调用 Extension lifecycle。

---

## 六、声明式 Adapter 安全合同

声明式 Adapter 必须满足：

- endpoint 只能是 Provider base_url 下的相对路径，禁止通过 manifest 改写到任意主机。
- method、header、模板变量、JSON path 和 stream framing 必须经过 schema 校验。
- header 名称和值禁止 CRLF；secret 不得进入日志、错误、事件或 projection。
- 请求模板变量只能来自统一请求字段；未知变量直接拒绝。
- 响应路径受大小、深度、数组和字段数量限制。
- SSE、NDJSON、JSON Lines 必须显式声明 framing，不能猜测。
- content、reasoning、tool call、finish reason、usage、provider error 和 done marker 分别声明。
- 流提前断开、解析失败和 provider error 不得伪装成正常完成。
- usage 缺失保持 None，不能用零 token 冒充真实计量。

声明式 Adapter 的输出必须归一化为 ChatResponse 和 StreamChunk，不能把 Provider 原始 JSON 泄漏到 Agent 业务层。

---

## 七、Secret 与 HTTP dispatch

Gateway 只依赖 SecretResolver.resolve_secret(secret_ref)。请求流程为：Router 获取 Provider/Model，Registry resolve Adapter，Adapter 构造 body/endpoint，Gateway 通过 secret_ref 获取临时 secret，按 scheme 注入 header，HTTP client 执行请求，Adapter 归一化响应。

Config、Gateway、UI DTO、日志和事件禁止出现 api_key、明文密钥或 secret 内容。secret_ref 是 Auth Store 的 opaque reference，不代表 Gateway 可以读取存储实现。

---

## 八、统一请求、响应与横切能力

统一 ChatRequest 至少包含 model、messages、tools、temperature、max_tokens、reasoning_effort、stream 和 extra。统一 ChatResponse/StreamChunk 至少包含模型标识、文本/结构化输出、finish reason、tool call、reasoning summary 和可选 TokenUsage。

Middleware 在 Gateway boundary 执行，具体 stage 由 Kernel Pipeline 承载。Quota、rate limit、审计和策略通过 Kernel Policy、Gateway 计量器和 EventBus 协作。Retry 只处理明确可重试的网络/服务错误。多模态图片、文件大小和 MIME 校验属于 Gateway request boundary，不进入 Adapter 业务实现。

---

## 九、UI catalog projection

ui_get_gateway_catalog 返回 protocols、providers、models projection。projection 只包含协议/Provider/Model 的展示和能力摘要，不包含模板、secret、内部 Rust 类型和���感错误细节。

UI 不硬编码 chat_completions、responses 或任意 Provider 分支。Provider、Model 或协议不可用时显示后端状态，不伪造能力。新增 Extension Provider/Model 只增加 projection 数据，不增加 Settings 分支。

---

## 十、测试与验收

必须覆盖 ApiProtocol round-trip、内建 Adapter Registry resolve、自定义 Adapter owner acquire/release、重复 ID、坏 adapterId、缺失 defaultModel、未知协议、Extension enable 回滚、secret_ref 隔离、SSE/NDJSON/JSON Lines framing、done marker、usage、finish reason、provider error、提前断开、模板/path/header/endpoint/大小限制和动态 catalog。

完成标准：新增模型、Provider 或协议时只新增 Adapter 实现或 Extension manifest；Gateway router、UI Settings 和 Kernel 不修改。所有未知能力 fail-closed，禁用 Extension 后无残留运行时资源。
