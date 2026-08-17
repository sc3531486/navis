import os

types_path = r"D:\myworkspace\Navis Go\src-tauri\src\extension\types.rs"

content = '''//! 框架层通用类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;

// Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig { pub endpoint: String, pub api_key: Option<String> }
impl CustomProtocolConfig {
    pub fn from_manifest(_m: &Value) -> Self { Self { endpoint: String::new(), api_key: None } }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProtocol { ChatCompletions, Responses, Custom(String) }
impl ApiProtocol {
    pub fn as_str(&self) -> &str { match self { Self::ChatCompletions => "chat_completions", Self::Responses => "responses", Self::Custom(s) => s } }
    pub fn from_str(s: &str) -> Self { match s { "responses" => Self::Responses, _ => Self::ChatCompletions } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String, pub name: String, pub api_key: Option<String>,
    pub provider_type: String, pub base_url: String, pub secret_ref: Option<String>,
    pub auth_profile: Option<String>, pub models: Vec<String>, pub default_model: Option<String>,
}
impl ProviderConfig { pub fn validate(&self) -> Result<(), String> { Ok(()) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String, pub name: String, pub provider_id: String,
    pub api_protocol: Option<String>, pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>, pub supports_streaming: bool,
    pub supports_multimodal: bool, pub supports_reasoning_effort: bool,
    pub supports_structured_output: bool, pub supports_tools: bool, pub supports_usage: bool,
}
impl ModelConfig {
    pub fn new(id: &str, name: &str, provider_id: &str) -> Self {
        Self { id: id.into(), name: name.into(), provider_id: provider_id.into(), api_protocol: None, context_window: None, max_output_tokens: None, supports_streaming: true, supports_multimodal: false, supports_reasoning_effort: false, supports_structured_output: false, supports_tools: true, supports_usage: true }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub protocols: Vec<String>, pub models: Vec<String>, pub tools: Vec<String>,
    pub streaming: bool, pub multimodal: bool, pub reasoning: bool,
    pub structured_output: bool, pub usage: bool, pub model_catalog: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityClipDiagnostic { pub version: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum GatewayProviderStatus { #[default] Available, Unavailable }

// MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig { pub id: String, pub command: String, pub args: Vec<String>, pub auto_start: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String, pub description: String, pub input_schema: Value,
    pub declared_risk: ToolRiskLevel, pub effective_risk: ToolRiskLevel, pub user_visible: bool,
}
impl ToolDefinition {
    pub fn new(name: &str, desc: &str, schema: Value) -> Self {
        Self { name: name.into(), description: desc.into(), input_schema: schema, declared_risk: ToolRiskLevel::None, effective_risk: ToolRiskLevel::None, user_visible: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride {
    pub name: String, pub enabled: bool, pub model_name: Option<String>,
    pub user_visible: bool, pub ui_hint: Option<String>, pub description: Option<String>,
    pub renderer_hint: Option<String>, pub declared_risk: Option<ToolRiskLevel>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ToolRiskLevel { #[default] None, Read, Write, Network, Command, Destructive }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRendererHint { Table, Diff, Terminal }
impl ToolRendererHint { pub fn new(s: &str) -> Self { match s { "table" => Self::Table, "diff" => Self::Diff, _ => Self::Terminal } } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolUiHint { Inline, Block }
impl ToolUiHint { pub fn new(s: &str) -> Self { match s { "inline" => Self::Inline, _ => Self::Block } } }

pub fn platform_risk_override(_name: &str) -> Option<ToolRiskLevel> { None }

pub struct SandboxStub;
impl SandboxStub {
    pub fn audit_recorder(&self) -> AuditRecorderStub { AuditRecorderStub }
}
impl AsRef<SandboxStub> for SandboxStub { fn as_ref(&self) -> &Self { self } }
pub struct AuditRecorderStub;

pub struct MCP { sandbox_field: SandboxStub }
impl MCP {
    pub fn new() -> Self { Self { sandbox_field: SandboxStub } }
    pub fn sandbox(&self) -> &SandboxStub { &self.sandbox_field }
    pub fn add_server(&self, _c: MCPServerConfig) -> Result<(), String> { Ok(()) }
    pub fn start_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn remove_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn register_tool(&self, _t: ToolDefinition) -> Result<(), String> { Ok(()) }
    pub fn unregister_server_tools(&self, _id: &str) -> Result<usize, String> { Ok(0) }
    pub fn apply_tool_override(&self, _: &str, _: &str, _: &str, _: ToolDefinitionOverride) -> Result<(), String> { Ok(()) }
    pub fn remove_tool_override(&self, _: &str, _: &str, _: &str) -> Result<(), String> { Ok(()) }
}

// LSP
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LanguageSource { Builtin, Extension, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPServerConfig { pub language_id: String, pub command: String, pub args: Vec<String> }

pub struct LspManager;
impl LspManager {
    pub fn new(_eb: std::sync::Arc<dyn crate::kernel::EventBus>) -> Result<Self, String> { Ok(Self) }
}

// Editor
pub struct BackendProcessManager;

pub struct PathManager;
impl PathManager {
    pub fn normalize(p: &str) -> String { p.to_string() }
    pub fn resolve(base: &str, path: &std::path::Path) -> String { format!("{}/{}", base, path.display()) }
}

// Session
pub struct SessionStore;

pub struct MemoryStore;
impl MemoryStore {
    pub fn new() -> Self { Self }
    pub fn save(&self, _m: &()) -> Result<(), String> { Ok(()) }
    pub fn search(&self, _q: &str) -> Result<Vec<()>, String> { Ok(vec![]) }
}

// Tool
pub const NAVIS_TOOL_SEARCH: &str = "tool_search";

// UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAgentTimelinePart {
    pub id: String, pub session_id: String, pub turn_id: String,
    pub message_id: String, pub sequence: i64, pub kind: String, pub data: Value,
}
'''

with open(types_path, "w", encoding="utf-8", newline="\n") as f:
    f.write(content)
print("Written types.rs")
