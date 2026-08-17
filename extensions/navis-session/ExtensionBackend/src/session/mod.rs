//! navis-session 扩展后端 — 会话管理业务域
//!
//! 会话生命周期、历史、快照、断点检查点、Composer 运行时。
//! 实际实现位于 `src-tauri/src/project/session/`，
//! 此处为扩展注册入口，建立命名空间归属。

// Re-export: 实际代码仍位于 src-tauri/src/project/session/
// 迁移完成后此处将持有完整实现
pub use crate::project::session::*;
