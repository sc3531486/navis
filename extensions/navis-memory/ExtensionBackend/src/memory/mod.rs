//! navis-memory 扩展后端 — AI 长期记忆持久化
//!
//! 记忆领域模型（Memory, MemoryCategory, MemoryScope 等）。
//! 实际实现位于 `src-tauri/src/project/memory/`，
//! 此处为扩展注册入口，建立命名空间归属。

// Re-export: 实际代码仍位于 src-tauri/src/project/memory/
pub use crate::project::memory::*;
