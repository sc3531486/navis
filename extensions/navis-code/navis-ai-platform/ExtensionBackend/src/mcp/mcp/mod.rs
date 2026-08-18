//! MCP 工具协议引擎

pub mod protocol;
pub mod builtin;
pub mod tools;
pub mod transport;

pub struct MCP;

impl MCP {
    pub fn new() -> Self { Self }
    pub fn add_server(&self, _config: protocol::MCPServerConfig) -> Result<(), String> { Ok(()) }
    pub fn start_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn remove_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn register_tool(&self, _tool: protocol::ToolDefinition) -> Result<(), String> { Ok(()) }
    pub fn unregister_server_tools(&self, _server_id: &str) -> Result<usize, String> { Ok(0) }
    pub fn apply_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str, _override: protocol::ToolDefinitionOverride) -> Result<(), String> { Ok(()) }
    pub fn remove_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str) -> Result<(), String> { Ok(()) }
}
