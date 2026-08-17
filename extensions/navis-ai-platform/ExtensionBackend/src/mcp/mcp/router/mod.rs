//! 工具路由
//!
//! 基于设计文档 §5，根据工具名路由到对应 Server，完成工具调用链路的查找阶段。
//!
//! 调用链路：Agent 决策 → Router（查找 MCP 工具能力目录）→ Sandbox 校验 → Executor
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

