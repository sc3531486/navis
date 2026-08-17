//! IPC 模块 - 前后端命令入口
//!
//! 基于设计文档 §2 实现，封装 Tauri IPC 调用，提供统一的命令处理能力
//!
//! # 流式数据说明
//! 流式推送使用 Tauri 2 原生 Channel<T> API（`crate::foundation::stream` 模块）。
//! IPC 不持有流状态；高频输出统一由 `crate::foundation::stream` 管理。
//!
//! # 使用 tracing 记录日志
//! 所有日志使用 tracing 宏（debug!, info!, warn!, error!）

pub mod codec;
pub mod error;
pub mod handler;

pub use codec::{Codec, JsonCodec};
pub use error::IpcError;
pub use handler::{IpcDispatcher, IpcHandler};
