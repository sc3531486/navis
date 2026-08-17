//! MCP Server 管理
//!
//! 基于设计文档 §4.1，管理 MCP Server 的完整生命周期：
//! - 添加/移除 Server
//! - 启动/停止 Server
//! - 健康检查
//! - 工具发现
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

