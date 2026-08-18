//! MCP 协议定义

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig { pub id: String, pub command: String, pub args: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition { pub name: String, pub description: String, pub input_schema: Value }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest { pub name: String, pub arguments: Value, pub session_id: Option<String> }
impl ToolCallRequest {
    pub fn new(name: &str, arguments: Value) -> Self { Self { name: name.to_string(), arguments, session_id: None } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult { pub call_id: String, pub tool_name: String, pub content: Vec<ToolContent>, pub is_error: bool, pub duration: std::time::Duration }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolContent { Text { text: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum ToolMetadataRiskLevel { #[default] Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRendererHint { Table, Diff, Terminal }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRiskLevel { Low, Medium, High }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolUiHint { Inline, Block }

pub fn platform_risk_override(_name: &str) -> Option<ToolMetadataRiskLevel> { None }
