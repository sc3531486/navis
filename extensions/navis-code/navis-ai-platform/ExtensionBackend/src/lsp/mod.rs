//! LSP 服务扩展实现适配层。

#[path = "lsp/mod.rs"]
mod implementation;

pub use implementation::*;
