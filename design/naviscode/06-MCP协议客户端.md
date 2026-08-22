# 13 - MCP 工具协议引擎详细设计

> 模块编号：13 | 层级：工具能力层
> 依赖：Kernel Registry/Pipeline/EventBus/Policy、05-Auth、02-IPC
> 被依赖：Agent Tool Runtime、Extension lifecycle、UI tool projection

---

## 一、定位

MCP 是外部工具能力的协议层，不等同于 Tool。MCP 负责 server、transport、tools/list、tools/call、错误、生命周期和可靠性；Agent Tool Runtime 负责工具选择、Provider-safe schema 投影、权限和执行回注。

MCP 不负责 Gateway 模型请求。Gateway 的工具字段投影只消费 MCP/Tool Runtime 提供的统一 ToolDefinition，不读取 MCP server 的私有 transport 实现。

---

## 二、职责边界

MCP 负责：

- MCP server 配置、启动、停止和健康状态。
- stdio、SSE、WebSocket、REST、gRPC 等 transport adapter。
- initialize、tools/list、tools/call 和协议错误归一化。
- Tool definition、server identity、transport identity 和运行时 registry。
- 熔断、重试、超时、取消和资源释放。

MCP 不负责：

- Agent 是否选择工具、工具排序和模型提示词。
- Sandbox、Permission、Quota 的最终裁决。
- Gateway 协议转换、Provider 路由或 Model 能力。
- Extension 安装和 Extension 全局生命周期。

---

## 三、运行时结构

```text
Extension manifest
    -> Extension lifecycle
    -> McpCapabilityPort
    -> MCP server/transport registry
    -> Server manager
    -> Transport adapter
    -> MCP session
    -> tools/list / tools/call
    -> Tool registry
    -> Agent Tool Projection
```

MCP server、transport 和 tool 的 ID 必须稳定且可诊断。Extension 资源使用 extension:<extensionId>/... 作用域；禁用或回滚时由 lifecycle 逆序释放。

MCP 复用 Kernel Registry 和 Pipeline 原语，但不把 MCP Server、Transport 或 Tool 业务类型加入 Kernel。

---

## 四、统一数据模型

ToolDefinition 至少包含 name、description、input_schema、server_id、user_visible、declared_risk、effective_risk 和 UI metadata。远端 MCP 使用标准 inputSchema；宿主扩展字段使用 camelCase，不接受同义 snake_case 别名。

Tool 的真实 canonical name 属于 MCP/Tool Registry。发送给模型时由 Tool Projection 生成 provider-safe name，并在启用阶段固定 canonical 到 provider-safe 的映射。接收 tool call 时只允许从本轮注入的映射表反查，不根据字符串猜测。

平台风险覆盖优先于 server/Extension 自声明：terminal、写文件、删除、network 等风险不能被第三方声明降低。Mode 和 Extension policy 只能继续收窄权限。

---

## 五、Extension contribution

Extension 通过 contributes.mcp_servers 声明 server，通过 transport 配置引用已注册的 transport kind。声明只包含经过 schema 校验的数据，不把任意模块路径当作宿主实现直接执行。

McpCapabilityPort 负责：

- 把 manifest declaration 转换为 MCP runtime DTO。
- 校验 server/transport ID、endpoint、权限、资源限制和 secret_ref。
- 注册 server owner、启动和停止资源。
- 在失败时回滚已经注册的 server、tool 和 policy constraint。

Extension lifecycle 不依赖 MCP 具体结构；MCP 具体实现只在 app composition root 注入 capability port。

---

## 六、Transport Adapter

Transport adapter 是 MCP 域内的协议边界，统一提供 connect、send request、receive response、stream、cancel 和 close。新增 transport 应通过 registry 注册，不在 MCP 主流程新增协议分支。

配置驱动 REST/SSE/WebSocket transport 必须：

- endpoint 和 redirect 经过 SSRF/权限校验。
- header 禁止 CRLF；secret 只通过 Auth resolver 获取。
- 响应大小、消息深度、并发数、超时和缓冲区受限。
- unknown framing、非法 JSON、远端 error 和提前断开 fail-closed。
- 重试只处理明确可重试的 transport 错误。

---

## 七、工具调用链路

1. Server manager 建立 session 并完成 initialize。
2. Registry 记录 server/transport 状态。
3. tools/list 结果经过 schema、风险和名称校验后进入 Tool Registry。
4. Agent Tool Runtime 按 Mode、Policy、当前上下文裁剪工具集合。
5. Tool Projection 生成本轮 provider-safe schema。
6. 模型返回 tool call 后，按映射表反查 canonical name。
7. Permission/Sandbox/Policy 校验后执行 MCP tools/call。
8. 结果归一化为 ToolCallResult，并写入 AgentTimelinePart、审计和 UI projection。

MCP 原始 server 响应不能直接进入模型上下文；错误必须保留 server、transport、tool 和可诊断分类，但脱敏 secret。

---

## 八、生命周期与事件

启用：校验 -> 注册 transport/server -> 启动 session -> 发现 tools -> 注册 tool projection -> 提交。

失败：按逆序关闭 session、注销 tool、server、transport 和 policy constraint。

禁用：停止新调用 -> 取消活动调用 -> 关闭 session -> 注销 tool/server/transport -> 更新 projection。

事件使用 Kernel EventBus，Tauri event 只作为前端出口。事件至少覆盖 server registered/started/stopped/failed、tool discovered、call started/completed/failed 和 lifecycle rollback。事件发布失败必须 tracing::warn，不得静默吞掉。

---

## 九、测试与验收

必须覆盖 transport registry、重复 ID、未知 transport、manifest unknown field、SSRF、header 注入、secret 隔离、tools/list schema、risk override、provider-safe name 映射、取消、超时、重试、提前断开、Extension rollback 和禁用后无残留资源。

新增 MCP server 或 transport 时只增加 Extension manifest、Transport Adapter 或宿主 DTO，不修改 Agent 主流程、Gateway 主流程或 Kernel 业务类型。
