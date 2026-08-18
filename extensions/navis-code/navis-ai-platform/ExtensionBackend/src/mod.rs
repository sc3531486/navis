//! navis-ai-platform 扩展后端入口。
//! Gateway、MCP、LSP 均属于本扩展，不进入 Navis 通用宿主。

pub mod gateway;
pub mod lsp;
pub mod mcp;
