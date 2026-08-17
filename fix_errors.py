import os

base = r"D:\myworkspace\Navis Go\src-tauri\src\domains"

# 1. 修复 session/store/store_models.rs - 添加 SessionStore
store_models_path = os.path.join(base, "session", "session", "store", "store_models", "mod.rs")
with open(store_models_path, "r", encoding="utf-8") as f:
    content = f.read()

# 在末尾添加 SessionStore
if "pub struct SessionStore" not in content:
    content += """

pub struct SessionStore {
    connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl SessionStore {
    pub fn new(connection: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        Self { connection }
    }
}
"""
    with open(store_models_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(content)
    print("Added SessionStore to store_models")

# 2. 修复 protocol/capability - GatewayCapabilityEvaluatorPort 应该是 trait
cap_path = os.path.join(base, "ai_platform", "gateway", "protocol", "capability", "mod.rs")
with open(cap_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 协议能力集

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityClipDiagnostic { pub version: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilityEvaluationInput { }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet { }

pub trait GatewayCapabilityEvaluatorPort: Send + Sync {
    fn evaluate(&self, _input: &CapabilityEvaluationInput) -> CapabilitySet { CapabilitySet::default() }
}

pub struct GatewayCapabilityPolicies;
pub struct GatewayCapabilityProjection;
pub struct IntersectionCapabilityEvaluator;
pub struct ModelIdentity;
pub struct ProviderIdentity;
pub const GATEWAY_CAPABILITY_PROJECTION_VERSION: &str = "1.0";
""")
print("Fixed protocol/capability")

# 3. 修复 chat_completions - ProviderAdapter 应该是 trait
cc_path = os.path.join(base, "ai_platform", "gateway", "protocol", "chat_completions", "mod.rs")
with open(cc_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! Chat Completions 协议适配器
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolAdapterInfo { pub id: String, pub name: String }

pub struct ProtocolAdapterRegistry;

pub trait ProviderAdapter: Send + Sync {
    fn name(&self) -> &str;
}

pub struct StreamFrame;
pub struct StreamFrameDecoder;
""")
print("Fixed protocol/chat_completions")

# 4. 修复 AgentToolEvent - 应该是 struct
events_path = os.path.join(base, "agent_core", "tool_runtime", "runtime", "events", "mod.rs")
with open(events_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 工具执行事件
pub struct AgentToolEvent;
pub struct AgentToolExecution;
pub enum AgentToolPhase { Pre, Main, Post }
pub struct AgentToolProgressCallback;
pub enum AgentToolStatus { Pending, Running, Done }
""")
print("Fixed events")

# 5. 修复 session/store/mod.rs - 添加 re-export
store_path = os.path.join(base, "session", "session", "store", "mod.rs")
with open(store_path, "w", encoding="utf-8", newline="\n") as f:
    f.write("""//! 会话存储

pub mod store_models;
pub mod store_timeline_status;

pub use store_models::*;
pub use store_timeline_status::*;
pub use store_models::SessionStore;
""")
print("Fixed store/mod.rs")

print("Done")
