//! LSP 客户端 — 语言服务器协议管理、诊断、同步
//! 迁移过渡期：re-export src-tauri/src/tool/lsp

pub use crate::tool::lsp::capabilities;
pub use crate::tool::lsp::client;
pub use crate::tool::lsp::diagnostics;
pub use crate::tool::lsp::event_helpers;
pub use crate::tool::lsp::indexer;
pub use crate::tool::lsp::manager;
pub use crate::tool::lsp::registry;
pub use crate::tool::lsp::sync;
