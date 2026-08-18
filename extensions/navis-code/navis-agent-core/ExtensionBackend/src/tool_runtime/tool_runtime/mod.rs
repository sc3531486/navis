pub mod pipeline; pub mod runtime; pub mod special;
pub use pipeline::AgentDefaultAllowConstraint;
pub use runtime::{AgentToolEvent, AgentToolPhase, AgentToolStatus};
pub const NAVIS_TOOL_SEARCH: &str = "tool_search";
pub struct AgentToolProgressCallback;
pub struct AgentExecutionContext;
impl AgentExecutionContext { pub fn new() -> Self { Self } }
pub trait ToolAvailability { fn allows_mcp_tool(&self, _: &str) -> bool; fn allows_file(&self) -> bool; fn allows_git(&self) -> bool; fn allows_lsp(&self) -> bool; fn allows_terminal(&self) -> bool; fn allows_web(&self) -> bool; }
pub fn agent_tool_definitions(_mcp: &(), _mode: &str) -> Vec<()> { vec![] }
pub fn sidechain_agent_tool_definitions(_mcp: &(), _mode: &str) -> Vec<()> { vec![] }
pub fn assistant_tool_message(_: &str, _: &str) -> String { String::new() }
pub fn assistant_tool_message_with_content(_: &str, _: &str) -> String { String::new() }
pub fn effective_gateway_tool_call(_: &str, _: &str) -> Option<String> { None }
pub fn execute_agent_tool_call_async(_: &(), _: &()) -> Result<(), String> { Ok(()) }
pub fn is_supported_gateway_tool(_: &str) -> bool { true }
pub fn is_supported_sidechain_gateway_tool(_: &str) -> bool { true }
pub fn parse_text_tool_call(_: &str) -> Option<()> { None }
pub fn parse_text_tool_calls(_: &str) -> Vec<()> { vec![] }
pub fn tool_call_arguments(_: &()) -> &str { "" }
pub fn tool_call_summary(_: &()) -> String { String::new() }
pub fn tool_started_event(_: &()) {}
pub struct AgentToolExecution;
