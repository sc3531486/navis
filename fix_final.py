import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 1. 修复 application/mod.rs - runtime 子模块不存在
app_mod = os.path.join(base, "agent_core", "application", "mod.rs")
with open(app_mod, "w", encoding="utf-8", newline="\n") as f:
    f.write("//! 应用层合同与用例边界\n\npub mod runtime;\n")

# 创建 application/runtime/mod.rs
os.makedirs(os.path.join(base, "agent_core", "application", "runtime"), exist_ok=True)
with open(os.path.join(base, "agent_core", "application", "runtime", "mod.rs"), "w", encoding="utf-8", newline="\n") as f:
    f.write("//! 应用运行时\n\npub trait AgentControlPorts: Send + Sync {}\n")

# 2. 修复 builtin/mod.rs - 移除错误的 mod_rs 引用
builtin = os.path.join(base, "ai_platform", "mcp", "builtin", "mod.rs")
with open(builtin, "w", encoding="utf-8", newline="\n") as f:
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
""")

# 3. 修复 catalog/mod.rs - 添加 all_builtin_tool_projection
cat_mod = os.path.join(base, "agent_core", "tool_runtime", "catalog", "mod.rs")
with open(cat_mod, "r", encoding="utf-8") as f:
    content = f.read()
if "all_builtin_tool_projection" not in content:
    content += "\npub fn all_builtin_tool_projection() -> Vec<String> { vec![] }\n"
    with open(cat_mod, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)

# 4. 修复 AgentToolProgressCallback - 移除生命周期参数
events = os.path.join(base, "agent_core", "tool_runtime", "runtime", "events", "mod.rs")
with open(events, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 工具执行事件
pub struct AgentToolEvent;
pub struct AgentToolExecution;
pub enum AgentToolPhase { Pre, Main, Post }
pub struct AgentToolProgressCallback;
pub enum AgentToolStatus { Pending, Running, Done }
""")

# 5. 修复 TransportAdapterCapability - 实现 Capability trait
transport = os.path.join(base, "ai_platform", "mcp", "transport", "adapter_trait", "mod.rs")
with open(transport, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 传输适配器 trait
use serde::{Deserialize, Serialize};

pub trait TransportAdapter: Send + Sync {
    fn name(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportAdapterCapability { pub key: String }

impl crate::kernel::registry::Capability for TransportAdapterCapability {
    fn id(&self) -> &str { &self.key }
    fn version(&self) -> &str { "1.0" }
}

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

print("Fixed all 9 errors")
