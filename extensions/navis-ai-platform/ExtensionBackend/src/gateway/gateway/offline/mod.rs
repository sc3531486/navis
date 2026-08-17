//! 离线降级策略模块
//!
//! 检测网络状态，自动切换到配置的 fallback 模型。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

