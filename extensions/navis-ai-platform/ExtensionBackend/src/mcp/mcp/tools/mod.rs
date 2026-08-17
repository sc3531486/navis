pub trait MCPTool: Send + Sync { fn name(&self) -> &str; fn description(&self) -> &str; }
