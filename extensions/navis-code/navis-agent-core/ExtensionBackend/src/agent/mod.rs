//! Agent 扩展实现适配层。
//! 实际模块位于当前扩展目录的 agent/ 子目录，使用显式路径避免依赖宿主 crate。

#[path = "agent/mod.rs"]
mod implementation;

pub use implementation::*;
