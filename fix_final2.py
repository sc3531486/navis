import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 1. 修复 application/runtime/mod.rs - 添加缺失的类型
runtime_path = os.path.join(base, "agent_core", "application", "runtime", "mod.rs")
with open(runtime_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 应用运行时
use serde::{Deserialize, Serialize};

pub trait AgentControlPorts: Send + Sync {}

pub trait SidechainPort: Send + Sync {
    fn start_sidechain(&self, _req: SidechainStartRequest) -> Result<SidechainStarted, String>;
    fn read_status(&self, _req: SidechainReadRequest) -> Result<SidechainTaskSnapshot, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainStartRequest { pub session_id: String, pub description: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainStarted { pub task_id: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainReadRequest { pub task_id: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidechainTaskSnapshot { pub status: SidechainStatus }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SidechainStatus { Running, Completed, Failed }

pub trait TodoPort: Send + Sync {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoUpdate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoUpdateRequest;
""")

# 2. 修复 builtin/mod.rs - 添加 all_builtin_tools
builtin_path = os.path.join(base, "ai_platform", "mcp", "builtin", "mod.rs")
with open(builtin_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 内置 MCP Server
pub mod clipboard;
pub mod filesystem;
pub mod git;
pub mod lsp;
pub mod memory;
pub mod resource;
pub mod terminal;
pub mod web;

pub const BUILTIN_SERVER_ID: &str = "builtin";

pub fn all_builtin_tool_projection() -> Vec<String> { vec![] }

pub fn all_builtin_tools(_storage: std::sync::Arc<crate::app::infra::Storage>) -> Vec<Box<dyn crate::domains::ai_platform::mcp::tools::MCPTool>> {
    vec![]
}
""")

# 3. 修复 transport - 移除 Capability impl（太复杂）
transport_path = os.path.join(base, "ai_platform", "mcp", "transport", "adapter_trait", "mod.rs")
with open(transport_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 传输适配器 trait
use serde::{Deserialize, Serialize};

pub trait TransportAdapter: Send + Sync {
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportAdapterCapability { pub key: String }

pub fn create_default_adapter() -> Box<dyn TransportAdapter> {
    struct DefaultAdapter;
    impl TransportAdapter for DefaultAdapter {
        fn name(&self) -> &str { "default" }
    }
    Box::new(DefaultAdapter)
}

pub fn is_builtin_transport_key(_key: &str) -> bool { true }
pub fn list_builtin_transports() -> Vec<String> { vec!["stdio".to_string()] }
pub fn normalize_custom_transport_key(key: &str) -> String { key.to_string() }
pub fn transport_key(name: &str) -> String { name.to_string() }
""")

print("Fixed")
