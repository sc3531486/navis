//! 存储能力缝 —— 持久化接口定义
//!
//! 业务域实现此 trait 获取持久化能力。
//! 基础设施实现在 `crate::app::infra::storage`。

/// 存储能力缝（地基接口）。业务域实现此 trait 获取持久化能力。
pub trait StoragePort: Send + Sync {
    /// 获取底层数据库连接（基础设施原语）。
    fn connection(&self) -> std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>;
}
