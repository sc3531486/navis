//! 会话状态机
//!
//! 定义会话状态转换规则：
//! - 创建 -> Active
//! - Active -> Archived（归档）
//! - Archived -> Active（恢复）
//! - Active/Archived -> Deleted（软删除）

