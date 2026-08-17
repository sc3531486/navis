//! 框架层通用类型定义
//!
//! 这些类型替代了原来的业务特定类型，使框架层不依赖具体业务实现

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ══════════════════════════════════════════════════════════
// Gateway 相关类型
// ══════════════════════════════════════════════════════════

/// 协议适配器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
}

/// API 协议类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiProtocol {
    ChatCompletions,
    Responses,
    Custom(String),
}

/// Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_key: Option<String>,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider_id: String,
}

/// 能力集
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub protocols: Vec<String>,
    pub models: Vec<String>,
}

// ══════════════════════════════════════════════════════════
// MCP 相关类型
// ══════════════════════════════════════════════════════════

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// 工具定义覆盖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinitionOverride {
    pub name: String,
    pub enabled: bool,
}

/// 工具风险级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolRiskLevel {
    Low,
    Medium,
    High,
}

/// 平台风险覆盖
pub fn platform_risk_override(_name: &str) -> Option<ToolRiskLevel> {
    None
}

// ══════════════════════════════════════════════════════════
// LSP 相关类型
// ══════════════════════════════════════════════════════════

/// 语言来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LanguageSource {
    Builtin,
    Extension,
    Custom(String),
}

/// LSP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSPServerConfig {
    pub language_id: String,
    pub command: String,
    pub args: Vec<String>,
}

// ══════════════════════════════════════════════════════════
// Editor 相关类型
// ══════════════════════════════════════════════════════════

/// 后端进程管理器（占位）
pub struct BackendProcessManager;

// ══════════════════════════════════════════════════════════
// Session 相关类型
// ══════════════════════════════════════════════════════════

/// 会话存储（占位）
pub struct SessionStore;
