//! Agent 工具投影层（projection，非事实源）
//!
//! 本模块是 MCP Registry 的领域投影层，维护模型可见工具名与 MCP 规范工具名
//! 之间的稳定映射。MCP Registry 是唯一事实源；本模块只把 `ToolDefinition`
//! 投影为 Gateway 可消费的 `ToolProjectionEntry`。
//!
//! Gateway adapter 只接收 provider-safe 名称；tool/agent 运行时在执行 MCP 前
//! 通过本投影解析名称。本模块不持有独立 HashMap，不维护状态。

