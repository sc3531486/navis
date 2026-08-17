//! navis-project 扩展后端 — 项目目录管理业务域
//!
//! 项目发现、配置解析、知识路径、最近 Worktree。
//! 实际实现位于 `src-tauri/src/project/catalog/`，
//! 此处为扩展注册入口，建立命名空间归属。

// Re-export: 实际代码仍位于 src-tauri/src/project/catalog/
pub use crate::project::catalog::*;
