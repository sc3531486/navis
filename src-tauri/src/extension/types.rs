//! 框架层通用类型定义
//!
//! 这些类型替代了原来的业务特定类型，使框架层不依赖具体业务实现

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ══════════════════════════════════════════════════════════
// Gateway 相关类型
// ══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl CustomProtocolConfig {
    pub fn from_manifest(_manifest: &Value) -> Self {
        Self { endpoint: String::new(), api_key: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProtocol {
    ChatCompletions,
    Responses,
    Custom(String),
}

impl ApiProtocol {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
            Self::Custom(s) => s,
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "responses" => Self::Responses,
            "chat_completions" => Self::ChatCompletions,
            _ => Self::Custom(s.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
    pub provider_type: String,
    pub base_url: String,
    pub secret_ref: Option<String>,
    pub auth_profile: Option<String>,
    pub models: Vec<String>,
    pub default_model: Option<String>,
}

impl ProviderConfig {
    pub fn validate(&self) -> Result<(), String> { Ok(()) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider_id: String,
}

impl ModelConfig {
    pub fn new(id: &str, name: &str, provider_id: &str) -> Self {
        Self { id: id.to_string(), name: name.to_string(), provider_id: provider_id.to_string() }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub protocols: Vec<String>,
    pub models: Vec<String>,
    pub tools: Vec<String>,
    pub streaming: bool,
    pub multimodal: bool,
    pub reasoning: bool,
    pub structured_output: bool,
    pub usage: bool,
    pub model_catalog: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityClipDiagnostic {
    pub version: String,
}

pub struct GatewayProviderStatus;

// ══════════════════════════════════════════════════════════
// MCP 相关类型
// ══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self { name: name.to_string(), description: description.to_string(), input_schema }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride {
    pub name: String,
    pub enabled: bool,
    pub model_name: Option<String>,
    pub user_visible: bool,
    pub ui_hint: Option<String>,
    pub description: Option<String>,
    pub renderer_hint: Option<String>,
    pub declared_risk: Option<ToolRiskLevel>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ToolRiskLevel {
    #[default]
    None,
    Read,
    Write,
    Network,
    Command,
    Destructive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRendererHint {
    Table,
    Diff,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolUiHint {
    Inline,
    Block,
}

pub fn platform_risk_override(_name: &str) -> Option<ToolRiskLevel> { None }

// ══════════════════════════════════════════════════════════
// MCP 引擎
// ══════════════════════════════════════════════════════════

pub struct MCP;

impl MCP {
    pub fn new() -> Self { Self }
    pub fn add_server(&self, _config: MCPServerConfig) -> Result<(), String> { Ok(()) }
    pub fn start_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn remove_server(&self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn register_tool(&self, _tool: ToolDefinition) -> Result<(), String> { Ok(()) }
    pub fn unregister_server_tools(&self, _server_id: &str) -> Result<usize, String> { Ok(0) }
    pub fn apply_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str, _override: ToolDefinitionOverride) -> Result<(), String> { Ok(()) }
    pub fn remove_tool_override(&self, _owner: &str, _server_id: &str, _tool_name: &str) -> Result<(), String> { Ok(()) }
}

// ══════════════════════════════════════════════════════════
// LSP 相关类型
// ══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LanguageSource {
    Builtin,
    Extension,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPServerConfig {
    pub language_id: String,
    pub command: String,
    pub args: Vec<String>,
}

pub struct LspManager;
impl LspManager {
    pub fn new(_event_bus: std::sync::Arc<dyn crate::kernel::EventBus>) -> Result<Self, String> { Ok(Self) }
}

// ══════════════════════════════════════════════════════════
// Editor 相关类型
// ══════════════════════════════════════════════════════════

pub struct BackendProcessManager;

pub struct PathManager;
impl PathManager {
    pub fn normalize(path: &str) -> String { path.to_string() }
    pub fn resolve(base: &str, path: &std::path::Path) -> String { format!("{}/{}", base, path.display()) }
}

// ══════════════════════════════════════════════════════════
// Session 相关类型
// ══════════════════════════════════════════════════════════

pub struct SessionStore;
pub struct MemoryStore;

// ══════════════════════════════════════════════════════════
// Tool 相关类型
// ══════════════════════════════════════════════════════════

pub const NAVIS_TOOL_SEARCH: &str = "tool_search";

// ══════════════════════════════════════════════════════════
// UI 相关类型
// ══════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiAgentTimelinePart {
    pub id: String,
    pub session_id: String,
    pub turn_id: String,
    pub message_id: String,
    pub sequence: i64,
    pub kind: String,
    pub data: Value,
}
