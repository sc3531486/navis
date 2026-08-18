//! Quota 计量模块
//!
//! 跟踪每个模型的 Token 使用量，支持每日/每月额度限制。
//! 这里不实现平行权限系统；Gateway 只产出策略输入 DTO，真正的允许/拒绝
//! 可由上层 Kernel Policy constraint 或调用方策略决定。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

