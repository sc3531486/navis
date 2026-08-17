//! 上下文压缩触发模块
//!
//! 监控上下文长度，在会话过长时通知 Context Manager 进行压缩。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

