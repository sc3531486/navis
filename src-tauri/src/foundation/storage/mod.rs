//! Storage 能力缝 —— 持久化接口定义
//!
//! 本模块只定义存储能力缝（`StoragePort` trait），不包含任何基础设施实现。
//! 基础设施实现在 `crate::app::infra`，业务域通过 trait 获取持久化能力。

pub mod port;

pub use port::StoragePort;
