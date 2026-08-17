# 03 - Config 配置管理详细设计

> 模块编号：03 | 层级：基础服务层
> 依赖：01-Logger、02-IPC、05-Auth、foundation/storage
> 被依赖：所有需要持久化配置的业务域

---

## 一、定位

Config 是分层配置的读取、校验、持久化和变更通知服务。它保存用户偏好和业务配置快照，但不实现 Gateway、Auth、Extension 或其他业务域的运行时行为。

Config 负责：

- 系统默认、用户、项目、模式和运行时配置的合并。
- JSON 配置读写、schema 校验、导入导出和热更新。
- 配置变更事件和只读 projection。
- 应用窗口状态、偏好和 Gateway Provider 配置的持久化。

Config 不负责：

- secret 加密、存储、轮换或解析，统一由 Auth/SecretResolver 负责。
- Gateway 协议请求、Provider 路由或 Model 能力计算。
- Extension manifest 解析、启用、禁用和回滚。
- Session 临时状态、数据库存储引擎和 UI 组件状态。

---

## 二、分层与事实源

读取优先级从高到低：运行时 > 模式 > 项目 > 用户 > 系统默认。

用户级配置由 Tauri app data 目录中的 config.json 承载。该文件保存配置结构和 opaque secret_ref，不保存 api_key 或其他明文 secret。Auth Store 是 secret 内容的唯一事实源；Config 只保存引用。

Gateway 配置的事实关系：

- Config 保存 Provider 的连接元数据、模型列表、默认模型和 secret_ref。
- Gateway 在启动或配置变更时读取 Config 快照，构造 Provider/Model 路由。
- Gateway 通过 SecretResolver 临时解析 secret，不把解析结果回写 Config。
- Extension gateway contribution 先经过 Extension lifecycle 校验和注册，不能绕过 lifecycle 直接写入 Gateway registry。

Config 不应把运行时 Registry snapshot 当作持久化事实源。Registry 是运行时目录，Config 是用户配置事实源，两者通过明确的 application composition 同步。

---

## 三、数据模型

配置 schema 使用稳定的 snake_case Rust 字段和明确的 JSON 边界命名。Gateway Provider 配置至少包含：

- id、provider_type、name、base_url。
- secret_ref，可为空，但不允许出现 api_key。
- models 和 default_model。

Model 配置至少包含：

- id、name、api_protocol。
- context_window、max_output_tokens。
- supports_tools、supports_streaming、supports_multimodal、supports_reasoning_effort。

Extension manifest 使用独立的 camelCase schema：contributes.gateway.adapters、contributes.gateway.providers、adapterId、baseUrl、secretRef、defaultModel、contextWindow、maxOutputTokens。Config 不复制 Extension manifest 的解析逻辑，Extension domain 负责 manifest schema 和 lifecycle plan。

敏感字段规则：

- secret_ref 可以出现在持久化配置和安全 projection 中。
- secret 内容只能存在于 Auth Store 的受保护边界和请求执行的短生命周期内。
- 配置日志、错误、事件和导出文件不得包含 secret 内容。
- 未知字段 fail-closed；不能通过双读双写保留旧 api_key 兼容路径。

---

## 四、模块边界

```text
Config Loader -> Schema Validator -> Layer Merger -> Config Store
                                      │
                                      ├─ Config change event
                                      └─ Application composition
                                           ├─ Gateway::init / update_config
                                           ├─ Auth secret reference resolution
                                           └─ UI projection
```

Config Store 可以依赖 foundation/storage，但业务模块不得直接访问配置文件路径或 SQLite 表。Config 对外提供 typed DTO 或只读 projection，避免各模块重复解析 JSON。

---

## 五、变更流程

1. UI 或应用服务提交 typed config DTO。
2. Config 校验字段、范围、引用关系和未知字段。
3. Config 写入用户或项目层，并生成版本/变更事件。
4. Application layer 根据变更类型更新 Gateway、Extension、Editor 等运行时服务。
5. 运行时更新失败时保留旧的有效运行时状态，并向调用方返回真实错误；不得保存半有效配置。

Gateway 变更至少校验：Provider ID 唯一、base_url 合法、Model ID 唯一、default_model 存在、api_protocol 已能被 Registry resolve。若配置引用 Extension Adapter，必须引用已启用且可用的 runtime protocol。

---

## 六、IPC 与 projection

所有前端配置读写通过 Tauri invoke 进入 ui/settings 或 ui/gateway 命令。UI 只消费 DTO，不直接读取 Config Store。

Gateway 配置 projection 必须脱敏：返回 provider、model、protocol、capability、secret 是否已配置等信息，不返回 secret 内容。协议选项来自 Gateway catalog projection，不在前端固定枚举。

---

## 七、热更新与回滚

支持热更新的配置必须声明 hot_reload 语义。不能热更新的配置采用 application-level transaction：先构建新 plan，再验证所有依赖，最后替换运行时引用。

Extension Gateway contribution 的启用、禁用和回滚由 Extension lifecycle transaction 负责；Config 不直接调用 Gateway registry。Gateway capability port、MCP capability port、LSP capability port 是各宿主域的隔离边界。

---

## 八、测试与验收

必须覆盖：

- 分层合并和覆盖顺序。
- 未知字段和非法值拒绝。
- Gateway provider/model schema 与 default model 引用。
- secret_ref 持久化但 secret 不落盘。
- Config 导出脱敏。
- Gateway catalog 与 Config 快照一致。
- Extension contribution 不绕过 lifecycle。
- 热更新失败保持旧有效状态。
- 不存在 api_key 双读双写兼容逻辑。

完成标准：Config 是配置事实源，Auth 是 secret 事实源，Gateway Registry 是运行时能力目录，Extension lifecycle 是 Extension contribution 的生命周期事实源，任何模块不越界读取其他模块内部存储。
