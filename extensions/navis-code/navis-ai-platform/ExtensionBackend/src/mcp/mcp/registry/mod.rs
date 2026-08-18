//! MCP 工具能力目录（Kernel-backed facade）
//!
//! 本模块是 MCP 工具注册层的 facade；能力登记、发现和生命周期
//! 委托给 `kernel::InMemoryRegistry`，MCP 层不维护平行 HashMap 注册表。
//!
//! MCP 层只保留工具定义、内置工具实例和查询 DTO；能力注册、发现和生命周期
//! 收敛到 `kernel::InMemoryRegistry`。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

