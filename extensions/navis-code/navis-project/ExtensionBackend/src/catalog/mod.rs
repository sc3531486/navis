//! 项目目录扩展实现适配层。

#[path = "catalog/mod.rs"]
mod implementation;

pub use implementation::*;
