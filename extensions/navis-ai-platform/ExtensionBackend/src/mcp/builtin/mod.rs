//! MCP 内置服务器 — 文件系统、终端、Git、剪贴板、记忆、Web 等
//! 迁移过渡期：re-export src-tauri/src/tool/mcp/builtin

pub use crate::tool::mcp::builtin::clipboard;
pub use crate::tool::mcp::builtin::filesystem;
pub use crate::tool::mcp::builtin::git;
pub use crate::tool::mcp::builtin::lsp;
pub use crate::tool::mcp::builtin::memory;
pub use crate::tool::mcp::builtin::resource;
pub use crate::tool::mcp::builtin::terminal;
pub use crate::tool::mcp::builtin::web;
