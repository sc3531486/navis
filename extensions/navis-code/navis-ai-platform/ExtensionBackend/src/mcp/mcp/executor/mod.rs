//! 工具执行器
//!
//! 基于设计文档 §5，负责工具调用的执行阶段。
//!
//! 调用链路：
//! - 内置工具 → 直接调用 Rust 函数
//! - 第三方工具 → JSON-RPC 调用对应 Server
//!
//! 执行前进行 Sandbox 安全校验，执行后记录 Kernel 事件和审计。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

