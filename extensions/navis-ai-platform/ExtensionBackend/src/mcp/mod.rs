//! MCP 引擎 — Model Context Protocol 服务器管理、路由、传输
//! 迁移过渡期：re-export src-tauri/src/tool/mcp

pub mod builtin;
pub mod tools;
pub mod transport;

pub use crate::tool::mcp::executor;
pub use crate::tool::mcp::protocol;
pub use crate::tool::mcp::registry;
pub use crate::tool::mcp::router;
pub use crate::tool::mcp::server_manager;
